using System.Text;
using System.Text.Json;
using SoulsFormats;

if (args.Length == 0)
{
    Console.Error.WriteLine("Usage:");
    Console.Error.WriteLine("  GetGoodsIconIds lookup <game_dir> <item_id> [...]");
    Console.Error.WriteLine("  GetGoodsIconIds export <game_dir> <icon_id> <output.png>");
    Console.Error.WriteLine("  GetGoodsIconIds convert <regulation_vanilla> <bosses_base.json> <bosses_dlc.json> <out checks.toml>");
    return 1;
}

return args[0] switch
{
    "lookup" => LookupIconIds(args.Skip(1).ToArray()),
    "export" => ExportIcon(args.Skip(1).ToArray()),
    "probe" => ProbeLayout(args.Skip(1).ToArray()),
    "convert" => ConvertChecklist(args.Skip(1).ToArray()),
    _ => LookupIconIds(args),
};

static int LookupIconIds(string[] args)
{
    if (args.Length < 2)
    {
        Console.Error.WriteLine("Usage: lookup <game_dir> <item_id> [...]");
        return 1;
    }

    var gameDir = args[0];
    EnsureOodleDll(gameDir);
    var regPath = Path.Combine(gameDir, "regulation.bin");
    if (!File.Exists(regPath))
    {
        Console.Error.WriteLine($"Missing regulation.bin: {regPath}");
        return 1;
    }

    var defPath = SmithboxPaths.EquipParamGoodsDef;
    if (!File.Exists(defPath))
    {
        Console.Error.WriteLine($"Missing paramdef: {defPath}");
        return 1;
    }

    var regBytes = File.ReadAllBytes(regPath);
    using var bnd = SFUtil.DecryptERRegulation(regBytes);

    PARAM? goodsParam = null;
    foreach (var file in bnd.Files)
    {
        if (!file.Name.EndsWith("EquipParamGoods.param", StringComparison.OrdinalIgnoreCase))
            continue;
        goodsParam = PARAM.ReadIgnoreCompression(file.Bytes);
        break;
    }

    if (goodsParam is null)
    {
        Console.Error.WriteLine("EquipParamGoods not found in regulation.bin");
        return 1;
    }

    EnsureOodleDll(gameDir);
    var paramdef = PARAMDEF.XmlDeserialize(defPath, true);
    goodsParam.ApplyParamdef(paramdef);

    foreach (var idStr in args.Skip(1))
    {
        if (!int.TryParse(idStr, out var id))
        {
            Console.Error.WriteLine($"Invalid item id: {idStr}");
            continue;
        }

        var row = goodsParam.Rows.FirstOrDefault(r => r.ID == id);
        if (row is null)
        {
            Console.Error.WriteLine($"{id},missing");
            continue;
        }

        var iconId = (ushort)row["iconId"].Value;
        Console.WriteLine($"{id},{iconId}");
    }

    return 0;
}

static int ExportIcon(string[] args)
{
    if (args.Length < 3)
    {
        Console.Error.WriteLine("Usage: export <game_dir> <icon_id> <output.png>");
        return 1;
    }

    var gameDir = args[0];
    if (!int.TryParse(args[1], out var iconId))
    {
        Console.Error.WriteLine($"Invalid icon id: {args[1]}");
        return 1;
    }

    var output = args[2];
    EnsureOodleDll(gameDir);
    return GoodsIconExporter.Export(gameDir, iconId, output) ? 0 : 1;
}

static int ProbeLayout(string[] args)
{
    if (args.Length < 1)
    {
        Console.Error.WriteLine("Usage: probe <game_dir> [needle]");
        return 1;
    }

    var gameDir = args[0];
    var needle = args.Length > 1 ? args[1] : "3050";
    EnsureOodleDll(gameDir);

    foreach (var tier in new[] { "hi", "low" })
    {
        var sblyPath = Path.Combine(gameDir, "menu", tier, "01_common.sblytbnd.dcx");
        if (!File.Exists(sblyPath))
            continue;

        var bytes = DCX.Decompress(File.ReadAllBytes(sblyPath));
        var bnd = BND4.Read(bytes);
        foreach (var file in bnd.Files)
        {
            if (!file.Name.EndsWith(".layout", StringComparison.OrdinalIgnoreCase))
                continue;

            var xml = new System.Xml.XmlDocument();
            xml.Load(new MemoryStream(file.Bytes.ToArray()));
            foreach (System.Xml.XmlNode atlasNode in xml.ChildNodes)
            {
                foreach (System.Xml.XmlNode sub in atlasNode.ChildNodes)
                {
                    var name = sub.Attributes?["name"]?.Value ?? "";
                    if (name.Contains(needle, StringComparison.OrdinalIgnoreCase))
                        Console.WriteLine($"{tier} :: {file.Name} :: {name}");
                }
            }
        }
    }

    return 0;
}

static int ConvertChecklist(string[] args)
{
    if (args.Length < 4)
    {
        Console.Error.WriteLine("Usage: convert <regulation_vanilla> <bosses_base.json> <bosses_dlc.json> <out checks.toml>");
        return 1;
    }

    var regPath = args[0];
    var baseJsonPath = args[1];
    var dlcJsonPath = args[2];
    var outPath = args[3];

    if (!File.Exists(regPath))
    {
        Console.Error.WriteLine($"Missing regulation.bin: {regPath}");
        return 1;
    }

    var defPath = SmithboxPaths.ItemLotParamDef;
    if (!File.Exists(defPath))
    {
        Console.Error.WriteLine($"Missing paramdef: {defPath}");
        return 1;
    }

    var gameDir = Path.GetDirectoryName(Path.GetFullPath(regPath)) ?? ".";
    EnsureOodleDll(gameDir);

    using var bnd = SFUtil.DecryptERRegulation(File.ReadAllBytes(regPath));

    PARAM? mapParam = null;
    foreach (var file in bnd.Files)
    {
        if (file.Name.EndsWith("ItemLotParam_map.param", StringComparison.OrdinalIgnoreCase))
            mapParam = PARAM.ReadIgnoreCompression(file.Bytes);
    }

    if (mapParam is null)
    {
        Console.Error.WriteLine("ItemLotParam_map not found in regulation.bin");
        return 1;
    }

    var def = PARAMDEF.XmlDeserialize(defPath, true);
    mapParam.ApplyParamdef(def);

    var mapFlags = BuildFlagMap(mapParam);

    var regions = new List<RegionData>();
    regions.AddRange(ParseChecklist(baseJsonPath, dlc: false));
    regions.AddRange(ParseChecklist(dlcJsonPath, dlc: true));

    var sb = new StringBuilder();
    sb.AppendLine("# Generated by GetGoodsIconIds convert. Do not edit by hand.");
    sb.AppendLine("# Source: ER_boss_checklist_R engus/bosses_base.json + bosses_dlc.json");
    sb.AppendLine();
    sb.AppendLine($"region_display_order = [{string.Join(", ", regions.Select(r => Quote(r.Name)))}]");
    sb.AppendLine();

    int total = 0, resolved = 0, unresolved = 0;
    foreach (var region in regions)
    {
        foreach (var c in region.Checks)
        {
            total++;
            sb.AppendLine("[[check]]");
            sb.AppendLine($"region = {TripleQuote(region.Name)}");
            sb.AppendLine($"name = {TripleQuote(c.Name)}");
            if (!string.IsNullOrEmpty(c.Place))
                sb.AppendLine($"place = {TripleQuote(c.Place)}");
            if (region.Dlc)
                sb.AppendLine("dlc = true");
            if (c.Scaling is int sc)
                sb.AppendLine($"scaling = {sc}");
            if (c.Rememberance is int rm)
                sb.AppendLine($"rememberance = {rm}");

            if (c.Randomized)
            {
                sb.AppendLine("dynamic = true");
                sb.AppendLine($"vanilla_flag = {c.FlagId}");
                if (mapFlags.TryGetValue((uint)c.FlagId, out var mrow))
                {
                    sb.AppendLine($"lot_id = {mrow}");
                    resolved++;
                }
                else
                {
                    sb.AppendLine("# UNRESOLVED: no ItemLotParam_map row with this getItemFlagId; falls back to vanilla_flag");
                    Console.Error.WriteLine($"warn: unresolved randomized check '{c.Name}' (flag {c.FlagId}) - no map lot found");
                    unresolved++;
                }
            }
            else
            {
                sb.AppendLine("dynamic = false");
                sb.AppendLine($"flag = {c.FlagId}");
            }

            sb.AppendLine();
        }
    }

    foreach (var region in regions)
    {
        sb.AppendLine("[[region]]");
        sb.AppendLine($"name = {TripleQuote(region.Name)}");
        sb.AppendLine($"subregions = [{string.Join(", ", region.Subregions)}]");
        sb.AppendLine();
    }

    File.WriteAllText(outPath, sb.ToString());
    Console.WriteLine($"Wrote {outPath}: {total} checks ({resolved} randomized resolved, {unresolved} unresolved).");
    return unresolved > 0 ? 2 : 0;
}

static Dictionary<uint, int> BuildFlagMap(PARAM p)
{
    var d = new Dictionary<uint, int>();
    foreach (var row in p.Rows)
    {
        var f = Convert.ToUInt32(row["getItemFlagId"].Value);
        if (f == 0)
            continue;
        if (!d.ContainsKey(f))
            d[f] = row.ID;
    }
    return d;
}

static List<RegionData> ParseChecklist(string path, bool dlc)
{
    var list = new List<RegionData>();
    using var doc = JsonDocument.Parse(File.ReadAllText(path));
    foreach (var regionEl in doc.RootElement.EnumerateArray())
    {
        var name = regionEl.GetProperty("region_name").GetString() ?? "";
        var subs = new List<int>();
        if (regionEl.TryGetProperty("regions", out var regsEl))
            foreach (var s in regsEl.EnumerateArray())
                subs.Add(s.GetInt32());

        var regionDlc = dlc || (regionEl.TryGetProperty("dlc", out var dlcEl) && dlcEl.GetInt32() == 1);

        var checks = new List<CheckData>();
        if (regionEl.TryGetProperty("bosses", out var bossesEl))
        {
            foreach (var b in bossesEl.EnumerateArray())
            {
                var cname = b.GetProperty("boss").GetString() ?? "";
                var place = b.TryGetProperty("place", out var pe) ? pe.GetString() ?? "" : "";
                var flag = b.TryGetProperty("flag_id", out var fe) ? fe.GetInt64() : 0L;
                var rnd = b.TryGetProperty("randomized", out var re) && re.GetBoolean();
                int? scaling = b.TryGetProperty("scaling", out var se) ? se.GetInt32() : null;
                int? rem = b.TryGetProperty("rememberance", out var rme) ? rme.GetInt32() : null;
                checks.Add(new CheckData(cname, place, flag, rnd, scaling, rem));
            }
        }

        list.Add(new RegionData(name, subs, regionDlc, checks));
    }
    return list;
}

static string Quote(string s) => "\"" + s.Replace("\\", "\\\\").Replace("\"", "\\\"") + "\"";

static string TripleQuote(string s) => "\"\"\"" + s + "\"\"\"";

static void EnsureOodleDll(string gameDir)
{
    var exeDir = AppContext.BaseDirectory;
    if (Directory.GetFiles(exeDir, "oo2core_*.dll").Length > 0)
        return;

    var src = Directory.GetFiles(gameDir, "oo2core_*.dll").FirstOrDefault();
    if (src is null)
        return;

    File.Copy(src, Path.Combine(exeDir, Path.GetFileName(src)), overwrite: true);
}

static class SmithboxPaths
{
    private static string DefsDir => Path.GetFullPath(Path.Combine(
        AppContext.BaseDirectory,
        "..", "..", "..", "..", "..", "..", "..",
        "Modding", "Smithbox", "code", "Smithbox",
        "src", "Smithbox.Data", "Assets", "PARAM", "ER", "Defs"));

    public static string EquipParamGoodsDef => Path.Combine(DefsDir, "EquipParamGoods.xml");

    // ItemLotParam_map uses the shared ItemLotParam paramdef.
    public static string ItemLotParamDef => Path.Combine(DefsDir, "ItemLotParam.xml");
}

internal record RegionData(string Name, List<int> Subregions, bool Dlc, List<CheckData> Checks);

internal record CheckData(string Name, string Place, long FlagId, bool Randomized, int? Scaling, int? Rememberance);

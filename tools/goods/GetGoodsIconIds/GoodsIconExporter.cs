using System.Text.RegularExpressions;
using System.Xml;
using Pfim;
using SixLabors.ImageSharp;
using SixLabors.ImageSharp.PixelFormats;
using SixLabors.ImageSharp.Processing;
using SoulsFormats;

internal static class GoodsIconExporter
{
    private static readonly string[] IconPrefixes =
    [
        "MENU_ItemIcon_",
        "MENU_Knowledge_",
    ];

    public static bool Export(string gameDir, int iconId, string outputPath)
    {
        foreach (var hiRes in new[] { true, false })
        {
            if (TryExportTier(gameDir, iconId, outputPath, hiRes))
                return true;
        }

        Console.Error.WriteLine($"iconId {iconId}: subtexture not found in sbly");
        return false;
    }

    private static bool TryExportTier(string gameDir, int iconId, string outputPath, bool hiRes)
    {
        var menuTier = hiRes ? "hi" : "low";
        var sblyPath = Path.Combine(gameDir, "menu", menuTier, "01_common.sblytbnd.dcx");
        var tpfPath = Path.Combine(gameDir, "menu", menuTier, "01_common.tpf.dcx");

        if (!File.Exists(sblyPath) || !File.Exists(tpfPath))
            return false;

        SubTextureRect? rect = null;
        string? atlasImage = null;

        foreach (var prefix in IconPrefixes)
        {
            if (TryFindSubTexture(sblyPath, prefix, iconId, out rect, out atlasImage))
                break;
            rect = null;
            atlasImage = null;
        }

        if (rect is null || atlasImage is null)
            return false;

        var tpfBytes = DCX.Decompress(File.ReadAllBytes(tpfPath));
        var tpf = TPF.Read(tpfBytes);
        var atlasName = Path.GetFileNameWithoutExtension(atlasImage);
        var texture = tpf.Textures.FirstOrDefault(t =>
            string.Equals(t.Name, atlasName, StringComparison.OrdinalIgnoreCase));

        if (texture is null)
        {
            Console.Error.WriteLine($"iconId {iconId}: texture '{atlasName}' not found in tpf ({menuTier})");
            return false;
        }

        using var atlas = DecodeDds(texture.Bytes.ToArray());
        var r = rect.Value;
        using var cropped = atlas.Clone(ctx => ctx.Crop(new Rectangle(r.X, r.Y, r.W, r.H)));
        Directory.CreateDirectory(Path.GetDirectoryName(outputPath)!);
        cropped.SaveAsPng(outputPath);
        return true;
    }

    private static bool TryFindSubTexture(
        string sblyPath,
        string prefix,
        int iconId,
        out SubTextureRect? rect,
        out string? atlasImage)
    {
        rect = null;
        atlasImage = null;

        var bytes = DCX.Decompress(File.ReadAllBytes(sblyPath));
        var bnd = BND4.Read(bytes);

        foreach (var file in bnd.Files)
        {
            if (!file.Name.EndsWith(".layout", StringComparison.OrdinalIgnoreCase))
                continue;

            var xml = new XmlDocument();
            xml.Load(new MemoryStream(file.Bytes.ToArray()));
            foreach (XmlNode atlasNode in xml.ChildNodes)
            {
                if (atlasNode.Attributes?["imagePath"] == null)
                    continue;

                foreach (XmlNode sub in atlasNode.ChildNodes)
                {
                    var name = sub.Attributes?["name"]?.Value ?? "";
                    var match = Regex.Match(name, $@"{Regex.Escape(prefix)}([0-9]+)");
                    if (!match.Success || int.Parse(match.Groups[1].Value) != iconId)
                        continue;

                    rect = new SubTextureRect(
                        int.Parse(sub.Attributes!["x"]!.Value),
                        int.Parse(sub.Attributes!["y"]!.Value),
                        int.Parse(sub.Attributes!["width"]!.Value),
                        int.Parse(sub.Attributes!["height"]!.Value));
                    atlasImage = atlasNode.Attributes["imagePath"]!.Value;
                    return true;
                }
            }
        }

        return false;
    }

    private static Image<Rgba32> DecodeDds(byte[] ddsBytes)
    {
        var image = Dds.Create(ddsBytes, new PfimConfig());
        if (image.Compressed)
            image.Decompress();

        return image.Format switch
        {
            ImageFormat.Rgba32 => Image.LoadPixelData<Rgba32>(image.Data, image.Width, image.Height),
            ImageFormat.Rgb24 => Image.LoadPixelData<Bgr24>(image.Data, image.Width, image.Height).CloneAs<Rgba32>(),
            _ => throw new NotSupportedException($"Unsupported DDS format: {image.Format}"),
        };
    }

    private readonly record struct SubTextureRect(int X, int Y, int W, int H);
}

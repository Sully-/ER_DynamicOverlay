using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;

namespace DemoDotnet;

[StructLayout(LayoutKind.Sequential)]
public unsafe struct ErMetricDesc
{
    public byte* Id;
    public uint Kind;
    public uint Reserved0;
    public uint Reserved1;
    public uint Reserved2;
}

[StructLayout(LayoutKind.Sequential)]
public unsafe struct ErMetricSample
{
    public long Value;
    public long Max;
    public byte* Text;
    public byte Available;
    public byte HasMax;
    public byte Pad0;
    public byte Pad1;
    public uint Reserved;
}

[StructLayout(LayoutKind.Sequential)]
public unsafe struct ErHostInfo
{
    public uint AbiVersion;
    public uint Pad;
    public byte* OverlayVersion;
    public byte* BaseDir;
    public delegate* unmanaged<uint, byte*, void> Log;
}

internal sealed unsafe class DemoPlugin : IDisposable
{
    public const uint MetricCount = 0;
    public const uint MetricTimeMs = 1;
    public const uint MetricText = 2;
    public const uint LogInfo = 2;

    public ulong TickMs;
    public ErMetricDesc* Descs;
    public byte* TextPtr;
    public delegate* unmanaged<uint, byte*, void> Log;

    private readonly nint[] _idAllocs;

    public DemoPlugin(ErHostInfo* host)
    {
        _idAllocs = new nint[3];
        Descs = (ErMetricDesc*)NativeMemory.Alloc((nuint)(3 * sizeof(ErMetricDesc)));
        NativeMemory.Clear(Descs, (nuint)(3 * sizeof(ErMetricDesc)));

        _idAllocs[0] = AllocUtf8("demo_dotnet.uptime");
        _idAllocs[1] = AllocUtf8("demo_dotnet.progress");
        _idAllocs[2] = AllocUtf8("demo_dotnet.rank");

        Descs[0].Id = (byte*)_idAllocs[0];
        Descs[0].Kind = MetricTimeMs;
        Descs[1].Id = (byte*)_idAllocs[1];
        Descs[1].Kind = MetricCount;
        Descs[2].Id = (byte*)_idAllocs[2];
        Descs[2].Kind = MetricText;

        string overlayVersion = "";
        string baseDir = "";
        if (host != null)
        {
            overlayVersion = PtrToString(host->OverlayVersion);
            baseDir = PtrToString(host->BaseDir);
            Log = host->Log;
        }

        HostLog(LogInfo, $"demo_dotnet: overlay {overlayVersion} @ {baseDir}");
    }

    public long Progress() => (long)((TickMs / 1000) % 61);

    public string Rank()
    {
        long p = Progress();
        if (p <= 14)
        {
            return "C";
        }

        if (p <= 29)
        {
            return "B";
        }

        if (p <= 44)
        {
            return "A";
        }

        return "S";
    }

    public void ClearText()
    {
        if (TextPtr != null)
        {
            NativeMemory.Free(TextPtr);
            TextPtr = null;
        }
    }

    public void SetText(string value)
    {
        ClearText();
        TextPtr = (byte*)AllocUtf8(value);
    }

    public void HostLog(uint level, string message)
    {
        if (Log == null)
        {
            return;
        }

        nint ptr = AllocUtf8(message);
        try
        {
            Log(level, (byte*)ptr);
        }
        finally
        {
            NativeMemory.Free((void*)ptr);
        }
    }

    public void Dispose()
    {
        ClearText();
        if (Descs != null)
        {
            NativeMemory.Free(Descs);
            Descs = null;
        }

        foreach (nint alloc in _idAllocs)
        {
            if (alloc != 0)
            {
                NativeMemory.Free((void*)alloc);
            }
        }
    }

    private static nint AllocUtf8(string value)
    {
        int byteCount = Encoding.UTF8.GetByteCount(value);
        byte* ptr = (byte*)NativeMemory.Alloc((nuint)(byteCount + 1));
        Span<byte> span = new(ptr, byteCount);
        Encoding.UTF8.GetBytes(value, span);
        ptr[byteCount] = 0;
        return (nint)ptr;
    }

    private static string PtrToString(byte* ptr)
    {
        if (ptr == null)
        {
            return "";
        }

        return Marshal.PtrToStringUTF8((nint)ptr) ?? "";
    }
}

public static unsafe class Exports
{
    private static ErMetricSample Unavailable()
    {
        return new ErMetricSample { Available = 0 };
    }

    private static DemoPlugin? FromCtx(void* ctx)
    {
        if (ctx == null)
        {
            return null;
        }

        GCHandle handle = GCHandle.FromIntPtr((nint)ctx);
        return handle.Target as DemoPlugin;
    }

    [UnmanagedCallersOnly(EntryPoint = "er_overlay_plugin_abi_version")]
    public static uint AbiVersion() => 1;

    [UnmanagedCallersOnly(EntryPoint = "er_overlay_plugin_create")]
    public static void* Create(ErHostInfo* host)
    {
        try
        {
            var plugin = new DemoPlugin(host);
            GCHandle handle = GCHandle.Alloc(plugin);
            return (void*)GCHandle.ToIntPtr(handle);
        }
        catch
        {
            return null;
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "er_overlay_plugin_metrics")]
    public static ErMetricDesc* Metrics(void* ctx, nuint* outLen)
    {
        try
        {
            if (outLen == null)
            {
                return null;
            }

            DemoPlugin? plugin = FromCtx(ctx);
            if (plugin == null)
            {
                *outLen = 0;
                return null;
            }

            *outLen = 3;
            return plugin.Descs;
        }
        catch
        {
            if (outLen != null)
            {
                *outLen = 0;
            }

            return null;
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "er_overlay_plugin_poll")]
    public static void Poll(void* ctx, ulong tickMs)
    {
        try
        {
            DemoPlugin? plugin = FromCtx(ctx);
            if (plugin == null)
            {
                return;
            }

            plugin.TickMs = tickMs;
            plugin.ClearText();
        }
        catch
        {
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "er_overlay_plugin_sample")]
    public static ErMetricSample Sample(void* ctx, nuint index)
    {
        try
        {
            DemoPlugin? plugin = FromCtx(ctx);
            if (plugin == null)
            {
                return Unavailable();
            }

            switch (index)
            {
                case 0:
                    return new ErMetricSample
                    {
                        Value = (long)plugin.TickMs,
                        Available = 1,
                    };
                case 1:
                    return new ErMetricSample
                    {
                        Value = plugin.Progress(),
                        Max = 60,
                        Available = 1,
                        HasMax = 1,
                    };
                case 2:
                    plugin.SetText(plugin.Rank());
                    return new ErMetricSample
                    {
                        Text = plugin.TextPtr,
                        Available = 1,
                    };
                default:
                    return Unavailable();
            }
        }
        catch
        {
            return Unavailable();
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "er_overlay_plugin_destroy")]
    public static void Destroy(void* ctx)
    {
        try
        {
            if (ctx == null)
            {
                return;
            }

            GCHandle handle = GCHandle.FromIntPtr((nint)ctx);
            if (handle.Target is DemoPlugin plugin)
            {
                plugin.Dispose();
            }

            handle.Free();
        }
        catch
        {
        }
    }
}

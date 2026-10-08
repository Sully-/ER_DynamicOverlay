# Metric plugins

A metric plugin is a native Windows **x64** DLL. Drop it in `plugins/` next to `er_overlay.dll` and the overlay loads it at startup. The plugin declares metric ids and returns a number (or a short string) every tick. The **layout file** chooses the label, icon, `show_max`, and tile position. The overlay never passes game data to the plugin: the plugin reads whatever it needs on its own.

> **A plugin is arbitrary code inside the Elden Ring process.** A buggy plugin crashes the game. There is no sandbox. Only load plugins you trust.

## Which API should I use?

There is **one** contract: six C exports defined in [`sdk/er_overlay_plugin.h`](../sdk/er_overlay_plugin.h) (ABI version `1`). Everything else is a convenience wrapper around that ABI.

| Language | What you use | Example |
| --- | --- | --- |
| **Rust** | Crate `er_overlay_plugin_sdk`: implement `MetricPlugin`, call `declare_plugin!` | [`examples/plugins/demo_rust`](../examples/plugins/demo_rust) |
| **C / C++** | Include `er_overlay_plugin.h`, implement the six exports | [`examples/plugins/demo_cpp`](../examples/plugins/demo_cpp) |
| **.NET** | NativeAOT (`PublishAot`) + `[UnmanagedCallersOnly]`; copy the struct layouts from the example | [`examples/plugins/demo_dotnet`](../examples/plugins/demo_dotnet) |

Classic managed .NET (loading CoreCLR into the game) is **not** supported. Go `c-shared` is discouraged: it starts a runtime and GC inside the game process.

The three demos expose the same shape of metrics under different id prefixes so you can install them side by side:

| Metric | Kind | Meaning |
| --- | --- | --- |
| `<prefix>.uptime` | `time_ms` | `tick_ms` from the host |
| `<prefix>.progress` | `count` with max `60` | `(tick_ms / 1000) % 61` |
| `<prefix>.rank` | `text` | `C` / `B` / `A` / `S` from progress |

Prefixes: `demo_rust`, `demo_cpp`, `demo_dotnet`.

For the smallest possible plugin (always returns `42`), see [`examples/plugins/answer`](../examples/plugins/answer).

## Integrate a plugin (any language)

1. **Build** a Windows x64 DLL that exports the six symbols (see the language guides below).
2. **Ship a metrics manifest** next to the DLL: `your_plugin.metrics.json` (same stem as the DLL). Same format as below. Keep it in sync with the ids your code declares — the layout editor cannot read DLLs.
3. **Copy** both `your_plugin.dll` and `your_plugin.metrics.json` into the `plugins/` directory next to `er_overlay.dll`. Create the folder if needed.
4. **Enable** plugins in `er_overlay.toml`:

```toml
[plugins]
enabled = true
dir = "plugins"
disabled = []   # e.g. ["demo_rust"] to skip demo_rust.dll without deleting it
```

5. **Layout editor (no game required):** in `layout_editor.html`, use **Import plugin metrics** and select one or more `*.metrics.json` files (the sidecar you just copied, or several at once). The ids appear in the palette. You can edit and export a layout before launching Elden Ring.
6. **Restart the game** when you want the overlay to load the DLL. Metric ids are fixed at startup; a rejected DLL is logged and skipped.
7. **Add a tile** in your layout (or finish the layout in the editor). Bundled layouts use sections:

```toml
[[section.tile]]
kind = "metric"
metric = "demo_rust.progress"
label = "PROGRESS"
col = 0
row = 0
show_max = true
```

For a layout without `[[section]]`, use `[[tile]]` instead (same fields). Hot-reload picks up layout edits without restarting. If the plugin is missing, the tile shows `---` and nothing else breaks.

8. **Check the log.** With `log_enabled = true`, `logs/er_overlay.log` lists each loaded plugin and how many metrics it registered. Failures (wrong ABI, missing export, `create` returned null) appear there too.
9. **Icons** (optional): put a PNG under `plugins/icons/` (or `assets/icons/`). Reference it with `icon = "my_icon"` (no `.png`).

### Metrics manifest (`*.metrics.json`)

Ship this file next to the DLL so authors and the layout editor know the metric list **without** running the game:

```json
{
  "metrics": [
    { "id": "demo_rust.uptime", "kind": "time_ms" },
    { "id": "demo_rust.progress", "kind": "count" },
    { "id": "demo_rust.rank", "kind": "text" }
  ]
}
```

`kind` is `count`, `time_ms`, or `text`. Examples: [`answer.metrics.json`](../examples/plugins/answer/answer.metrics.json), [`demo_rust.metrics.json`](../examples/plugins/demo_rust/demo_rust.metrics.json).

After the overlay loads plugins in-game, it also writes an aggregated `plugins/metrics.json`. That file is optional for editing layouts — the per-plugin sidecar is enough.

## Rust guide

### Dependencies

`Cargo.toml` of a standalone crate (outside the overlay workspace):

```toml
[workspace]

[package]
name = "demo_rust"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
# From a local clone:
er_overlay_plugin_sdk = { path = "path/to/ER_DynamicOverlay/sdk/er_overlay_plugin_sdk" }
# Or from git (tag that contains the SDK):
# er_overlay_plugin_sdk = { git = "https://github.com/Sully-/ER_DynamicOverlay.git", tag = "v1.4.3", package = "er_overlay_plugin_sdk" }

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
```

`panic = "abort"` is required: a panic must not unwind across the C boundary. `crate-type = ["cdylib"]` produces a DLL.

### Implement the plugin

```rust
use er_overlay_plugin_sdk::{
    abi, declare_plugin, HostInfo, MetricKind, MetricPlugin, MetricSample,
};

struct Demo {
    tick_ms: u64,
}

impl Demo {
    fn new() -> Self {
        Self { tick_ms: 0 }
    }
}

impl MetricPlugin for Demo {
    fn metrics() -> &'static [&'static str] {
        &["demo_rust.uptime", "demo_rust.progress", "demo_rust.rank"]
    }

    // Required for time_ms / text: MetricSample::time_ms alone does NOT set the kind.
    fn kind(index: usize) -> MetricKind {
        match index {
            0 => MetricKind::TimeMs,
            1 => MetricKind::Count,
            2 => MetricKind::Text,
            _ => MetricKind::Count,
        }
    }

    fn on_create(&mut self, host: &HostInfo) {
        host.log(
            abi::ER_LOG_INFO,
            &format!("demo_rust: overlay {} @ {}", host.overlay_version, host.base_dir),
        );
    }

    fn poll(&mut self, tick_ms: u64) {
        self.tick_ms = tick_ms;
    }

    fn sample(&mut self, index: usize) -> MetricSample {
        let progress = ((self.tick_ms / 1000) % 61) as i64;
        match index {
            0 => MetricSample::time_ms(self.tick_ms as i64),
            1 => MetricSample::count_with_max(progress, 60),
            2 => MetricSample::text(if progress <= 14 { "C" } else if progress <= 29 { "B" } else if progress <= 44 { "A" } else { "S" }),
            _ => MetricSample::unavailable(),
        }
    }
}

declare_plugin!(Demo, Demo::new());
```

Full sources: [`examples/plugins/demo_rust`](../examples/plugins/demo_rust).

### Build

```bash
cargo build --release
```

DLL: `target/release/demo_rust.dll` (or `target/x86_64-pc-windows-msvc/release/demo_rust.dll` when that target is used).

Ship [`demo_rust.metrics.json`](../examples/plugins/demo_rust/demo_rust.metrics.json) next to the DLL (copy both into `plugins/`).

## C++ guide

### Prerequisites

- MSVC (Visual Studio “Desktop development with C++”) or MinGW-w64.
- CMake 3.20+ (recommended), or a one-liner with `cl`.

### Sources

Include [`sdk/er_overlay_plugin.h`](../sdk/er_overlay_plugin.h). The header defines `ER_OVERLAY_PLUGIN_API` as `__declspec(dllexport)` on Windows, so you do not need a `.def` file.

Rules specific to C++:

- Catch exceptions inside every export; never let them escape.
- Keep metric id string literals (or owned buffers) alive until `destroy`.
- Keep sample `text` alive until the **next** `poll` (the demo stores it in a `std::string` member cleared at the start of `poll`).
- Prefer the static CRT (`/MT`) so the plugin does not depend on a matching `VCRUNTIME` DLL next to the game.

Full sources and CMake: [`examples/plugins/demo_cpp`](../examples/plugins/demo_cpp).

### Build with CMake

From `examples/plugins/demo_cpp`:

```bash
cmake -S . -B build -A x64
cmake --build build --config Release
```

DLL: `build/Release/demo_cpp.dll` (MSVC) or `build/demo_cpp.dll` (Ninja / MinGW).

Ship [`demo_cpp.metrics.json`](../examples/plugins/demo_cpp/demo_cpp.metrics.json) next to the DLL (copy both into `plugins/`).

### Build with `cl` (MSVC Developer Command Prompt)

```bat
cl /LD /EHsc /MT /O2 /I..\..\..\sdk demo_cpp.cpp /Fe:demo_cpp.dll
```

## .NET guide (NativeAOT)

Classic .NET Framework / CoreCLR hosting inside Elden Ring is **not** supported. You must publish a **native** DLL with NativeAOT.

### Prerequisites

- .NET 8 SDK or newer.
- The Visual Studio workload **Desktop development with C++** (NativeAOT needs the MSVC linker).

### Project settings

Key properties (see [`examples/plugins/demo_dotnet/DemoDotnet.csproj`](../examples/plugins/demo_dotnet/DemoDotnet.csproj)):

```xml
<PropertyGroup>
  <TargetFramework>net8.0</TargetFramework>
  <RuntimeIdentifier>win-x64</RuntimeIdentifier>
  <PublishAot>true</PublishAot>
  <NativeLib>Shared</NativeLib>
  <AllowUnsafeBlocks>true</AllowUnsafeBlocks>
  <AssemblyName>demo_dotnet</AssemblyName>
</PropertyGroup>
```

Export the six entry points with `[UnmanagedCallersOnly(EntryPoint = "er_overlay_plugin_...")]`. Match `ErMetricDesc` / `ErMetricSample` / `ErHostInfo` field-for-field (`[StructLayout(LayoutKind.Sequential)]`). Keep UTF-8 id buffers alive until `destroy`; free sample text on the next `poll`.

NativeAOT embeds its own GC in the game process. Keep `poll` / `sample` cheap and avoid heavy allocations every tick.

Full sources: [`examples/plugins/demo_dotnet`](../examples/plugins/demo_dotnet).

### Build

```bash
dotnet publish -c Release
```

DLL: `bin/Release/net8.0/win-x64/native/demo_dotnet.dll` (path may vary slightly by SDK version; look under `native/`).

Ship [`demo_dotnet.metrics.json`](../examples/plugins/demo_dotnet/demo_dotnet.metrics.json) next to the DLL (copy both into `plugins/`).

## ABI reference (version 1)

Contract header: [`sdk/er_overlay_plugin.h`](../sdk/er_overlay_plugin.h). Layout sizes are pinned by tests in `er_overlay_plugin_abi` and by `static_assert` in the header:

| Struct | Size |
| --- | --- |
| `ErMetricDesc` | 24 bytes |
| `ErMetricSample` | 32 bytes |
| `ErHostInfo` | 32 bytes |

### Call order

All calls run on the overlay **poll thread** (~250 ms), never on the frame/render thread. `poll` / `sample` run only once the game state reader is ready (not on the title screen).

| Export | When |
| --- | --- |
| `er_overlay_plugin_abi_version` | Once, before anything else. Must return `1`. |
| `er_overlay_plugin_create` | Once. Return null to refuse loading. |
| `er_overlay_plugin_metrics` | Once, right after create. The id list is frozen afterwards. |
| `er_overlay_plugin_poll` | Every tick, before samples. Argument `tick_ms` is milliseconds since the poll thread started. |
| `er_overlay_plugin_sample` | Once per accepted metric per tick. Argument is an **index** into the metrics array, not a string. |
| `er_overlay_plugin_destroy` | When the overlay unloads. |

`poll` is a heartbeat, not a sampling guarantee. For events shorter than ~250 ms, start your own thread and let `sample` read the last published value. A `poll` that takes **50 ms or more** is logged as slow.

### Pointer ownership

- Do **not** retain the `ErHostInfo*` pointer. Copy `overlay_version` / `base_dir` if needed; `log` stays valid until `destroy` returns.
- Metric ids you return must remain valid until `destroy`.
- Sample `text` must remain valid until the next `poll`.
- State (counters, files under `base_dir`) belongs to the plugin. The overlay never saves or resets it.

### Value kinds

The descriptor carries an id and a kind. The kind tells the overlay how to render:

| Kind | Value | Rendering |
| --- | --- | --- |
| `count` | `0` | Integer. With `has_max` on the sample and `show_max` on the tile: `1250/4000`. Reaching max turns the tile border green. |
| `time_ms` | `1` | Milliseconds, rendered `HH:MM:SS`. |
| `text` | `2` | String as-is (`S`, `A+`). Not usable as a challenge personal-best source. |

`available = 0` renders `---`.

**Host range:** for `count` and `time_ms`, the overlay converts `value` (and `max` when present) with `u32::try_from`. Negative values or values above `u32::MAX` become `---`.

In the Rust SDK: `MetricSample::count`, `count_with_max`, `time_ms`, `text`, `unavailable`. Override `MetricPlugin::kind` when the metric is not a plain counter — `time_ms()` only fills the numeric field.

### Metric ids

Form: `plugin_name.metric_name`. Rules:

- lowercase ASCII; digits and `_` allowed after the first character of each segment;
- at least one dot (two or more segments);
- 3–128 characters.

Valid: `answer.value`, `score.total`. Invalid: `Score.Total`, `score.`, `igt`.

Reserved (skipped with a warning): `igt`, `deaths`, `ng_cycle`, `scadutree_blessing`, `bosses`, `checks`, `checks_base`, `checks_dlc`, `pb`, `challenge_pb`, `nbtries`, `tries`, `challenge_tries`.

Duplicate ids: first loaded plugin wins (DLL names sorted), later ones skipped with a warning. A plugin cannot hide a built-in good or aggregate that already owns that id.

A numeric plugin metric can feed a challenge personal best or budget; a text metric cannot.

## Pitfalls checklist

- Windows **x64** only; struct layout must match bit for bit.
- No C++ exception / Rust panic / .NET exception across the boundary.
- No heavy work in `DllMain` (loader lock).
- Do not keep the `ErHostInfo*` pointer after `create` returns.
- Keep `poll` under 50 ms (warning otherwise); do not block the poll thread.
- Restart the game after changing which metrics a plugin declares.
- Enable logging when debugging load failures.

## Reading the game from Rust

A plugin may depend on the `er_game_state` crate and use `GameStateSource` (event flags, boss table, RVA scan). That is a compile-time dependency only: the overlay still passes nothing at runtime. Each DLL repeats the RVA scan in its own statics (idempotent, safe). You are coupled to that crate’s version.

## Other languages

Anything that produces a Windows x64 DLL exporting C symbols can implement the ABI.

- **Low friction:** Rust (SDK), C, C++, Zig, D, Odin, Nim, Delphi.
- **Possible with constraints:** C# via NativeAOT only (see above). Go `c-shared` starts a runtime/GC in-process — not supported here.
- **Not directly:** Python, JavaScript, Lua, Java (need an interpreter). Someone could ship one plugin that embeds CPython and exposes scripts as metrics; the overlay does not need to know.

## Configuration reference

```toml
[plugins]
enabled = true       # false loads nothing
dir = "plugins"      # relative to er_overlay.dll, or an absolute path
disabled = []        # file name or stem, case-insensitive: "answer" or "answer.dll"
```

`dir` is not created for you. An install with no `plugins` folder behaves as before (no plugins).

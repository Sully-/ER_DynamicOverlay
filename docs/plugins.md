# Metric plugins

A metric plugin is a native Windows x64 DLL. You drop it in `plugins/` next to `er_overlay.dll` and the overlay starts calling it. It declares metric ids and returns a number (or a short string) every tick. The layout file decides the label, the icon, `show_max` and where the tile sits. The overlay does not give the plugin any game data: the plugin reads whatever it needs on its own.

> **A plugin is arbitrary code inside the Elden Ring process.** A buggy plugin crashes the game. There is no sandbox. Only load plugins you trust.

## A plugin that returns 42

This is the whole plugin. It exposes one metric, `answer.value`, which is always `42`.

```rust
use er_overlay_plugin_sdk::{declare_plugin, MetricPlugin, MetricSample};

struct Answer;

impl MetricPlugin for Answer {
    fn metrics() -> &'static [&'static str] {
        &["answer.value"]
    }

    fn sample(&mut self, _index: usize) -> MetricSample {
        MetricSample::count(42)
    }
}

declare_plugin!(Answer, Answer);
```

The same source lives in [`examples/plugins/answer`](../examples/plugins/answer). Copy that folder if you would rather not start from a blank crate.

### 1. Create the crate

From a directory of your choice:

```bash
cargo new --lib answer
```

`Cargo.toml`:

```toml
[package]
name = "answer"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
er_overlay_plugin_sdk = { path = "path/to/ER_DynamicOverlay/sdk/er_overlay_plugin_sdk" }

[profile.dev]
panic = "abort"

[profile.release]
panic = "abort"
```

`panic = "abort"` is required. A panic must not unwind across the C boundary; it would abort anyway, and this makes that explicit. `crate-type = ["cdylib"]` produces a DLL instead of a Rust library.

Put the source above in `src/lib.rs`.

### 2. Build it

On 64-bit Windows the default target is already correct:

```bash
cargo build --release
```

The DLL is `target/release/answer.dll`. The fully qualified form, useful when cross-compiling, is:

```bash
cargo build --release --target x86_64-pc-windows-msvc
```

That writes `target/x86_64-pc-windows-msvc/release/answer.dll`.

### 3. Install it

Copy `answer.dll` into the `plugins/` directory next to `er_overlay.dll`. Create the directory if it is not there. Restart the game: the list of metrics is fixed at startup.

`er_overlay.toml`:

```toml
[plugins]
enabled = true
dir = "plugins"
disabled = []   # e.g. ["answer"] to skip answer.dll without deleting it
```

With logging on (`log_enabled = true`), `logs/er_overlay.log` records each plugin that loaded and how many metrics it registered. A rejected DLL (wrong ABI version, missing export, `create` returned null) is logged and skipped; the rest of the overlay still starts.

### 4. Show it

The plugin does not choose a name or an icon. The layout tile does, the same way it does for `igt`. Add this to your layout:

```toml
[[tile]]
kind = "metric"
metric = "answer.value"
label = "ANSWER"
col = 0
row = 0
```

Reload the layout (it is picked up on the same hot-reload as the rest of the file, without restarting). The tile shows `42`. If the plugin is missing, the tile shows `---` and nothing else breaks.

An icon is a PNG key, resolved in `assets/icons` first and then in `plugins/icons`:

```toml
icon = "answer"
show_max = false
```

## What the overlay calls

Six C functions, in this order, all on the overlay's poll thread (about every 250 ms), never on the frame thread:

| Export | When |
| --- | --- |
| `er_overlay_plugin_abi_version` | Once, before anything else. Must return `1`. |
| `er_overlay_plugin_create` | Once. Return null to refuse loading. |
| `er_overlay_plugin_metrics` | Once, right after create. The id list is frozen afterwards. |
| `er_overlay_plugin_poll` | Every tick, before the samples. |
| `er_overlay_plugin_sample` | Once per metric per tick. The argument is an index into that list, not a string. |
| `er_overlay_plugin_destroy` | When the overlay unloads. |

`poll` is a heartbeat, not a sampling guarantee. If you have to notice something shorter than 250 ms, start your own thread and let `sample` read the last value you published.

State belongs to the plugin. Counters, latched flags, files under `base_dir` (the directory of `er_overlay.dll`): the overlay never saves or resets them. A plugin whose metric list depends on a config file must be reloaded — restart the game — to pick up a new list, which matches the fact that the layout has to be edited anyway.

## Value kinds

The descriptor only carries an id and a kind. The kind tells the overlay how to render the number:

| Kind | Meaning |
| --- | --- |
| `count` (`0`) | An integer. With `has_max` on the sample and `show_max` on the tile, it renders `1250/4000`. The max may change every tick. Reaching it turns the tile border green. |
| `time_ms` (`1`) | Milliseconds, rendered `HH:MM:SS`. |
| `text` (`2`) | A string, rendered as-is (`S`, `A+`). Not usable as a challenge personal-best source. |

`available = 0` renders `---`.

In the Rust SDK, `MetricSample::count`, `count_with_max`, `time_ms`, `text` and `unavailable` build these. Override `MetricPlugin::kind` when a metric is not a plain counter. `on_create` receives `HostInfo` (`overlay_version`, `base_dir`, and `log` which writes to `logs/er_overlay.log`).

## Ids

An id is `plugin_name.metric_name`: lowercase ASCII, digits and `_` allowed after the first character of each segment, at least one dot, 128 characters at most. `answer.value` and `score.total` are valid. `Score.Total`, `score.` and `igt` are not.

These ids are reserved and ignored, with a warning: `igt`, `deaths`, `ng_cycle`, `scadutree_blessing`, `bosses`, `checks`, `checks_base`, `checks_dlc`, `pb`, `challenge_pb`, `nbtries`, `tries`, `challenge_tries`.

Two plugins declaring the same id: the first one loaded wins (DLL names are sorted), the second is skipped with a warning. A plugin also cannot hide a good or an aggregate group that already has that id.

A numeric plugin metric can be the source of a challenge personal best or budget, because it resolves like any other count. A text metric cannot.

## Languages

Anything that produces a Windows x64 DLL exporting C symbols can implement this. The struct layout in [`sdk/er_overlay_plugin.h`](../sdk/er_overlay_plugin.h) is the contract; the sizes are pinned by tests in `er_overlay_plugin_abi`.

- **No friction:** Rust (use the SDK), C, C++, Zig, D, Odin, Nim, Delphi.
- **Possible, with a constraint:** C# only via NativeAOT (`PublishAot` and `[UnmanagedCallersOnly]`), which emits a real native DLL. Classic .NET would mean hosting CoreCLR inside the game, which this overlay does not do. Go can emit `c-shared`, but it starts its runtime and garbage collector inside the game process; that is a bad idea here and not supported.
- **Not directly:** Python, JavaScript, Lua, Java. They need an interpreter. Nothing stops someone from shipping one plugin that embeds CPython and exposes scripts as metrics; the overlay does not have to know about it.

Shared constraints, whatever the language: x64, the struct layout must match bit for bit, no exception crosses the boundary, and heavy initialization happens in `er_overlay_plugin_create`, never in `DllMain` (the loader lock forbids almost everything there).

## Reading the game from Rust

A plugin may depend on the `er_game_state` crate like any other crate and use `GameStateSource` (event flags, boss table, RVA scan) instead of reimplementing them. That is a compile-time dependency. The overlay still passes the plugin nothing. Each DLL repeats the RVA scan in its own statics; those reads are idempotent, so that is safe. It does couple you to that crate's version.

## Layout editor

On startup the overlay writes `plugins/metrics.json` next to the DLLs: a list of `{ "id", "kind" }` and nothing else. In the layout editor, **Import plugin metrics** reads that file and adds the ids to the palette. Dropping one onto the grid pre-fills the label from the id (`score.total` becomes `TOTAL`); rewrite it. Importing a layout that already references an unknown id keeps that id instead of silently resetting it to `igt`.

## Configuration reference

```toml
[plugins]
enabled = true       # false loads nothing
dir = "plugins"      # relative to er_overlay.dll, or an absolute path
disabled = []        # file name or stem, case-insensitive: "answer" or "answer.dll"
```

`dir` is not created for you. An install with no `plugins` folder behaves exactly as before.

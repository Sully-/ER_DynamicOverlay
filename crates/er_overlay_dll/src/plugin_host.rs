//! Loads metric-plugin DLLs and turns their samples into [`MetricValue`]s.
//!
//! Discovery and the FFI boundary live here. Id validation and collision handling go through
//! [`PluginSet`], which is also what the tests drive, so they never need a real DLL.

use std::collections::{HashMap, HashSet};
use std::ffi::{c_char, c_void, CStr, CString};
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use er_overlay_common::{default_base_dir, GameTime, PluginsConfig};
use er_overlay_plugin_abi::{
    ErHostInfo, ErMetricDesc, ErMetricSample, MetricKind, ABI_VERSION, ER_LOG_DEBUG, ER_LOG_ERROR,
    ER_LOG_INFO, ER_LOG_TRACE, ER_LOG_WARN,
};
use er_overlay_ui::MetricValue;
use tracing::{info, warn};
use windows::core::{PCSTR, PCWSTR};
use windows::Win32::Foundation::{FreeLibrary, HMODULE};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

/// A plugin `poll()` longer than this is logged. It stalls metric updates, not the frame.
const SLOW_POLL: Duration = Duration::from_millis(50);
const MAX_METRICS: usize = 4096;

type AbiVersionFn = unsafe extern "C" fn() -> u32;
type CreateFn = unsafe extern "C" fn(*const ErHostInfo) -> *mut c_void;
type MetricsFn = unsafe extern "C" fn(*mut c_void, *mut usize) -> *const ErMetricDesc;
type PollFn = unsafe extern "C" fn(*mut c_void, u64);
type SampleFn = unsafe extern "C" fn(*mut c_void, usize) -> ErMetricSample;
type DestroyFn = unsafe extern "C" fn(*mut c_void);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetricIdError {
    Malformed,
    Reserved,
}

const RESERVED_METRIC_IDS: &[&str] = &[
    "igt",
    "deaths",
    "ng_cycle",
    "scadutree_blessing",
    "bosses",
    "checks",
    "checks_base",
    "checks_dlc",
    "pb",
    "challenge_pb",
    "nbtries",
    "tries",
    "challenge_tries",
];

/// `plugin.metric`, lowercase, at least two dot-separated segments.
pub fn classify_metric_id(id: &str) -> Result<(), MetricIdError> {
    if RESERVED_METRIC_IDS.contains(&id) {
        return Err(MetricIdError::Reserved);
    }
    if !is_well_formed_metric_id(id) {
        return Err(MetricIdError::Malformed);
    }
    Ok(())
}

fn is_well_formed_metric_id(id: &str) -> bool {
    if id.len() < 3 || id.len() > 128 {
        return false;
    }
    let mut segments = 0;
    for segment in id.split('.') {
        segments += 1;
        let mut chars = segment.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if !first.is_ascii_lowercase() {
            return false;
        }
        if !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            return false;
        }
    }
    segments >= 2
}

#[derive(Debug, Clone)]
pub struct DeclaredMetric {
    pub id: String,
    pub kind: MetricKind,
    /// Index passed back to `sample`. Stays the plugin's index even if a neighbour was rejected.
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct RawSample {
    pub value: i64,
    pub max: i64,
    pub text: Option<String>,
    pub available: bool,
    pub has_max: bool,
}

pub trait MetricProvider {
    fn source_name(&self) -> &str;
    fn declared_metrics(&self) -> &[DeclaredMetric];
    fn poll(&mut self, tick_ms: u64);
    fn sample(&mut self, index: usize) -> RawSample;
}

#[derive(Clone)]
struct Binding {
    id: String,
    kind: MetricKind,
    provider: usize,
    index: usize,
}

/// Accepted metrics across every provider. First id wins; later duplicates are dropped.
pub struct PluginSet {
    providers: Vec<Box<dyn MetricProvider>>,
    bindings: Vec<Binding>,
    taken: HashSet<String>,
}

impl PluginSet {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            bindings: Vec::new(),
            taken: HashSet::new(),
        }
    }

    pub fn add(&mut self, provider: Box<dyn MetricProvider>) {
        let source = provider.source_name().to_string();
        let declared = provider.declared_metrics().to_vec();
        let provider_index = self.providers.len();
        let mut accepted = 0;
        for metric in declared {
            match classify_metric_id(&metric.id) {
                Err(MetricIdError::Reserved) => {
                    warn!(
                        plugin = %source,
                        id = %metric.id,
                        "plugin metric id is reserved by the overlay, skipping"
                    );
                }
                Err(MetricIdError::Malformed) => {
                    warn!(
                        plugin = %source,
                        id = %metric.id,
                        "plugin metric id is malformed (expected plugin.metric), skipping"
                    );
                }
                Ok(()) if self.taken.contains(&metric.id) => {
                    warn!(
                        plugin = %source,
                        id = %metric.id,
                        "duplicate metric id, keeping the first plugin"
                    );
                }
                Ok(()) => {
                    self.taken.insert(metric.id.clone());
                    self.bindings.push(Binding {
                        id: metric.id,
                        kind: metric.kind,
                        provider: provider_index,
                        index: metric.index,
                    });
                    accepted += 1;
                }
            }
        }
        info!(plugin = %source, metrics = accepted, "registered metric plugin");
        self.providers.push(provider);
    }

    pub fn entries(&self) -> Vec<(String, MetricKind)> {
        self.bindings
            .iter()
            .map(|binding| (binding.id.clone(), binding.kind))
            .collect()
    }

    pub fn poll_all(&mut self, tick_ms: u64) -> HashMap<String, MetricValue> {
        for provider in &mut self.providers {
            let name = provider.source_name().to_string();
            let started = Instant::now();
            provider.poll(tick_ms);
            let elapsed = started.elapsed();
            if elapsed >= SLOW_POLL {
                warn!(
                    plugin = %name,
                    elapsed_ms = elapsed.as_millis() as u64,
                    "metric plugin poll is slow"
                );
            }
        }

        let plan: Vec<Binding> = self.bindings.clone();
        let mut out = HashMap::with_capacity(plan.len());
        for binding in plan {
            let sample = self.providers[binding.provider].sample(binding.index);
            out.insert(binding.id, metric_value_from_sample(binding.kind, &sample));
        }
        out
    }
}

impl Default for PluginSet {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn metric_value_from_sample(kind: MetricKind, sample: &RawSample) -> MetricValue {
    if !sample.available {
        return MetricValue::Unavailable;
    }
    match kind {
        MetricKind::Count => {
            let Ok(current) = u32::try_from(sample.value) else {
                return MetricValue::Unavailable;
            };
            let max = if sample.has_max {
                u32::try_from(sample.max).ok()
            } else {
                None
            };
            MetricValue::Count {
                current: Some(current),
                max,
            }
        }
        MetricKind::TimeMs => match u32::try_from(sample.value) {
            Ok(ms) => MetricValue::Time(GameTime::from_ms(ms)),
            Err(_) => MetricValue::Unavailable,
        },
        MetricKind::Text => MetricValue::Text(sample.text.clone().unwrap_or_default()),
    }
}

/// DLLs loaded from the plugins directory. Created and polled on the poll thread only.
pub struct PluginHost {
    set: PluginSet,
    // Kept alive so plugins that retained `base_dir` / `overlay_version` stay valid until drop.
    // Declared after `set` so plugins are destroyed first (Rust drops fields in order).
    _version: CString,
    _base_dir: CString,
}

impl PluginHost {
    pub fn load(config: &PluginsConfig) -> Self {
        Self::load_in(&default_base_dir(), config)
    }

    fn load_in(base: &Path, config: &PluginsConfig) -> Self {
        let version =
            CString::new(env!("CARGO_PKG_VERSION")).unwrap_or_else(|_| must_cstring("unknown"));
        let base_dir = path_cstring(base);
        if !config.enabled {
            info!("metric plugins disabled by config");
            return Self {
                set: PluginSet::new(),
                _version: version,
                _base_dir: base_dir,
            };
        }

        let dir = config.directory(base);
        if !dir.is_dir() {
            info!(
                dir = %dir.display(),
                "metric plugins directory not found, skipping"
            );
            return Self {
                set: PluginSet::new(),
                _version: version,
                _base_dir: base_dir,
            };
        }

        let mut set = PluginSet::new();
        for path in list_plugin_dlls(&dir, &config.disabled) {
            match LoadedDll::open(&path, version.as_ptr(), base_dir.as_ptr()) {
                Ok(dll) => set.add(Box::new(dll)),
                Err(err) => {
                    warn!(path = %path.display(), error = %err, "failed to load metric plugin")
                }
            }
        }

        let json_path = dir.join("metrics.json");
        if let Err(err) = write_metrics_json(&json_path, &set.entries()) {
            warn!(
                path = %json_path.display(),
                error = %err,
                "failed to write plugin metric list"
            );
        } else {
            info!(path = %json_path.display(), "wrote plugin metric list");
        }

        Self {
            set,
            _version: version,
            _base_dir: base_dir,
        }
    }

    pub fn poll_all(&mut self, tick_ms: u64) -> HashMap<String, MetricValue> {
        self.set.poll_all(tick_ms)
    }
}

fn list_plugin_dlls(dir: &Path, disabled: &[String]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            warn!(dir = %dir.display(), error = %err, "failed to read plugins directory");
            return found;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.to_ascii_lowercase().ends_with(".dll") {
            continue;
        }
        if name.eq_ignore_ascii_case("er_overlay.dll") {
            continue;
        }
        if is_disabled(name, disabled) {
            info!(plugin = name, "metric plugin disabled by config");
            continue;
        }
        found.push(path);
    }
    found.sort();
    found
}

fn is_disabled(file_name: &str, disabled: &[String]) -> bool {
    let lower = file_name.to_ascii_lowercase();
    let stem = Path::new(file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    disabled.iter().any(|entry| {
        let entry = entry.to_ascii_lowercase();
        entry == lower || entry == stem
    })
}

fn write_metrics_json(path: &Path, entries: &[(String, MetricKind)]) -> std::io::Result<()> {
    let mut body = String::from("{\n  \"metrics\": [\n");
    for (i, (id, kind)) in entries.iter().enumerate() {
        let comma = if i + 1 == entries.len() { "" } else { "," };
        body.push_str(&format!(
            "    {{\"id\": \"{}\", \"kind\": \"{}\"}}{}\n",
            json_escape(id),
            kind.as_str(),
            comma
        ));
    }
    body.push_str("  ]\n}\n");
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, body)
}

fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

struct Library(HMODULE);

impl Library {
    fn load(path: &Path) -> Result<Self, String> {
        let wide: Vec<u16> = std::ffi::OsStr::new(path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let module =
            unsafe { LoadLibraryW(PCWSTR(wide.as_ptr())) }.map_err(|err| err.to_string())?;
        Ok(Self(module))
    }

    fn steal(&mut self) -> HMODULE {
        let module = self.0;
        self.0 = HMODULE::default();
        module
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        if self.0.is_invalid() {
            return;
        }
        unsafe {
            let _ = FreeLibrary(self.0);
        }
    }
}

/// Calls `destroy` before freeing the module if `open` fails after `create`.
struct PendingPlugin {
    library: Library,
    ctx: *mut c_void,
    destroy: Option<DestroyFn>,
}

impl Drop for PendingPlugin {
    fn drop(&mut self) {
        if let Some(destroy) = self.destroy.take() {
            if !self.ctx.is_null() {
                unsafe { destroy(self.ctx) };
                self.ctx = std::ptr::null_mut();
            }
        }
    }
}

struct LoadedDll {
    module: HMODULE,
    ctx: *mut c_void,
    destroy: DestroyFn,
    poll_fn: PollFn,
    sample_fn: SampleFn,
    source_name: String,
    declared: Vec<DeclaredMetric>,
}

impl LoadedDll {
    fn open(path: &Path, version: *const c_char, base_dir: *const c_char) -> Result<Self, String> {
        let source_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let mut pending = PendingPlugin {
            library: Library::load(path)?,
            ctx: std::ptr::null_mut(),
            destroy: None,
        };
        let module = pending.library.0;

        let abi_version = load_fn::<AbiVersionFn>(module, "er_overlay_plugin_abi_version")?;
        let reported = unsafe { abi_version() };
        if reported != ABI_VERSION {
            return Err(format!(
                "ABI version {reported} does not match the overlay ({ABI_VERSION})"
            ));
        }

        let create = load_fn::<CreateFn>(module, "er_overlay_plugin_create")?;
        let info = ErHostInfo {
            abi_version: ABI_VERSION,
            _pad: 0,
            overlay_version: version,
            base_dir,
            log: Some(host_log),
        };
        let ctx = unsafe { create(&info) };
        if ctx.is_null() {
            return Err("er_overlay_plugin_create returned null".into());
        }
        pending.ctx = ctx;
        pending.destroy = Some(load_fn::<DestroyFn>(module, "er_overlay_plugin_destroy")?);
        let metrics = load_fn::<MetricsFn>(module, "er_overlay_plugin_metrics")?;
        let poll_fn = load_fn::<PollFn>(module, "er_overlay_plugin_poll")?;
        let sample_fn = load_fn::<SampleFn>(module, "er_overlay_plugin_sample")?;
        let declared = read_declared(ctx, metrics)?;

        let destroy = pending.destroy.take().expect("destroy was just set");
        pending.ctx = std::ptr::null_mut();
        let module = pending.library.steal();
        Ok(Self {
            module,
            ctx,
            destroy,
            poll_fn,
            sample_fn,
            source_name,
            declared,
        })
    }
}

impl MetricProvider for LoadedDll {
    fn source_name(&self) -> &str {
        &self.source_name
    }

    fn declared_metrics(&self) -> &[DeclaredMetric] {
        &self.declared
    }

    fn poll(&mut self, tick_ms: u64) {
        unsafe { (self.poll_fn)(self.ctx, tick_ms) };
    }

    fn sample(&mut self, index: usize) -> RawSample {
        let raw = unsafe { (self.sample_fn)(self.ctx, index) };
        let text = if raw.text.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(raw.text) }
                    .to_string_lossy()
                    .into_owned(),
            )
        };
        RawSample {
            value: raw.value,
            max: raw.max,
            text,
            available: raw.available != 0,
            has_max: raw.has_max != 0,
        }
    }
}

impl Drop for LoadedDll {
    fn drop(&mut self) {
        unsafe {
            (self.destroy)(self.ctx);
            if !self.module.is_invalid() {
                let _ = FreeLibrary(self.module);
            }
        }
    }
}

fn read_declared(ctx: *mut c_void, metrics: MetricsFn) -> Result<Vec<DeclaredMetric>, String> {
    let mut len = 0usize;
    let ptr = unsafe { metrics(ctx, &mut len) };
    if len == 0 {
        return Ok(Vec::new());
    }
    if ptr.is_null() {
        return Err("er_overlay_plugin_metrics returned a null pointer".into());
    }
    if len > MAX_METRICS {
        return Err(format!(
            "er_overlay_plugin_metrics returned {len} entries (limit {MAX_METRICS})"
        ));
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    let mut declared = Vec::with_capacity(len);
    for (index, desc) in slice.iter().enumerate() {
        if desc.id.is_null() {
            warn!(index, "plugin metric has a null id, skipping");
            continue;
        }
        let id = unsafe { CStr::from_ptr(desc.id) }
            .to_string_lossy()
            .into_owned();
        let Some(kind) = MetricKind::from_u32(desc.kind) else {
            warn!(
                id = %id,
                kind = desc.kind,
                "plugin metric has an unknown kind, skipping"
            );
            continue;
        };
        declared.push(DeclaredMetric { id, kind, index });
    }
    Ok(declared)
}

fn load_fn<T>(module: HMODULE, name: &str) -> Result<T, String> {
    let c_name = CString::new(name).map_err(|_| format!("bad export name {name}"))?;
    let Some(proc) = (unsafe { GetProcAddress(module, PCSTR(c_name.as_ptr() as *const u8)) })
    else {
        return Err(format!("missing export {name}"));
    };
    if std::mem::size_of::<T>() != std::mem::size_of_val(&proc) {
        return Err(format!("export {name} has an unexpected pointer size"));
    }
    Ok(unsafe { std::mem::transmute_copy(&proc) })
}

unsafe extern "C" fn host_log(level: u32, msg: *const c_char) {
    if msg.is_null() {
        return;
    }
    let text = CStr::from_ptr(msg).to_string_lossy();
    match level {
        ER_LOG_ERROR => tracing::error!(target: "er_overlay_plugin", "{text}"),
        ER_LOG_WARN => tracing::warn!(target: "er_overlay_plugin", "{text}"),
        ER_LOG_INFO => tracing::info!(target: "er_overlay_plugin", "{text}"),
        ER_LOG_DEBUG => tracing::debug!(target: "er_overlay_plugin", "{text}"),
        ER_LOG_TRACE => tracing::trace!(target: "er_overlay_plugin", "{text}"),
        _ => tracing::trace!(target: "er_overlay_plugin", "{text}"),
    }
}

fn path_cstring(path: &Path) -> CString {
    CString::new(path.to_string_lossy().as_ref()).unwrap_or_else(|_| must_cstring("."))
}

fn must_cstring(value: &str) -> CString {
    CString::new(value).unwrap_or_else(|_| CString::new(".").expect("cstring"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use er_overlay_ui::MetricValue;

    struct Fake {
        name: String,
        metrics: Vec<DeclaredMetric>,
        samples: Vec<RawSample>,
        polls: u32,
    }

    impl MetricProvider for Fake {
        fn source_name(&self) -> &str {
            &self.name
        }

        fn declared_metrics(&self) -> &[DeclaredMetric] {
            &self.metrics
        }

        fn poll(&mut self, _tick_ms: u64) {
            self.polls += 1;
        }

        fn sample(&mut self, index: usize) -> RawSample {
            self.samples.get(index).cloned().unwrap_or(RawSample {
                value: 0,
                max: 0,
                text: None,
                available: false,
                has_max: false,
            })
        }
    }

    fn count_metric(id: &str, index: usize) -> DeclaredMetric {
        DeclaredMetric {
            id: id.to_string(),
            kind: MetricKind::Count,
            index,
        }
    }

    fn available_count(value: i64, max: Option<i64>) -> RawSample {
        RawSample {
            value,
            max: max.unwrap_or(0),
            text: None,
            available: true,
            has_max: max.is_some(),
        }
    }

    #[test]
    fn rejects_reserved_and_malformed_ids() {
        assert_eq!(classify_metric_id("igt"), Err(MetricIdError::Reserved));
        assert_eq!(classify_metric_id("checks"), Err(MetricIdError::Reserved));
        assert_eq!(
            classify_metric_id("challenge_tries"),
            Err(MetricIdError::Reserved)
        );
        assert_eq!(classify_metric_id(""), Err(MetricIdError::Malformed));
        assert_eq!(
            classify_metric_id("Score.Total"),
            Err(MetricIdError::Malformed)
        );
        assert_eq!(classify_metric_id("score."), Err(MetricIdError::Malformed));
        assert_eq!(classify_metric_id(".value"), Err(MetricIdError::Malformed));
        assert_eq!(
            classify_metric_id("score..value"),
            Err(MetricIdError::Malformed)
        );
        assert!(classify_metric_id("score.total").is_ok());
        assert!(classify_metric_id("answer.value").is_ok());
    }

    #[test]
    fn first_plugin_wins_duplicate_ids() {
        let mut set = PluginSet::new();
        set.add(Box::new(Fake {
            name: "a.dll".into(),
            metrics: vec![count_metric("score.total", 0), count_metric("igt", 1)],
            samples: vec![available_count(10, Some(40)), available_count(1, None)],
            polls: 0,
        }));
        set.add(Box::new(Fake {
            name: "b.dll".into(),
            metrics: vec![
                count_metric("score.total", 0),
                count_metric("Bad Id", 1),
                DeclaredMetric {
                    id: "score.rank".into(),
                    kind: MetricKind::Text,
                    index: 2,
                },
            ],
            samples: vec![
                available_count(99, None),
                available_count(1, None),
                RawSample {
                    value: 0,
                    max: 0,
                    text: Some("S".into()),
                    available: true,
                    has_max: false,
                },
            ],
            polls: 0,
        }));

        let values = set.poll_all(0);
        assert_eq!(
            values.get("score.total"),
            Some(&MetricValue::Count {
                current: Some(10),
                max: Some(40),
            })
        );
        assert!(!values.contains_key("igt"));
        assert!(!values.contains_key("Bad Id"));
        assert_eq!(
            values.get("score.rank"),
            Some(&MetricValue::Text("S".into()))
        );
        assert_eq!(set.entries().len(), 2);
    }

    #[test]
    fn sample_kinds_become_metric_values() {
        let time = metric_value_from_sample(MetricKind::TimeMs, &available_count(2_700_000, None));
        assert!(matches!(time, MetricValue::Time(t) if t.format_hms() == "00:45:00"));

        let text = metric_value_from_sample(
            MetricKind::Text,
            &RawSample {
                value: 0,
                max: 0,
                text: Some("A+".into()),
                available: true,
                has_max: false,
            },
        );
        assert_eq!(text, MetricValue::Text("A+".into()));

        let missing = metric_value_from_sample(
            MetricKind::Count,
            &RawSample {
                value: 1,
                max: 0,
                text: None,
                available: false,
                has_max: false,
            },
        );
        assert_eq!(missing, MetricValue::Unavailable);
    }

    #[test]
    fn disabled_dll_names_are_skipped() {
        assert!(is_disabled("answer.dll", &["answer".into()]));
        assert!(is_disabled("Answer.DLL", &["answer.dll".into()]));
        assert!(!is_disabled("answer.dll", &["other.dll".into()]));
    }

    #[test]
    fn missing_plugins_directory_is_not_created() {
        let dir =
            std::env::temp_dir().join(format!("er_overlay_plugins_missing_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let config = PluginsConfig {
            enabled: true,
            dir: dir.to_string_lossy().into_owned(),
            disabled: Vec::new(),
        };
        let host = PluginHost::load_in(Path::new("."), &config);
        assert!(host.set.entries().is_empty());
        assert!(!dir.exists());
    }

    #[test]
    fn empty_plugins_directory_writes_metric_list() {
        let dir =
            std::env::temp_dir().join(format!("er_overlay_plugins_empty_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("notes.txt"), b"not a plugin").unwrap();
        let config = PluginsConfig {
            enabled: true,
            dir: dir.to_string_lossy().into_owned(),
            disabled: Vec::new(),
        };
        let host = PluginHost::load_in(Path::new("."), &config);
        assert!(host.set.entries().is_empty());
        let raw = fs::read_to_string(dir.join("metrics.json")).unwrap();
        assert!(raw.contains("\"metrics\""));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn loads_answer_plugin_when_present() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let candidates = [
            manifest.join("../../examples/plugins/answer/target/debug/answer.dll"),
            manifest.join("../../examples/plugins/answer/target/release/answer.dll"),
            manifest.join(
                "../../examples/plugins/answer/target/x86_64-pc-windows-msvc/debug/answer.dll",
            ),
            manifest.join(
                "../../examples/plugins/answer/target/x86_64-pc-windows-msvc/release/answer.dll",
            ),
        ];
        let Some(dll) = candidates.into_iter().find(|path| path.is_file()) else {
            eprintln!("skipping answer.dll integration: plugin has not been built");
            return;
        };
        let dir =
            std::env::temp_dir().join(format!("er_overlay_plugins_answer_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::copy(&dll, dir.join("answer.dll")).unwrap();
        let config = PluginsConfig {
            enabled: true,
            dir: dir.to_string_lossy().into_owned(),
            disabled: Vec::new(),
        };
        let mut host = PluginHost::load_in(Path::new("."), &config);
        let values = host.poll_all(0);
        assert_eq!(
            values.get("answer.value"),
            Some(&MetricValue::Count {
                current: Some(42),
                max: None,
            })
        );
        let listed = fs::read_to_string(dir.join("metrics.json")).unwrap();
        assert!(listed.contains("answer.value"));
        drop(host);
        let _ = fs::remove_dir_all(&dir);
    }
}

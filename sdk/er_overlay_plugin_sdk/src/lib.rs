//! Idiomatic wrapper around the overlay metric-plugin ABI.
//!
//! A plugin implements [`MetricPlugin`] and calls [`declare_plugin!`]. The macro emits the six
//! `extern "C"` exports, keeps the id strings alive, and copies host info out of the raw pointer
//! during `create` (do not retain that pointer yourself).

use std::ffi::{c_char, CStr, CString};

use er_overlay_plugin_abi::ErHostInfo;
pub use er_overlay_plugin_abi::{self as abi, MetricKind, ABI_VERSION};

/// One reading returned from [`MetricPlugin::sample`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricSample {
    pub(crate) value: i64,
    pub(crate) max: Option<i64>,
    pub(crate) text: Option<String>,
    pub(crate) available: bool,
}

impl MetricSample {
    /// An integer counter with no maximum.
    pub fn count(value: i64) -> Self {
        Self {
            value,
            max: None,
            text: None,
            available: true,
        }
    }

    /// An integer counter whose maximum may change from tick to tick.
    pub fn count_with_max(value: i64, max: i64) -> Self {
        Self {
            value,
            max: Some(max),
            text: None,
            available: true,
        }
    }

    /// A duration in milliseconds, rendered `HH:MM:SS` by the overlay.
    pub fn time_ms(value: i64) -> Self {
        Self::count(value)
    }

    /// Free-form text, such as a rank. Not usable as a challenge personal-best source.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            value: 0,
            max: None,
            text: Some(text.into()),
            available: true,
        }
    }

    /// The tile shows `---` until a later sample is available.
    pub fn unavailable() -> Self {
        Self {
            value: 0,
            max: None,
            text: None,
            available: false,
        }
    }

    /// Splits the sample so the generated FFI glue, which lives in the plugin crate, can read it.
    pub fn into_parts(self) -> SampleParts {
        SampleParts {
            value: self.value,
            max: self.max,
            text: self.text,
            available: self.available,
        }
    }
}

/// Owned pieces of a [`MetricSample`], visible to the plugin crate's generated exports.
#[derive(Debug)]
pub struct SampleParts {
    pub value: i64,
    pub max: Option<i64>,
    pub text: Option<String>,
    pub available: bool,
}

/// What the overlay told the plugin during `create`. Strings are owned copies.
#[derive(Debug, Clone)]
pub struct HostInfo {
    pub abi_version: u32,
    pub overlay_version: String,
    pub base_dir: String,
    log: Option<unsafe extern "C" fn(u32, *const c_char)>,
}

impl HostInfo {
    /// # Safety
    /// `info` is null or points at a valid [`ErHostInfo`] for the duration of this call.
    pub unsafe fn from_raw(info: *const ErHostInfo) -> Self {
        if info.is_null() {
            return Self {
                abi_version: 0,
                overlay_version: String::new(),
                base_dir: String::new(),
                log: None,
            };
        }
        let info = &*info;
        Self {
            abi_version: info.abi_version,
            overlay_version: cstr_to_string(info.overlay_version),
            base_dir: cstr_to_string(info.base_dir),
            log: info.log,
        }
    }

    /// Forwards a line to `logs/er_overlay.log` when the host provided a logger.
    pub fn log(&self, level: u32, message: &str) {
        let Some(log) = self.log else {
            return;
        };
        let Ok(message) = CString::new(message.replace('\0', "")) else {
            return;
        };
        unsafe { log(level, message.as_ptr()) };
    }
}

fn cstr_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

/// Implemented by a metric plugin. All calls happen on one thread, never concurrently.
pub trait MetricPlugin {
    /// Metric ids, conventionally `plugin_name.metric_name`.
    fn metrics() -> &'static [&'static str];

    /// Value kind for `metrics()[index]`. Defaults to a plain counter.
    fn kind(_index: usize) -> MetricKind {
        MetricKind::Count
    }

    /// Called once after construction. `host.base_dir` is the overlay DLL directory.
    fn on_create(&mut self, _host: &HostInfo) {}

    /// Heartbeat, about every 250 ms. Not a guarantee of sampling rate.
    fn poll(&mut self, _tick_ms: u64) {}

    /// Read metric `index` (the position in [`metrics`](MetricPlugin::metrics)).
    fn sample(&mut self, index: usize) -> MetricSample;
}

/// Emits the six C exports for a [`MetricPlugin`].
///
/// `$init` builds the plugin (`Answer` for a unit struct, `Score::new()` for anything else).
/// Put `panic = "abort"` on the plugin crate: a panic must not unwind across this boundary.
#[macro_export]
macro_rules! declare_plugin {
    ($ty:ty, $init:expr) => {
        struct __ErOverlayPluginState {
            plugin: $ty,
            ids: Vec<::std::ffi::CString>,
            descs: Vec<$crate::abi::ErMetricDesc>,
            texts: Vec<::std::ffi::CString>,
        }

        fn __er_overlay_build_state(plugin: $ty) -> Option<__ErOverlayPluginState> {
            let metric_ids = <$ty as $crate::MetricPlugin>::metrics();
            let mut ids = Vec::with_capacity(metric_ids.len());
            for id in metric_ids {
                ids.push(::std::ffi::CString::new(*id).ok()?);
            }
            let descs = ids
                .iter()
                .enumerate()
                .map(|(index, id)| $crate::abi::ErMetricDesc {
                    id: id.as_ptr(),
                    kind: <$ty as $crate::MetricPlugin>::kind(index).as_u32(),
                    _reserved: [0; 3],
                })
                .collect();
            Some(__ErOverlayPluginState {
                plugin,
                ids,
                descs,
                texts: Vec::new(),
            })
        }

        fn __er_overlay_state<'a>(
            ctx: *mut ::std::ffi::c_void,
        ) -> Option<&'a mut __ErOverlayPluginState> {
            if ctx.is_null() {
                None
            } else {
                Some(unsafe { &mut *(ctx as *mut __ErOverlayPluginState) })
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn er_overlay_plugin_abi_version() -> u32 {
            $crate::ABI_VERSION
        }

        #[no_mangle]
        pub unsafe extern "C" fn er_overlay_plugin_create(
            host: *const $crate::abi::ErHostInfo,
        ) -> *mut ::std::ffi::c_void {
            let host = $crate::HostInfo::from_raw(host);
            let mut plugin = $init;
            $crate::MetricPlugin::on_create(&mut plugin, &host);
            match __er_overlay_build_state(plugin) {
                Some(state) => Box::into_raw(Box::new(state)) as *mut ::std::ffi::c_void,
                None => ::std::ptr::null_mut(),
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn er_overlay_plugin_metrics(
            ctx: *mut ::std::ffi::c_void,
            out_len: *mut usize,
        ) -> *const $crate::abi::ErMetricDesc {
            let Some(state) = __er_overlay_state(ctx) else {
                if !out_len.is_null() {
                    *out_len = 0;
                }
                return ::std::ptr::null();
            };
            if !out_len.is_null() {
                *out_len = state.descs.len();
            }
            state.descs.as_ptr()
        }

        #[no_mangle]
        pub unsafe extern "C" fn er_overlay_plugin_poll(
            ctx: *mut ::std::ffi::c_void,
            tick_ms: u64,
        ) {
            let Some(state) = __er_overlay_state(ctx) else {
                return;
            };
            state.texts.clear();
            $crate::MetricPlugin::poll(&mut state.plugin, tick_ms);
        }

        #[no_mangle]
        pub unsafe extern "C" fn er_overlay_plugin_sample(
            ctx: *mut ::std::ffi::c_void,
            index: usize,
        ) -> $crate::abi::ErMetricSample {
            let unavailable = $crate::abi::ErMetricSample {
                value: 0,
                max: 0,
                text: ::std::ptr::null(),
                available: 0,
                has_max: 0,
                _pad: [0; 2],
                _reserved: 0,
            };
            let Some(state) = __er_overlay_state(ctx) else {
                return unavailable;
            };
            if index >= state.descs.len() {
                return unavailable;
            }
            let sample = $crate::MetricPlugin::sample(&mut state.plugin, index).into_parts();
            let text = match sample.text {
                Some(text) => {
                    let Ok(c_text) = ::std::ffi::CString::new(text.replace('\0', "")) else {
                        return unavailable;
                    };
                    let ptr = c_text.as_ptr();
                    state.texts.push(c_text);
                    ptr
                }
                None => ::std::ptr::null(),
            };
            $crate::abi::ErMetricSample {
                value: sample.value,
                max: sample.max.unwrap_or(0),
                text,
                available: u8::from(sample.available),
                has_max: u8::from(sample.max.is_some()),
                _pad: [0; 2],
                _reserved: 0,
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn er_overlay_plugin_destroy(ctx: *mut ::std::ffi::c_void) {
            if ctx.is_null() {
                return;
            }
            drop(Box::from_raw(ctx as *mut __ErOverlayPluginState));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_host_info_is_empty() {
        let host = unsafe { HostInfo::from_raw(std::ptr::null()) };
        assert!(host.overlay_version.is_empty());
        assert!(host.base_dir.is_empty());
        assert!(host.log.is_none());
    }

    #[test]
    fn count_sample_has_no_max_unless_asked() {
        let plain = MetricSample::count(42);
        assert!(plain.available);
        assert!(plain.max.is_none());
        let capped = MetricSample::count_with_max(1, 4);
        assert_eq!(capped.max, Some(4));
        assert!(!MetricSample::unavailable().available);
        assert_eq!(MetricSample::text("S").text.as_deref(), Some("S"));
    }
}

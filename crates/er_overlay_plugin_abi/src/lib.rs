//! Stable C ABI (version 1) between the overlay and a metric plugin DLL.
//!
//! Plugins are black boxes: they declare metric ids and return a number (or a short string).
//! Label, icon and `show_max` belong to the layout tile, not to the plugin.
//!
//! Layout of every `#[repr(C)]` struct is part of the contract. The tests in this crate pin
//! `size_of` / `align_of` / field offsets so an accidental change fails compilation instead of
//! silently breaking already-shipped plugins. The C equivalent lives in `sdk/er_overlay_plugin.h`.

use std::ffi::c_char;

/// ABI version this crate speaks. A plugin reporting anything else is not loaded.
pub const ABI_VERSION: u32 = 1;

/// Integer counter. Rendered as `N` or `N/M` when the sample carries a max.
pub const ER_METRIC_COUNT: u32 = 0;
/// Duration in milliseconds, rendered `HH:MM:SS` like in-game time.
pub const ER_METRIC_TIME_MS: u32 = 1;
/// Free-form text (a rank such as `S`). Not usable as a challenge personal-best source.
pub const ER_METRIC_TEXT: u32 = 2;

pub const ER_LOG_ERROR: u32 = 0;
pub const ER_LOG_WARN: u32 = 1;
pub const ER_LOG_INFO: u32 = 2;
pub const ER_LOG_DEBUG: u32 = 3;
pub const ER_LOG_TRACE: u32 = 4;

/// How a sample should be interpreted. Unknown raw values are rejected at load time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MetricKind {
    Count = ER_METRIC_COUNT,
    TimeMs = ER_METRIC_TIME_MS,
    Text = ER_METRIC_TEXT,
}

impl MetricKind {
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            ER_METRIC_COUNT => Some(Self::Count),
            ER_METRIC_TIME_MS => Some(Self::TimeMs),
            ER_METRIC_TEXT => Some(Self::Text),
            _ => None,
        }
    }

    pub fn as_u32(self) -> u32 {
        self as u32
    }

    /// Stable name written to `plugins/metrics.json` for the layout editor.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Count => "count",
            Self::TimeMs => "time_ms",
            Self::Text => "text",
        }
    }
}

/// One metric the plugin exposes. Presentation (label, icon) is not part of this struct.
///
/// `id` is UTF-8, NUL-terminated, and must stay valid until `er_overlay_plugin_destroy`.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ErMetricDesc {
    pub id: *const c_char,
    pub kind: u32,
    pub _reserved: [u32; 3],
}

/// One reading. `text` is only meaningful for [`ER_METRIC_TEXT`] and must stay valid until the
/// next `er_overlay_plugin_poll`. `max` is ignored unless `has_max` is non-zero, and it may
/// change from tick to tick.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ErMetricSample {
    pub value: i64,
    pub max: i64,
    pub text: *const c_char,
    pub available: u8,
    pub has_max: u8,
    pub _pad: [u8; 2],
    pub _reserved: u32,
}

/// Passed to `er_overlay_plugin_create`. Do not retain this pointer. The two strings and `log`
/// stay valid until `er_overlay_plugin_destroy` returns; copy the strings if you need them later.
///
/// `log` may be null. Levels are the `ER_LOG_*` constants.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct ErHostInfo {
    pub abi_version: u32,
    pub _pad: u32,
    pub overlay_version: *const c_char,
    pub base_dir: *const c_char,
    pub log: Option<unsafe extern "C" fn(u32, *const c_char)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_struct_layout_is_pinned() {
        assert_eq!(std::mem::size_of::<ErMetricDesc>(), 24);
        assert_eq!(std::mem::align_of::<ErMetricDesc>(), 8);
        assert_eq!(std::mem::offset_of!(ErMetricDesc, id), 0);
        assert_eq!(std::mem::offset_of!(ErMetricDesc, kind), 8);
        assert_eq!(std::mem::offset_of!(ErMetricDesc, _reserved), 12);

        assert_eq!(std::mem::size_of::<ErMetricSample>(), 32);
        assert_eq!(std::mem::align_of::<ErMetricSample>(), 8);
        assert_eq!(std::mem::offset_of!(ErMetricSample, value), 0);
        assert_eq!(std::mem::offset_of!(ErMetricSample, max), 8);
        assert_eq!(std::mem::offset_of!(ErMetricSample, text), 16);
        assert_eq!(std::mem::offset_of!(ErMetricSample, available), 24);
        assert_eq!(std::mem::offset_of!(ErMetricSample, has_max), 25);
        assert_eq!(std::mem::offset_of!(ErMetricSample, _pad), 26);
        assert_eq!(std::mem::offset_of!(ErMetricSample, _reserved), 28);

        assert_eq!(std::mem::size_of::<ErHostInfo>(), 32);
        assert_eq!(std::mem::align_of::<ErHostInfo>(), 8);
        assert_eq!(std::mem::offset_of!(ErHostInfo, abi_version), 0);
        assert_eq!(std::mem::offset_of!(ErHostInfo, _pad), 4);
        assert_eq!(std::mem::offset_of!(ErHostInfo, overlay_version), 8);
        assert_eq!(std::mem::offset_of!(ErHostInfo, base_dir), 16);
        assert_eq!(std::mem::offset_of!(ErHostInfo, log), 24);
    }

    #[test]
    fn metric_kind_roundtrip() {
        for kind in [MetricKind::Count, MetricKind::TimeMs, MetricKind::Text] {
            assert_eq!(MetricKind::from_u32(kind.as_u32()), Some(kind));
        }
        assert_eq!(MetricKind::from_u32(99), None);
        assert_eq!(MetricKind::Count.as_str(), "count");
        assert_eq!(MetricKind::TimeMs.as_str(), "time_ms");
        assert_eq!(MetricKind::Text.as_str(), "text");
    }
}

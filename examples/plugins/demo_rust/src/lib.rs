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

    fn progress(&self) -> i64 {
        ((self.tick_ms / 1000) % 61) as i64
    }

    fn rank(&self) -> &'static str {
        match self.progress() {
            0..=14 => "C",
            15..=29 => "B",
            30..=44 => "A",
            _ => "S",
        }
    }
}

impl MetricPlugin for Demo {
    fn metrics() -> &'static [&'static str] {
        &["demo_rust.uptime", "demo_rust.progress", "demo_rust.rank"]
    }

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
            &format!(
                "demo_rust: overlay {} @ {}",
                host.overlay_version, host.base_dir
            ),
        );
    }

    fn poll(&mut self, tick_ms: u64) {
        self.tick_ms = tick_ms;
    }

    fn sample(&mut self, index: usize) -> MetricSample {
        match index {
            0 => MetricSample::time_ms(self.tick_ms as i64),
            1 => MetricSample::count_with_max(self.progress(), 60),
            2 => MetricSample::text(self.rank()),
            _ => MetricSample::unavailable(),
        }
    }
}

declare_plugin!(Demo, Demo::new());

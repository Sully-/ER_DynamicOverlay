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

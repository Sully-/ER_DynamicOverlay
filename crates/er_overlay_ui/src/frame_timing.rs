//! Sliding-window Present-thread timing for the overlay render path.

use std::time::{Duration, Instant};

/// Last completed timing window (≈1 s) plus the most recent frame's phase costs.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameTimingSnapshot {
    /// Most recent frame: config reload phase (µs).
    pub reload_us: u64,
    /// Most recent frame: view-model drain (µs).
    pub drain_us: u64,
    /// Most recent frame: ImGui draw (µs).
    pub draw_us: u64,
    /// Most recent frame: menu-cursor write (µs).
    pub cursor_us: u64,
    /// Most recent frame: total overlay work (µs).
    pub total_us: u64,
    /// Average total over the last completed 1 s window (µs).
    pub avg_total_us: u64,
    /// Max total over the last completed 1 s window (µs).
    pub max_total_us: u64,
    /// Samples in the last completed window.
    pub samples: u32,
}

/// Accumulates per-phase Present costs and publishes a snapshot every second.
pub struct FrameTimingAccum {
    window_start: Instant,
    sum_total_us: u64,
    max_total_us: u64,
    samples: u32,
    snapshot: FrameTimingSnapshot,
}

impl Default for FrameTimingAccum {
    fn default() -> Self {
        Self {
            window_start: Instant::now(),
            sum_total_us: 0,
            max_total_us: 0,
            samples: 0,
            snapshot: FrameTimingSnapshot::default(),
        }
    }
}

impl FrameTimingAccum {
    pub fn record(
        &mut self,
        reload: Duration,
        drain: Duration,
        draw: Duration,
        cursor: Duration,
        total: Duration,
    ) {
        let reload_us = reload.as_micros().min(u64::MAX as u128) as u64;
        let drain_us = drain.as_micros().min(u64::MAX as u128) as u64;
        let draw_us = draw.as_micros().min(u64::MAX as u128) as u64;
        let cursor_us = cursor.as_micros().min(u64::MAX as u128) as u64;
        let total_us = total.as_micros().min(u64::MAX as u128) as u64;

        self.snapshot.reload_us = reload_us;
        self.snapshot.drain_us = drain_us;
        self.snapshot.draw_us = draw_us;
        self.snapshot.cursor_us = cursor_us;
        self.snapshot.total_us = total_us;

        self.sum_total_us = self.sum_total_us.saturating_add(total_us);
        self.max_total_us = self.max_total_us.max(total_us);
        self.samples = self.samples.saturating_add(1);

        if self.window_start.elapsed() >= Duration::from_secs(1) {
            let avg = if self.samples > 0 {
                self.sum_total_us / u64::from(self.samples)
            } else {
                0
            };
            self.snapshot.avg_total_us = avg;
            self.snapshot.max_total_us = self.max_total_us;
            self.snapshot.samples = self.samples;
            self.window_start = Instant::now();
            self.sum_total_us = 0;
            self.max_total_us = 0;
            self.samples = 0;
        }
    }

    pub fn snapshot(&self) -> FrameTimingSnapshot {
        self.snapshot
    }
}

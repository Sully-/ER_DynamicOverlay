use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameTime {
    pub total_ms: u32,
}

impl GameTime {
    pub fn from_ms(total_ms: u32) -> Self {
        Self { total_ms }
    }

    pub fn hours(&self) -> u32 {
        self.total_ms / 3_600_000
    }

    pub fn minutes(&self) -> u32 {
        (self.total_ms % 3_600_000) / 60_000
    }

    pub fn seconds(&self) -> u32 {
        (self.total_ms % 60_000) / 1_000
    }

    pub fn format_hms(&self) -> String {
        format!(
            "{:02}:{:02}:{:02}",
            self.hours(),
            self.minutes(),
            self.seconds()
        )
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameStateDiagnostics {
    pub backend: BackendKind,
    pub gamedata_man_resolved: bool,
    pub event_flag_man_resolved: bool,
    pub world_chr_man_resolved: bool,
    pub igt_readable: bool,
    pub death_count_readable: bool,
    pub inventory_readable: bool,
    pub boss_flags_loaded: u32,
    pub great_rune_flags_loaded: u32,
    pub field_area_resolved: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    #[default]
    Unavailable,
    FromSoftwareRs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackKind {
    Unique { acquired: Option<bool> },
    Countable { count: Option<u32> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_time_formatting() {
        let t = GameTime::from_ms(3_661_000);
        assert_eq!(t.format_hms(), "01:01:01");
    }
}

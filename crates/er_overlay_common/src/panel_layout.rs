//! Panel position/size parsing (compatible with ER_boss_checklist_R `panel_pos`).

/// Resolved window geometry in screen pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelRect {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub pivot: [f32; 2],
}

/// Parses `x,y,width,height` where each value may be pixels or a percentage (`50%`).
/// Returns `None` for `auto`, empty, or invalid input.
pub fn parse_panel_layout(raw: &str) -> Option<[f32; 4]> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        return None;
    }

    let values = split_layout_values(trimmed);
    if values.len() < 4 {
        return None;
    }

    Some([values[0], values[1], values[2], values[3]])
}

/// Converts parsed layout values to pixel position, size and ImGui pivot.
pub fn resolve_panel_rect(viewport: [f32; 2], spec: [f32; 4]) -> PanelRect {
    let pivot = [
        if spec[0] >= 0.0 { 0.0 } else { 1.0 },
        if spec[1] >= 0.0 { 0.0 } else { 1.0 },
    ];
    PanelRect {
        pos: [
            resolve_axis(viewport[0], spec[0]),
            resolve_axis(viewport[1], spec[1]),
        ],
        size: [
            resolve_axis(viewport[0], spec[2]),
            resolve_axis(viewport[1], spec[3]),
        ],
        pivot,
    }
}

fn resolve_axis(axis_len: f32, n: f32) -> f32 {
    if n >= 1.0 {
        n
    } else if n >= 0.0 {
        axis_len * n
    } else if n <= -1.0 {
        axis_len + n
    } else {
        axis_len + axis_len * n
    }
}

fn split_layout_values(s: &str) -> Vec<f32> {
    let mut elems = Vec::new();

    for item in s.split(',') {
        let item = item.trim();
        if item.is_empty() {
            elems.push(0.0);
            continue;
        }

        let mut token = item.to_string();
        if token.ends_with('%') {
            token.pop();
            if let Ok(val) = token.parse::<f32>() {
                elems.push(val.clamp(-99.9999, 99.9999) / 100.0);
                continue;
            }
        }

        if let Ok(val) = token.parse::<f32>() {
            elems.push(val);
        }
    }

    elems
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_auto_is_none() {
        assert!(parse_panel_layout("auto").is_none());
        assert!(parse_panel_layout("").is_none());
    }

    #[test]
    fn parse_checklist_r_default() {
        let spec = parse_panel_layout("-5,10,50%,92%").unwrap();
        assert_eq!(spec[0], -5.0);
        assert_eq!(spec[1], 10.0);
        assert!((spec[2] - 0.5).abs() < f32::EPSILON);
        assert!((spec[3] - 0.92).abs() < f32::EPSILON);
    }

    #[test]
    fn resolve_matches_checklist_r_semantics() {
        let spec = parse_panel_layout("-5,10,50%,92%").unwrap();
        let rect = resolve_panel_rect([1920.0, 1080.0], spec);
        assert_eq!(rect.pos, [1915.0, 10.0]);
        assert_eq!(rect.pivot, [1.0, 0.0]);
        assert_eq!(rect.size[0], 960.0);
        assert!((rect.size[1] - 993.6).abs() < 0.01);
    }

    #[test]
    fn resolve_percent_position() {
        let spec = parse_panel_layout("75%, 8%, 22%, 45%").unwrap();
        let rect = resolve_panel_rect([1920.0, 1080.0], spec);
        assert_eq!(rect.pos, [1440.0, 86.4]);
        assert_eq!(rect.pivot, [0.0, 0.0]);
        assert_eq!(rect.size, [422.4, 486.0]);
    }
}

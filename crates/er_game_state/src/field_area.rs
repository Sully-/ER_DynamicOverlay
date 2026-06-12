use std::sync::Mutex;

use eldenring::cs::{BlockId, PlayerIns};
use fromsoftware_shared::program::Program;
use pelite::pattern;
use pelite::pe64::Pe;
use tracing::debug;

/// Pattern for the global `FieldArea` pointer (same as ER_boss_checklist_R).
/// Leading `'` captures the match RVA in `save[0]`.
const FIELD_AREA_PATTERN: &[pelite::pattern::Atom] = pattern!(
    "' 48 8B 0D ? ? ? ? 48 ? ? ? 44 0F B6 61 ? E8 ? ? ? ? 48 63 87 ? ? ? ? 48 ? ? ? 48 85 C0"
);

static FIELD_AREA_STATIC: Mutex<Option<usize>> = Mutex::new(None);

fn resolve_field_area_static() -> Option<usize> {
    let pe = Program::current();
    let mut save = [0u32; 2];
    let mut matches = pe.scanner().matches_code(FIELD_AREA_PATTERN);
    if !matches.next(&mut save) {
        debug!("FieldArea pattern not found");
        return None;
    }
    let match_rva = save[0];
    let insn_va = pe.rva_to_va(match_rva).ok()? as usize;
    // `mov rcx, [rip+disp32]` — disp32 at +3, instruction length 7.
    let disp = i32::from_le_bytes([
        unsafe { *(insn_va as *const u8).add(3) },
        unsafe { *(insn_va as *const u8).add(4) },
        unsafe { *(insn_va as *const u8).add(5) },
        unsafe { *(insn_va as *const u8).add(6) },
    ]);
    let static_addr = insn_va.wrapping_add(7).wrapping_add(disp as usize);
    debug!("FieldArea static resolved at {static_addr:#x}");
    Some(static_addr)
}

fn field_area_static_addr() -> Option<usize> {
    let mut cache = FIELD_AREA_STATIC.lock().ok()?;
    if let Some(addr) = *cache {
        return Some(addr);
    }
    let addr = resolve_field_area_static()?;
    *cache = Some(addr);
    Some(addr)
}

/// Whether the `FieldArea` global pointer was resolved via pattern scan.
pub fn field_area_available() -> bool {
    field_area_static_addr().is_some()
}

/// Encodes a `BlockId` the same way bosses.json subregion keys work (`area * 100 + block`),
/// scaled by 1000 to match `FieldArea` map ids used in ER_boss_checklist_R.
pub fn subregion_id_from_block_id(block: BlockId) -> u32 {
    let key = block.area() as u32 * 100 + block.block() as u32;
    key * 1000
}

fn read_from_field_area() -> Option<u32> {
    let static_addr = field_area_static_addr()?;
    // Global holds a pointer to the live `FieldArea` instance.
    let field_area = unsafe { std::ptr::read_unaligned(static_addr as *const usize) };
    if field_area == 0 {
        return None;
    }
    // ER_boss_checklist_R: `*(field_area) + 0xE8` → map id (NOT an extra indirection via offset 0).
    let map_id = unsafe { std::ptr::read_unaligned(field_area.wrapping_add(0xE8) as *const u32) };
    (map_id != 0).then_some(map_id)
}

fn read_from_player() -> Option<u32> {
    let player = unsafe { PlayerIns::local_player().ok()? };
    let block = player.current_block_id;
    if block != BlockId::none() {
        let id = subregion_id_from_block_id(block);
        debug!(
            "map_id from player current_block_id {block}: {id} (key {})",
            id / 1000
        );
        return Some(id);
    }

    let play_region = player.play_region_id;
    if play_region != 0 {
        debug!("map_id from player play_region_id: {play_region}");
        return Some(play_region);
    }

    None
}

/// Reads the current map / subregion id. Tries `FieldArea + 0xE8` first, then the local
/// player's `current_block_id` / `play_region_id` as fallbacks.
pub fn read_current_subregion_id() -> Option<u32> {
    if let Some(id) = read_from_field_area() {
        debug!("map_id from FieldArea: {id} (key {})", id / 1000);
        return Some(id);
    }
    read_from_player()
}

#[cfg(test)]
mod tests {
    use super::*;
    use eldenring::cs::BlockId;

    #[test]
    fn limgrave_overworld_block_encodes_to_6100_key() {
        let block = BlockId::from_parts(61, 0, 0, 0);
        assert_eq!(subregion_id_from_block_id(block), 6_100_000);
    }

    #[test]
    fn stormfoot_catacombs_block_encodes_to_3002_key() {
        let block = BlockId::from_parts(30, 2, 0, 0);
        assert_eq!(subregion_id_from_block_id(block), 3_002_000);
    }
}

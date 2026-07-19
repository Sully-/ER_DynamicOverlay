#[cfg(feature = "game")]
pub mod game {
    use std::collections::HashSet;

    use eldenring::cs::{
        EquipInventoryData, EquipInventoryDataListEntry, ItemCategory, ItemId, WorldChrMan,
    };
    use fromsoftware_shared::{FromStatic, NonEmptyIteratorExt};
    use tracing::trace;

    use crate::ItemKind;

    fn item_category(kind: ItemKind) -> ItemCategory {
        match kind {
            ItemKind::Goods => ItemCategory::Goods,
            ItemKind::Accessory => ItemCategory::Accessory,
        }
    }

    /// Iterates every non-empty inventory entry, scanning each backing buffer
    /// across its full allocated capacity.
    ///
    /// This deliberately avoids the game's own `items()` iterator, which reads
    /// key items through `key_items_accessor` bounded by `key_items_len`. Two
    /// separate issues made that unreliable for the tracker:
    ///
    /// 1. **Multiplayer:** the game repoints `key_items_accessor` to
    ///    `multiplay_key_items` during *any* multiplayer session (co-op,
    ///    invasions, seamless co-op, ...). That list only mirrors the
    ///    regenerative-material / wondrous-physick-tear entries, so genuine key
    ///    items (medallions, keys, great runes, ...) vanish from `items()`.
    /// 2. **`key_items_len` under-counting:** unlike `normal_entries` (which the
    ///    game itself scans across `normal_items_capacity`), `key_entries` is
    ///    bounded by `key_items_len`. A key item sitting in a slot at or beyond
    ///    that length (e.g. items added out-of-band, or a transient torn read of
    ///    the length while the list is being reorganized) is then skipped — the
    ///    item is in the inventory in-game yet reads as absent, and can later
    ///    reappear once the list is recompacted.
    ///
    /// Scanning `key_items_head` / `multiplay_key_items_head` /
    /// `normal_items_head` across their `*_capacity` and dropping empty slots
    /// is safe (empty slots hold a known empty pattern, exactly how the game's
    /// `normal_entries` already works) and immune to both problems. Collecting
    /// into a `HashSet` downstream de-duplicates any overlap between the lists.
    fn all_inventory_entries(
        inv: &EquipInventoryData,
    ) -> impl Iterator<Item = &EquipInventoryDataListEntry> {
        let data = &inv.items_data;
        // Safety: `*_head` point to buffers allocated for `*_capacity` entries,
        // each initialized to a valid `MaybeEmpty` pattern (mirrors the game's
        // own `normal_entries`).
        let key = unsafe {
            std::slice::from_raw_parts(
                data.key_items_head.as_ptr(),
                data.key_items_capacity as usize,
            )
        };
        let multiplay = unsafe {
            std::slice::from_raw_parts(
                data.multiplay_key_items_head.as_ptr(),
                data.multiplay_key_items_capacity as usize,
            )
        };
        key.iter()
            .chain(multiplay.iter())
            .chain(data.normal_entries().iter())
            .non_empty()
    }

    pub fn quantity_of(category: ItemCategory, param_id: u32) -> Option<u32> {
        let wcm = unsafe { WorldChrMan::instance().ok()? };
        let player = wcm.main_player.as_ref()?;
        let pgd = unsafe { player.player_game_data.as_ref() };
        let inv = &pgd.equipment.equip_inventory_data;
        let target = ItemId::new(category, param_id).ok()?;
        let qty = all_inventory_entries(inv)
            .find(|e| e.item_id == target)
            .map(|e| e.quantity)
            .unwrap_or(0);
        trace!(param_id, qty, "inventory quantity");
        Some(qty)
    }

    /// Single inventory walk — use for per-frame talisman / accessory checks.
    ///
    /// Stores the full [`ItemId`] value (category + param id) so lookups can be
    /// scoped to a category and avoid collisions between, say, a goods param id
    /// and an unrelated accessory/key-item sharing the same numeric param id.
    pub fn owned_item_ids() -> Option<HashSet<u32>> {
        let wcm = unsafe { WorldChrMan::instance().ok()? };
        let player = wcm.main_player.as_ref()?;
        let pgd = unsafe { player.player_game_data.as_ref() };
        let inv = &pgd.equipment.equip_inventory_data;
        Some(
            all_inventory_entries(inv)
                .map(|e| e.item_id.into_inner())
                .collect(),
        )
    }

    /// Whether `param_id` of the given `kind` is present in `owned`
    /// (a set produced by [`owned_item_ids`]).
    pub fn owned_contains(owned: &HashSet<u32>, param_id: u32, kind: ItemKind) -> bool {
        match ItemId::new(item_category(kind), param_id) {
            Ok(id) => owned.contains(&id.into_inner()),
            Err(_) => false,
        }
    }

    fn collect_equipped_ids(entries: &eldenring::cs::ChrAsmEquipEntries) -> HashSet<u32> {
        let mut ids = HashSet::new();
        for slot in entries
            .accessories
            .iter()
            .chain(std::iter::once(&entries.covenant))
            .chain(entries.quick_tems.iter())
            .chain(entries.pouch.iter())
        {
            if let Some(id) = slot.as_valid() {
                ids.insert(id.into_inner());
            }
        }
        ids
    }

    /// All currently equipped item ids (accessories, covenant, quick slots, pouch).
    pub fn equipped_item_ids() -> Option<HashSet<u32>> {
        let wcm = unsafe { WorldChrMan::instance().ok()? };
        let player = wcm.main_player.as_ref()?;
        let pgd = unsafe { player.player_game_data.as_ref() };
        Some(collect_equipped_ids(&pgd.equipment.equipment_entries))
    }

    /// Whether `param_id` of the given `kind` is present in `equipped`
    /// (a set produced by [`equipped_item_ids`]).
    pub fn equipped_contains(equipped: &HashSet<u32>, param_id: u32, kind: ItemKind) -> bool {
        match ItemId::new(item_category(kind), param_id) {
            Ok(id) => equipped.contains(&id.into_inner()),
            Err(_) => false,
        }
    }

    pub fn inventory_available() -> bool {
        unsafe { WorldChrMan::instance().is_ok() }
    }
}

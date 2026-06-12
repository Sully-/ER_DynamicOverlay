#[cfg(feature = "game")]
pub mod game {
    use std::collections::HashSet;

    use eldenring::cs::{ItemCategory, ItemId, WorldChrMan};
    use fromsoftware_shared::FromStatic;
    use tracing::trace;

    use crate::ItemKind;

    fn item_category(kind: ItemKind) -> ItemCategory {
        match kind {
            ItemKind::Goods => ItemCategory::Goods,
            ItemKind::Accessory => ItemCategory::Accessory,
        }
    }

    pub fn quantity_of(category: ItemCategory, param_id: u32) -> Option<u32> {
        let wcm = unsafe { WorldChrMan::instance().ok()? };
        let player = wcm.main_player.as_ref()?;
        let pgd = unsafe { player.player_game_data.as_ref() };
        let inv = &pgd.equipment.equip_inventory_data;
        let target = ItemId::new(category, param_id).ok()?;
        let qty = inv
            .items_data
            .items()
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
            inv.items_data
                .items()
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

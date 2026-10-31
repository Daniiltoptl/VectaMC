//! `minecraft:container` component helpers for block-entity inventories.
//!
//! Vanilla `BaseContainerBlockEntity` stores its inventory implicitly in the
//! `minecraft:container` component so a picked block item carries the contents
//! instead of duplicating them in `Items`.

use std::io;

use vecta_registry::data_components::components::ItemContainerContents;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::item_stack_template::ItemStackTemplate;

/// Builds the implicit container component for `items`.
///
/// Trailing empty slots are dropped, matching `ItemContainerContents.fromItems`.
pub fn collect_container_component(items: &[ItemStack]) -> io::Result<ItemContainerContents> {
    let Some(last_non_empty) = items.iter().rposition(|item| !item.is_empty()) else {
        return Ok(ItemContainerContents::empty());
    };

    let mut contents = Vec::with_capacity(last_non_empty + 1);
    for item in &items[..=last_non_empty] {
        if item.is_empty() {
            contents.push(None);
        } else {
            contents.push(Some(ItemStackTemplate::from_stack(item)?));
        }
    }
    ItemContainerContents::new(contents)
}

/// Copies `items` into `destination`, clearing the slots Vanilla clears.
///
/// `ItemContainerContents.copyInto` fills the whole destination with empty
/// stacks first, so slots beyond the stored contents are emptied.
pub fn apply_container_items(destination: &mut [ItemStack], items: Vec<ItemStack>) {
    destination.fill_with(ItemStack::empty);
    for (slot, item) in destination.iter_mut().zip(items) {
        *slot = item;
    }
}

#[cfg(test)]
mod tests {
    use vecta_registry::{init_vanilla_registry, vanilla_items};

    use super::*;

    #[test]
    fn trailing_empty_slots_are_dropped() {
        init_vanilla_registry();
        let mut items = vec![ItemStack::empty(); 27];
        items[0] = ItemStack::new(&vanilla_items::STONE);
        items[2] = ItemStack::with_count(&vanilla_items::DIAMOND, 4);

        let contents = collect_container_component(&items).expect("items encode");

        assert_eq!(contents.items().len(), 3);
        assert!(contents.items()[1].is_none());
    }

    #[test]
    fn applying_clears_slots_beyond_the_stored_contents() {
        init_vanilla_registry();
        let mut destination = vec![ItemStack::new(&vanilla_items::STONE); 3];

        apply_container_items(
            &mut destination,
            vec![ItemStack::new(&vanilla_items::DIAMOND)],
        );

        assert!(destination[0].is(&vanilla_items::DIAMOND));
        assert!(destination[1].is_empty());
        assert!(destination[2].is_empty());
    }
}

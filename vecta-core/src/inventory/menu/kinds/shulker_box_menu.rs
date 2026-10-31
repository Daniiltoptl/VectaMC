//! Shulker box menu.

use vecta_registry::vanilla_menu_types;
use vecta_utils::{DowncastType, DowncastTypeKey};

use crate::behavior::ITEM_BEHAVIORS;
use crate::block_entity::entities::SHULKER_BOX_SLOTS;
use crate::inventory::prelude::*;
use crate::player::player_inventory::PlayerInventory;

/// Builds a shulker box menu with 3 rows of 9 slots plus the player inventory.
#[must_use]
pub fn shulker_box(
    inventory: Shared<PlayerInventory>,
    container_id: u8,
    container: impl Into<ContainerRef>,
) -> Menu {
    let container = container.into();
    shulker_box_with_kind(
        inventory,
        container_id,
        container.clone(),
        ShulkerBoxKind::new(container),
    )
}

/// Builds the same layout as [`shulker_box`] under a caller-supplied menu kind.
#[must_use]
pub fn shulker_box_with_kind(
    inventory: Shared<PlayerInventory>,
    container_id: u8,
    container: impl Into<ContainerRef>,
    kind: impl MenuKind + 'static,
) -> Menu {
    let container = container.into();

    let mut builder = MenuBuilder::new(&vanilla_menu_types::SHULKER_BOX, container_id);
    let shulker_box = builder.section_with(
        &container,
        SHULKER_BOX_SLOTS,
        SectionKind::restricted(|_slot, stack| {
            ITEM_BEHAVIORS
                .get_behavior(stack.item())
                .can_fit_inside_container_items()
        }),
    );
    let player = builder.player_inventory(&inventory);

    builder.route(shulker_box, player.all(), FillDirection::Backward);
    builder.route(player.all(), shulker_box, FillDirection::Forward);

    builder.build(kind)
}

/// Callback invoked once when a shulker box menu closes.
pub type ShulkerBoxCloseHook = Box<dyn Fn(&Player) + Send + Sync>;

/// Per-menu shulker box state: the backing container for the validity check and
/// an optional close hook, mirroring `ShulkerBoxMenu.removed`.
pub struct ShulkerBoxKind {
    container: ContainerRef,
    on_close: Option<ShulkerBoxCloseHook>,
}

// SAFETY: This Vecta-owned key uniquely identifies the concrete menu kind
// within the process.
unsafe impl DowncastType for ShulkerBoxKind {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:menu/shulker_box");
}

impl ShulkerBoxKind {
    /// Creates a shulker box kind for `container`.
    #[must_use]
    pub const fn new(container: ContainerRef) -> Self {
        Self {
            container,
            on_close: None,
        }
    }

    /// Adds a callback that runs when the menu closes.
    #[must_use]
    pub fn with_close_hook(mut self, hook: impl Fn(&Player) + Send + Sync + 'static) -> Self {
        self.on_close = Some(Box::new(hook));
        self
    }
}

impl MenuKind for ShulkerBoxKind {
    fn still_valid(&self, _behavior: &MenuBehavior, player: &Player) -> bool {
        self.container.still_valid(player)
    }

    /// `ShulkerBoxMenu.removed` calls `container.stopOpen(player)`.
    fn removed(&mut self, _behavior: &mut MenuBehavior, player: &Player) {
        if let Some(on_close) = &self.on_close {
            on_close(player);
        }
    }
}

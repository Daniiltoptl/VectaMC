use std::sync::Arc;

use super::{CraftingKind, crafting};
use crate::{
    behavior::init_behaviors,
    entity::{Entity as _, entities::ItemEntity},
    inventory::{
        click::{Click, MouseButton},
        container::Container as _,
    },
    test_support::{TestPlayerBuilder, fresh_test_world, insert_ready_full_chunk},
};
use glam::DVec3;
use vecta_registry::{
    init_vanilla_registry, item_stack::ItemStack, vanilla_blocks, vanilla_entities, vanilla_items,
};
use vecta_utils::types::UpdateFlags;
use vecta_utils::{BlockPos, ChunkPos, Downcast as _, WorldAabb};

#[test]
fn partial_result_overflow_uses_the_default_drop_policy() {
    init_vanilla_registry();
    init_behaviors();
    let world_fixture = fresh_test_world("crafting_menu_partial_result_overflow");
    let world = &world_fixture.world;
    let pos = BlockPos::new(0, 64, 0);
    insert_ready_full_chunk(world, ChunkPos::from_block_pos(pos));
    assert!(world.set_block(
        pos,
        vanilla_blocks::CRAFTING_TABLE.default_state(),
        UpdateFlags::UPDATE_ALL,
    ));
    let player = TestPlayerBuilder::new(Arc::clone(world), "Crafter", 1).build();
    player.base().set_position_local(DVec3::new(0.5, 64.0, 0.5));
    let mut menu = crafting(Arc::clone(&player.inventory), 1, pos);
    let Some(kind) = menu.kind().downcast_ref::<CraftingKind>() else {
        panic!("crafting builder should create a crafting menu");
    };
    let (crafting_container, result_id) =
        (kind.handler.crafting_container(), kind.handler.result_id());

    *menu.behavior_mut().carried_mut() = ItemStack::new(&vanilla_items::OAK_LOG);
    menu.clicked(
        Click::Pickup {
            slot: 1,
            button: MouseButton::Left,
        },
        &player,
    )
    .expect("menu operation should succeed");
    let result = menu
        .behavior()
        .lock_all_containers()
        .get(result_id)
        .expect("result container is registered with the menu")
        .get_item(0)
        .clone();
    assert!(result.is(&vanilla_items::OAK_PLANKS));
    assert_eq!(result.count(), 4);

    let mut matching = result;
    matching.set_count(63);
    {
        let mut inventory = player.inventory.lock();
        for slot in 0..36 {
            inventory.set_item(slot, ItemStack::with_count(&vanilla_items::DIRT, 64));
        }
        inventory.set_item(8, matching);
    }

    menu.clicked(Click::QuickMove { slot: 0 }, &player)
        .expect("menu operation should succeed");

    assert!(crafting_container.lock().get_item(0).is_empty());
    assert!(
        menu.behavior()
            .lock_all_containers()
            .get(result_id)
            .expect("result container is registered with the menu")
            .get_item(0)
            .is_empty()
    );
    assert_eq!(player.inventory.lock().get_item(8).count(), 64);
    let dropped = world.get_entities_in_aabb_matching(
        &WorldAabb::new(-2.0, 62.0, -2.0, 2.0, 68.0, 2.0),
        |entity| entity.entity_type() == &vanilla_entities::ITEM,
    );
    assert_eq!(dropped.len(), 1);
    let Some(item) = dropped[0].as_ref().downcast_ref::<ItemEntity>() else {
        panic!("dropped entity should retain its concrete item type");
    };
    assert!(item.get_item().is(&vanilla_items::OAK_PLANKS));
    assert_eq!(item.get_item().count(), 3);
    assert_eq!(item.get_thrower(), None);
}

/// Crafting-result accounting: the real count, the crafted statistic and the
/// take paths that vanilla 26.3 distinguishes.
mod crafted_result {
    use std::sync::Arc;

    use glam::DVec3;

    use crate::{
        behavior::init_behaviors,
        entity::{Entity as _, entities::ItemEntity},
        inventory::{
            click::{Click, MouseButton, SwapTarget},
            container::{Container as _, CraftingContainer},
            lock::ContainerId,
            menu::{Menu, kinds::CraftingKind},
        },
        player::Player,
        test_support::{TestPlayerBuilder, TestWorld, fresh_test_world, insert_ready_full_chunk},
    };
    use vecta_registry::{
        init_vanilla_registry, item_stack::ItemStack, items::ItemRef, stat::vanilla_stat_types,
        vanilla_blocks, vanilla_entities, vanilla_items,
    };
    use vecta_utils::locks::Shared;
    use vecta_utils::types::{GameType, UpdateFlags};
    use vecta_utils::{BlockPos, ChunkPos, Downcast as _, WorldAabb};

    /// Menu slot index of the crafting result.
    const RESULT_SLOT: usize = 0;
    /// Menu slot index of the first player-inventory slot.
    const FIRST_INVENTORY_SLOT: usize = 10;

    struct Fixture {
        world: TestWorld,
        player: Arc<Player>,
        menu: Menu,
        grid: Shared<CraftingContainer>,
        result: ContainerId,
    }

    fn fixture(name: &'static str) -> Fixture {
        init_vanilla_registry();
        init_behaviors();
        let world_fixture = fresh_test_world(name);
        let pos = BlockPos::new(0, 64, 0);
        insert_ready_full_chunk(&world_fixture.world, ChunkPos::from_block_pos(pos));
        assert!(world_fixture.world.set_block(
            pos,
            vanilla_blocks::CRAFTING_TABLE.default_state(),
            UpdateFlags::UPDATE_ALL,
        ));
        let player = TestPlayerBuilder::new(Arc::clone(&world_fixture.world), name, 1).build();
        player.base().set_position_local(DVec3::new(0.5, 64.0, 0.5));
        let menu = super::crafting(Arc::clone(&player.inventory), 1, pos);
        let Some(kind) = menu.kind().downcast_ref::<CraftingKind>() else {
            panic!("crafting builder should create a crafting menu");
        };
        let grid = kind.handler.crafting_container();
        let result = kind.handler.result_id();
        Fixture {
            world: world_fixture,
            player,
            menu,
            grid,
            result,
        }
    }

    impl Fixture {
        fn set_grid(&self, slot: usize, stack: ItemStack) {
            self.grid.lock().set_item(slot, stack);
        }

        /// Recomputes the preview through a real click on an empty slot.
        fn refresh(&mut self) {
            self.click(Click::QuickMove {
                slot: FIRST_INVENTORY_SLOT,
            });
        }

        fn click(&mut self, click: Click) {
            self.menu
                .clicked(click, &self.player)
                .expect("menu operation should succeed");
        }

        fn preview(&self) -> ItemStack {
            self.menu
                .behavior()
                .lock_all_containers()
                .get(self.result)
                .expect("result container is registered with the menu")
                .get_item(0)
                .clone()
        }

        fn carried(&self) -> ItemStack {
            self.menu.behavior().carried().clone()
        }

        fn crafted(&self, item: ItemRef) -> i32 {
            self.player
                .get_stat(vanilla_stat_types::ITEM_CRAFTED.get(item))
        }

        fn inventory_count(&self, item: ItemRef) -> i32 {
            let inventory = self.player.inventory.lock();
            (0..36)
                .map(|slot| {
                    let stack = inventory.get_item(slot);
                    if stack.is(item) { stack.count() } else { 0 }
                })
                .sum()
        }

        fn dropped_items(&self) -> Vec<ItemStack> {
            self.world
                .world
                .get_entities_in_aabb_matching(
                    &WorldAabb::new(-2.0, 62.0, -2.0, 2.0, 68.0, 2.0),
                    |entity| entity.entity_type() == &vanilla_entities::ITEM,
                )
                .into_iter()
                .map(|entity| {
                    let Some(item) = entity.as_ref().downcast_ref::<ItemEntity>() else {
                        panic!("dropped entity should retain its concrete item type");
                    };
                    item.get_item()
                })
                .collect()
        }

        fn fill_grid_with_a_log(&mut self) {
            self.set_grid(0, ItemStack::new(&vanilla_items::OAK_LOG));
            self.refresh();
        }
    }

    fn planks() -> ItemRef {
        ItemStack::new(&vanilla_items::OAK_PLANKS).item()
    }

    #[test]
    fn taking_the_result_awards_the_whole_output_count() {
        let mut fixture = fixture("CrafterCount");
        fixture.fill_grid_with_a_log();
        assert_eq!(fixture.preview().count(), 4);

        fixture.click(Click::Pickup {
            slot: RESULT_SLOT,
            button: MouseButton::Left,
        });

        assert!(fixture.carried().is(&vanilla_items::OAK_PLANKS));
        assert_eq!(fixture.carried().count(), 4);
        assert_eq!(fixture.crafted(planks()), 4);
        assert!(fixture.grid.lock().get_item(0).is_empty());
    }

    #[test]
    fn taking_half_of_the_result_takes_and_counts_half() {
        let mut fixture = fixture("CrafterHalf");
        fixture.fill_grid_with_a_log();

        fixture.click(Click::Pickup {
            slot: RESULT_SLOT,
            button: MouseButton::Right,
        });

        assert_eq!(fixture.carried().count(), 2);
        assert_eq!(fixture.crafted(planks()), 2);
    }

    #[test]
    fn a_second_craft_adds_to_the_previous_crafted_count() {
        let mut fixture = fixture("CrafterTwice");
        fixture.fill_grid_with_a_log();
        fixture.click(Click::Pickup {
            slot: RESULT_SLOT,
            button: MouseButton::Left,
        });

        fixture.fill_grid_with_a_log();
        fixture.click(Click::Pickup {
            slot: RESULT_SLOT,
            button: MouseButton::Left,
        });

        assert_eq!(fixture.crafted(planks()), 8);
        assert_eq!(
            fixture.inventory_count(planks()) + fixture.carried().count(),
            8
        );
    }

    #[test]
    fn shift_clicking_the_result_awards_the_moved_count() {
        let mut fixture = fixture("CrafterShift");
        fixture.fill_grid_with_a_log();

        fixture.click(Click::QuickMove { slot: RESULT_SLOT });

        assert_eq!(fixture.inventory_count(planks()), 4);
        assert!(fixture.preview().is_empty());
        assert_eq!(fixture.crafted(planks()), 4);
    }

    #[test]
    fn swapping_the_result_into_an_empty_hotbar_slot_awards_its_count() {
        let mut fixture = fixture("CrafterSwap");
        fixture.fill_grid_with_a_log();

        fixture.click(Click::Swap {
            slot: RESULT_SLOT,
            with: SwapTarget::Hotbar(0),
        });

        assert!(
            fixture
                .player
                .inventory
                .lock()
                .get_item(0)
                .is(&vanilla_items::OAK_PLANKS)
        );
        assert_eq!(fixture.player.inventory.lock().get_item(0).count(), 4);
        assert!(fixture.grid.lock().get_item(0).is_empty());
        assert_eq!(fixture.crafted(planks()), 4);
    }

    #[test]
    fn throwing_one_item_drops_one_item_and_counts_one() {
        let mut fixture = fixture("CrafterThrow");
        fixture.fill_grid_with_a_log();

        fixture.click(Click::Throw {
            slot: RESULT_SLOT,
            whole_stack: false,
        });

        let dropped = fixture.dropped_items();
        assert_eq!(dropped.len(), 1);
        assert!(dropped[0].is(&vanilla_items::OAK_PLANKS));
        assert_eq!(dropped[0].count(), 1);
        assert_eq!(fixture.crafted(planks()), 1);
    }

    #[test]
    fn creative_clone_keeps_the_inputs_and_awards_nothing() {
        let mut fixture = fixture("CrafterClone");
        fixture.player.restore_game_modes(GameType::Creative, None);
        fixture.fill_grid_with_a_log();

        fixture.click(Click::Clone { slot: RESULT_SLOT });

        assert!(fixture.carried().is(&vanilla_items::OAK_PLANKS));
        // `Slot.safeClone` clones a full stack without touching the inputs.
        assert_eq!(fixture.carried().count(), 64);
        assert!(fixture.grid.lock().get_item(0).is(&vanilla_items::OAK_LOG));
        assert_eq!(fixture.crafted(planks()), 0);
    }

    #[test]
    fn a_shifted_shaped_recipe_consumes_only_its_own_grid_slots() {
        let mut fixture = fixture("CrafterShifted");
        // Torch is a 1x2 pattern placed at column 2, rows 1-2 of the 3x3 grid.
        fixture.set_grid(5, ItemStack::new(&vanilla_items::COAL));
        fixture.set_grid(8, ItemStack::new(&vanilla_items::STICK));
        fixture.refresh();
        assert!(fixture.preview().is(&vanilla_items::TORCH));
        assert_eq!(fixture.preview().count(), 4);

        fixture.click(Click::Pickup {
            slot: RESULT_SLOT,
            button: MouseButton::Left,
        });

        assert!(fixture.carried().is(&vanilla_items::TORCH));
        assert!(fixture.grid.lock().get_item(5).is_empty());
        assert!(fixture.grid.lock().get_item(8).is_empty());
        assert_eq!(
            fixture.crafted(ItemStack::new(&vanilla_items::TORCH).item()),
            4
        );
    }

    #[test]
    fn a_changed_grid_makes_the_stale_preview_untakeable() {
        let mut fixture = fixture("CrafterStale");
        fixture.fill_grid_with_a_log();
        assert_eq!(fixture.preview().count(), 4);
        // An external change replaces the input after the preview was computed.
        fixture.set_grid(0, ItemStack::new(&vanilla_items::DIRT));

        fixture.click(Click::Pickup {
            slot: RESULT_SLOT,
            button: MouseButton::Left,
        });

        assert!(fixture.carried().is_empty());
        assert_eq!(fixture.crafted(planks()), 0);
        assert!(fixture.preview().is_empty());
        assert!(fixture.grid.lock().get_item(0).is(&vanilla_items::DIRT));
    }

    #[test]
    fn a_remainder_that_cannot_merge_goes_to_the_inventory() {
        let mut fixture = fixture("CrafterOverflow");
        fixture.set_grid(0, ItemStack::with_count(&vanilla_items::MILK_BUCKET, 2));
        fixture.set_grid(1, ItemStack::new(&vanilla_items::MILK_BUCKET));
        fixture.set_grid(2, ItemStack::new(&vanilla_items::MILK_BUCKET));
        fixture.set_grid(3, ItemStack::new(&vanilla_items::SUGAR));
        fixture.set_grid(4, ItemStack::new(&vanilla_items::EGG));
        fixture.set_grid(5, ItemStack::new(&vanilla_items::SUGAR));
        for slot in 6..9 {
            fixture.set_grid(slot, ItemStack::new(&vanilla_items::WHEAT));
        }
        fixture.refresh();
        assert!(fixture.preview().is(&vanilla_items::CAKE));

        fixture.click(Click::Pickup {
            slot: RESULT_SLOT,
            button: MouseButton::Left,
        });

        assert!(fixture.carried().is(&vanilla_items::CAKE));
        let grid = fixture.grid.lock();
        assert!(
            grid.get_item(0).is(&vanilla_items::MILK_BUCKET),
            "the consumed slot keeps the milk bucket it did not use"
        );
        assert_eq!(grid.get_item(0).count(), 1);
        drop(grid);
        assert_eq!(
            fixture.inventory_count(ItemStack::new(&vanilla_items::BUCKET).item()),
            1
        );
    }

    #[test]
    fn crafting_remainders_replace_the_consumed_grid_slots() {
        let mut fixture = fixture("CrafterRemainder");
        for slot in 0..3 {
            fixture.set_grid(slot, ItemStack::new(&vanilla_items::MILK_BUCKET));
        }
        fixture.set_grid(3, ItemStack::new(&vanilla_items::SUGAR));
        fixture.set_grid(4, ItemStack::new(&vanilla_items::EGG));
        fixture.set_grid(5, ItemStack::new(&vanilla_items::SUGAR));
        for slot in 6..9 {
            fixture.set_grid(slot, ItemStack::new(&vanilla_items::WHEAT));
        }
        fixture.refresh();
        assert!(fixture.preview().is(&vanilla_items::CAKE));

        fixture.click(Click::Pickup {
            slot: RESULT_SLOT,
            button: MouseButton::Left,
        });

        assert!(fixture.carried().is(&vanilla_items::CAKE));
        let grid = fixture.grid.lock();
        for slot in 0..3 {
            assert!(
                grid.get_item(slot).is(&vanilla_items::BUCKET),
                "milk buckets must return empty buckets into their own grid slot"
            );
        }
        for slot in 3..9 {
            assert!(grid.get_item(slot).is_empty());
        }
    }
}

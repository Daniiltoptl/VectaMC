use std::io::Cursor;
use std::sync::Arc;

use glam::DVec3;
use simdnbt::borrow::{NbtCompound as NbtCompoundView, read_compound};
use simdnbt::owned::NbtCompound;
use vecta_registry::item_stack::ItemStack;
use vecta_registry::items::ItemRef;
use vecta_registry::vanilla_game_rules::ENTITY_DROPS;
use vecta_registry::{
    init_vanilla_registry, vanilla_blocks, vanilla_damage_types, vanilla_entities, vanilla_items,
};
use vecta_utils::types::{InteractionHand, UpdateFlags};
use vecta_utils::{BlockPos, ChunkPos, Direction, Downcast as _};

use super::{BlockAttachedEntity as _, ItemFrame, ItemFrameEntity};
use crate::behavior::{
    BlockHitResult, ITEM_BEHAVIORS, InteractionResult, UseOnContext, init_behaviors,
};
use crate::entity::damage::DamageSource;
use crate::entity::entities::GlowItemFrameEntity;
use crate::entity::{Entity as _, SharedEntity, entities::ItemEntity, init_entities};
use crate::player::Player;
use crate::test_support::{
    TestPlayerBuilder, TestWorld, dropped_items, fresh_test_world, insert_ready_full_chunk,
};
use crate::world::World;

/// Frame block position used by every scenario.
const FRAME_POS: BlockPos = BlockPos::new(8, 64, 8);

struct Fixture {
    world_fixture: TestWorld,
    player: Arc<Player>,
}

impl Fixture {
    fn new(name: &'static str) -> Self {
        init_vanilla_registry();
        init_behaviors();
        init_entities();
        let world_fixture = fresh_test_world(name);
        insert_ready_full_chunk(&world_fixture.world, ChunkPos::new(0, 0));
        let player = TestPlayerBuilder::new(Arc::clone(&world_fixture.world), "Framer", 1).build();
        assert!(
            !player.has_infinite_materials(),
            "scenarios must run in survival unless stated otherwise"
        );
        Self {
            world_fixture,
            player,
        }
    }

    fn world(&self) -> &Arc<World> {
        &self.world_fixture.world
    }

    fn block(&self, pos: BlockPos, state: vecta_utils::BlockStateId) {
        assert!(
            self.world()
                .set_block(pos, state, UpdateFlags::UPDATE_KNOWN_SHAPE)
        );
    }

    /// Places the block that a south-facing frame at [`FRAME_POS`] hangs on.
    fn backing_block(&self) {
        self.block(
            FRAME_POS.relative(Direction::North),
            vanilla_blocks::STONE.default_state(),
        );
    }

    fn frame(&self) -> ItemFrameEntity {
        ItemFrameEntity::new_attached(
            &vanilla_entities::ITEM_FRAME,
            42,
            FRAME_POS,
            Direction::South,
            Arc::downgrade(self.world()),
        )
    }

    fn glow_frame(&self) -> GlowItemFrameEntity {
        GlowItemFrameEntity::new_attached(
            &vanilla_entities::GLOW_ITEM_FRAME,
            43,
            FRAME_POS,
            Direction::South,
            Arc::downgrade(self.world()),
        )
    }

    fn hold(&self, hand: InteractionHand, item: ItemRef, count: i32) {
        self.player
            .inventory
            .lock()
            .set_item_in_hand(hand, ItemStack::with_count(item, count));
    }

    fn held(&self, hand: InteractionHand) -> ItemStack {
        self.player.inventory.lock().get_item_in_hand(hand).clone()
    }

    fn use_on(
        &self,
        clicked: BlockPos,
        face: Direction,
        hand: InteractionHand,
    ) -> InteractionResult {
        let item = self.held(hand).item();
        let mut context = UseOnContext::new(
            &self.player,
            hand,
            BlockHitResult {
                block_pos: clicked,
                direction: face,
                location: DVec3::new(
                    f64::from(clicked.x()) + 0.5,
                    f64::from(clicked.y()) + 0.5,
                    f64::from(clicked.z()) + 0.5,
                ),
                miss: false,
                inside: false,
                world_border_hit: false,
            },
            self.world(),
            Arc::clone(&self.player.inventory),
        );
        ITEM_BEHAVIORS.get_behavior(item).use_on(&mut context)
    }

    fn frames_in_world(&self) -> Vec<SharedEntity> {
        self.world().get_entities_in_aabb_matching(
            &vecta_utils::WorldAabb::of_size(
                FRAME_POS.0.as_dvec3() + DVec3::splat(0.5),
                4.0,
                4.0,
                4.0,
            ),
            |entity| {
                entity.entity_type() == &vanilla_entities::ITEM_FRAME
                    || entity.entity_type() == &vanilla_entities::GLOW_ITEM_FRAME
            },
        )
    }
}

fn framed(stack: ItemRef) -> ItemStack {
    ItemStack::new(stack)
}

/// Re-reads an owned compound as the borrowed view used by `load_additional`.
fn with_borrowed_compound(nbt: &NbtCompound, f: impl FnOnce(NbtCompoundView<'_, '_>)) {
    let mut bytes = Vec::new();
    nbt.write(&mut bytes);
    let mut cursor = Cursor::new(bytes.as_slice());
    let Ok(base) = read_compound(&mut cursor) else {
        panic!("saved item frame data should reborrow");
    };
    f(NbtCompoundView::from(&base));
}

#[test]
fn empty_frame_takes_the_held_item_and_consumes_one_from_each_hand() {
    for hand in [InteractionHand::MainHand, InteractionHand::OffHand] {
        let fixture = Fixture::new("item_frame_insert_both_hands");
        fixture.backing_block();
        let frame = fixture.frame();
        fixture.hold(hand, &vanilla_items::DIAMOND, 3);

        assert_eq!(
            frame.interact(&fixture.player, hand, DVec3::ZERO),
            InteractionResult::Success
        );
        assert!(frame.item().is(&vanilla_items::DIAMOND));
        assert_eq!(frame.item().count, 1);
        assert_eq!(fixture.held(hand).count, 2);
    }
}

#[test]
fn empty_frame_ignores_an_empty_hand_and_stays_empty() {
    let fixture = Fixture::new("item_frame_empty_hand");
    fixture.backing_block();
    let frame = fixture.frame();

    assert_eq!(
        frame.interact(&fixture.player, InteractionHand::MainHand, DVec3::ZERO),
        InteractionResult::Pass
    );
    assert!(frame.item().is_empty());
}

#[test]
fn filled_frame_rotates_without_consuming_the_held_stack() {
    let fixture = Fixture::new("item_frame_rotate");
    fixture.backing_block();
    let frame = fixture.frame();
    frame.set_item(framed(&vanilla_items::DIAMOND));
    fixture.hold(InteractionHand::MainHand, &vanilla_items::DIAMOND, 1);

    for expected in 1..=8 {
        assert_eq!(
            frame.interact(&fixture.player, InteractionHand::MainHand, DVec3::ZERO),
            InteractionResult::Success
        );
        assert_eq!(frame.rotation(), expected % 8);
        assert_eq!(frame.analog_output(), expected % 8 + 1);
        assert_eq!(fixture.held(InteractionHand::MainHand).count, 1);
    }
}

#[test]
fn fixed_frame_ignores_interaction_and_support_loss() {
    let fixture = Fixture::new("item_frame_fixed");
    let frame = fixture.frame();
    let mut nbt = NbtCompound::new();
    nbt.insert("Fixed", 1_i8);
    with_borrowed_compound(&nbt, |borrowed| frame.load_additional(borrowed));
    fixture.hold(InteractionHand::MainHand, &vanilla_items::DIAMOND, 1);

    assert!(frame.is_fixed());
    assert!(
        frame.survives(),
        "a fixed frame survives without a backing block"
    );
    assert_eq!(
        frame.interact(&fixture.player, InteractionHand::MainHand, DVec3::ZERO),
        InteractionResult::Pass
    );
    assert!(frame.item().is_empty());
    assert_eq!(fixture.held(InteractionHand::MainHand).count, 1);
}

#[test]
fn frame_without_a_solid_backing_block_does_not_survive() {
    let fixture = Fixture::new("item_frame_support");
    let frame = fixture.frame();
    assert!(!frame.survives());

    fixture.backing_block();
    assert!(frame.survives());

    fixture.block(
        FRAME_POS.relative(Direction::North),
        vanilla_blocks::AIR.default_state(),
    );
    assert!(!frame.survives());
}

#[test]
fn frame_does_not_survive_inside_a_solid_block() {
    let fixture = Fixture::new("item_frame_obstructed");
    fixture.backing_block();
    let frame = fixture.frame();
    assert!(frame.survives());

    fixture.block(FRAME_POS, vanilla_blocks::STONE.default_state());
    assert!(!frame.survives());
}

#[test]
fn breaking_a_filled_frame_drops_the_frame_and_the_framed_item() {
    let fixture = Fixture::new("item_frame_drop");
    fixture.backing_block();
    let frame = fixture.frame();
    frame.set_item(framed(&vanilla_items::DIAMOND));

    frame.drop_item(None);

    let drops = dropped_items(fixture.world(), FRAME_POS.0.as_dvec3() + DVec3::splat(0.5));
    assert_eq!(drops.len(), 2, "frame and framed item must both drop");
    let stacks = drops
        .iter()
        .filter_map(|entity| {
            entity
                .downcast_ref::<ItemEntity>()
                .map(ItemEntity::get_item)
        })
        .collect::<Vec<_>>();
    assert!(
        stacks
            .iter()
            .any(|stack| stack.is(&vanilla_items::ITEM_FRAME)),
        "the frame item itself must drop"
    );
    assert!(
        stacks.iter().any(|stack| stack.is(&vanilla_items::DIAMOND)),
        "the framed item must drop"
    );
    assert!(frame.item().is_empty(), "dropping clears the framed item");
}

#[test]
fn breaking_a_frame_obeys_the_entity_drops_gamerule() {
    let fixture = Fixture::new("item_frame_drop_gamerule");
    fixture.backing_block();
    fixture.world().set_game_rule(&ENTITY_DROPS, false);
    let frame = fixture.frame();
    frame.set_item(framed(&vanilla_items::DIAMOND));

    frame.drop_item(None);

    assert!(
        dropped_items(fixture.world(), FRAME_POS.0.as_dvec3() + DVec3::splat(0.5)).is_empty(),
        "doEntityDrops=false must suppress both drops"
    );
    assert!(frame.item().is_empty());
}

#[test]
fn hurting_a_filled_frame_drops_only_the_framed_item() {
    let fixture = Fixture::new("item_frame_hurt_item");
    fixture.backing_block();
    let frame = fixture.frame();
    frame.set_item(framed(&vanilla_items::DIAMOND));
    let source = DamageSource::environment(&vanilla_damage_types::PLAYER_ATTACK);

    assert!(frame.hurt(fixture.world(), &source, 0.0));

    assert!(!frame.is_removed(), "the frame itself survives the hit");
    assert!(frame.item().is_empty());
    let drops = dropped_items(fixture.world(), FRAME_POS.0.as_dvec3() + DVec3::splat(0.5));
    assert_eq!(drops.len(), 1);
}

#[test]
fn hurting_an_empty_frame_breaks_it() {
    let fixture = Fixture::new("item_frame_hurt_empty");
    fixture.backing_block();
    let frame = fixture.frame();
    let source = DamageSource::environment(&vanilla_damage_types::PLAYER_ATTACK);

    assert!(frame.hurt(fixture.world(), &source, 0.0));

    assert!(frame.is_removed());
    let drops = dropped_items(fixture.world(), FRAME_POS.0.as_dvec3() + DVec3::splat(0.5));
    assert_eq!(drops.len(), 1, "an empty frame drops only the frame");
}

#[test]
fn saved_frame_state_round_trips_through_nbt() {
    let fixture = Fixture::new("item_frame_save_load");
    fixture.backing_block();
    let frame = fixture.frame();
    frame.set_item(framed(&vanilla_items::DIAMOND));
    frame.set_rotation(3, false);

    let mut nbt = NbtCompound::new();
    frame.save_additional(&mut nbt);

    let restored = ItemFrameEntity::new_attached(
        &vanilla_entities::ITEM_FRAME,
        44,
        FRAME_POS,
        Direction::North,
        Arc::downgrade(fixture.world()),
    );
    with_borrowed_compound(&nbt, |borrowed| restored.load_additional(borrowed));

    assert!(restored.item().is(&vanilla_items::DIAMOND));
    assert_eq!(restored.rotation(), 3);
    assert_eq!(restored.direction(), Direction::South);
    assert!(!restored.is_fixed());
    assert_eq!(restored.analog_output(), 4);
}

#[test]
fn fixed_and_drop_chance_are_persisted() {
    let fixture = Fixture::new("item_frame_persist_flags");
    let frame = fixture.frame();
    let mut nbt = NbtCompound::new();
    nbt.insert("Fixed", 1_i8);
    nbt.insert("ItemDropChance", 0.0_f32);
    with_borrowed_compound(&nbt, |borrowed| frame.load_additional(borrowed));

    let mut saved = NbtCompound::new();
    frame.save_additional(&mut saved);
    assert_eq!(saved.byte("Fixed"), Some(1));

    let restored = ItemFrameEntity::new_attached(
        &vanilla_entities::ITEM_FRAME,
        45,
        FRAME_POS,
        Direction::South,
        Arc::downgrade(fixture.world()),
    );
    with_borrowed_compound(&saved, |borrowed| restored.load_additional(borrowed));
    assert!(restored.is_fixed());
}

#[test]
fn glow_frames_report_their_own_item_and_sounds() {
    let fixture = Fixture::new("item_frame_glow_variant");
    fixture.backing_block();
    let frame = fixture.glow_frame();

    assert!(frame.frame_item_stack().is(&vanilla_items::GLOW_ITEM_FRAME));
    frame.play_placement_sound();
    assert!(frame.survives());
}

#[test]
fn placing_an_item_frame_consumes_one_item_and_spawns_the_entity() {
    let fixture = Fixture::new("item_frame_place");
    let clicked = BlockPos::new(8, 64, 7);
    fixture.block(clicked, vanilla_blocks::STONE.default_state());
    fixture.hold(InteractionHand::MainHand, &vanilla_items::ITEM_FRAME, 2);

    assert_eq!(
        fixture.use_on(clicked, Direction::South, InteractionHand::MainHand),
        InteractionResult::Success
    );

    assert_eq!(fixture.held(InteractionHand::MainHand).count, 1);
    let frames = fixture.frames_in_world();
    assert_eq!(frames.len(), 1);
    let frame = frames[0]
        .as_item_frame()
        .expect("placed frame is an item frame");
    assert_eq!(frame.direction(), Direction::South);
}

#[test]
fn placing_a_glow_item_frame_uses_the_glow_entity_type() {
    let fixture = Fixture::new("item_frame_place_glow");
    let clicked = BlockPos::new(8, 64, 7);
    fixture.block(clicked, vanilla_blocks::STONE.default_state());
    fixture.hold(
        InteractionHand::MainHand,
        &vanilla_items::GLOW_ITEM_FRAME,
        1,
    );

    assert_eq!(
        fixture.use_on(clicked, Direction::South, InteractionHand::MainHand),
        InteractionResult::Success
    );

    let frames = fixture.frames_in_world();
    assert_eq!(frames.len(), 1);
    assert_eq!(
        frames[0].entity_type(),
        &vanilla_entities::GLOW_ITEM_FRAME,
        "glow_item_frame must place a GlowItemFrame, not a plain ItemFrame"
    );
}

#[test]
fn placing_into_an_obstructed_cell_consumes_the_click_without_spawning() {
    let fixture = Fixture::new("item_frame_place_obstructed");
    let clicked = BlockPos::new(8, 64, 7);
    fixture.block(clicked, vanilla_blocks::STONE.default_state());
    fixture.block(
        clicked.relative(Direction::South),
        vanilla_blocks::STONE.default_state(),
    );
    fixture.hold(InteractionHand::MainHand, &vanilla_items::ITEM_FRAME, 1);

    assert_eq!(
        fixture.use_on(clicked, Direction::South, InteractionHand::MainHand),
        InteractionResult::Consume
    );
    assert_eq!(fixture.held(InteractionHand::MainHand).count, 1);
    assert!(fixture.frames_in_world().is_empty());
}

#[test]
fn placing_without_a_backing_block_is_refused_without_consuming() {
    let fixture = Fixture::new("item_frame_place_unsupported");
    let clicked = BlockPos::new(8, 64, 7);
    // No block at `clicked`, so `may_use_item_at` still passes; the frame's own
    // support check rejects the cell because the clicked block is air.
    fixture.hold(InteractionHand::MainHand, &vanilla_items::ITEM_FRAME, 1);

    assert_eq!(
        fixture.use_on(clicked, Direction::South, InteractionHand::MainHand),
        InteractionResult::Consume
    );
    assert_eq!(fixture.held(InteractionHand::MainHand).count, 1);
    assert!(fixture.frames_in_world().is_empty());
}

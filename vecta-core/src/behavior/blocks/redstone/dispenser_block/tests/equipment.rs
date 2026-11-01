use super::*;
use crate::entity::{LivingEntity as _, Mob as _};
use crate::world::game_event::{GameEventContext, GameEventListener};
use std::sync::Weak;
use vecta_registry::data_components::vanilla_components::EQUIPPABLE;
use vecta_registry::equipment::EquipmentSlot;
use vecta_registry::game_events::GameEventRef;
use vecta_utils::{SectionPos, locks::SyncMutex};

struct RecordingListener {
    position: DVec3,
    player: Weak<Player>,
    events: Arc<SyncMutex<Vec<GameEventRef>>>,
}

impl GameEventListener for RecordingListener {
    fn listener_pos(&self) -> Option<DVec3> {
        Some(self.position)
    }

    fn listener_radius(&self) -> i32 {
        16
    }

    fn handle_game_event(
        &self,
        _world: &Arc<World>,
        event: GameEventRef,
        _context: &GameEventContext<'_>,
        _source_pos: DVec3,
    ) -> bool {
        if let Some(player) = self.player.upgrade() {
            // A game-event listener may inspect the inventory that emitted EQUIP.
            let _inventory = player.inventory.lock();
        }
        self.events.lock().push(event);
        true
    }
}

fn record_events(
    world: &Arc<World>,
    pos: BlockPos,
    player: &Arc<Player>,
) -> Arc<SyncMutex<Vec<GameEventRef>>> {
    let events = Arc::new(SyncMutex::new(Vec::new()));
    world.register_game_event_listener(
        SectionPos::from_block_pos(pos),
        Arc::new(RecordingListener {
            position: DVec3::new(
                f64::from(pos.x()) + 0.5,
                f64::from(pos.y()) + 0.5,
                f64::from(pos.z()) + 0.5,
            ),
            player: Arc::downgrade(player),
            events: Arc::clone(&events),
        }),
    );
    events
}

fn target_player(player: &Player) {
    player
        .try_set_position(DVec3::new(4.5, 64.0, 5.5))
        .expect("loaded target block");
}

fn equipment(player: &Player) -> ItemStack {
    let mut item = ItemStack::empty();
    player.with_equipment_slot(EquipmentSlot::Head, &mut |equipped| {
        item = equipped.copy_with_count(equipped.count());
    });
    item
}

#[test]
fn a_dispenser_equips_a_player_and_consumes_only_one_item() {
    let (fixture, pos, player, packets) =
        listening_dispenser_fixture("dispenser_equip_player", Direction::South);
    target_player(&player);
    let mut input = ItemStack::new(&vanilla_items::DIAMOND_HELMET);
    input.set_damage_value(7);
    let expected = input.copy_with_count(1);
    set_item_at(&fixture, pos, 0, input);
    packets.lock().clear();

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    assert!(player.as_mob().is_none());
    assert!(ItemStack::matches(&equipment(&player), &expected));
    assert!(item_at(&fixture, pos, 0).is_empty());
    assert!(ejected_items(&fixture, pos).is_empty());
    assert_eq!(
        sent_level_events(&packets),
        [
            (SOUND_DISPENSER_DISPENSE, pos, 0),
            (
                PARTICLES_SHOOT_SMOKE,
                pos,
                Direction::South.get_3d_data_value()
            ),
        ]
    );
}

#[test]
fn rejected_equipment_falls_back_to_an_item_entity() {
    for case in ["occupied", "not_dispensable", "outside", "dead"] {
        let (fixture, pos, player, _) =
            listening_dispenser_fixture("dispenser_equip_rejected", Direction::South);
        target_player(&player);
        let mut input = ItemStack::new(&vanilla_items::DIAMOND_HELMET);
        match case {
            "occupied" => player.set_item_slot(
                EquipmentSlot::Head,
                ItemStack::new(&vanilla_items::IRON_HELMET),
            ),
            "not_dispensable" => {
                let mut equippable = input.get_equippable().expect("helmet component").clone();
                equippable.dispensable = false;
                input.set(EQUIPPABLE, equippable);
            }
            "outside" => player
                .try_set_position(DVec3::new(6.5, 64.0, 5.5))
                .expect("loaded block outside the target"),
            "dead" => player.set_health(0.0),
            _ => unreachable!(),
        }
        set_item_at(&fixture, pos, 0, input);
        pulse(
            &DispenserBlock::new(&vanilla_blocks::DISPENSER),
            &fixture,
            pos,
        );
        assert!(item_at(&fixture, pos, 0).is_empty(), "{case}");
        assert_eq!(ejected_items(&fixture, pos).len(), 1, "{case}");
        assert!(
            !equipment(&player).is(&vanilla_items::DIAMOND_HELMET),
            "{case}"
        );
    }
}

#[test]
fn unregistered_items_select_equipment_by_component() {
    let (fixture, pos, player, _) =
        listening_dispenser_fixture("dispenser_component_equipment", Direction::South);
    target_player(&player);
    let mut stone = ItemStack::with_count(&vanilla_items::STONE, 3);
    let helmet = ItemStack::new(&vanilla_items::DIAMOND_HELMET);
    stone.set(
        EQUIPPABLE,
        helmet.get_equippable().expect("helmet component").clone(),
    );
    set_item_at(&fixture, pos, 0, stone);

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    assert!(equipment(&player).is(&vanilla_items::STONE));
    assert_eq!(item_at(&fixture, pos, 0).count(), 2);
    assert!(ejected_items(&fixture, pos).is_empty());
}

#[test]
fn a_registered_projectile_wins_over_the_equippable_component() {
    let (fixture, pos, player, _) =
        listening_dispenser_fixture("dispenser_projectile_priority", Direction::South);
    target_player(&player);
    let mut input = ItemStack::new(&vanilla_items::SNOWBALL);
    let helmet = ItemStack::new(&vanilla_items::DIAMOND_HELMET);
    input.set(
        EQUIPPABLE,
        helmet.get_equippable().expect("helmet component").clone(),
    );
    set_item_at(&fixture, pos, 0, input);

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    assert!(equipment(&player).is_empty());
    assert_eq!(snowballs(&fixture, pos).len(), 1);
}

#[test]
fn a_dispensed_saddle_makes_the_pig_persistent_with_guaranteed_drops() {
    use crate::entity::entities::PigEntity;

    let (fixture, pos) = dispenser_fixture("dispenser_equip_pig", Direction::South);
    let pig = Arc::new(PigEntity::new(
        &vanilla_entities::PIG,
        next_entity_id(),
        DVec3::new(4.5, 64.0, 5.5),
        Arc::downgrade(&fixture.world),
    ));
    fixture
        .world
        .try_add_entity(Arc::clone(&pig) as SharedEntity)
        .expect("pig in a loaded chunk");
    assert!(!pig.can_pick_up_loot());
    set_item_at(&fixture, pos, 0, ItemStack::new(&vanilla_items::SADDLE));

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );

    pig.with_equipment_slot(EquipmentSlot::Saddle, &mut |item| {
        assert!(item.is(&vanilla_items::SADDLE));
    });
    assert!(pig.is_persistence_required());
    assert!(pig.equipment_drop_chance(EquipmentSlot::Saddle) > 1.0);
    assert!(item_at(&fixture, pos, 0).is_empty());
    assert!(ejected_items(&fixture, pos).is_empty());
}

#[test]
fn a_dropper_never_equips_the_player_in_front_of_it() {
    use crate::behavior::blocks::redstone::dropper_block::DropperBlock;

    let (fixture, pos, player, _) =
        listening_dispenser_fixture("dropper_does_not_equip", Direction::South);
    target_player(&player);
    assert!(
        fixture.world.set_block(
            pos,
            vanilla_blocks::DROPPER
                .default_state()
                .set_value(FACING, Direction::South),
            UpdateFlags::UPDATE_ALL,
        )
    );
    set_item_at(
        &fixture,
        pos,
        0,
        ItemStack::new(&vanilla_items::DIAMOND_HELMET),
    );
    DropperBlock::new(&vanilla_blocks::DROPPER).tick(
        fixture.world.get_block_state(pos),
        &fixture.world,
        pos,
    );
    assert!(equipment(&player).is_empty());
    assert_eq!(ejected_items(&fixture, pos).len(), 1);
}

#[test]
fn silent_player_equipment_emits_reentrant_equip_and_unequip_events() {
    let (fixture, pos, player, _) =
        listening_dispenser_fixture("dispenser_equip_events", Direction::South);
    target_player(&player);
    player.set_first_tick(false);
    player.set_silent(true);
    let events = record_events(&fixture.world, pos, &player);
    set_item_at(
        &fixture,
        pos,
        0,
        ItemStack::new(&vanilla_items::DIAMOND_HELMET),
    );

    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );
    player.set_item_slot(EquipmentSlot::Head, ItemStack::empty());
    player.set_item_slot(
        EquipmentSlot::MainHand,
        ItemStack::new(&vanilla_items::STONE),
    );
    player.set_item_slot(EquipmentSlot::Head, ItemStack::empty());

    assert_eq!(
        *events.lock(),
        [&vanilla_game_events::EQUIP, &vanilla_game_events::UNEQUIP]
    );
}

#[test]
fn an_empty_dispenser_fires_block_activate_to_a_real_listener() {
    let (fixture, pos, player, _) =
        listening_dispenser_fixture("dispenser_empty_game_event", Direction::South);
    let events = record_events(&fixture.world, pos, &player);
    pulse(
        &DispenserBlock::new(&vanilla_blocks::DISPENSER),
        &fixture,
        pos,
    );
    assert_eq!(*events.lock(), [&vanilla_game_events::BLOCK_ACTIVATE]);
}

use simdnbt::owned::NbtTag;
use vecta_registry::{REGISTRY, init_vanilla_registry, vanilla_attributes, vanilla_entities};

use super::*;

#[test]
fn packed_apply_preserves_operation_indices_and_unconditional_dirty_sync() {
    init_vanilla_registry();
    let mut attributes = AttributeMap::new_for_entity(&vanilla_entities::SHEEP);
    let attribute = vanilla_attributes::MAX_HEALTH;
    let id = Identifier::new_static("vecta", "packed");
    attributes.set_base_value(attribute, 10.0);
    attributes.add_modifier(
        attribute,
        AttributeModifier {
            id: id.clone(),
            amount: 2.0,
            operation: AttributeModifierOperation::AddValue,
        },
        false,
    );
    attributes.apply_packed(
        attribute,
        10.0,
        vec![AttributeModifier {
            id: id.clone(),
            amount: 0.5,
            operation: AttributeModifierOperation::AddMultipliedTotal,
        }],
    );
    assert_eq!(attributes.get_value(attribute), Some(18.0));
    assert!(attributes.remove_modifier(attribute, &id));
    assert_eq!(
        attributes.get_value(attribute),
        Some(12.0),
        "Java remove affects the currently ID-indexed operation"
    );
    assert!(
        !attributes
            .get_instance(attribute)
            .expect("attribute")
            .has_modifier(&id)
    );
    attributes.drain_dirty_sync();
    attributes.apply_packed(attribute, 10.0, Vec::new());
    assert!(
        attributes
            .drain_dirty_sync()
            .iter()
            .any(|snapshot| snapshot.attribute_id == attribute.id() as i32)
    );
    attributes.add_modifier(
        attribute,
        AttributeModifier {
            id: id.clone(),
            amount: 4.0,
            operation: AttributeModifierOperation::AddValue,
        },
        false,
    );
    assert_eq!(attributes.get_value(attribute), Some(14.0));
    assert!(attributes.remove_modifier(attribute, &id));
    assert_eq!(attributes.get_value(attribute), Some(10.0));
}

#[test]
fn all_generated_entity_default_attributes_resolve() {
    init_vanilla_registry();

    for (_, entity_type) in REGISTRY.entity_types.iter() {
        let _ = AttributeMap::new_for_entity(entity_type);
    }
}

#[test]
fn player_gravity_is_initialized_from_default_attributes() {
    init_vanilla_registry();

    let attributes = AttributeMap::new_for_entity(&vanilla_entities::PLAYER);

    assert_eq!(
        attributes
            .required_value(vanilla_attributes::GRAVITY)
            .to_bits(),
        vanilla_attributes::GRAVITY.default_value.to_bits()
    );
}

#[test]
fn vanilla_nbt_only_contains_permanent_modifiers() {
    init_vanilla_registry();
    let mut attributes = AttributeMap::new_for_entity(&vanilla_entities::PLAYER);
    attributes.add_modifier(
        vanilla_attributes::MAX_HEALTH,
        AttributeModifier {
            id: Identifier::vanilla_static("transient_test"),
            amount: 1.0,
            operation: AttributeModifierOperation::AddValue,
        },
        false,
    );
    attributes.add_modifier(
        vanilla_attributes::MAX_HEALTH,
        AttributeModifier {
            id: Identifier::vanilla_static("permanent_test"),
            amount: 2.0,
            operation: AttributeModifierOperation::AddMultipliedBase,
        },
        true,
    );

    let NbtList::Compound(packed_attributes) = attributes.to_vanilla_nbt() else {
        panic!("attributes should serialize as a compound list");
    };
    let max_health = packed_attributes
        .iter()
        .find(|attribute| {
            attribute.string("id").is_some_and(|id| {
                id.to_str().as_ref() == vanilla_attributes::MAX_HEALTH.key.to_string()
            })
        })
        .unwrap_or_else(|| panic!("max health should be serialized"));
    let Some(NbtTag::List(NbtList::Compound(modifiers))) = max_health.get("modifiers") else {
        panic!("permanent modifier should be serialized");
    };

    assert_eq!(modifiers.len(), 1);
    assert_eq!(
        modifiers[0].string("id").map(ToString::to_string),
        Some("minecraft:permanent_test".to_owned())
    );
    assert_eq!(modifiers[0].double("amount"), Some(2.0));
    assert_eq!(
        modifiers[0].string("operation").map(ToString::to_string),
        Some("add_multiplied_base".to_owned())
    );
}

#[test]
fn removing_all_modifiers_preserves_base_value() {
    init_vanilla_registry();
    let mut attributes = AttributeMap::new_for_entity(&vanilla_entities::PLAYER);
    attributes.set_base_value(vanilla_attributes::MAX_HEALTH, 30.0);

    let transient_id = Identifier::vanilla_static("transient_test");
    let permanent_id = Identifier::vanilla_static("permanent_test");
    assert!(attributes.add_modifier(
        vanilla_attributes::MAX_HEALTH,
        AttributeModifier {
            id: Identifier::vanilla_static("transient_test"),
            amount: 2.0,
            operation: AttributeModifierOperation::AddValue,
        },
        false,
    ));
    assert!(attributes.add_modifier(
        vanilla_attributes::MAX_HEALTH,
        AttributeModifier {
            id: Identifier::vanilla_static("permanent_test"),
            amount: 3.0,
            operation: AttributeModifierOperation::AddValue,
        },
        true,
    ));

    attributes.remove_all_modifiers();

    assert_eq!(
        attributes
            .get_base_value(vanilla_attributes::MAX_HEALTH)
            .map(f64::to_bits),
        Some(30.0_f64.to_bits())
    );
    assert_eq!(
        attributes
            .get_value(vanilla_attributes::MAX_HEALTH)
            .map(f64::to_bits),
        Some(30.0_f64.to_bits())
    );
    assert!(!attributes.has_modifier(vanilla_attributes::MAX_HEALTH, &transient_id));
    assert!(!attributes.has_modifier(vanilla_attributes::MAX_HEALTH, &permanent_id));
}

#[test]
fn assigning_base_values_only_changes_attributes_present_in_both_maps() {
    init_vanilla_registry();
    let mut source = AttributeMap::new_for_entity(&vanilla_entities::ZOMBIE);
    let mut destination = AttributeMap::new_for_entity(&vanilla_entities::PLAYER);

    source.set_base_value(vanilla_attributes::MAX_HEALTH, 37.0);
    source.set_base_value(vanilla_attributes::ATTACK_DAMAGE, 8.0);
    destination.set_base_value(vanilla_attributes::LUCK, 9.0);
    destination.drain_dirty_updates();
    destination.drain_dirty_sync();

    destination.assign_base_values(&source);
    let Some(max_health_id) = vanilla_attributes::MAX_HEALTH.try_id() else {
        panic!("max health should be registered");
    };
    let Some(attack_damage_id) = vanilla_attributes::ATTACK_DAMAGE.try_id() else {
        panic!("attack damage should be registered");
    };

    assert_eq!(
        destination
            .get_base_value(vanilla_attributes::MAX_HEALTH)
            .map(f64::to_bits),
        Some(37.0_f64.to_bits())
    );
    assert_eq!(
        destination
            .get_base_value(vanilla_attributes::LUCK)
            .map(f64::to_bits),
        Some(9.0_f64.to_bits())
    );
    assert_eq!(
        destination
            .get_base_value(vanilla_attributes::ATTACK_DAMAGE)
            .map(f64::to_bits),
        Some(8.0_f64.to_bits())
    );
    let dirty_updates = destination.drain_dirty_updates();
    assert!(
        dirty_updates
            .iter()
            .any(|attribute| attribute.key == vanilla_attributes::MAX_HEALTH.key)
    );
    assert!(
        dirty_updates
            .iter()
            .any(|attribute| attribute.key == vanilla_attributes::ATTACK_DAMAGE.key)
    );
    let dirty_sync = destination.drain_dirty_sync();
    assert!(
        dirty_sync
            .iter()
            .any(|snapshot| snapshot.attribute_id == max_health_id as i32)
    );
    assert!(
        dirty_sync
            .iter()
            .all(|snapshot| snapshot.attribute_id != attack_damage_id as i32)
    );

    destination.assign_base_values(&source);

    assert!(!destination.has_dirty_updates());
    assert!(!destination.has_dirty_sync());
}

#[test]
fn assigning_permanent_modifiers_adds_only_permanent_source_modifiers() {
    init_vanilla_registry();
    let mut source = AttributeMap::new_for_entity(&vanilla_entities::PLAYER);
    let mut destination = AttributeMap::new_for_entity(&vanilla_entities::PLAYER);
    let source_transient_id = Identifier::vanilla_static("source_transient_test");
    let source_permanent_id = Identifier::vanilla_static("source_permanent_test");
    let destination_transient_id = Identifier::vanilla_static("destination_transient_test");
    let destination_permanent_id = Identifier::vanilla_static("destination_permanent_test");

    source.set_base_value(vanilla_attributes::MAX_HEALTH, 40.0);
    assert!(source.add_modifier(
        vanilla_attributes::MAX_HEALTH,
        AttributeModifier {
            id: source_transient_id.clone(),
            amount: 7.0,
            operation: AttributeModifierOperation::AddValue,
        },
        false,
    ));
    assert!(source.add_modifier(
        vanilla_attributes::MAX_HEALTH,
        AttributeModifier {
            id: source_permanent_id.clone(),
            amount: 5.0,
            operation: AttributeModifierOperation::AddValue,
        },
        true,
    ));
    destination.set_base_value(vanilla_attributes::MAX_HEALTH, 25.0);
    assert!(destination.add_modifier(
        vanilla_attributes::MAX_HEALTH,
        AttributeModifier {
            id: destination_transient_id.clone(),
            amount: 2.0,
            operation: AttributeModifierOperation::AddValue,
        },
        false,
    ));
    assert!(destination.add_modifier(
        vanilla_attributes::MAX_HEALTH,
        AttributeModifier {
            id: destination_permanent_id.clone(),
            amount: 3.0,
            operation: AttributeModifierOperation::AddValue,
        },
        true,
    ));
    destination.drain_dirty_updates();
    destination.drain_dirty_sync();

    destination.assign_permanent_modifiers(&source);

    assert_eq!(
        destination
            .get_base_value(vanilla_attributes::MAX_HEALTH)
            .map(f64::to_bits),
        Some(25.0_f64.to_bits())
    );
    assert_eq!(
        destination
            .get_value(vanilla_attributes::MAX_HEALTH)
            .map(f64::to_bits),
        Some(35.0_f64.to_bits())
    );
    assert!(!destination.has_modifier(vanilla_attributes::MAX_HEALTH, &source_transient_id));
    assert!(destination.has_modifier(vanilla_attributes::MAX_HEALTH, &source_permanent_id));
    assert!(destination.has_modifier(vanilla_attributes::MAX_HEALTH, &destination_transient_id));
    assert!(destination.has_modifier(vanilla_attributes::MAX_HEALTH, &destination_permanent_id));
    assert!(
        destination
            .get_instance(vanilla_attributes::MAX_HEALTH)
            .is_some_and(|instance| {
                instance
                    .permanent_modifiers()
                    .any(|modifier| modifier.id == source_permanent_id)
            })
    );
    assert!(destination.has_dirty_updates());
    assert!(destination.has_dirty_sync());
}

#[test]
#[should_panic(expected = "is already applied")]
fn assigning_permanent_modifiers_rejects_duplicate_ids() {
    init_vanilla_registry();
    let mut source = AttributeMap::new_for_entity(&vanilla_entities::PLAYER);
    let mut destination = AttributeMap::new_for_entity(&vanilla_entities::PLAYER);
    let modifier_id = Identifier::vanilla_static("duplicate_test");

    for attributes in [&mut source, &mut destination] {
        assert!(attributes.add_modifier(
            vanilla_attributes::MAX_HEALTH,
            AttributeModifier {
                id: modifier_id.clone(),
                amount: 1.0,
                operation: AttributeModifierOperation::AddValue,
            },
            true,
        ));
    }

    destination.assign_permanent_modifiers(&source);
}

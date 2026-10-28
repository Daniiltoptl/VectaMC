use super::*;

#[test]
fn declined_cursor_override_still_clears_clicked_bundle_selection() {
    let mut fixture = bundle_fixture("bundle_review_override_fallthrough");
    let mut selected = bundle_with(&vanilla_items::DIAMOND, 3);
    let mut contents = selected
        .get(BUNDLE_CONTENTS)
        .expect("stack should keep bundle contents")
        .as_mutable();
    contents.toggle_selected_item(0);
    selected.set(
        BUNDLE_CONTENTS,
        contents
            .to_immutable()
            .expect("bundle contents should convert"),
    );
    fixture.container.lock().set_item(0, selected);
    *fixture.menu.behavior_mut().carried_mut() = bundle_with(&vanilla_items::STONE, 2);

    fixture
        .menu
        .clicked(
            Click::Pickup {
                slot: fixture.slot,
                button: MouseButton::Right,
            },
            &fixture.player,
        )
        .expect("menu operation should succeed");

    let contents = fixture
        .menu
        .behavior()
        .carried()
        .get(BUNDLE_CONTENTS)
        .expect("stack should keep bundle contents");
    assert_eq!(contents.items()[0].item(), &*vanilla_items::DIAMOND);
    assert_eq!(contents.selected_item_index(), -1);
}

#[test]
fn bundle_selection_applies_to_the_component_on_other_items() {
    let mut fixture = bundle_fixture("bundle_review_component_selection");
    let mut stone = ItemStack::new(&vanilla_items::STONE);
    stone.set(
        BUNDLE_CONTENTS,
        BundleContents::new(vec![ItemStackTemplate::new(&vanilla_items::DIAMOND)]),
    );
    fixture.container.lock().set_item(0, stone);

    fixture
        .menu
        .set_selected_bundle_item_index(0, 0)
        .expect("menu operation should succeed");

    let stone = fixture.container.lock().get_item(0).clone();
    assert!(stone.is(&vanilla_items::STONE));
    assert_eq!(
        stone
            .get(BUNDLE_CONTENTS)
            .expect("stack should keep bundle contents")
            .selected_item_index(),
        0
    );
}

#[test]
fn fraction_overflow_aborts_a_click_before_taking_the_source_stack() {
    use vecta_registry::data_components::vanilla_components::MAX_STACK_SIZE;

    let mut fixture = bundle_fixture("bundle_review_capacity_overflow");
    let templates = [97, 89, 83, 79]
        .into_iter()
        .map(|size| {
            let mut stack = ItemStack::new(&vanilla_items::STONE);
            stack.set(MAX_STACK_SIZE, size);
            ItemStackTemplate::from_stack(&stack).expect("test item should persist")
        })
        .collect();
    let mut bundle = ItemStack::new(&vanilla_items::BUNDLE);
    bundle.set(BUNDLE_CONTENTS, BundleContents::new(templates));
    *fixture.menu.behavior_mut().carried_mut() = bundle;
    let mut stack = ItemStack::new(&vanilla_items::STONE);
    stack.set(MAX_STACK_SIZE, 73);
    fixture.container.lock().set_item(0, stack);

    assert!(
        fixture
            .menu
            .clicked(
                Click::Pickup {
                    slot: fixture.slot,
                    button: MouseButton::Left,
                },
                &fixture.player
            )
            .is_err()
    );

    assert_eq!(fixture.container.lock().get_item(0).count(), 1);
    assert_eq!(bundle_contents(fixture.menu.behavior().carried()).len(), 4);
}

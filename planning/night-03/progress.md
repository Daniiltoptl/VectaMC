# Night 03 — progress

Base: `codex/night-vanilla-02` tip `7db788bc7c44b12bf31842a6f3c0176a5d3f0d93`
(`0.16.1+mc26.3`, reference `minecraft-src` 26.3 / 5037 files).
Work branch: `codex/night-vanilla-03`.

Untouched foreign change: `planning/night-01/report.md` (never staged here).

## W03-01 — container button clicks + working stonecutter

Status: implemented and verified server-side; client recipe-list sync is a
separate missing foundation (see `blockers.md`).

Gap before: `Player::handle_container_button_click` only logged and carried a
TODO. There was no stonecutter block behavior, no stonecutter menu kind, and no
`clickMenuButton` equivalent in the menu API, so every recipe-driven workstation
button (stonecutter, loom, lectern, enchanting table) was unreachable.

Vanilla sources used:
- `server/network/ServerGamePacketListenerImpl.handleContainerButtonClick`
  (resetLastActionTime → container-id/spectator gate → stillValid → clickMenuButton
  → broadcastChanges only when accepted).
- `world/inventory/StonecutterMenu.java` (slots 0 input / 1 result / 2..38 player,
  `setupRecipeList` on input *item* change, `setupResultSlot`, button semantics,
  `quickMoveStack`, `removed`, `canTakeItemForPickAll`).
- `world/level/block/StonecutterBlock.java` (facing = opposite of placement
  direction, `useWithoutItem` opens the menu + `INTERACT_WITH_STONECUTTER`).
- `world/item/crafting/RecipeManager.finalizeRecipeLoading` +
  `SelectableRecipe.SingleInputSet.selectByInput` (list = full stonecutter set
  filtered by the input ingredient, registry key order).
- `world/inventory/AbstractContainerMenu.doClick` QUICK_MOVE loop (repeat while
  the slot refills with the same item).

Vecta changes:
- `inventory/menu/kind.rs`: new `MenuKind::click_menu_button` hook, default
  `false` (vanilla's default for menus without buttons).
- `inventory/menu/mod.rs`: `Menu::click_menu_button` forwards to the kind under
  one container-lock scope.
- `player/player_inventory/player_handlers.rs`: real
  `handle_container_button_click` (action-time reset, spectator gate, container-id
  gate via `take_open_menu_for_callback`, `still_valid` check, conditional
  broadcast, menu returned through `finish_open_menu_callback`). Test-only
  `Player::with_open_menu` inspection hook.
- `inventory/stonecutter.rs`: `StonecutterSelection` (recipe list + selected
  index shared by menu and result slot) and `write_result`.
- `inventory/menu/kinds/stonecutter_menu.rs`: stonecutter menu kind
  (`STONECUTTER` menu type, input/result sections, selection data slot, vanilla
  button/quick-move/pick-all/close semantics, `still_valid` = block + 4.0 range).
- `inventory/slots/stonecutter_slots.rs`: result handler consuming one input,
  refilling only while the input still feeds the recipe, and playing
  `UI_STONECUTTER_TAKE_RESULT` at most once per game tick.
- `inventory/recipe_manager.rs`: `StonecuttingRecipeRef` alias.
- `behavior/blocks/container/stonecutter_block.rs`: placement facing, menu open,
  stat.

Verification:
- `cargo test --locked -p vecta-core --lib stonecutter -j 1` → 7 passed
  (two recipes, input-item reset, out-of-range button, quick-move crafting loop,
  stale container id, removed block, menu kind).
- `cargo test --locked -p vecta-core --lib menu -j 1` → 111 passed.
- `cargo clippy --locked -p vecta-core --all-targets --all-features -j 1 -- -D warnings`
  → exit 0. `cargo fmt --all --check` → exit 0.

Not covered here: `ResultSlot` crafted-count / `ITEM_CRAFTED` / recipe award
(W03-06, W03-65), and the client recipe list (W03-01 blocker).

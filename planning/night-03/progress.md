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

## W03-02 — loom menu and banner patterns

Status: implemented and verified server-side.

Gap before: `minecraft:loom` had no block behavior and no menu kind, so the loom
could not be opened at all; banner patterns, `provides_banner_patterns` and the
`banner_pattern/no_item_required` tag existed in the registry but nothing read
them.

Vanilla sources used:
- `world/inventory/LoomMenu.java` (three restricted inputs, `selectablePatterns`
  from the pattern item or from `BannerPatternTags.NO_ITEM_REQUIRED`,
  `slotsChanged` single-pattern auto-select / previous-selection re-lookup /
  six-layer cap, `clickMenuButton`, `quickMoveStack` order, `removed` clearing the
  inputs, `onTake` consuming one banner + one dye, take sound once per tick).
- `world/level/block/LoomBlock.java` (facing = opposite of placement direction,
  `useWithoutItem` opens the menu + `INTERACT_WITH_LOOM`, no other state).

Vecta changes:
- `inventory/menu/kinds/loom_menu.rs`: `loom(...)` builder (3 restricted inputs,
  result slot, player inventory, selection data slot 0) and `LoomKind` with
  vanilla `slots_changed`, `click_menu_button`, `quick_move`, `still_valid`
  (`LOOM` + 4.0), `removed`, `can_take_item_for_pick_all`. Input gating uses the
  `#minecraft:banners`, `#minecraft:loom_dyes` + `minecraft:dye` and
  `#minecraft:loom_patterns` + `minecraft:provides_banner_patterns` data, never a
  hardcoded item list.
- `inventory/slots/loom_slots.rs`: `LoomResultHandler` consuming one banner and
  one dye per take and playing `UI_LOOM_TAKE_RESULT` at most once per game tick.
- `behavior/blocks/container/loom_block.rs`: placement facing, menu open, stat.
- `vecta-registry/src/registry/holder_set.rs`: `RegistryHolderSet::entries()`
  plus a `holder_set_tag_entries` tag arm, so a `provides_banner_patterns` holder
  set expands in tag order exactly like `ImmutableList.copyOf(HolderSet)`.
- Registered `loom` in `inventory/menu/kinds/mod.rs`, `inventory/slots/mod.rs`
  and `behavior/blocks/container/mod.rs`.

Verified vanilla quirk (kept on purpose): `slotsChanged` refuses to preview a
seventh layer, but `clickMenuButton` calls `setupResultSlot` without repeating
that check, so pressing a pattern button on a full banner still yields a
seven-layer preview. Vecta reproduces it and the regression test documents it
instead of diverging silently.

Verification:
- `cargo test --locked -p vecta-core --lib loom -j 1` → 5 passed (input
  restrictions, plain pattern + take consumption, pattern item replacing the
  list, six-layer behaviour, quick-move).
- `cargo test --locked -p vecta-core --lib menu:: -j 1` → 89 passed.
- `cargo clippy --locked -p vecta-core --all-targets --all-features -j 1 -- -D warnings`
  → exit 0; `-p vecta-registry --all-targets --all-features` → exit 0.
- `cargo fmt --all --check` → exit 0.

Not covered here: client-side pattern list sync (same missing recipe/display
foundation as W03-01, see `blockers.md`), `ResultSlot` crafted accounting
(W03-06), and the loom's banner save/load round-trip through item components
(already component-based, no menu state involved).

## W03-03 — lectern: book, pages, taking it back, redstone

Status: implemented and verified server-side. Real-client rendering is untested
(no client harness here); the lectern menu has no player slots, so only the book
slot and the page data slot are synced.

Gap before: `minecraft:lectern` had no block entity (the block entity type
existed in the registry with no factory, so every lectern loaded as an
`UnimplementedBlockEntity`), no block behavior and no menu. Books could not be
placed, read or taken back, and the comparator output was always 0.

Vanilla sources used:
- `world/level/block/LecternBlock.java` (`getStateForPlacement` with the
  creative `block_entity_data` check, `useItemOn` / `useWithoutItem` result
  matrix, `tryPlaceBook` + `placeBook`, `resetBookState`, `signalPageChange` →
  two-tick `POWERED` impulse with level event 1043, `ownSignal` /
  `getDirectSignal` / `getAnalogOutputSignal`, `tick`, `affectNeighborsAfterRemoval`).
- `world/level/block/entity/LecternBlockEntity.java` (`bookAccess` container
  semantics, `dataAccess` page slot, `setBook` / `setPage` / `getPageCount` /
  `getRedstoneSignal`, NBT `Book` + `Page`, `preRemoveSideEffects` drop in front
  of the block).
- `world/inventory/LecternMenu.java` (single slot, `DATA_COUNT = 1`, button ids
  1/2/3 and `100 + page`, `setData` → `setPage` → broadcast, `stillValid`).

Vecta changes:
- `block_entity/entities/lectern.rs`: `LecternBlockEntity` +
  `LecternBookContainer`. The container owns the book, the page, the page count
  and the pending `has_book` / page-impulse flags, and publishes them through
  `ContainerRef::owned_by_block_entity_with_callback`, so no world access ever
  happens under the container mutex. `set_item` is a no-op and `can_place_item`
  rejects everything, exactly like vanilla's `bookAccess`.
- `inventory/menu/kinds/lectern_menu.rs`: `lectern(...)` builder (one restricted
  slot + page data slot, `minecraft:lectern` layout of exactly one slot) and
  `LecternKind` with `click_menu_button` (prev / next / take / jump),
  `on_tick` page mirroring for several viewers, `still_valid` (container valid
  and a book present) and an empty `quick_move`. Page reads and writes go
  through the guard the hooks already hold, because re-locking the book
  container would deadlock.
- `behavior/blocks/container/lectern_block.rs`: placement facing + creative
  `has_book`, the full `useItemOn` / `useWithoutItem` matrix, book placement with
  page resolution, menu open + `INTERACT_WITH_LECTERN`, the two-tick powered
  impulse, own/direct/analog signals and the neighbor update below.
- `world/events.rs`: `drop_item_stack_at(position, item)` for vanilla's
  single-entity drop with default throw velocity, used by the lectern's book
  drop in front of the block.
- Registered the block entity factory and exported the new modules.

Written-content resolution reuses the fixed snapshot path: the placed copy is
taken out of the inventory *before* resolving, and resolution never runs while
an inventory lock is held. Block behaviors only receive `&Player`, so the
resolution source is the console fallback with the player as the entity
override; that only affects selector/score placeholders inside an unresolved
book at the moment it is placed, which the item path resolves identically when
the book is read in hand. Load-time resolution (`resolveBook(book, null)` in
vanilla) is not implemented because a block entity has no server handle here;
books are already resolved by the time they are saved, so the practical
difference is nil.

Verified vanilla quirks kept on purpose:
- `getPageCount` returns 0 for a book-less lectern and `Mth.clamp(page, 0, -1)`
  therefore loads `Page = -1`, while `onBookItemRemove` resets it to 0. Both are
  reproduced and asserted.
- `getRedstoneSignal` reports 14 for a book-less lectern; `LecternBlock`
  gates it behind `has_book`, which is why the comparator reads 0.

Verification:
- `cargo test --locked -p vecta-core --lib lectern -j 1` → 21 passed (8 block
  entity, 6 block behavior, 7 menu: clamps at both ends, jump buttons, invalid
  ids, take book + permission, two viewers, no placement, drop on break).
- `cargo test --locked -p vecta-core --lib -j 1` → 2963 passed, 0 failed,
  1 ignored.
- `cargo clippy --locked -p vecta-core --all-targets --all-features -j 1 -- -D warnings`
  → exit 0. `cargo fmt --all --check` → exit 0.

Not covered here: the lectern's `HAS_BOOK`/`POWERED` shape variants are already
data-driven, page-turn sound is emitted through level event 1043 only (no
explicit `SoundEvents.PAGE_TURN` call, matching vanilla), and a real client is
still required to confirm the book render and the page buttons.

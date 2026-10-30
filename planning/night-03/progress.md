# Night 03 — progress

Codex follow-up: [current review](../reviews/night-03-follow-up.md).
The owner resumed verification after the earlier codec-only commit. The
`slot_display` registry has now been extracted with SteelExtractor for 26.3;
generated typed entries replace handwritten numeric IDs. Regression tests cover
the three holder/ingredient codec fixes. The current review also corrects a
partial stonecutter quick-move result loss and the lectern's placement-time
book resolution context. See the linked report for actual check results and
remaining scope; the historical stage logs below are not full-parity claims.

Base: `codex/night-vanilla-02` tip `7db788bc7c44b12bf31842a6f3c0176a5d3f0d93`
(`0.16.1+mc26.3`, reference `minecraft-src` 26.3 / 5037 files).
Work branch: `codex/night-vanilla-03`.

Untouched foreign change: `planning/night-01/report.md` (never staged here).

## W03-01 — container button clicks + working stonecutter

Status: server menu implemented and tested. Recipe-list sync was added later in
this branch; see the current review and `blockers.md`. A real-client session is
still untested.

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

Written-content resolution runs on a snapshot before taking any inventory
locks. Codex review corrected the source to use the placing player, the lectern
world and its center. The original console/spawn fallback could resolve `@s`
and relative block NBT incorrectly; the entity override only supplied a default
scoreboard name and did not fix selectors.

Load-time resolution (`resolveBook(book, null)` in vanilla) remains open because
block entities have no server read context. Saved or command-created unresolved
books are possible, so this omission cannot be called unobservable. See B-W03-06
for the pending architecture question.

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

## W03-04 — chiseled bookshelf: reproduced divergences

Status: audited line by line and fixed; the implementation was **not** rewritten.

Gap before: the block behavior gated both interactions on the *stored stack*
(`bookshelf.item(slot).is_empty()`) while Vanilla gates them on the
`slot_N_occupied` block-state property. The two normally agree because
`updateState` recomputes every property from the contents, but
`BlockItem.updateBlockEntityComponents` → `applyComponentsFromItemStack` →
`applyImplicitComponents` copies the `minecraft:container` items **without**
recomputing the properties. A bookshelf placed from such a stack therefore holds
books that the block state calls absent, and the two implementations diverged:

| case | Vanilla (state-gated) | Vecta before (content-gated) |
| --- | --- | --- |
| empty hand on a "free" occupied slot | `CONSUME`, book stays | book extracted into the inventory |
| book in hand on a "free" occupied slot | `setItem` overwrites, hidden book destroyed | `TRY_WITH_EMPTY_HAND` → extraction |

Also fixed: `useItemOn` checked the item tag before the block entity (Vanilla
returns `PASS` when the block has no `ChiseledBookShelfBlockEntity`) and
`useWithoutItem` skipped the pickup sound and the game event when the slot was
empty.

Vanilla sources used: `ChiseledBookShelfBlock` (`useItemOn`, `useWithoutItem`,
`addBook`, `removeBook`, `getStateForPlacement`, `affectNeighborsAfterRemoval`,
`getAnalogOutputSignal`), `ChiseledBookShelfBlockEntity` (`updateState`,
`loadAdditional`/`saveAdditional`, `acceptsItemType`, `removeItem`, `setItem`,
`canTakeItem`, `applyImplicitComponents`, `collectImplicitComponents`),
`SelectableSlotContainer.getHitSlot`, `ListBackedContainer.canPlaceItem`,
`BlockItem.updateBlockEntityComponents`, `Containers.dropContents` /
`updateNeighboursAfterDestroy`, `ServerPlayerGameMode.useItemOn`.

Vecta changes:
- `behavior/blocks/container/chiseled_bookshelf_block.rs`: new
  `OCCUPIED_PROPERTIES` table + `is_slot_occupied`; `use_item_on` now resolves the
  block entity first, then the tag, then the hit slot, then the **state
  property**; `use_without_item` gates on the state property, always plays the
  pickup sound and always fires `BLOCK_CHANGE`.
- `block_entity/entities/chiseled_bookshelf.rs`: `insert_book` no longer refuses
  an occupied slot — Vanilla's `setItem` override writes unconditionally, so the
  occupancy gate belongs to the block behavior only.

Audited and already matching (no change): hit-region math for all four facings
and both rows (`Mth.clamp(Mth.floor(px / (16/3|16/2)), 0, n-1)`, face-origin
subtraction), `BOOKSHELF_BOOKS` tag gate, one-book insertion, enchanted-book
insert/pickup sound selection, `removeItem` ignoring `count`, oversized stacks
kept unclamped, `last_interacted_slot` + `Items` NBT round trip (including a
reused target), `getLastInteractedSlot() + 1` comparator (14 for slot 5, 0 when
never touched), `Containers.updateNeighboursAfterDestroy` →
`updateNeighbourForOutputSignal`, `preRemoveSideEffects` → `dropContents` with
Vanilla's per-item block-local offsets, `applyImplicitComponents` /
`collectImplicitComponents` equivalence, and full-container (unsided) automation
access because the block entity is a `Container`.

Verification:
- `cargo test --locked -p vecta-core --lib chiseled_bookshelf -j 1` → 17 passed.
  New: stale-property gating for both interactions, off-hand insertion, removal
  into a full inventory (drop at the player's feet), automation access through
  the container ref publishing `slot_0_occupied`.
- `cargo test --locked -p vecta-core --lib -j 1` → 2967 passed, 0 failed,
  1 ignored.
- `cargo clippy --locked -p vecta-core --all-targets --all-features -j 1 -- -D warnings`
  → exit 0. `cargo fmt --all --check` → exit 0.

Not covered here: `handle_pick_item_from_block` still carries
`// TODO: If include_data, copy the block entity data into the picked item
stack.`, so `collectImplicitComponents` has no consumer — creative pick-block
returns a bookshelf without its `minecraft:container` component. That is a
cross-cutting block-entity component gap (see `blockers.md`, B-W03-04), not a
chiseled-bookshelf-specific defect.

## W03-05 — smithing table: upgrades and trims

Status: implemented end to end (menu + block + assembly); server-authoritative.

Gap before: `minecraft:smithing_table` had no block behavior and no menu, and the
`SmithingRecipe` data in `vecta-registry` was passive — nothing called `assemble`,
nothing derived the three input predicates, and there was no trim assembly.

Vanilla sources used: `SmithingMenu` (slot layout, `createInputSlotDefinitions`,
`onTake`, `shrinkStackInSlot`, `slotsChanged`/`hasRecipeError`,
`canMoveIntoInputSlots`, `canTakeItemForPickAll`), `ItemCombinerMenu`
(`slotsChanged` → `createResult` only for the input container, `removed` →
`clearContainer`, `stillValid`, `quickMoveStack` branch order), `SmithingTableBlock`
(menu provider + `container.upgrade` title + `INTERACT_WITH_SMITHING_TABLE`),
`SmithingTransformRecipe.assemble` → `TransmuteRecipe.createWithOriginalComponents`,
`SmithingTrimRecipe.assemble` → `applyTrim`, `RecipeManager.RECIPE_PROPERTY_SETS`
(`SMITHING_TEMPLATE` / `SMITHING_BASE` / `SMITHING_ADDITION`).

Vecta changes:
- `inventory/recipe_manager.rs`: `SmithingRecipeRef`, `find_smithing_recipe`,
  `assemble_smithing` (transform = result template + base component patch; trim =
  `apply_trim` with `provides_trim_material` + the recipe's pattern, empty when the
  trim is unchanged), and `SmithingItemTests::collect()` which derives the three
  role item sets from every registered smithing recipe the way
  `finalizeRecipeLoading` does (no `RecipePropertySet` registry exists here).
- `inventory/slots/smithing_slots.rs`: `SmithingResultHandler` — shrinks one item
  from slots 0/1/2, notifies the inputs, plays `levelEvent(1044)`.
- `inventory/menu/kinds/smithing_menu.rs` + `tests.rs`: `SMITHING` menu with three
  per-slot restricted inputs, virtual result, `hasRecipeError` data slot 0,
  `slots_changed` recompute, `still_valid` (block + 4.0), `removed` clears the
  result, `canTakeItemForPickAll` excluding the result, and vanilla's
  `quickMoveStack` order including `canMoveIntoInputSlots`.
- `behavior/blocks/container/smithing_table_block.rs`: placement state, menu open
  with the `container.upgrade` title and the interaction stat.

Verified vanilla quirks kept on purpose:
- The result is recomputed from the *current* three inputs on every change, so a
  swapped base changes the result before any take (tested).
- Re-applying the same material+pattern yields an empty result (`applyTrim`'s
  equality check), while a different pattern re-trims (tested).
- `hasRecipeError` is 1 only when all three inputs are filled *and* the result is
  empty; it is not an "invalid recipe" flag.

Verification:
- `cargo test --locked -p vecta-core --lib smithing -j 1` → 7 passed (upgrade with
  carried damage/custom name, take consumes one from each input, trim
  material+pattern, identical trim refused / different pattern applied,
  mismatched template+base → error flag, base swap before take, full-inventory
  quick-move).
- `cargo test --locked -p vecta-core --lib -j 1` → 2974 passed, 0 failed,
  1 ignored.
- `cargo clippy --locked -p vecta-core --all-targets --all-features -j 1 -- -D warnings`
  → exit 0. `cargo fmt --all --check` → exit 0.

Not covered here: `onTake`'s `carried.onCraftedBy` and
`resultSlots.awardUsedRecipes` (crafted-count / `ITEM_CRAFTED` / recipe awards are
W03-06 and W03-65, and `ResultSlot::on_take` already carries that TODO), and the
client recipe-book preview (B-W03-01). The result slot also has no
`setRecipeUsed` equivalent, so the smithing recipe is not recorded for
advancements yet.

## W03-01 (client path) — `ClientboundUpdateRecipesPacket`

Status: implemented; the packet is built, encoded and sent on login. The wire
format is covered by tests. The `vecta-core` runtime test for the collection
functions could not be linked in this session (see `blockers.md`, B-W03-05).

Gap before: `blockers.md` claimed the packet needed the whole `RecipeDisplay`
family. Reading `ClientboundUpdateRecipesPacket.java` and `SelectableRecipe.java`
shows the packet carries only a property-set map and
`SingleInputSet<StonecutterRecipe>` entries, and `SelectableRecipe.noRecipeCodec()`
writes **only** the `SlotDisplay`; the recipe itself is `Optional.empty()`.
`RecipeDisplay` is not on this wire at all.

Vanilla sources used:
- `network/protocol/game/ClientboundUpdateRecipesPacket.java` (two fields and
  their codecs).
- `world/item/crafting/RecipePropertySet.java` (`Item.STREAM_CODEC` list; the
  nine `TYPE_KEY` entries).
- `world/item/crafting/SelectableRecipe.java` (`noRecipeCodec` chain,
  `SingleInputEntry`, `SingleInputSet`).
- `world/item/crafting/Ingredient.java` (`CONTENTS_STREAM_CODEC` =
  `ByteBufCodecs.holderSet(ITEM)`).
- `world/item/crafting/display/SlotDisplay.java` + `SlotDisplays.java` (variant
  payloads and built-in registration order).
- `world/item/crafting/StonecutterRecipe.java` (`resultDisplay()` returns
  `SlotDisplay.ItemStackSlotDisplay(result)`).
- `world/item/crafting/RecipeManager.java` (`finalizeRecipeLoading`,
  `RECIPE_PROPERTY_SETS`, the stonecutter list build).
- `server/players/PlayerList.java` (sent once in `placeNewPlayer`, after the held
  slot and before the permission level).

Vecta changes:
- `vecta-registry/src/recipe/display.rs`: `SlotDisplay` (all eleven kinds) with
  `WriteTo`/`ReadFrom`, `SingleInputEntry`, `RecipePropertySet`,
  `RecipePropertySetKey`, and the holder-set/ingredient encoding.
- `vecta-protocol/src/packets/game/c_update_recipes.rs`: `CUpdateRecipes`
  (`C_UPDATE_RECIPES` id was already generated).
- `vecta-core/src/inventory/recipe_manager.rs`: `synchronized_item_properties`
  (one pass over `REGISTRY.recipes` in registration order, mirroring
  `finalizeRecipeLoading`) and `synchronized_stonecutter_recipes`.
- `vecta-core/src/player/lifecycle/spawn_sync.rs`: send on `InitialJoin` only;
  respawn and world change do not resend, matching `placeNewPlayer` versus
  `respawn`.

Verification:
- `cargo test -p vecta-protocol --lib c_update_recipes -j 1` → **4 passed**:
  empty packet (two zero counts), property-set entry (key then item ids),
  stonecutter entry (named holder set, `item_stack` kind 5, template), composite
  display. Expected bytes are written out by hand from the vanilla codec rules.
- `cargo check -p vecta-core --lib --tests -j 1` → exit 0 (sources and tests
  type-check).
- `cargo test -p vecta-core --lib sync_tests` → **not run**: the test binary
  cannot be linked in this session (B-W03-05).

Known limitations:
- `slot_display` kind ids are a Rust enum mirroring `SlotDisplays.bootstrap`
  because no `slot_display` extractor output exists in the repo and the
  SteelExtractor checkout is absent. This is the one place where a built-in
  registry order is transcribed; it is not a guess, but it is not extracted data.
- The property-set map and the stonecutter list are recomputed on every login
  instead of being cached at load time like Vanilla's `RecipeManager` fields.
  Correct, but wasteful for many simultaneous joins.
- Brewing property sets are sent empty because Vecta loads no brewing recipes.
- Only `SlotDisplay::ItemStack` is ever produced today; the other kinds are
  encoded/decoded but unused until the recipe book (W03-65).

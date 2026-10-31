# Night 03 — progress

Lectern NBT follow-up: [explicit load read context](../reviews/lectern-load-context.md).
The owner selected a separate read context. Initial disk/worldgen decoding
precedes Vanilla level binding and leaves pages unresolved; live command NBT
updates receive the context explicitly. No World-to-Server link was added.
Runtime verification is now complete on `015152051`: five load-context tests
passed, followed by the workspace suite, **3890 passed / 0 failed / 18 ignored**.
B-W03-07 is closed. The owner explicitly requested continuation without restart;
the saved 16 GB D: pagefile setting has not yet taken effect. Test commands and
build settings remained unchanged. Next gameplay stage: W03-06.

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

Load-time resolution was narrowed after checking `BlockEntity.loadStatic` and
`LevelChunk.setBlockEntity`: initial decoding occurs before level binding and
does not resolve books. Attached NBT updates now receive the owner-approved
read context. See B-W03-06 and the linked load-context review for scope and checks.

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

## W04-001..006 — результат крафта: реальный счётчик, ITEM_CRAFTED и контракты слотов

Связь: разворачивает W03-06. W03-65 (книга рецептов) остаётся открытой.

Что теперь делает игрок:
- Взятие результата крафта (ЛКМ, Shift, номер слота, Q, Ctrl+Q, ПКМ) начисляет
  `minecraft:crafted` за фактически взятое количество, включая выход больше 1.
- ПКМ по результату берёт половину, Q выбрасывает один предмет. Раньше
  `ResultSlot::remove` забирал весь preview независимо от запрошенного
  количества, поэтому Q выбрасывал весь результат и засчитывал один крафт.
- Камнерез и кузнечный стол начисляют `crafted` за взятый стак (как в 26.3);
  наковальня, точило и ткацкий станок не начисляют.
- Тара возвращается в свой слот сетки, несовместимый остаток уходит в инвентарь,
  сдвинутый shaped-рецепт расходует только свои слоты.

Проверенные Java-методы 26.3: `ResultSlot.remove/onQuickCraft/onSwapCraft/
checkTakeAchievements/onTake/safeClone/isFake`, `Slot.tryRemove/safeTake/
safeClone/onQuickCraft`, `AbstractContainerMenu.doClick` (PICKUP, QUICK_MOVE,
SWAP, CLONE, THROW, PICKUP_ALL), `CraftingMenu.quickMoveStack`,
`StonecutterMenu.quickMoveStack` и его анонимный result slot,
`SmithingMenu.onTake`, result slots `AnvilMenu`/`GrindstoneMenu`/`LoomMenu`,
`ItemStack.onCraftedBy/onCraftedBySystem`, `Item.onCraftedBy/
onCraftedPostProcess`, `MapItem.onCraftedPostProcess`.

Vecta paths: `inventory/slots/slot.rs` (`on_quick_craft`, `on_swap_craft`),
`inventory/slots/result_slot.rs` (`CraftedStat`, `remove`, `on_take`),
`inventory/menu/builder.rs` (`result_slot_with`), `inventory/menu/layout.rs`,
`inventory/menu/mod.rs` (`do_swap`), `inventory/menu/kinds/{crafting,inventory,
stonecutter,smithing}_menu.rs`, `player/stats_counter.rs` (`Player::get_stat`).

Общие контракты и их consumers: `Slot` получил два no-op хука, реализует их
только `ResultSlot`. Способ начисления выбирает каждое меню через `CraftedStat`,
поэтому crafting-семантика не навязана наковальне, точилу и ткацкому станку.

Проверки: `cargo test -p vecta-core --lib` — 2996 passed / 0 failed / 1 ignored;
новые 11 тестов `crafting_menu::tests::crafted_result` плюс регрессии камнереза,
кузнечного стола и сетки 2x2 инвентаря. Clippy/fmt — см. итоговый отчёт.

Незакрытое:
- `Item.onCraftedBy` → `onCraftedPostProcess`: единственное переопределение в
  26.3 — `MapItem` (снимает `MAP_POST_PROCESSING`, lock/scale через
  `MapItemSavedData`). Карт и map saved data в Vecta нет, поэтому пустой hook не
  добавлялся; зависимость — W04-092..095. Рецепт `map_extending` уже ставит
  `MAP_POST_PROCESSING`, но потребителя нет.
- `ItemStack.onCraftedBySystem` — crafter, W04-030.
- `RecipeCraftingHolder.awardUsedRecipes` — W04-008/010 (книга рецептов).

## W04-011..014 (частично) — implicit components блок-сущностей и pick-block с данными

Связь: закрывает B-W03-04 в части общего hook и первого настоящего consumer.

Что теперь делает игрок:
- Средний клик в креативе по книжной полке с книгами даёт предмет с теми же
  книгами в `minecraft:container`; установка этого предмета возвращает книги.
- NBT предмета (`minecraft:block_entity_data`) не дублирует содержимое: поле
  `Items` снимается через `removeComponentsFromTag`, как в 26.3.
- `/data get block` снова показывает поле `components`, включая custom name,
  который был на предмете при установке.

Java 26.3: `BlockEntity.applyComponentsFromItemStack/applyComponents/
collectComponents/applyImplicitComponents/collectImplicitComponents/
removeComponentsFromTag`, `saveWithFullMetadata`/`saveWithoutMetadata`/
`saveCustomOnly`, `ServerGamePacketListenerImpl.handlePickItemFromBlock` и
`addBlockDataToItem`, `BlockItem.setBlockEntityData`/`updateBlockEntityComponents`,
`ChiseledBookShelfBlockEntity` (container + discard `Items`),
`BaseContainerBlockEntity` (контракт CONTAINER).

Vecta paths: `block_entity/mod.rs` (хуки, `stored_components`/
`set_stored_components`, `save_with_full_metadata` с `components`),
`block_entity/entities/chiseled_bookshelf.rs`,
`behavior/blocks/container/chiseled_bookshelf_block.rs` (`set_placed_by` больше
не применяет компонент), `player/game_mode/block_interaction.rs`
(`add_block_data_to_item`), `vecta-registry` `item_stack.rs` (`set_raw`),
`patch.rs` (`clear_raw`).

Общий контракт: `apply_components_from_item` теперь сам применяет implicit
часть и сохраняет оставшийся patch, исключая `block_entity_data` и
`block_state`. Книжная полка — первый consumer; shulker box, табличка и голова
пока переопределяют `apply_components_from_item` целиком и будут переведены на
`apply_implicit_components` отдельным шагом W04-012/014.

Проверки: `cargo test -p vecta-core --lib chiseled_bookshelf` — 18 passed,
включая новый round-trip pick → place; полный core suite и clippy — в отчёте.

Незакрытое:
- W04-012 остаётся частичной: `updateCustomBlockEntityTag` (BLOCK_ENTITY_DATA →
  NBT) по-прежнему применяется per-block (`set_placed_by` жукбокса, лектерна,
  табличек), общего placement-пути для NBT нет.
- W04-014 частично: save-сторона (`save_with_full_metadata` с `components`)
  готова; owned/borrowed load и chunk unload/load ещё не покрыты регрессией.

## W04-021 (частично) — одиночный сундук как контейнер

Связь: W03-10. Основание: готовый контейнер/меню бочки.

Что теперь делает игрок:
- ПКМ по сундуку открывает меню 3x9 с заголовком `container.chest` и
  начисляет `open_chest`; закрытие меню освобождает зрителя.
- Содержимое переживает NBT round-trip, компаратор показывает заполненность,
  разрушение выбрасывает предметы, установка ставит сундук лицом к игроку.

Java 26.3: `ChestBlock` (facing/тип/waterlogged, `useWithoutItem`,
`getAnalogOutputSignal`), `ChestBlockEntity` (27 слотов, `Items`, open count,
block event 1), `ChestMenu`/`ChestKind` (закрытие через `removed`),
`ContainerOpenersCounter.incrementOpeners/decrementOpeners` в части
наблюдаемого сервером счётчика и block event.

Vecta paths: `block_entity/entities/chest.rs`,
`behavior/blocks/container/chest_block.rs`, `block_entity/registry.rs`
(фабрика CHEST), `inventory/menu/kinds/chest_menu.rs` (`ChestKind::new`,
`with_close_hook`, `removed`).

Проверки: `cargo test -p vecta-core --lib chest_block` — 3 passed (открытие/
закрытие зрителя, сигнал компаратора, NBT round-trip).

Незакрытое:
- W04-022: двойной сундук (`CHEST_TYPE`, `CompoundContainer`, порядок половин).
- W04-023: `ContainerOpenersCounter` recheck по scheduled tick и spectator;
  сейчас счётчик меняется только на open/close и сбрасывается при removal.
- W04-016: loot table сундука (`container_loot`) не распаковывается.
- W04-017: lock/custom name меню не проверяются.

## W04-017 — CustomName, Lock и права открытия (общий слой)

Связь: W04-021. Java 26.3: `BaseContainerBlockEntity` (`loadAdditional`/
`saveAdditional` для `CustomName`/`lock`, `getName`/`getDisplayName`/`canOpen`,
`sendChestLockedNotifications`, `isLocked`, `createMenu` с проверкой lock,
`applyImplicitComponents`/`collectImplicitComponents`/
`removeComponentsFromTag`), `LockCode.canUnlock`, `ServerPlayer.openMenu`,
`ChestBlock`/`BarrelBlock`/`BeaconBlock`/`ShulkerBoxBlock.useWithoutItem`,
`ShulkerBoxBlock.playerWillDestroy` (`collectComponents` на предмете),
`ItemPredicate.test`/`DataComponentMatchers.test`/`CollectionPredicate`.

Что теперь делает игрок:
- Сундук, бочка, шалкер и бикон сохраняют `CustomName` и `lock` в мире и
  показывают имя как заголовок меню.
- Запертый контейнер не открывается без подходящего предмета в основной руке:
  оверлей `container.isLocked` + звук `block.chest.locked`, зритель не
  считается (как `ChestBlockEntity.startOpen`); спектатор открывает всегда.
- Шалкер, поставленный с именем/замком/содержимым, и выпавший шалкер несут
  `custom_name`, `lock` и `minecraft:container` — как `collectComponents`.
- Бочка и бикон применяют эти компоненты при установке предмета.

Vecta paths: `vecta-registry/src/data_component_predicate/*` (полная цепочка
`matches`: exact → partial → `ItemPredicate` → `LockCode`),
`vecta-registry/src/item_predicate.rs` (`ItemPredicate::test`,
`IntBounds::matches`, `DoubleBounds::matches`, `NbtPredicate::matches`,
`LockCode::unlocks_with`/`is_no_lock`/`Default`),
`block_entity/container_identity.rs` (новый общий `ContainerIdentity`),
`block_entity/container_component.rs` (новый общий CONTAINER-хелпер),
`block_entity/entities/{chest,barrel,shulker_box,beacon}.rs`,
`behavior/blocks/container/{chest,barrel,shulker_box,beacon}_block.rs`.

Проверки: `chest_block` 11 passed, `barrel` 4, `shulker_box` 2,
`barrel_block` 2, `shulker_box_block` 2, `container_identity` 3,
`container_component` 2, `data_component_predicate` 7.

Незакрытое:
- W04-016: loot table (`LootTable`/`LootTableSeed`) по-прежнему не
  распаковывается; `unpackLootTable` в шалкере остаётся TODO.
- W04-018..020: меню-жизненный цикл и синхронизация.
- Медные сундуки: блок-энтити ещё нет (`copper_chest_block.rs` — TODO).

## W04-016 — контейнеры со loot table без преждевременной генерации

Связь: W04-017. Java 26.3: `RandomizableContainerBlockEntity` (`isEmpty`/
`getItem`/`setItem`/`removeItem` → `unpackLootTable(null)`, `canOpen` с
`lootTable != null && spectator`, `createMenu` → `unpackLootTable(player)`,
`applyImplicitComponents`/`collectImplicitComponents`/`removeComponentsFromTag`
для `minecraft:container_loot`), `RandomizableContainer`
(`tryLoadLootTable`/`trySaveLootTable`/`unpackLootTable`),
`LootTable.fill` (`getAvailableSlots`, `shuffleAndSplitItems`),
`Containers.dropContents`.

Что теперь делает игрок:
- Сундук/бочка/шалкер из структуры с `LootTable` больше не открывается пустым:
  при первом открытии, чтении компаратором или разрушении таблица
  разыгрывается один раз, ссылка сбрасывается, дальше контейнер обычный.
- Пока таблица не разыграна, в сейв идут `LootTable`/`LootTableSeed` и НЕ идут
  `Items` (как `trySaveLootTable`/`tryLoadLootTable`); после розыгрыша —
  наоборот.
- `minecraft:container_loot` переносится на предмет (шалкер в т.ч. через
  `shulker_box_as_item`) и применяется при установке.

Vecta paths: `vecta-registry/src/loot_table/entries.rs` (`LootTable::fill`,
`available_slots`, `shuffle_and_split_items`, `shuffle`),
`block_entity/container_loot.rs` (новый `ContainerLoot`),
`block_entity/entities/{chest,barrel,shulker_box}.rs` (`unpack_loot_table`,
load/save, компоненты, `pre_remove_side_effects`),
`behavior/blocks/container/{chest,barrel,shulker_box}_block.rs` (открытие,
компаратор, разрушение).

Проверки: `container_loot` 4 passed, `chest_block` 12, `barrel` 5.

Блокер (точный): детерминированный `LootTableSeed` не воспроизводится.
`vecta-registry::loot_table` считает лут на `rand::Rng`, а ванильный
`LootContext.Builder.withOptionalRandomSeed` использует
`RandomSource.create(seed)` = `LegacyRandomSource`, чья
последовательность (`nextInt(bound)`, `nextFloat`) не совпадает с `rand`.
`vecta_utils::random::legacy_random::LegacyRandom` реализует Java-совместимый
трейт `Random`, а не `rand::RngCore`, поэтому сид сейчас не применяется:
лут разыгрывается один раз (как в ванили), но набор предметов для ненулевого
сида не совпадает с ванильным. Чтобы закрыть, нужно перевести весь обход
`NumberProvider`/`LootCondition`/`LootFunction` на `vecta_utils::random::Random`
(и `RandomSequences` для `random_sequence`), это отдельный этап.

Незакрытое:
- `Container::set_item` (автоматизация/хоппер) не разыгрывает таблицу перед
  вставкой: в ванили `RandomizableContainerBlockEntity.setItem` вызывает
  `unpackLootTable(null)`, а в Vecta `Container` реализован на данных
  контейнера без обратной ссылки на блок-энтити. Вставка в лут-сундук
  заполнит пустые слоты, а таблица разыграется позже в остальные.
- Компаратор разыгрывает лут (как ванильный
  `AbstractContainerMenu.getRedstoneSignalFromContainer`), но делает это под
  собственным чтением слотов — отдельного `randomizable`-контейнера нет.

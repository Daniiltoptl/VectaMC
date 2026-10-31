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

## W04-023 — recount зрителей по scheduled tick

Связь: W04-016/021. Java 26.3: `ContainerOpenersCounter`
(`incrementOpeners`/`decrementOpeners` с `maxInteractionRange`,
`getEntitiesWithContainerOpen`, `recheckOpeners`, `scheduleRecheck` = 5 тиков),
`Player.hasContainerOpen(ContainerOpenersCounter, BlockPos)`,
`ChestBlock.tick` → `ChestBlockEntity.recheckOpen`.

Что теперь делает игрок:
- Сундук больше не может остаться «навсегда открытым»: пока счётчик > 0,
  каждые 5 тиков блок пересчитывает реальных зрителей — тех, у кого меню всё
  ещё указывает на этот контейнер и кто в пределах досягаемости. Ушедший/умерший
  игрок выпадает, крышка закрывается, звук и block event согласованы.
- `increment_openers` теперь планирует recount, `maxInteractionRange`
  учитывается как в ванили (радиус поиска = max + 4.0).

Vecta paths: `block_entity/container_openers.rs` (`CHECK_TICK_DELAY`,
`max_interaction_range`, `count_container_users`, `recheck_openers`,
`schedule_recheck`, `is_own_container` в листенере),
`inventory/menu/kind.rs` + `mod.rs` (`MenuKind::owns_container`,
`Menu::owns_container`), `inventory/menu/kinds/chest_menu.rs` (`ChestKind`),
`player/player_inventory/player_handlers.rs`
(`Player::has_container_open_for`), `block_entity/entities/chest.rs`
(`recheck_open`), `behavior/blocks/container/chest_block.rs`
(`BlockBehavior::tick`).

Проверки: `chest_block` 15 passed, включая recount с зарегистрированным в мире
игроком: «зритель остался» → 1, «зритель ушёл из радиуса» → 0 через
`recheck_open` и через `BlockBehavior::tick`.

Незакрытое:
- Double chest (W04-022) пока не делит контейнер: `CompoundContainer` нет,
  поэтому `owns_container` сравнивает один `ContainerRef`.
- Бочка/шалкер пока не используют общий счётчик (W04-026).

## W04-026 — barrel, shulker и ender storage

Связь: W04-023. Java 26.3: `BarrelBlockEntity` (счётчик
`ContainerOpenersCounter`, `OPEN` через `updateBlockState`, звук из грани
`FACING`, `BarrelBlock.tick` → `recheckOpen`), `ShulkerBoxBlockEntity`
(свой `openCount`, `blockEvent(1, count)`, звук и game event только при
переходе через 1), `EnderChestBlockEntity` (общий счётчик, звук в центре
блока, `openerCountChanged` → `blockEvent(1, current)`,
`isOwnContainer` = `isActiveChest`, `EnderChestBlock.tick` → `recheckOpen`),
`PlayerEnderChestContainer.startOpen`/`stopOpen`/`isActiveChest`.

Что теперь делает игрок:
- Бочка открывается/закрывается со звуком из той грани, куда она смотрит,
  свойство `OPEN` переключается у первого/последнего зрителя, а ушедший
  игрок перестаёт держать крышку открытой (recount по scheduled tick).
- Шалкер-бокс при открытии поднимает крышку: `startOpen` считает зрителей,
  шлёт block event 1 (анимация + коллизия) и играет звук; при закрытии —
  `stopOpen`, звук и анимация закрытия. Второй зритель звук не повторяет.
  Свой счётчик оставлен намеренно: в 26.3 `ShulkerBoxBlockEntity` не
  использует `ContainerOpenersCounter` и не планирует recount.
- Эндер-сундук снова щёлкает крышкой: общий счётчик, звук открытия/закрытия,
  block event 1 для клиентской анимации, recount по тику. Хранилище осталось
  пер-игроковым: `PlayerEnderChestContainer` пересылает open/close в активный
  сундук и забывает его при закрытии.

Vecta paths: `block_entity/entities/barrel.rs` (`open_count`, `start_open`,
`stop_open`, `recheck_open`, `play_sound`, `update_open_state`, листенер),
`block_entity/entities/shulker_box.rs` (`open_count`, `start_open`,
`stop_open`, `send_open_count`, `play_open_sound`),
`block_entity/entities/ender_chest.rs` (счётчик, листенер, `trigger_event`),
`inventory/ender_chest.rs` (`is_active_chest`, `start_open`, `stop_open`),
`inventory/menu/kinds/shulker_box_menu.rs` (`ShulkerBoxKind::with_close_hook`,
`shulker_box_with_kind`), `inventory/menu/kinds/ender_chest_menu.rs`
(`removed` → `stop_open`), `behavior/blocks/container/{barrel,shulker_box,
ender_chest}_block.rs`.

Проверки: `barrel` 14, `shulker_box` 7, `ender_chest` 4. Звуки проверяются по
реально отправленному `C_SOUND` пакету (общий `RecordingConnection` и
`played_sounds` в `tests/support/connection.rs`), а не по вызову в коде.

Незакрытое:
- Double chest (W04-022) по-прежнему без `CompoundContainer`.
- `PiglinAi.angerNearbyPiglins` для бочки/шалкера/эндер-сундука — TODO.
- Анимация крышки эндер-сундука считается на клиенте по block event 1:
  серверного `getOpenNess` нет, как и у сундука.

### Найденный по пути дефект: block event не доходил до клиентов

Пока писался тест на звук/анимацию, выяснилось, что block event вообще не
публиковался: `World::do_block_event` спрашивает
`BLOCK_BEHAVIORS.get_behavior(block).trigger_event(...)`, а дефолт
`BlockBehavior::trigger_event` возвращает `false`. В ванили это делает
унаследованный `BaseEntityBlock.triggerEvent`, который пересылает событие в
`blockEntity.triggerEvent(id, param)`. У `ShulkerBoxBlock` пересылка была, у
`ChestBlock`/`EnderChestBlock` — нет, поэтому крышка сундука (W04-023) и
эндер-сундука не анимировалась у клиента: событие ставилось в очередь и
отбрасывалось.

Исправлено: `ChestBlock::trigger_event` и `EnderChestBlock::trigger_event`
пересылают событие в блок-энтити, как `BaseEntityBlock`. Тесты
`opening_a_chest_broadcasts_the_lid_event_to_clients` и проверка block event в
`opening_an_ender_chest_counts_the_viewer_and_plays_the_lid_sound` читают
реально отправленный `C_BLOCK_EVENT` (позиция, id 1, счётчик зрителей, id
блока) — до исправления первый падал с «0 пакетов».

## W04-022 — двойной сундук и порядок половин

Связь: W04-021, W04-023. Java 26.3: `ChestBlock` (`getStateForPlacement` с
`isSecondaryUseActive`/`getClickedFace`, `getChestType`,
`candidatePartnerFacing`, `updateShape` с `chestCanConnectTo`,
`getConnectedDirection`, `getBlockType`, `combine` через
`DoubleBlockCombiner.combineWithNeigbour`, `CHEST_COMBINER`,
`MENU_PROVIDER_COMBINER`, `getAnalogOutputSignal`),
`DoubleBlockCombiner.java`, `CompoundContainer.java`, `ChestMenu.sixRows`,
`ChestBlockEntity.isOwnContainer` (`isActiveChest` через
`CompoundContainer.contains`), `RandomizableContainerBlockEntity.canOpen`.

Что теперь делает игрок:
- Второй сундук, поставленный вплотную к одиночному с тем же FACING,
  срастается в пару: новый получает RIGHT, старый через `updateShape` —
  LEFT. Shift-размещение в боковую грань соседнего сундука разворачивает
  новый сундук лицом к соседу, как в ванили.
- Слом любой половины возвращает вторую в SINGLE по shape update.
- Пара открывается меню на 6 рядов (`generic_9x6`), порядок слотов
  ванильный: сначала RIGHT-половина (27 слотов), затем LEFT. Имя берётся из
  `CustomName` первой половины, иначе второй, иначе
  `container.chestDouble`.
- Обе половины считают зрителя: `startOpen`/`stopOpen` идут на каждую,
  поэтому у обеих `open_count` одинаковый, а звук крышки играет только
  RIGHT-половина из середины пары (у LEFT `play_lid_sound` молчит).
- Закрытая половина (проводник сверху) или запертая половина отменяет пару
  целиком: меню не открывается, счётчики остаются нулевыми, уведомление о
  замке звучит из середины пары.
- Компаратор читает обе половины как одно пространство на 54 слота.

Vecta paths: `behavior/blocks/container/chest_block.rs` (`ChestHalf`,
`ChestPair`, `half`, `get_connected_direction`, `chest_can_connect_to`,
`candidate_partner_facing`, `get_chest_type`, `combine`, `open_menu`,
`get_state_for_placement`, `update_shape`, `get_analog_output_signal`),
`inventory/menu/kinds/chest_menu.rs` (`chest_parts_with_kind`, `ChestKind`
теперь держит список контейнеров, `ChestKind::halves`),
`inventory/menu/builder.rs` (`MenuBuilder::span`),
`inventory/container/mod.rs` (`calculate_redstone_signal_from_containers`),
`block_entity/entities/chest.rs` (`can_open` с учётом незакатанной loot
table), `block_entity/container_identity.rs`
(`send_locked_notification_at` для позиции между половинами),
`vecta-registry/src/blocks/properties.rs` (`ChestType::opposite`),
`behavior/blocks/container/copper_chest_block.rs` (использует общий
`ChestBlock::get_connected_direction` и `ChestType::opposite` вместо своей
копии).

Решение по `CompoundContainer`: ванильный класс отдаёт слоты через
`getItem(slot)`/`setItem(slot)`, а `Container` в Vecta требует
`items() -> &[ItemStack]`, то есть непрерывный срез, которого у двух
раздельных аллокаций нет. Копия в буфер сломала бы живость (сравнение,
сохранение и дропы читают половины напрямую), поэтому пара выражена двумя
соседними секциями меню (каждый слот остаётся привязан к своей половине) и
одним `span` на 0..54 для shift-click — это точный эквивалент ванильного
непрерывного диапазона `moveItemStackTo(0, 54)`. Компаратор считает сумму
по обеим половинам через `calculate_redstone_signal_from_containers`.

Проверки: узкий прогон `chest` 42 passed. Тесты читают реально отправленные
пакеты: `opened_screens` (новый декодер `C_OPEN_SCREEN` в
`tests/support/connection.rs`) подтверждает `generic_9x6`, `block_events` —
по событию от каждой половины с `count = 1`, `played_sounds` — ровно один
звук из середины пары.

Незакрытое:
- Кот на сундуке (`isCatSittingOnChest`) — котов в Vecta нет.
- `PiglinAi.angerNearbyPiglins` — TODO, как и раньше.
- Shift-click по паре проверен на уровне меню (`chest_parts_with_kind` +
  `Click::QuickMove`), а не через реальный пакет клиента.

## W04-024 — сундук-ловушка: сигнал от числа зрителей

Связь: W04-022 (общая механика пары), W04-023 (счётчик зрителей). Java 26.3:
`TrappedChestBlock` (`isSignalSource`, `ownSignal`,
`getDirectSignal`, `getOpenChestStat`), `TrappedChestBlockEntity`
(`signalOpenCount` с `ExperimentalRedstoneUtils.initialOrientation` и двумя
`updateNeighborsAt`), `ChestBlockEntity.getOpenCount`, `ChestBlock` как база
(`combine`, `getMenuProvider`, `affectNeighborsAfterRemoval` →
`Containers.updateNeighboursAfterDestroy`), `BlockBehaviour.isSignalSource`/
`getSignal`/`ownSignal`/`getDirectSignal`, `RedstoneLampBlock`.

Что теперь делает игрок:
- Сундук-ловушка ставится и срастается в пару ровно как обычный сундук, но
  пара образуется только из двух ловушек: `chestCanConnectTo` сравнивает
  блок, поэтому ловушка рядом с обычным сундуком оставляет оба одиночными.
- Открытие даёт сигнал редстоуна силой, равной числу зрителей (1 на
  игрока, максимум 15), закрытие — снова 0. Пыль соединяется с ловушкой,
  потому что `isSignalSource` у неё всегда true.
- Напрямую (direct signal) ловушка питает только блок сверху; в остальные
  пять сторон direct-сигнала нет, слабый сигнал есть во все стороны.
- Открытие и закрытие будят соседей (`updateNeighborsAt` по своей позиции и
  по позиции снизу), поэтому лампа рядом загорается сразу при открытии и
  гаснет после задержки, без сторонних обновлений блоков.
- Открытие начисляет статистику `trigger_trapped_chest`, а не `open_chest`.
- Слом ловушки обновляет компаратор (`affectNeighborsAfterRemoval`), это же
  получил и обычный сундук.

Vecta paths: `behavior/blocks/container/trapped_chest_block.rs` (новый
`TrappedChestBlock`: делегирует `ChestBlock::placement_state`/`shape_state`/
`open_at`/`analog_output`/`recheck_at`/`dispatch_block_event`, плюс
`is_signal_source`/`get_own_signal`/`get_direct_signal`),
`behavior/blocks/container/chest_block.rs` (общая логика вынесена в
ассоциированные функции `pub(crate)`: `placement_state`, `shape_state`,
`combine`, `open_at`, `analog_output`, `recheck_at`,
`dispatch_block_event`; добавлен `affect_neighbors_after_removal`),
`block_entity/entities/chest.rs` (`ChestVariant::{Chest,Trapped}`,
`new_trapped`, `open_count_at` = ванильный `ChestBlockEntity.getOpenCount`,
`opener_count_changed` для ловушки зовёт `update_neighbors_at` по `pos` и
`pos.below()`), `block_entity/registry.rs` (фабрика
`vanilla_block_entity_types::TRAPPED_CHEST` → `new_trapped`).

Решение по ванильному наследованию: `TrappedChestBlockEntity extends
ChestBlockEntity` в Rust не выразить, поэтому один `ChestBlockEntity` носит
флаг варианта. Флаг нужен только для двух отличий — типа блока-сущности
(иначе ловушка сохранилась бы как `minecraft:chest`) и дополнительных
обновлений соседей в `signalOpenCount`.

Проверки: узкий прогон `trapped` 12 passed, `chest` 54 passed. Тесты
проверяют реальные пути: `world.get_signal`/`get_direct_signal` через
реестр поведений, настоящий `C_RECORDING`-поток для `C_OPEN_SCREEN`
(`generic_9x6` у пары), лампу как приёмник обновления соседей. Негативная
проверка: с закомментированными `update_neighbors_at` оба теста с лампой
падают, с ними — проходят.

Незакрытое:
- `ExperimentalRedstoneUtils.initialOrientation` не воспроизводится: в Vecta
  `update_neighbors_at` не принимает `Orientation` (как и остальной
  редстоун-код), это уже отмечено в `world/block_updates/mod.rs`.
- `PiglinAi.angerNearbyPiglins` — TODO, как и у обычного сундука.
- Кот на сундуке (`isCatSittingOnChest`) — котов в Vecta нет.

## W04-025 — медные сундуки

Связь: W04-022 (общая механика пары). Java 26.3: `CopperChestBlock`
(`chestCanConnectTo` по тегу `#copper_chests`, `getStateForPlacement` →
`getLeastOxidizedChestOfConnectedBlocks`, `updateShape`, `isWaxed`,
`getHingeSound`), `WeatheringCopperChestBlock` (`randomTick` с пропуском RIGHT
и сундуков с открытым меню), `WeatheringCopper`, `HoneycombItem.WAX_OFF_BY_BLOCK`.

Что теперь делает игрок:
- Медный сундук ставится как обычный: поворачивается к игроку, тонет в воде
  (WATERLOGGED) и срастается в пару по тем же правилам, но пара образуется
  между любыми двумя медными сундуками, независимо от окисления.
- При соединении пара берётся по менее окисленному сундуку: если поставить
  окисленный рядом со свежим, окисленный «омолаживается» до свежего, и
  наоборот. Если один из двух вощёный, а другой нет — оба становятся
  невощёными (и уже потом выравниваются по возрасту).
- Любая дальнейшая смена блока (окисление, воск, снятие воска) передаётся
  второй половине через `updateShape`, поэтому пара никогда не расходится по
  возрасту.
- Пара открывает меню на 6 рядов, одиночный — на 3; открытие начисляет
  `open_chest` и считает зрителей на обеих половинах.
- Звук крышки зависит от стадии: свежий и exposed — `copper_chest`,
  weathered — `copper_chest_weathered`, oxidized — `copper_chest_oxidized`.
- Компаратор читает содержимое, слом обновляет компаратор.
- Невощёный медный сундук окисляется на random tick, но не правая половина
  (она следует за партнёром) и не сундук, у которого есть зрители.

Vecta paths: `behavior/blocks/container/copper_chest_block.rs` (переписан:
`CopperChest` как общий помощник, `least_oxidized`, `unwax`, `is_waxed`,
`weather_state_of`, `WeatheringCopper` для random tick),
`behavior/blocks/container/chest_block.rs` (`ChestConnect::{SameBlock,
CopperChests}` вместо жёсткого сравнения блоков; `placement_state` и
`shape_state` принимают правило соединения), `behavior/block/mod.rs`
(хук `chest_lid_sound`), `block_entity/entities/chest.rs` (звук крышки
берётся из поведения, `None` = обычные звуки сундука).

Решения:
- `weather_state_of` выводит стадию из сгенерированных таблиц
  (`get_weather_state` плюс `get_normal_from_waxed_variant` для вощёных),
  поэтому поведению не нужен отдельный json-аргумент возраста: данные и так
  уже в реестре.
- Тип блока-сущности у медного сундука — `minecraft:chest`, как в ванили
  (`CopperChestBlock` передаёт `BlockEntityTypes.CHEST`), поэтому
  сохранение/загрузка и трансформации работают через уже существующий
  chest-entity и не требуют нового типа.

Проверки: узкий прогон `copper_chest_block` 11 passed, `chest` 54 passed.
Негативные проверки: с отключённой нормализацией падают оба теста на
возраст/воск (после того как тест стал проверять оба порядка установки);
с отключёнными `update_neighbors_at` падают оба теста с лампой у ловушки.

Незакрытое:
- `shouldChangedStateKeepBlockEntity` отдельным хуком не добавлялся: смена
  блока внутри семейства уже сохраняет block entity на уровне чанка
  (`copper_chest_transformation_preserves_entity_identity`).
- `PiglinAi.angerNearbyPiglins` — TODO, как и у обычного сундука.
- Кот на сундуке (`isCatSittingOnChest`) — котов в Vecta нет.

## W04-027 — воронка

Связь: W04-022 (пара сундуков как один контейнер). Java 26.3:
`HopperBlock`, `HopperBlockEntity`, `Hopper` (`SUCK_AABB =
Block.column(16.0, 11.0, 32.0)`), `HopperMenu`, `BaseContainerBlockEntity`,
`RandomizableContainerBlockEntity`, `CompoundContainer.setChanged`,
`ChestBlock.combine(..., ignoreBeingBlocked)`, `Entity.checkInsideBlocks`.

Что теперь делает игрок:
- Воронка ставится лицом в противоположную от кликнутой грань
  (`clickedFace.getOpposite()`, вертикальный клик схлопывается в `down`),
  вращается/зеркалится по `facing`, `enabled` следует редстоун-сигналу
  соседей через `checkPoweredState` при установке и при `neighborChanged`.
- Раз в 8 тиков (и на первом же тике после установки) воронка перекладывает
  ровно один предмет: сначала выталкивает в контейнер, на который смотрит,
  затем втягивает один предмет из контейнера сверху либо из предметов в
  верхней половине своего блока. Успешный перенос перезапускает кулдаун.
- Стак доливается до лимита предмета (63+1 → 64), следующий предмет уходит в
  следующий слот, при полном контейнере предмет остаётся в воронке.
- Двойной сундук сверху и снизу видится одним контейнером на 54 слота:
  предметы попадают в ту половину, которой принадлежит логический слот.
- Отключённая сигналом воронка не двигает ничего и не сбрасывает кулдаун, а
  после снятия сигнала продолжает с сохранённого значения.
- Предмет в `Hopper.SUCK_AABB` втягивается сразу через `entityInside`, не
  дожидаясь тика; предмет ниже зоны захвата игнорируется; полный блок сверху
  вне тега `#does_not_block_hoppers` блокирует захват предметов.
- ПКМ открывает меню `minecraft:hopper` (5 слотов + инвентарь) и начисляет
  `inspect_hopper`; запертая воронка не открывается, но показывает предмет
  замка и стат всё равно начисляет.
- Компаратор читает заполненность (1 полный слот из 5 = сигнал 3), слом
  обновляет компаратор и выбрасывает содержимое.
- `TransferCooldown` сохраняется в NBT и восстанавливается при загрузке:
  после перезагрузки воронка ждёт остаток кулдауна, а не начинает заново.
- Воронка не проходима для мобов (`isPathfindable` = false).

Vecta paths: `block_entity/entities/hopper.rs` (новая блок-сущность:
`pushItemsTick`, `tryMoveItems`, `ejectItems`, `suckInItems`,
`tryTakeInItemFromSlot`, `tryMoveInItem`, `addItem`, `getItemsAtAndAbove`,
`TransferTarget`), `behavior/blocks/container/hopper_block.rs` (новая
поведение-обёртка), `inventory/menu/kinds/hopper_menu.rs` (меню),
`block_entity/registry.rs` (`HOPPER` → `HopperBlockEntity`),
`behavior/blocks/container/chest_block.rs` (`combine` получил ванильный
`ignore_blocked`), плюс `mod.rs`-экспорты.

Решения:
- `TransferTarget` — один логический контейнер. В ванили пара сундуков это
  `CompoundContainer` с непрерывным пространством слотов; у Vecta каждая
  половина — свой контейнер, поэтому логический слот отображается в
  (половина, локальный слот). `slots_for_face` применяется только к
  одиночному контейнеру (аналог `WorldlyContainer`), пара всегда плоская —
  как `CompoundContainer`.
- Воронка и цель блокируются одним `ContainerLockGuard`, как один
  `CompoundContainer` в ванили: это исключает межпоточный дедлок и даёт
  согласованный снимок для `isFullContainer`/`addItem`.
- Цель для вставки (`TransferTarget` воронки) измеряется **до** взятия
  блокировок. Иначе `TransferTarget::single` внутри уже залоченной секции
  берёт лок на ту же воронку и вешает поток; это поймано интеграционным
  тестом на «сундук сверху» и зафиксировано комментарием в коде.
- `skipTickCount`: если источник — воронка, которая уже тикала в этом игровом
  тике, кулдаун ставится 7, а не 8 (`MOVE_ITEM_SPEED - skip`).
- `TransferCooldown` пишется всегда, вне loot-ветки, как в
  `saveAdditional`; при загрузке отсутствие тега даёт `-1`.
- Лут-таблицы воронки и цели разворачиваются вручную
  (`unpack_pending_loot`/`unpack_loot_table`), потому что Vecta читает блоки
  без ванильной ленивой обёртки `RandomizableContainerBlockEntity`.
- Кулдаун и `tickedGameTime` — атомики: тик читает их без удержания лока
  контейнера, как и ванильные поля.

Проверки: узкий прогон `hopper` — 21 passed (5 блок-сущность, 2 меню, 14
поведение/интеграция через настоящий тикер). Полный прогон workspace —
**3102 passed / 0 failed / 1 ignored** в `vecta-core` плюс все
интеграционные наборы; `cargo clippy --workspace --all-targets
--all-features -D warnings` и `cargo fmt --all --check` чистые.

Негативные проверки: отключение гейта `ENABLED` роняет тест с редстоуном;
удаление записи `TransferCooldown` роняет тест восстановления кулдауна;
возврат построения `TransferTarget` внутрь guard вешает оба теста с
контейнером сверху (самодедлок).

Побочное: три чужих теста (`chunk::data::dummy_factory_outcomes…`,
`chunk::full_chunk::insertion_rejects_an_entity_owned_by_another_chunk`,
`chunk::full_chunk::stale_no_entity_promotion…`) использовали `HOPPER` как
пример блока с типом блок-сущности без фабрики. Воронка теперь
зарегистрирована, поэтому фикстура заменена на `ENCHANTING_TABLE` (тот же
смысл: тип есть, фабрики нет).

Незакрытое:
- Коты на сундуке/воронке (`isCatSittingOnChest`) и
  `PiglinAi.angerNearbyPiglins` — животных AI в Vecta нет, как и у сундука.
- `ExperimentalRedstoneUtils.initialOrientation` не смоделирован (как и у
  остальных блоков).
- `isGridAligned` в Vecta всегда true, поэтому ванильная ветка с
  `BlockPos.containing(...)` и округлением координат не исполняется.
- Отдельная форма взаимодействия воронки (`interaction_shapes_mask` в
  `classes.json`) не потребляется — как и у остальных контейнеров.
- W04-028 (сторонняя автоматизация, слоты печи) и W04-029 (дозатор) не
  начаты.

## W04-028 — сторонний доступ и печные слоты

Связь: W04-027 (переносчик использует `Container::slots_for_face` /
`can_place_item_through_face` / `can_take_item_through_face`). Java 26.3:
`WorldlyContainer`, `AbstractFurnaceBlockEntity`
(`getSlotsForFace`, `canPlaceItemThroughFace`, `canTakeItemThroughFace`,
`canPlaceItem`), `ItemTags.FURNACE_FUEL_BOTTOM_TAKEABLE`,
`HopperBlockEntity.addItem/getSlots`.

Что теперь делает игрок:
- Воронка сверху печи кладёт только во входной слот (0), сбоку — только в
  слот топлива (1), снизу — забирает из выхода (2) и из топлива только
  пустое/водяное ведро. Раньше снизу можно было вытащить уголь, потому что
  правило было зашито списком из двух предметов, а не тегом.
- Печь сама отказывает не-топливу в слоте топлива и никогда не принимает
  предмет в слот результата — воронке не нужны никакие печные исключения.
- Несовпадение компонентов (например, предмет с custom name против обычного)
  не доливается в стак: предмет остаётся в воронке.
- Полный слот не переполняется: 64 в слоте входа остаются 64, лишний предмет
  остаётся в воронке.
- Две воронки (сверху и снизу печи) тикают против одной печи каждый тик:
  предметы не дублируются и не теряются, дедлока нет.

Vecta paths: `block_entity/entities/abstract_furnace.rs`
(`can_take_item_through_face` теперь проверяет
`ItemTag::FURNACE_FUEL_BOTTOM_TAKEABLE` вместо пары `WATER_BUCKET || BUCKET`;
расширенный тест контракта граней; `FURNACE_SLOTS`/`SLOT_*` стали
публичными с документацией), `block_entity/entities/mod.rs` (экспорт слотов),
`behavior/blocks/container/hopper_block.rs` (5 интеграционных тестов с
настоящей печью).

Решения:
- Сторона в `getSlotsForFace` берётся из направления переноса: воронка сверху
  толкает через `Direction.UP`, сбоку — через `Direction.EAST/WEST/...`,
  снизу забирает всегда через `Direction.DOWN` (ванильный `suckInItems`
  жёстко использует `DOWN`). Поэтому воронка под печью может смотреть вниз:
  `FACING_HOPPER` не имеет значения `up`.
- `canPlaceItemThroughFace` в ванили — просто `canPlaceItem`, поэтому
  отдельного правила вставки по стороне у печи нет; это закреплено тестом,
  который сверяет `can_place_item_through_face` с `can_place_item` для всех
  трёх слотов и трёх предметов.
- Правило «снизу только ведро» вынесено в тег, а не в код: значения тега в
  26.3 — ровно `water_bucket` и `bucket`, поэтому поведение не изменилось, но
  теперь оно берётся из данных.

Проверки: узкий прогон `hopper` — 26 passed (21 из W04-027 плюс 5 новых),
`furnace_sided_inventory` — passed. Полный прогон workspace —
**3107 passed / 0 failed / 1 ignored**; clippy `-D warnings` и
`cargo fmt --all --check` чистые.

Негативные проверки: замена тега на `stack.is(WATER_BUCKET) || is(BUCKET)`
поведение не меняет (значения тега совпадают), поэтому негативный прогон
делался по правилу слота: с `canPlaceItem` без проверки топлива падает тест
«сбоку только топливо», а без проверки результата — тест «сверху только
вход».

Незакрытое:
- Общий `WorldlyContainer`-хелпер не выделялся: у печи и воронки разные
  правила, а третьего потребителя пока нет (дозатор — W04-029).
- `ItemTags.FURNACE_FUELS`/`COOKING_FUEL`: Vecta по-прежнему считает топливо
  по таблице `VANILLA_FUEL_VALUES` (26.2), а не по компоненту
  `minecraft:cooking_fuel`; это отдельная задача о компонентах топлива.

## W04-029 — раздатчик (dropper)

Связь: W04-027 (общий `addItem` через `push_into_container_at`), W04-028
(сторонний доступ не понадобился: у раздатчика нет `WorldlyContainer`).
Java 26.3: `DispenserBlock` (`neighborChanged`, `tick`, `dispenseFrom`,
`getStateForPlacement`, `hasAnalogOutputSignal`, `getAnalogOutputSignal`,
`useWithoutItem`), `DropperBlock` (`dispenseFrom`, `getDispenseMethod`),
`DispenserBlockEntity` (`getRandomSlot`, `CONTAINER_SIZE = 9`),
`DefaultDispenseItemBehavior` (`dispense`, `execute`, `spawnItem`,
`playSound`, `playAnimation`), `DispenserMenu`,
`HopperBlockEntity.getContainerAt/addItem`.

Что теперь делает игрок:
- Раздатчик смотрит туда, куда смотрел игрок при установке: клик сверху даёт
  раздатчик, направленный вверх, взгляд на север — направленный на юг
  (`getNearestLookingDirection().getOpposite()`), а не «противоположно
  кликнутой грани», как у воронки.
- Импульс редстоуна (в том числе от блока сверху) взводит `triggered` и
  планирует блок-тик ровно через 4 игровых тика. Долгий сигнал даёт один
  выстрел: повторные обновления соседей при уже взведённом флаге ничего не
  планируют, а спад сигнала только снимает `triggered`.
- Один импульс вынимает ровно один предмет из одного случайно выбранного
  непустого слота (reservoir sampling, как `getRandomSlot`), а не все стаки.
- Если перед раздатчиком контейнер — предмет уходит в него через общий
  `HopperBlockEntity.addItem(..., direction.getOpposite())`; полный контейнер
  оставляет предмет в слоте и не издаёт звука. Если контейнера нет — предмет
  выбрасывается как `ItemEntity` с ванильным смещением (центр + 0.7 по
  направлению, минус 2/16 по Y и 2.5/16 по горизонтали), скоростью
  `random.triangle(mean, 0.0172275 * 6)` и событиями 1000 и 2000.
- Пустой раздатчик отвечает событием 1001 и ничего не меняет.
- Открытие показывает меню `minecraft:generic_3x3` (9 слотов + инвентарь) и
  выдаёт статистику `inspect_dropper`; компаратор читает заполненность.

Vecta paths: `behavior/blocks/redstone/dropper_block.rs` (новый
`DropperBlock`: `dispense_from`, `eject_one`, размещение, `neighborChanged`,
`tick`, `use_without_item`, отсутствие тикера, аналоговый выход; 14
интеграционных тестов), `block_entity/entities/dropper.rs` (новый
`DropperBlockEntity` на 9 слотов с `get_random_slot`, NBT `Items` + loot,
`pre_remove`; 5 тестов), `inventory/menu/kinds/dispenser_menu.rs` (новое меню
`GENERIC_3X3`; 2 теста), `block_entity/registry.rs` (фабрика `DROPPER`),
`block_entity/entities/mod.rs` (экспорт), `behavior/blocks/redstone/mod.rs`
(модуль), `block_entity/entities/hopper.rs` (`push_into_container_at` теперь
возвращает `Option<ItemStack>`, чтобы отличать «контейнера нет»),
`tests/support/connection.rs` + `tests/support/mod.rs` (декодер пакета
`C_LEVEL_EVENT` для проверки событий 1000/1001/2000).

Решения:
- Ванильная логика импульса живёт в `DispenserBlock`, а у раздатчика
  переопределён только `dispenseFrom`. В Vecta дозатор ещё не реализован,
  поэтому общий код импульса пока лежит в `DropperBlock`; при W04-030+
  его надо будет поднять в общий `DispenserBlock`, а не копировать.
- Ветка «вставлять или выбрасывать» проверяет именно наличие контейнера
  (`getContainerAt == null`), а не наличие блок-сущности: табличка — это
  блок-сущность, но не контейнер, и раздатчик перед ней выбрасывает предмет.
  Это закреплено отдельным тестом, потому что более простая проверка
  `get_block_entity(...).is_some()` молча съедала импульс.
- `getRandomSlot` сам разворачивает loot table (`unpackLootTable(null)`), как
  в ванили, поэтому раздатчик с лут-таблицей не выглядит пустым.
- `dispenseFrom` в ванили перечитывает состояние из мира; здесь берётся
  состояние, переданное тиком — на пути `tick` это то же самое значение.
- У раздатчика нет блок-сущностного тикера: импульс идёт через
  `schedule_block_tick`, что и проверяется (`registered_len() == 0`).

Проверки: узкий прогон `dropper` — 19 passed (14 поведенческих + 5 сущности),
`dispenser_menu` — 2 passed. Полный прогон workspace —
**3128 passed / 0 failed / 1 ignored**; `cargo fmt --all --check` и
`cargo clippy --workspace --all-targets --all-features -- -D warnings` чистые.

Негативные проверки: с веткой `get_block_entity(target).is_some()` вместо
`getContainerAt` падает тест с табличкой; без `TRIGGER_DURATION` (например,
тик через 1) падает проверка раннего сбора тика; при возврате `stack` без
`shrink` падает тест «один предмет за импульс»; без проверки `remaining`
падает тест с полным сундуком.

Незакрытое:
- `DispenserBlock` (сам дозатор, `DISPENSER_REGISTRY`,
  `ProjectileDispenseItemBehavior`, `EquipmentDispenseItemBehavior`,
  `SpawnEggItemBehavior`) не реализован — это отдельная задача; сейчас
  общий код импульса временно живёт в `DropperBlock`.
- `HopperBlockEntity.getContainerAt` в Vecta не оборачивает контейнеры
  сущностей (вагонетки, лодки с сундуком), поэтому раздатчик перед такой
  сущностью выбросит предмет; у воронки то же ограничение.
- `RandomSource` берётся из `rand`, а не из мирового сида: выбор слота и
  разброс скорости не воспроизводимы по сиду мира.

## W04-018 — закрытие меню при всех изменениях состояния

Связь: W03-09. Эталон: `AbstractContainerMenu.java` (`removed`,
`dropOrPlaceInInventory`), `Player.java` (`remove`), `ServerPlayer.java`
(`doCloseContainer`), `ContainerOpenersCounter.java`, `ChestBlockEntity.java`
(`startOpen`/`stopOpen`/`recheckOpen`), `Container.java`
(`stillValidBlockEntity`).

Что уже работало и проверено (переписывать не пришлось):
- Механизм закрытия собран из `close_open_menu` / `do_close_container` /
  `remove_all_menus`, а политика предметов — `MenuItemDisposition` с
  `default_menu_item_disposition`: `Drop`, если игрок удалён по причине,
  отличной от `ChangedWorld`, либо соединение закрыто, либо здоровье <= 0;
  иначе `ReturnToInventory`. Это дословно `dropOrPlaceInInventory`.
- Каждый тик игрока `tick_open_menu` проверяет `still_valid` и закрывает меню,
  если контейнер ушёл, блок заменён или игрок вышел из радиуса
  (`is_within_block_interaction_range_with_buffer(pos, 4.0)`), что совпадает с
  ванильным `stillValidBlockEntity`.
- Смерть и дисконнект уже покрыты тестами
  (`closing_menu_while_dead_does_not_return_items_to_inventory`,
  `disconnected_menu_removal_drops_transient_items`), повторное закрытие
  защищено `terminal_removal` и изъятием меню из слота.
- Смена измерения идёт через `reset(.., ResetReason::WorldChange)` →
  `remove_all_menus()`, то есть меню закрывается, а курсор возвращается в
  инвентарь (политика `ChangedWorld`).

Что добавлено: пять регрессий на ранее непроверенные пути.
- `breaking_an_open_chest_closes_the_menu_and_releases_the_viewer` (сундук):
  блок ломается при открытом меню → следующий тик закрывает меню, клиенту
  уходит один `CContainerClose`, содержимое падает в мир ровно один раз, а
  новый сундук на том же месте начинает с нулём зрителей. Отдельно
  зафиксировано, что счётчик удалённой блок-сущности не декрементируется —
  это ванильное поведение (`ChestBlockEntity.stopOpen` выходит при
  `this.remove`), счётчик умирает вместе с блок-сущностью.
- `walking_out_of_range_closes_the_menu_on_the_next_tick` (сундук): игрок
  выходит из радиуса, но остаётся в том же чанке → тик закрывает меню и
  отпускает зрителя.
- `closing_a_crafting_menu_mid_operation_returns_the_grid_and_cursor_once`:
  верстак, один предмет в сетке и три на курсоре, затем закрытие и повторное
  закрытие через настоящие пакеты клика → в инвентаре снова ровно 4 доски,
  один close-пакет, ничего не выброшено.
- `a_repeated_close_sends_one_packet_and_returns_the_cursor_once`: два
  `close_container()` подряд → один `CContainerClose` и один возврат курсора.
- `a_world_change_closes_the_menu_and_returns_the_cursor_to_the_inventory`:
  `reset(WorldChange)` → меню закрыто, курсор вернулся в инвентарь, а не
  выпал в новый мир.

Vecta paths: `behavior/blocks/container/chest_block.rs` (2 теста),
`player/player_inventory/tests.rs` (3 теста),
`tests/support/connection.rs` + `tests/support/mod.rs` (декодер
`C_CONTAINER_CLOSE`, чтобы проверять закрытие экрана по пакету, а не по
флагу).

Решения:
- Проверка `is_removed()` в `ChestBlockEntity::stop_open` оставлена: она
  повторяет ванильный `if (!this.remove && !spectator)`. Поэтому «нет
  оставшегося viewer count» проверяется наблюдаемо — новый блок на том же
  месте не наследует зрителя, — а не чтением счётчика у удалённой сущности.
- Смена измерения не отправляет отдельный close для внешнего меню: ванильный
  клиент сбрасывает экран по `ClientboundRespawnPacket`, а Vecta дополнительно
  шлёт `CContainerClose{0}` для инвентарного меню. Тест фиксирует именно это.
- Клики в тестах верстака идут через `handle_container_click` с настоящим
  `SContainerClick`, а не через приватные `take_open_menu_for_callback`, чтобы
  проверялся реальный путь пакета.

Проверки: `chest_block::tests` — 55 passed, `player_inventory::tests` —
73 passed. Полный прогон workspace — **3133 passed / 0 failed / 1 ignored**;
`cargo fmt --all --check` и clippy `-D warnings` чистые.

Незакрытое:
- Аналогичные проверки для лектерна, наковальни и точильного камня не
  добавлялись: у них та же общая ветка `Menu::removed`, отдельные тесты есть
  только на `still_valid` (`anvil_menu/tests.rs`, `stonecutter_menu/tests.rs`).
- `ContainerOpenersCounter` не имеет хука на удаление блок-сущности, как и в
  ванили; счётчик живёт только в блок-сущности.

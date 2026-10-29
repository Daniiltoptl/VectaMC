# Night 02 — progress journal

Branch: `codex/night-vanilla-02`
Base: `6f8b7fec3` (docs: prepare second vanilla development night after bundle review)
Verified game commit inherited from night 01: `d5866e78e1214393142136d57e210459765be230`

Each entry lists: what was implemented, the vanilla reference used, the concrete
checks that were actually executed, and anything left open.

---

## N03 — Books: writing, signing, opening, cloning

Commit: `8619e9d56` — `feat(books): implement edit, sign and open book paths (N03)`

### Vanilla reference

- `network/protocol/game/ServerboundEditBookPacket.java`
  (`VAR_INT slot`, `stringUtf8(1024).apply(list(100))` pages, `stringUtf8(32).apply(optional)` title)
- `network/protocol/game/ClientboundOpenBookPacket.java` (`InteractionHand.STREAM_CODEC`)
- `util/Utf8String.java` — bounds by UTF-16 code units, encoded payload capped at 3× that
- `server/network/ServerGamePacketListenerImpl#handleEditBook/updateBookContents/signBook/filterableFromOutgoing`
- `world/item/WritableBookItem.java`, `WrittenBookItem.java`
- `server/level/ServerPlayer#openItemGui` — only a *written* book sends `ClientboundOpenBookPacket`
- `world/item/KnowledgeBookItem.java`, `world/item/crafting/BookCloningRecipe.java`

### Implemented

- `vecta-protocol/src/packets/game/s_edit_book.rs` — new `SEditBook` codec with the
  vanilla per-field bounds (`MAX_EDIT_BOOK_PAGES = 100`,
  `MAX_EDIT_BOOK_PAGE_LENGTH = 1024`, `MAX_EDIT_BOOK_TITLE_LENGTH = 32`) using the
  existing `read_utf` helper, which measures UTF-16 units and caps encoded bytes at 3×.
- `vecta-protocol/src/packets/game/c_open_book.rs` — new `COpenBook` client packet.
- `Player::handle_edit_book` in `player/player_inventory/player_handlers.rs`:
  rejects negative and non-hotbar/non-offhand slots, then re-reads the stack from the
  inventory and requires the `WRITABLE_BOOK_CONTENT` component (component presence, not
  a hard-coded item id). Saving replaces writable content; signing keeps the existing
  component patch, switches the item to `written_book`, drops writable content and sets
  `WrittenBookContent` with the player profile name, generation 0, `resolved = true`.
- `WrittenBookContent::resolve` / `mark_resolved` in
  `vecta-registry/src/data_components/components/books.rs`, including vanilla
  `isPageTooLarge` (JSON-encode, then `encodesLongerThan(..., 32767)`) and the rule
  that a page whose resolution throws keeps its original component.
- `vecta-core/src/behavior/items/books.rs` — `WritableBookItem` and `WrittenBookItem`
  behaviours: award `ITEM_USED`, resolve unresolved written books through the existing
  `CommandTextResolver`, and send `COpenBook` only when the stack still carries written
  content.
- Regression tests for book cloning (generation bump 0 → 1, and refusal outside the
  datapack range, which has no `allowed_generations` field so vanilla's default
  `between(0, 1)` applies).

### Checks actually run

- `cargo check -p vecta-core` → exit 0
- `cargo test -p vecta-core --lib book` → 20 passed
- `cargo test -p vecta-core --lib behavior::items::books` → 4 passed
- `cargo test -p vecta-core --lib book_cloning` → 2 passed
- `cargo test -p vecta-registry --lib books` → 8 passed
- `cargo test -p vecta-protocol --lib` → 74 passed
- `cargo test -p vecta-registry --lib` → 415 passed
- `cargo test -p vecta-core --lib` → 2907 passed, 0 failed, 1 ignored
- `cargo clippy -p vecta-core -p vecta-registry -p vecta-protocol --all-targets --all-features -- -D warnings` → clean
- `cargo fmt --all` applied

### Open in this stage

- Text filtering: Vecta has no `TextFilter` service, so `TextFilter.DUMMY` pass-through
  semantics apply and both `filterableFromOutgoing` branches collapse to
  `Filterable::pass_through`. No filter result is invented and no blocking wait was added
  to the tick.
- Knowledge book is **not** implemented — see `blockers.md`.

---

## N19 — Pick-block for entities

### Vanilla reference

- `network/protocol/game/ServerboundPickItemFromEntityPacket.java` (`VAR_INT id`, `BOOL includeData`)
- `server/network/ServerGamePacketListenerImpl#handlePickItemFromEntity` and the shared
  `#tryPickItem`
- `world/entity/Entity#getPickResult` (null), `Mob#getPickResult` (spawn egg),
  `ItemFrame#getPickResult`, `LeashFenceKnotEntity#getPickResult`,
  `EndCrystal#getPickResult`, `MinecartChest#getPickResult`
- `world/item/SpawnEggItem#byId` — item registry scan on `DataComponents.ENTITY_DATA`
- `world/entity/player/Player#isWithinEntityInteractionRange(Entity, double)`
- `world/entity/player/Inventory#pickSlot`, `#addAndPickItem`, `#getSuitableHotbarSlot`,
  `#getFreeSlot`, `#isHotbarSlot`, `#findSlotMatchingItem`

### Implemented

- `vecta-protocol/src/packets/game/s_pick_item_from_entity.rs` — `SPickItemFromEntity`
  with `#[read(as = VarInt)] id` and `include_data`, wired in `packets/game/mod.rs`.
- `ItemRegistry::by_entity_data_type` in `vecta-registry/src/items/mod.rs`, the
  `SpawnEggItem.byId` equivalent (component-driven, not class-restricted).
- `Entity::get_pick_result` (default `None`, delegating the mob rule to
  `Mob::get_pick_result`), `Mob::get_pick_result` (spawn egg by entity type), and
  overrides on `ItemFrameEntity`, `GlowItemFrameEntity` (delegating to the inner frame),
  `LeashFenceKnotEntity`, `EndCrystalEntity` and `ChestMinecartEntity`.
- `Player::is_entity_within_interaction_range` in `player/game_mode/player_methods.rs`,
  mirroring vanilla's removed-entity rejection before the bounding-box measurement.
- `Player::handle_pick_item_from_entity` in `player/game_mode/entity_interaction.rs`:
  entity lookup by network id → range check with vanilla's 3.0 buffer → `get_pick_result`
  → shared `try_pick_item`.
- `Player::try_pick_item` in `player/game_mode/block_interaction.rs`, the shared
  `tryPickItem`: match on item + components, hotbar ⇒ select, otherwise `pick_slot`;
  creative-only `add_and_pick_item`; always `CSetHeldSlot` plus a menu broadcast.
  `handle_pick_item_from_block` now calls it instead of duplicating the logic.
- `PlayerInventory::pick_slot` and `add_and_pick_item` were aligned with vanilla
  `Inventory`: both first select `get_suitable_hotbar_slot()`, and `add_and_pick_item`
  relocates an occupying stack to `get_free_slot()` instead of scanning for the first
  empty hotbar slot. This is a shared-contract change — the block pick path uses it too,
  and it removes the previous deviation rather than adding one.
- Dispatch: `S_PICK_ITEM_FROM_ENTITY` arm, `ScheduledPlayPacketKind::PickItemFromEntity`,
  handler arm, and `PlayerLocal` classification in `execution()`.

### Deliberate gaps (recorded, not silently substituted)

- `includeData` has exactly one vanilla consumer for entities: printing an `Avatar` game
  profile for game-master players. Vecta has no `Avatar` profile command foundation, so
  that branch is absent and documented at the call site.
- Paintings and armor stands do not exist as Vecta entity implementations, so they have
  no pick result to wire. The task allowed this ("if their implementations are ready").
- `Level.getEntityOrPart` falls back to entity *parts*; Vecta has no multi-part entity
  bodies (no `EnderDragonPart`), so the plain entity lookup is equivalent today.
- `ItemStack.isItemEnabled(enabledFeatures)` has no Vecta counterpart; all registered
  items are enabled.

### Checks actually run

- `cargo test -p vecta-core --lib pick_item` → 12 passed
- `cargo test -p vecta-registry --lib` → 416 passed (was 415)
- `cargo test -p vecta-protocol --lib` → 77 passed (was 74)
- `cargo test -p vecta-core --lib` → 2919 passed, 0 failed, 1 ignored
- `cargo clippy … -D warnings` → clean
- `cargo fmt --all` applied

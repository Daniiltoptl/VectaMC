# Night 03 — blockers

## B-W03-01 — client recipe list: resolved for the stonecutter, not for the recipe book

Status: **the packet is implemented and sent.** This section records what the
26.3 sources actually require, because the previous text over-stated the
dependency.

Checked in the local 26.3 sources:

- `network/protocol/game/ClientboundUpdateRecipesPacket.java` carries exactly two
  fields: `Map<ResourceKey<RecipePropertySet>, RecipePropertySet> itemSets` and
  `SelectableRecipe.SingleInputSet<StonecutterRecipe> stonecutterRecipes`.
- `SelectableRecipe.noRecipeCodec()` writes **only** `optionDisplay`; the recipe
  field is encoded as `Optional.empty()`. `RecipeDisplay` is therefore *not* part
  of this packet, and no `RecipeDisplay` variants are needed to send it.
- The chain is `Ingredient.CONTENTS_STREAM_CODEC` (= `ByteBufCodecs.holderSet(ITEM)`)
  → `SlotDisplay.STREAM_CODEC` (= `ByteBufCodecs.registry(SLOT_DISPLAY).dispatch(...)`).
- `StonecutterRecipe.resultDisplay()` is `SlotDisplay.ItemStackSlotDisplay(result)`,
  so every stonecutter entry is `item_stack` (kind 5) plus an item stack template.

Implemented in this branch: `SlotDisplay`, `SingleInputEntry`,
`RecipePropertySet`/`RecipePropertySetKey` in `vecta-registry/src/recipe/display.rs`,
`CUpdateRecipes` in `vecta-protocol`, the `finalizeRecipeLoading`-equivalent
collection in `inventory/recipe_manager.rs`, and the login-time send in
`player/lifecycle/spawn_sync.rs`.

Remaining, not blocked by architecture but by scope:

- The **recipe book** (W03-65) still needs the full `RecipeDisplay` family and the
  recipe-book packets. Only the two display kinds that the update-recipes packet
  can carry are exercised today.
- `slot_display` kind ids are the built-in registration order from
  `world/item/crafting/display/SlotDisplays.java`. They live in Rust as
  `SlotDisplayKind` because no `slot_display` entry exists in
  `vecta-registry/build_assets/`, and the SteelExtractor checkout is not present
  on this machine. If a `slot_display` extractor output is added later, the enum
  should be generated from it instead.

Not blocked: server-authoritative stonecutter selection, result, consumption,
quick-move, and now the client-visible recipe list for the stonecutter.

## B-W03-01b — loom does not depend on the recipe packet (previous text was wrong)

`LoomMenu.selectablePatterns` is built server-side only:

- `getSelectablePatterns(patternStack)` returns `BannerPatternTags.NO_ITEM_REQUIRED`
  when the pattern slot is empty, or the stack's `minecraft:provides_banner_patterns`
  component otherwise.
- `clickMenuButton` indexes that list; nothing arrives over the wire.

The client rebuilds the same list from its own synced registries and tags, so the
only network requirement is the tag packet. Vecta already sends
`ClientboundUpdateTagsPacket` with `BANNER_PATTERN_REGISTRY` tags
(`server/registry_cache.rs::build_tags_packet`), so the loom's client path was
never blocked by the missing recipe packet. The earlier blocker entry was wrong.

## B-W03-04 — block entities never publish implicit item components

Checked:
- `player/game_mode/block_interaction.rs::handle_pick_item_from_block` still
  carries `// TODO: If include_data, copy the block entity data into the picked
  item stack.` — `get_clone_item_stack` returns the plain block item.
- There is no block-entity equivalent of `applyImplicitComponents` /
  `collectImplicitComponents`: `apply_implicit_item_components` exists on
  `Entity` only (`entity/entity/mod.rs`), and nothing in `block_entity/` exposes
  a component map.
- Vanilla `BlockEntity.collectImplicitComponents` is what puts
  `minecraft:container` (chiseled bookshelf, chest-like contents), banner
  patterns, written-book content, etc. on the stack returned by pick-block.

Consequence: creative pick-block (and any "clone block with contents" flow)
yields a block item without its contents. Chiseled bookshelf reproduces the
*placement* half of the contract (`set_placed_by` reads `minecraft:container`),
but the *collection* half (`collect_container_component`) has no caller, so the
round trip cannot be completed from the client.

Risk of guessing: the component set is per block entity and versioned
(`DataComponentMap`), and there is no 26.3 client here to confirm the encoding.

Needed decision/output: a block-entity component hook (`collect_implicit_components`
/ `apply_implicit_components`) plus wiring it into pick-block, or an explicit
decision to keep pick-block contents out of scope.

Blocks: full W03-04 pick-block acceptance, the same round trip for every
container block entity implemented later (chest W03-10..12, hopper W03-14,
dropper W03-17).

## B-W03-05 — `vecta-core` test binaries cannot be linked in this session

Checked:
- `cargo test -p vecta-core --lib <filter>` fails during codegen with
  `memory allocation of 160432144 bytes failed` (repeatable, 5 attempts, both
  from bash and PowerShell).
- `cargo check -p vecta-core --lib --tests` succeeds, so the sources and tests
  are type-correct; the failure is resource-related, not a diagnostic.
- System state at the time: commit charge 44.8 GB of a 47.9 GB limit
  (32 GB RAM + 16 GB pagefile on `D:`), 12 GB physical free. A `qemu-system-x86_64`
  VM holds ~16.5 GB of commit; `idea64`, two `java` processes and the rest of the
  desktop take the remainder.
- `vecta-registry` and `vecta-protocol` test binaries build and run normally.

Not caused by this branch: no dependency, toolchain or profile change is involved.

Workarounds attempted, all failed:

1. `--config 'build.rustflags=["-Zthreads=2"]'` (the repo's `-Zthreads=0` lets
   rustc use all 12 cores). This got past the first OOM and compiled 156 crates,
   then the build stalled with an idle `rustc` and was stopped after 27 minutes.
2. `CARGO_PROFILE_TEST_DEBUG=0` — same `memory allocation of 2097152 bytes failed`.
3. Retried the default flags after `D:` was freed to 85 GB — same failure.

The pagefile is capped at ~15.3 GB (`D://pagefile.sys`), so the commit ceiling
stays near 47.5 GB regardless of free disk space.

Needed from the owner: free commit charge (close the VM) so the `vecta-core`
test binary links, or accept that this session's core-level results rest on
type-checking plus the protocol/registry suites.

Blocks: running any `vecta-core` unit test, including the existing suite, in this
session. It does not block writing or type-checking code.

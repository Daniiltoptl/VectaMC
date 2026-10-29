# Night 03 — blockers

## B-W03-01 — client never receives the recipe display list

Checked:
- Vecta `vecta-protocol/src/packets/game/` has no `ClientboundUpdateRecipes`
  packet (108 packet files, no `c_update_recipes`); nothing sends
  `ClientboundUpdateRecipesPacket` at login.
- `rg SlotDisplay|slot_display` over the workspace → no hits, so neither
  `SlotDisplay` nor `RecipeDisplay` variants exist as Rust types.
- Vanilla `ClientboundUpdateRecipesPacket` carries the property-set map, the
  `SelectableRecipe.SingleInputSet<StonecutterRecipe>` list and the per-recipe
  displays; `StonecutterMenu` only filters that client-side list locally.

Missing: `SlotDisplay` (empty/any_fuel/item/item_stack/tag/smithing_trim/
with_remainder/composite), `RecipeDisplay` (crafting_shaped, crafting_shapeless,
furnace, stonecutter, smithing, brewing, ...), the `Ingredient` holder-set stream
codec, `RecipePropertySet` sync, and the login-time send. This is a shared
protocol foundation, not a stonecutter-only detail.

Risk of guessing: a hand-rolled codec that disagrees with 26.3 would desync or
disconnect real clients, and there is no client in this environment to verify
against.

Server-side consequence: `handle_container_button_click`, recipe selection,
result computation, consumption, quick-move and close all work and are covered
by tests, but a real 26.3 client shows an empty stonecutter recipe grid because
it never received the list.

Needed decision/output: approve building the `ClientboundUpdateRecipesPacket`
foundation (SlotDisplay/RecipeDisplay/Ingredient codecs + login send) as its own
stage, or confirm that recipe-list sync stays out of scope for now.

Blocks: full client-visible W03-01 acceptance, W03-02 (loom pattern list),
W03-65 (recipe book / knowledge book), and any recipe-book driven UI.

Not blocked: server-authoritative recipe selection, result, consumption and
quick-move for the stonecutter (implemented), and the enchanting-table/lectern
button paths that do not need the display list.

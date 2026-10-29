# Night 02 — remaining foundations

The implementation journal previously referenced this file before it existed.
The following is a verified status record, not a claim that the original agent
asked the owner any of these questions.

- KnowledgeBook: `KnowledgeBookItem.java` requires recipe lookup followed by
  `Player.awardRecipes`, including a server recipe book and client unlocks.
  No equivalent player API was found in Vecta. The item is not implemented.
- Book page size: Vecta counts serde JSON, while the vanilla component codec
  collapses plain components to strings. The current TextComponents dependency
  supplies codec NBT but no matching codec JSON API. Exact boundary validation,
  including nested components, remains incomplete; see the review for sources.
- N19: Avatar profile printing, multipart body lookup and enabled-feature gating
  need their corresponding foundations. Painting and ArmorStand are not yet
  entity implementations in Vecta; pick results for them remain future work.

None of these gaps authorizes fake data, placeholder callbacks, accepting the
Minecraft EULA or claiming that all of N03/N19 is complete.

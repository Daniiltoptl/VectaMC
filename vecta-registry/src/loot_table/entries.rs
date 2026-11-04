use super::{
    ConditionalLootFunction, Identifier, ItemStack, LootCondition, LootContext, LootType,
    NumberProvider, REGISTRY, RegistryExt, TaggedRegistryExt, next_int,
};
use vecta_utils::random::{Random as _, RandomSource};

type LootConsumer<'ctx, 'output> = dyn FnMut(ItemStack, &mut LootContext<'ctx>) + 'output;

/// A loot table entry that can generate items.
#[derive(Debug, Clone)]
pub enum LootEntry {
    /// Drop a specific item.
    Item {
        name: Identifier,
        weight: i32,
        quality: i32,
        conditions: &'static [LootCondition],
        functions: &'static [ConditionalLootFunction],
    },
    /// Reference another loot table by name.
    LootTableRef {
        name: Identifier,
        weight: i32,
        quality: i32,
        conditions: &'static [LootCondition],
        functions: &'static [ConditionalLootFunction],
    },
    /// Inline loot table (embedded pools directly in entry).
    InlineLootTable {
        pools: &'static [LootPool],
        weight: i32,
        quality: i32,
        conditions: &'static [LootCondition],
        functions: &'static [ConditionalLootFunction],
    },
    /// Drop items from a tag.
    Tag {
        name: Identifier,
        expand: bool,
        weight: i32,
        quality: i32,
        conditions: &'static [LootCondition],
        functions: &'static [ConditionalLootFunction],
    },
    /// Try children in order, use first that matches.
    Alternatives {
        children: &'static [LootEntry],
        conditions: &'static [LootCondition],
    },
    /// Use all children.
    Group {
        children: &'static [LootEntry],
        conditions: &'static [LootCondition],
    },
    /// Use children in sequence until one fails.
    Sequence {
        children: &'static [LootEntry],
        conditions: &'static [LootCondition],
    },
    /// Empty entry (no drop).
    Empty {
        weight: i32,
        conditions: &'static [LootCondition],
    },
    /// Dynamic content (e.g., block entity contents).
    Dynamic {
        name: Identifier,
        conditions: &'static [LootCondition],
    },
    /// Select items from specific block entity slots.
    Slots {
        /// Slots to select from (can be single slot or range).
        slots: SlotRange,
        conditions: &'static [LootCondition],
        functions: &'static [ConditionalLootFunction],
    },
}

/// A range of slots for the Slots entry type.
#[derive(Debug, Clone, Copy)]
pub enum SlotRange {
    /// A single specific slot index.
    Single(i32),
    /// A range of slots (inclusive).
    Range { min: i32, max: i32 },
    /// All contents slots.
    Contents,
    /// Specific named slots (for entities).
    Named(&'static [&'static str]),
}

impl LootEntry {
    /// Get the weight of this entry for random selection.
    #[must_use]
    pub const fn weight(&self) -> i32 {
        match self {
            Self::Item { weight, .. } => *weight,
            Self::LootTableRef { weight, .. } => *weight,
            Self::InlineLootTable { weight, .. } => *weight,
            Self::Tag { weight, .. } => *weight,
            Self::Empty { weight, .. } => *weight,
            // Composite entries don't have weight
            Self::Alternatives { .. }
            | Self::Group { .. }
            | Self::Sequence { .. }
            | Self::Dynamic { .. }
            | Self::Slots { .. } => 1,
        }
    }

    /// Get the quality modifier for luck-based weight adjustment.
    #[must_use]
    pub const fn quality(&self) -> i32 {
        match self {
            Self::Item { quality, .. } => *quality,
            Self::LootTableRef { quality, .. } => *quality,
            Self::InlineLootTable { quality, .. } => *quality,
            Self::Tag { quality, .. } => *quality,
            Self::Empty { .. }
            | Self::Alternatives { .. }
            | Self::Group { .. }
            | Self::Sequence { .. }
            | Self::Dynamic { .. }
            | Self::Slots { .. } => 0,
        }
    }

    /// Get the effective weight adjusted for luck.
    /// Formula: max(floor(weight + quality * luck), 0)
    #[must_use]
    pub fn effective_weight(&self, luck: f32) -> i32 {
        let base = self.weight() as f32;
        let quality = self.quality() as f32;
        (base + quality * luck).floor().max(0.0) as i32
    }

    /// Get the conditions for this entry.
    #[must_use]
    pub const fn conditions(&self) -> &'static [LootCondition] {
        match self {
            Self::Item { conditions, .. } => conditions,
            Self::LootTableRef { conditions, .. } => conditions,
            Self::InlineLootTable { conditions, .. } => conditions,
            Self::Tag { conditions, .. } => conditions,
            Self::Alternatives { conditions, .. } => conditions,
            Self::Group { conditions, .. } => conditions,
            Self::Sequence { conditions, .. } => conditions,
            Self::Empty { conditions, .. } => conditions,
            Self::Dynamic { conditions, .. } => conditions,
            Self::Slots { conditions, .. } => conditions,
        }
    }

    /// Get the functions for this entry.
    #[must_use]
    pub const fn functions(&self) -> &'static [ConditionalLootFunction] {
        match self {
            Self::Item { functions, .. } => functions,
            Self::LootTableRef { functions, .. } => functions,
            Self::InlineLootTable { functions, .. } => functions,
            Self::Tag { functions, .. } => functions,
            Self::Slots { functions, .. } => functions,
            Self::Empty { .. }
            | Self::Alternatives { .. }
            | Self::Group { .. }
            | Self::Sequence { .. }
            | Self::Dynamic { .. } => &[],
        }
    }
}

/// A pool of loot entries with roll counts.
#[derive(Debug, Clone)]
pub struct LootPool {
    pub rolls: NumberProvider,
    pub bonus_rolls: f32,
    pub entries: &'static [LootEntry],
    pub conditions: &'static [LootCondition],
    pub functions: &'static [ConditionalLootFunction],
}

/// A complete loot table definition.
#[derive(Debug)]
pub struct LootTable {
    pub key: Identifier,
    pub loot_type: LootType,
    pub pools: &'static [LootPool],
    pub functions: &'static [ConditionalLootFunction],
    pub random_sequence: Option<Identifier>,
}

impl LootTable {
    /// Generate random items from this loot table.
    ///
    /// # Arguments
    /// * `ctx` - The loot context containing RNG, luck, block state, tool, etc.
    ///
    /// This follows vanilla's approach:
    /// 1. For each pool, check conditions
    /// 2. Roll `rolls + floor(bonus_rolls * luck)` times
    /// 3. Each roll does weighted random selection among valid entries
    /// 4. Apply entry-level functions to each item
    /// 5. Apply pool-level functions to all items from that pool
    /// 6. Apply table-level functions to all items from the table
    pub fn get_random_items(&self, ctx: &mut LootContext<'_>) -> Vec<ItemStack> {
        let mut result = Vec::new();
        self.get_random_items_raw(ctx, &mut |item, _| {
            let max_stack_size = item.max_stack_size();
            if item.count() < max_stack_size {
                result.push(item);
                return;
            }
            let mut count = item.count();
            while count > 0 {
                let size = max_stack_size.min(count);
                result.push(item.copy_with_count(size));
                count -= size;
            }
        });
        result
    }

    fn get_random_items_raw<'ctx>(
        &self,
        ctx: &mut LootContext<'ctx>,
        output: &mut LootConsumer<'ctx, '_>,
    ) {
        let mut decorated = |mut item: ItemStack, context: &mut LootContext<'ctx>| {
            ConditionalLootFunction::apply_all(self.functions, &mut item, context);
            output(item, context);
        };
        for pool in self.pools {
            pool.add_random_items(ctx, &mut decorated);
        }
    }

    /// Fills the empty slots of a container with this table's loot.
    ///
    /// Mirrors Vanilla `LootTable.fill`: rolled stacks are split so that a
    /// container with spare slots spreads its loot instead of stacking
    /// everything into a few slots, then the result is shuffled and written
    /// into the empty slots in a shuffled order.
    pub fn fill(&self, slots: &mut [ItemStack], ctx: &mut LootContext<'_>) {
        let mut items = self.get_random_items(ctx);
        let mut available = available_slots(slots, ctx.rng);
        shuffle_and_split_items(&mut items, available.len(), ctx.rng);

        for item in items {
            let Some(slot) = available.pop() else {
                log::warn!("Tried to over-fill a container");
                return;
            };
            slots[slot] = item;
        }
    }
}

/// Empty slots of `slots`, in the shuffled order Vanilla fills them.
fn available_slots(slots: &[ItemStack], rng: &mut RandomSource) -> Vec<usize> {
    let mut available: Vec<usize> = slots
        .iter()
        .enumerate()
        .filter(|(_, stack)| stack.is_empty())
        .map(|(slot, _)| slot)
        .collect();
    shuffle(&mut available, rng);
    available
}

/// Vanilla `Util.shuffle`.
fn shuffle<T>(list: &mut [T], rng: &mut RandomSource) {
    for index in (1..list.len()).rev() {
        let swap_with = rng.next_i32_bounded(index as i32 + 1) as usize;
        list.swap(index, swap_with);
    }
}

/// Vanilla `LootTable.shuffleAndSplitItems`.
fn shuffle_and_split_items(
    result: &mut Vec<ItemStack>,
    available_slots: usize,
    rng: &mut RandomSource,
) {
    let mut splittable: Vec<ItemStack> = Vec::new();
    result.retain(|stack| {
        if stack.is_empty() {
            return false;
        }
        if stack.count() > 1 {
            splittable.push(stack.clone());
            return false;
        }
        true
    });

    while available_slots > result.len() + splittable.len() && !splittable.is_empty() {
        let index = next_int(rng, 0, splittable.len() as i32 - 1) as usize;
        let mut stack = splittable.remove(index);
        let remove = next_int(rng, 1, stack.count() / 2);
        let copy = stack.split(remove);
        if stack.count() > 1 && rng.next_bool() {
            splittable.push(stack);
        } else {
            result.push(stack);
        }
        if copy.count() > 1 && rng.next_bool() {
            splittable.push(copy);
        } else {
            result.push(copy);
        }
    }

    result.extend(splittable);
    shuffle(result, rng);
}

impl LootPool {
    /// Add random items from this pool to the result.
    fn add_random_items<'ctx>(
        &self,
        ctx: &mut LootContext<'ctx>,
        output: &mut LootConsumer<'ctx, '_>,
    ) {
        if !self.conditions.iter().all(|condition| condition.test(ctx)) {
            return;
        }
        let roll_count =
            self.rolls.get_int_with_context(ctx) + (self.bonus_rolls * ctx.luck).floor() as i32;
        let mut decorated = |mut item: ItemStack, context: &mut LootContext<'ctx>| {
            ConditionalLootFunction::apply_all(self.functions, &mut item, context);
            output(item, context);
        };
        for _ in 0..roll_count {
            self.add_random_item(ctx, &mut decorated);
        }
    }

    /// Select and add a single random item from this pool.
    fn add_random_item<'ctx>(
        &self,
        ctx: &mut LootContext<'ctx>,
        output: &mut LootConsumer<'ctx, '_>,
    ) {
        // Collect valid entries with their effective weights
        let mut valid_entries: Vec<(&LootEntry, i32)> = Vec::new();
        let mut total_weight = 0;

        for entry in self.entries {
            // Check entry conditions
            let passes_conditions = entry.conditions().iter().all(|c| c.test(ctx));

            if !passes_conditions {
                continue;
            }

            let weight = entry.effective_weight(ctx.luck);
            if weight > 0 {
                valid_entries.push((entry, weight));
                total_weight += weight;
            }
        }

        if total_weight == 0 || valid_entries.is_empty() {
            return;
        }

        // Weighted random selection
        let selected = if valid_entries.len() == 1 {
            valid_entries[0].0
        } else {
            // Vanilla `WeightedRandom.getRandomItem` draws `nextInt(totalWeight)`.
            let mut index = ctx.rng.next_i32_bounded(total_weight);
            let mut selected_entry = valid_entries[0].0;
            for (entry, weight) in &valid_entries {
                index -= weight;
                if index < 0 {
                    selected_entry = entry;
                    break;
                }
            }
            selected_entry
        };

        // Generate item(s) from the selected entry
        selected.create_items(ctx, output);
    }
}

impl LootEntry {
    /// Emits items through entry, pool and table modifiers before generating the next item.
    fn create_items<'ctx>(&self, ctx: &mut LootContext<'ctx>, output: &mut LootConsumer<'ctx, '_>) {
        let mut emit = |mut item: ItemStack, context: &mut LootContext<'ctx>| {
            ConditionalLootFunction::apply_all(self.functions(), &mut item, context);
            output(item, context);
        };
        match self {
            LootEntry::Item { name, .. } => {
                if let Some(item_ref) = REGISTRY.items.by_key(name) {
                    emit(ItemStack::new(item_ref), ctx);
                }
            }
            LootEntry::LootTableRef { name, .. } => {
                if let Some(table) = REGISTRY.loot_tables.by_key(name) {
                    table.get_random_items_raw(ctx, &mut emit);
                }
            }
            LootEntry::InlineLootTable { pools, .. } => {
                for pool in *pools {
                    pool.add_random_items(ctx, &mut emit);
                }
            }
            LootEntry::Tag { name, expand, .. } => {
                if let Some(items) = REGISTRY.items.get_tag(name) {
                    if *expand {
                        if !items.is_empty() {
                            let index = ctx.rng.next_i32_bounded(items.len() as i32) as usize;
                            emit(ItemStack::new(items[index]), ctx);
                        }
                    } else {
                        for item_ref in items {
                            emit(ItemStack::new(item_ref), ctx);
                        }
                    }
                }
            }
            LootEntry::Alternatives { children, .. } => {
                for child in *children {
                    if !child
                        .conditions()
                        .iter()
                        .all(|condition| condition.test(ctx))
                    {
                        continue;
                    }
                    let mut emitted = false;
                    child.create_items(ctx, &mut |item, context| {
                        emitted = true;
                        emit(item, context);
                    });
                    if emitted {
                        break;
                    }
                }
            }
            LootEntry::Group { children, .. } => {
                for child in *children {
                    if child
                        .conditions()
                        .iter()
                        .all(|condition| condition.test(ctx))
                    {
                        child.create_items(ctx, &mut emit);
                    }
                }
            }
            LootEntry::Sequence { children, .. } => {
                for child in *children {
                    if !child
                        .conditions()
                        .iter()
                        .all(|condition| condition.test(ctx))
                    {
                        break;
                    }
                    child.create_items(ctx, &mut emit);
                }
            }
            LootEntry::Empty { .. } => {}
            LootEntry::Dynamic { name, .. } => {
                // Dynamic entries are used for block entity contents (like shulker boxes)
                // The name identifies what content to retrieve:
                // - "contents" = block entity inventory contents
                // - Other names may exist for specific use cases
                //
                // TODO: Implement when block entity system supports inventory retrieval
                // This requires:
                // 1. Block entity reference in LootContext
                // 2. Method to get inventory contents from block entity
                // 3. Adding those items to the result
                let _ = name;
            }
            LootEntry::Slots {
                slots, functions, ..
            } => {
                // Slots entries select items from specific block entity slots
                // TODO: Implement when block entity system supports slot access
                // This requires:
                // 1. Block entity reference in LootContext
                // 2. Method to get items from specific slots
                // 3. Apply functions to each retrieved item
                let _ = slots;
                let _ = functions;
            }
        }
    }
}

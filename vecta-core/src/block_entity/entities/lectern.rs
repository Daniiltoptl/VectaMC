//! Lectern block entity implementation.
//!
//! The lectern stores exactly one book plus the page the readers are on. The
//! block's `has_book` and `powered` states are derived from that storage, so the
//! container keeps the pending state changes and publishes them through the
//! unlocked owner callback instead of touching the world while its mutex is held.

use std::mem;
use std::sync::{Arc, Weak};

use simdnbt::ToNbtTag;
use simdnbt::borrow::{BaseNbtCompound as BorrowedNbtCompound, NbtCompound as NbtCompoundView};
use simdnbt::owned::{NbtCompound, NbtTag};
use vecta_registry::blocks::block_state_ext::BlockStateExt as _;
use vecta_registry::blocks::properties::BlockStateProperties;
use vecta_registry::data_components::vanilla_components::{
    WRITABLE_BOOK_CONTENT, WRITTEN_BOOK_CONTENT,
};
use vecta_registry::item_stack::ItemStack;
use vecta_registry::level_events::SOUND_PAGE_TURN;
use vecta_registry::vanilla_block_entity_types;
use vecta_registry::{vanilla_blocks, vanilla_game_events};
use vecta_utils::types::UpdateFlags;
use vecta_utils::{BlockPos, BlockStateId, DowncastType, DowncastTypeKey, locks::SyncMutex};

use crate::block_entity::{BlockEntity, BlockEntityBase};
use crate::inventory::container::Container;
use crate::inventory::lock::{ContainerLockGuard, ContainerRef, SharedContainer};
use crate::world::{LevelReader as _, World, game_event::GameEventContext};

/// Vanilla `LecternBlock.PAGE_CHANGE_IMPULSE_TICKS`.
const PAGE_CHANGE_IMPULSE_TICKS: i32 = 2;
/// Vanilla `LecternBlockEntity.SLOT_BOOK`.
pub const LECTERN_BOOK_SLOT: usize = 0;
/// Vanilla `LecternBlockEntity.NUM_SLOTS`.
pub const LECTERN_SLOTS: usize = 1;
const BOOK_NBT_KEY: &str = "Book";
const PAGE_NBT_KEY: &str = "Page";

/// Storage for the lectern's single book plus the shared page position.
///
/// The container is public so menu kinds can read and advance the page through
/// the [`ContainerLockGuard`] they already hold instead of re-locking it.
pub struct LecternBookContainer {
    items: Vec<ItemStack>,
    page: i32,
    page_count: i32,
    /// The `has_book` block state has to be republished.
    has_book_pending: bool,
    /// The page moved, so the block has to emit its redstone impulse.
    page_signal_pending: bool,
}

/// Lectern block entity.
pub struct LecternBlockEntity {
    base: Arc<BlockEntityBase>,
    container: Arc<SyncMutex<LecternBookContainer>>,
    container_ref: ContainerRef,
}

// SAFETY: This key is owned by Vecta and uniquely identifies
// `LecternBlockEntity`.
unsafe impl DowncastType for LecternBlockEntity {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:block_entity/lectern");
}

// SAFETY: This key is owned by Vecta and uniquely identifies the independently
// lockable book storage used by a lectern block entity.
unsafe impl DowncastType for LecternBookContainer {
    const TYPE_KEY: DowncastTypeKey = DowncastTypeKey::new("vecta:container/lectern");
}

impl LecternBlockEntity {
    /// Creates an empty lectern block entity.
    #[must_use]
    pub fn new(level: Weak<World>, pos: BlockPos, state: BlockStateId) -> Self {
        let callback_level = Weak::clone(&level);
        let base = Arc::new(BlockEntityBase::new(
            &vanilla_block_entity_types::LECTERN,
            level,
            pos,
            state,
        ));
        let container = Arc::new(SyncMutex::new(LecternBookContainer {
            items: vec![ItemStack::empty(); LECTERN_SLOTS],
            page: 0,
            page_count: 0,
            has_book_pending: false,
            page_signal_pending: false,
        }));
        let callback_container = Arc::clone(&container);
        let after_changed = Arc::new(move || {
            Self::publish_pending_state(&callback_level, pos, &callback_container);
        });
        let shared_container: SharedContainer =
            Arc::<SyncMutex<LecternBookContainer>>::clone(&container);

        Self {
            container_ref: ContainerRef::owned_by_block_entity_with_callback(
                shared_container,
                Arc::clone(&base),
                after_changed,
            ),
            base,
            container,
        }
    }

    /// Returns the stored book.
    #[must_use]
    pub fn book(&self) -> ItemStack {
        self.container.lock().items[LECTERN_BOOK_SLOT].clone()
    }

    /// Returns whether the stored stack is a written or writable book.
    #[must_use]
    pub fn has_book(&self) -> bool {
        self.container.lock().has_book()
    }

    /// Replaces the stored book and resets the page, mirroring
    /// `LecternBlockEntity.setBook`.
    pub fn set_book(&self, book: ItemStack) {
        let id = self.container_ref.container_id();
        let mut guard = ContainerLockGuard::lock_all(&[&self.container_ref]);
        if let Some(container) = guard.get_typed_mut::<LecternBookContainer>(id) {
            container.set_book(book);
            container.has_book_pending = true;
        }
        guard.set_changed(id);
    }

    /// Returns the current page.
    #[must_use]
    pub fn page(&self) -> i32 {
        self.container.lock().page
    }

    /// Returns the number of pages in the stored book.
    #[must_use]
    pub fn page_count(&self) -> i32 {
        self.container.lock().page_count
    }

    /// Sets the page, clamped to the book, mirroring `LecternBlockEntity.setPage`.
    ///
    /// Returns the page the lectern ends up on.
    #[must_use]
    pub fn set_page(&self, page: i32) -> i32 {
        let id = self.container_ref.container_id();
        let mut guard = ContainerLockGuard::lock_all(&[&self.container_ref]);
        let changed = guard
            .get_typed_mut::<LecternBookContainer>(id)
            .is_some_and(|container| container.set_page(page).1);
        if changed {
            guard.set_changed(id);
        }
        drop(guard);
        self.page()
    }

    /// Returns the comparator output, mirroring
    /// `LecternBlockEntity.getRedstoneSignal`.
    #[must_use]
    pub fn redstone_signal(&self) -> i32 {
        let container = self.container.lock();
        let page_progress = if container.page_count > 1 {
            container.page as f32 / (container.page_count - 1) as f32
        } else {
            1.0
        };
        (page_progress * 14.0).floor() as i32 + i32::from(container.has_book())
    }

    /// Publishes the pending `has_book` / `powered` changes once no container
    /// lock is held.
    fn publish_pending_state(
        level: &Weak<World>,
        pos: BlockPos,
        container: &SyncMutex<LecternBookContainer>,
    ) {
        let (has_book, page_signal) = {
            let mut container = container.lock();
            let has_book = if container.has_book_pending {
                container.has_book_pending = false;
                Some(container.has_book())
            } else {
                None
            };
            let page_signal = container.page_signal_pending;
            container.page_signal_pending = false;
            (has_book, page_signal)
        };
        if has_book.is_none() && !page_signal {
            return;
        }

        let Some(world) = level.upgrade() else {
            return;
        };
        let state = world.get_block_state(pos);
        if state.get_block() != &vanilla_blocks::LECTERN {
            return;
        }

        let mut updated_state = state;
        if let Some(has_book) = has_book {
            updated_state = updated_state.set_value(&BlockStateProperties::HAS_BOOK, has_book);
        }
        if page_signal {
            updated_state = updated_state.set_value(&BlockStateProperties::POWERED, true);
        }
        if updated_state != state {
            world.set_block(pos, updated_state, UpdateFlags::UPDATE_ALL);
            world.game_event(
                &vanilla_game_events::BLOCK_CHANGE,
                pos,
                &GameEventContext::new(None, Some(updated_state)),
            );
        }

        // Vanilla updates the block below in both `resetBookState` and
        // `changePowered`, so a comparator underneath re-reads the signal.
        world.update_neighbors_at(pos.below(), &vanilla_blocks::LECTERN);

        if page_signal {
            world.schedule_block_tick_default(
                pos,
                &vanilla_blocks::LECTERN,
                PAGE_CHANGE_IMPULSE_TICKS,
            );
            world.level_event(SOUND_PAGE_TURN, pos, 0, None);
        }
    }
}

impl BlockEntity for LecternBlockEntity {
    fn base(&self) -> &BlockEntityBase {
        &self.base
    }

    fn pre_remove_side_effects(&self, pos: BlockPos, state: BlockStateId) {
        if !state.get_value(&BlockStateProperties::HAS_BOOK) {
            return;
        }
        let book = self.container.lock().items[LECTERN_BOOK_SLOT].clone();
        if book.is_empty() {
            return;
        }
        let Some(world) = self.get_level() else {
            return;
        };
        // Vanilla drops the book slightly in front of the lectern, at eye height.
        let (step_x, _, step_z) = state
            .get_value(&BlockStateProperties::HORIZONTAL_FACING)
            .offset();
        let position = glam::DVec3::new(
            f64::from(pos.x()) + 0.5 + 0.25 * f64::from(step_x),
            f64::from(pos.y()) + 1.0,
            f64::from(pos.z()) + 0.5 + 0.25 * f64::from(step_z),
        );
        world.drop_item_stack_at(position, book);
    }

    fn load_additional(&self, nbt: &BorrowedNbtCompound<'_>) {
        let nbt: NbtCompoundView<'_, '_> = nbt.into();
        let book = nbt
            .compound(BOOK_NBT_KEY)
            .and_then(|book| ItemStack::from_borrowed_compound(&book))
            .unwrap_or_else(ItemStack::empty);
        let page_count = page_count_of(&book);
        let saved_page = nbt.int(PAGE_NBT_KEY).unwrap_or(0);

        let mut container = self.container.lock();
        container.items[LECTERN_BOOK_SLOT] = book;
        container.page_count = page_count;
        // Vanilla clamps with `Mth.clamp`, which yields `max` when `max < min`.
        container.page = vanilla_clamp(saved_page, 0, page_count - 1);
    }

    fn save_additional(&self, nbt: &mut NbtCompound) {
        let container = self.container.lock();
        let book = &container.items[LECTERN_BOOK_SLOT];
        if book.is_empty() {
            return;
        }
        if let NbtTag::Compound(book_nbt) = book.clone().to_nbt_tag() {
            nbt.insert(BOOK_NBT_KEY, book_nbt);
        }
        nbt.insert(PAGE_NBT_KEY, container.page);
    }

    fn container_ref(&self) -> Option<ContainerRef> {
        Some(self.container_ref.clone())
    }
}

impl LecternBookContainer {
    /// Returns the current page.
    #[must_use]
    pub const fn page(&self) -> i32 {
        self.page
    }

    /// Returns the number of pages in the stored book.
    #[must_use]
    pub const fn page_count(&self) -> i32 {
        self.page_count
    }

    /// Returns whether the stored stack is a written or writable book.
    #[must_use]
    pub fn has_book(&self) -> bool {
        let book = &self.items[LECTERN_BOOK_SLOT];
        book.has(WRITTEN_BOOK_CONTENT) || book.has(WRITABLE_BOOK_CONTENT)
    }

    /// Mirrors `LecternBlockEntity.setBook`.
    pub fn set_book(&mut self, book: ItemStack) {
        self.items[LECTERN_BOOK_SLOT] = book;
        self.page = 0;
        self.page_count = page_count_of(&self.items[LECTERN_BOOK_SLOT]);
    }

    /// Mirrors `LecternBlockEntity.onBookItemRemove`.
    const fn on_book_item_removed(&mut self) {
        self.page = 0;
        self.page_count = 0;
        self.has_book_pending = true;
    }

    /// Mirrors `LecternBlockEntity.setPage`, including vanilla's clamp when the
    /// book has no pages. Returns the page after clamping and whether it moved.
    pub fn set_page(&mut self, page: i32) -> (i32, bool) {
        let new_page = vanilla_clamp(page, 0, self.page_count - 1);
        if new_page == self.page {
            return (self.page, false);
        }
        self.page = new_page;
        self.page_signal_pending = true;
        (self.page, true)
    }
}

/// Vanilla `Mth.clamp`: `min(max, max(min, value))`.
///
/// The order matters when the book has no pages, where vanilla passes
/// `max = -1` and therefore stores `-1` instead of `0`.
fn vanilla_clamp(value: i32, min: i32, max: i32) -> i32 {
    value.max(min).min(max)
}

/// Vanilla `LecternBlockEntity.getPageCount`.
fn page_count_of(book: &ItemStack) -> i32 {
    if let Some(content) = book.get(WRITTEN_BOOK_CONTENT) {
        return i32::try_from(content.pages().len()).unwrap_or(i32::MAX);
    }
    if let Some(content) = book.get(WRITABLE_BOOK_CONTENT) {
        return i32::try_from(content.pages().len()).unwrap_or(i32::MAX);
    }
    0
}

impl Container for LecternBookContainer {
    fn items(&self) -> &[ItemStack] {
        &self.items
    }

    fn items_mut(&mut self) -> &mut [ItemStack] {
        &mut self.items
    }

    fn get_container_size(&self) -> usize {
        LECTERN_SLOTS
    }

    /// Vanilla's `bookAccess.setItem` is a no-op: the book is only ever replaced
    /// through [`LecternBlockEntity::set_book`].
    fn set_item(&mut self, _slot: usize, _stack: ItemStack) {}

    fn remove_item(&mut self, slot: usize, count: i32) -> ItemStack {
        if slot != LECTERN_BOOK_SLOT {
            return ItemStack::empty();
        }
        let removed = self.items[LECTERN_BOOK_SLOT].split(count);
        if self.items[LECTERN_BOOK_SLOT].is_empty() {
            self.on_book_item_removed();
        }
        removed
    }

    fn remove_item_no_update(&mut self, slot: usize) -> ItemStack {
        if slot != LECTERN_BOOK_SLOT {
            return ItemStack::empty();
        }
        let removed = mem::take(&mut self.items[LECTERN_BOOK_SLOT]);
        self.on_book_item_removed();
        removed
    }

    fn get_max_stack_size(&self) -> i32 {
        1
    }

    /// Vanilla's `bookAccess.canPlaceItem` rejects every stack.
    fn can_place_item(&self, _slot: usize, _stack: &ItemStack) -> bool {
        false
    }

    fn set_changed(&mut self) {}
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use simdnbt::borrow::read_compound as read_borrowed_compound;
    use vecta_registry::data_components::components::{Filterable, WritableBookContent};
    use vecta_registry::{init_vanilla_registry, vanilla_items};

    use super::*;

    fn lectern() -> LecternBlockEntity {
        init_vanilla_registry();
        LecternBlockEntity::new(
            Weak::new(),
            BlockPos::new(2, 70, -3),
            vanilla_blocks::LECTERN.default_state(),
        )
    }

    fn writable_book(pages: usize) -> ItemStack {
        let mut book = ItemStack::new(&vanilla_items::WRITABLE_BOOK);
        let content = WritableBookContent::new(
            (0..pages)
                .map(|page| Filterable::pass_through(format!("page {page}")))
                .collect(),
        )
        .expect("test pages are valid");
        book.set(WRITABLE_BOOK_CONTENT, content);
        book
    }

    #[test]
    fn setting_a_book_resets_the_page_and_counts_its_pages() {
        let entity = lectern();
        entity.set_book(writable_book(3));

        assert!(entity.has_book());
        assert_eq!(entity.page(), 0);
        assert_eq!(entity.page_count(), 3);
    }

    #[test]
    fn set_page_clamps_to_the_book_and_vanilla_without_pages() {
        let entity = lectern();
        entity.set_book(writable_book(3));

        assert_eq!(entity.set_page(2), 2);
        assert_eq!(entity.set_page(9), 2);
        assert_eq!(entity.set_page(-5), 0);

        let empty = lectern();
        assert_eq!(empty.page_count(), 0);
        assert_eq!(empty.set_page(0), -1);
    }

    #[test]
    fn redstone_signal_follows_the_page_progress() {
        let entity = lectern();
        entity.set_book(writable_book(4));

        assert_eq!(entity.redstone_signal(), 1);
        assert_eq!(entity.set_page(1), 1);
        assert_eq!(entity.redstone_signal(), 5);
        assert_eq!(entity.set_page(3), 3);
        assert_eq!(entity.redstone_signal(), 15);
    }

    #[test]
    fn removing_the_book_clears_the_page_state() {
        let entity = lectern();
        entity.set_book(writable_book(4));
        assert_eq!(entity.set_page(2), 2);

        {
            let mut container = entity.container.lock();
            let removed = container.remove_item(LECTERN_BOOK_SLOT, 1);
            assert_eq!(removed.count(), 1);
        }

        assert!(!entity.has_book());
        assert_eq!(entity.page(), 0);
        assert_eq!(entity.page_count(), 0);
        // Vanilla's `getRedstoneSignal` treats a book-less lectern as "one page
        // of one", so the raw block entity value is 14. `LecternBlock` gates it
        // behind `has_book`, which is why the comparator sees 0.
        assert_eq!(entity.redstone_signal(), 14);
    }

    #[test]
    fn book_and_page_round_trip_through_vanilla_nbt_keys() {
        let source = lectern();
        source.set_book(writable_book(5));
        assert_eq!(source.set_page(3), 3);

        let mut nbt = NbtCompound::new();
        source.save_additional(&mut nbt);
        assert_eq!(nbt.int(PAGE_NBT_KEY), Some(3));
        assert!(nbt.compound(BOOK_NBT_KEY).is_some());

        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed = read_borrowed_compound(&mut Cursor::new(bytes.as_slice()))
            .expect("test NBT should reborrow");
        let loaded = lectern();
        loaded.load_additional(&borrowed);

        assert_eq!(loaded.book().item(), &*vanilla_items::WRITABLE_BOOK);
        assert_eq!(loaded.page(), 3);
        assert_eq!(loaded.page_count(), 5);
    }

    #[test]
    fn loading_without_a_book_yields_the_empty_vanilla_defaults() {
        let nbt = NbtCompound::new();
        let mut bytes = Vec::new();
        nbt.write(&mut bytes);
        let borrowed = read_borrowed_compound(&mut Cursor::new(bytes.as_slice()))
            .expect("test NBT should reborrow");
        let loaded = lectern();
        loaded.set_book(writable_book(2));
        loaded.load_additional(&borrowed);

        assert!(loaded.book().is_empty());
        // Vanilla clamps the saved page against `pageCount - 1`, so a book-less
        // lectern loads with page `-1` rather than `0`.
        assert_eq!(loaded.page(), -1);
        assert_eq!(loaded.page_count(), 0);
    }

    #[test]
    fn empty_lectern_saves_no_book_keys() {
        let entity = lectern();
        let mut nbt = NbtCompound::new();
        entity.save_additional(&mut nbt);

        assert!(nbt.compound(BOOK_NBT_KEY).is_none());
        assert_eq!(nbt.int(PAGE_NBT_KEY), None);
    }

    #[test]
    fn the_book_slot_rejects_insertion_and_ignores_direct_sets() {
        let entity = lectern();
        let container = entity.container.lock();

        assert!(!container.can_place_item(0, &writable_book(1)));
        assert_eq!(container.get_max_stack_size(), 1);
        drop(container);

        let mut container = entity.container.lock();
        container.set_item(0, writable_book(1));
        assert!(container.get_item(0).is_empty());
    }
}

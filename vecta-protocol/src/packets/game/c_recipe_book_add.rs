//! Clientbound packet that adds recipe book entries on the client.

use std::io::{Result, Write};

use vecta_macros::ClientPacket;
use vecta_registry::packets::play::C_RECIPE_BOOK_ADD;
use vecta_registry::recipe::RecipeDisplayEntry;
use vecta_utils::serial::WriteTo;

/// Vanilla `ClientboundRecipeBookAddPacket`.
///
/// `replace = true` clears the client book first (initial sync); `false`
/// appends the entries (unlock notifications). Each entry carries the flag
/// bits of vanilla `ClientboundRecipeBookAddPacket.Entry`.
#[derive(ClientPacket, Clone, Debug)]
#[packet_id(Play = C_RECIPE_BOOK_ADD)]
pub struct CRecipeBookAdd {
    /// One entry per known (or newly unlocked) recipe display.
    pub entries: Vec<Entry>,
    /// Vanilla `replace`: `true` resets the client book before applying.
    pub replace: bool,
}

/// Vanilla `ClientboundRecipeBookAddPacket.Entry`.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// The display entry (`RecipeDisplayEntry.STREAM_CODEC`).
    pub contents: RecipeDisplayEntry,
    /// Vanilla `flags` byte: `FLAG_NOTIFICATION | FLAG_HIGHLIGHT`.
    pub flags: u8,
}

impl Entry {
    /// Show the unlock toast (`recipe.value().showNotification()`).
    pub const FLAG_NOTIFICATION: u8 = 1;
    /// Highlight the entry as new in the book.
    pub const FLAG_HIGHLIGHT: u8 = 2;

    /// Vanilla convenience constructor packing both booleans into the flags.
    #[must_use]
    pub fn new(contents: RecipeDisplayEntry, notification: bool, highlight: bool) -> Self {
        Self {
            contents,
            flags: (u8::from(notification) * Self::FLAG_NOTIFICATION)
                | (u8::from(highlight) * Self::FLAG_HIGHLIGHT),
        }
    }

    /// Vanilla `notification()`.
    #[must_use]
    pub const fn notification(&self) -> bool {
        (self.flags & Self::FLAG_NOTIFICATION) != 0
    }

    /// Vanilla `highlight()`.
    #[must_use]
    pub const fn highlight(&self) -> bool {
        (self.flags & Self::FLAG_HIGHLIGHT) != 0
    }
}

impl WriteTo for Entry {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.contents.write(writer)?;
        self.flags.write(writer)
    }
}

impl WriteTo for CRecipeBookAdd {
    /// `Entry.STREAM_CODEC.apply(ByteBufCodecs.list())`, then `BOOL`.
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.entries.write(writer)?;
        self.replace.write(writer)
    }
}

#[cfg(test)]
mod tests {
    use vecta_registry::recipe::DisplayIndex;
    use vecta_registry::{init_vanilla_registry, vanilla_recipes};
    use vecta_utils::codec::VarInt;
    use vecta_utils::serial::WriteTo as _;

    use super::{CRecipeBookAdd, Entry};

    fn entry_bytes(entry: &Entry) -> Vec<u8> {
        let mut bytes = Vec::new();
        entry
            .contents
            .write(&mut bytes)
            .expect("display entry should encode");
        bytes.push(entry.flags);
        bytes
    }

    /// Builds the real display entries for the baked potato recipe through
    /// the vanilla-registry display index, mirroring the server flow.
    fn baked_potato_entry() -> Entry {
        init_vanilla_registry();
        let index = DisplayIndex::build(&vecta_registry::REGISTRY.recipes);
        let info = index
            .list_displays_for_recipe(vanilla_recipes::BAKED_POTATO.key())
            .next()
            .expect("baked potato must have at least one display");
        Entry::new(info.display.clone(), true, true)
    }

    #[test]
    fn empty_packet_writes_zero_count_and_replace_false() {
        let packet = CRecipeBookAdd {
            entries: Vec::new(),
            replace: false,
        };
        let mut bytes = Vec::new();
        packet.write(&mut bytes).expect("packet should encode");
        assert_eq!(bytes, vec![0, 0]);
    }

    #[test]
    fn packet_writes_count_then_entries_then_replace_flag() {
        let entry = baked_potato_entry();
        let packet = CRecipeBookAdd {
            entries: vec![entry.clone()],
            replace: true,
        };
        let mut bytes = Vec::new();
        packet.write(&mut bytes).expect("packet should encode");

        let mut expected = Vec::new();
        VarInt(1).write(&mut expected).expect("count should encode");
        expected.extend(entry_bytes(&entry));
        expected.push(1);
        assert_eq!(bytes, expected);
    }

    #[test]
    fn entry_flags_match_the_vanilla_bits() {
        let contents = baked_potato_entry().contents;
        let both = Entry::new(contents.clone(), true, true);
        assert!(both.notification());
        assert!(both.highlight());
        assert_eq!(both.flags, Entry::FLAG_NOTIFICATION | Entry::FLAG_HIGHLIGHT);

        let only_highlight = Entry::new(contents.clone(), false, true);
        assert!(!only_highlight.notification());
        assert!(only_highlight.highlight());

        let none = Entry::new(contents, false, false);
        assert_eq!(none.flags, 0);
        assert!(!none.notification());
        assert!(!none.highlight());
    }
}

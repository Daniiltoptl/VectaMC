//! Serverbound packet for saving or signing a writable book.

use std::io::{Cursor, Error, ErrorKind, Result};

use vecta_macros::ServerPacket;
use vecta_utils::{
    codec::VarInt,
    serial::{ReadFrom, prefixed_read::read_utf},
};

/// Upper bound of `ByteBufCodecs.list(100)` on the page list.
pub const MAX_EDIT_BOOK_PAGES: usize = 100;
/// `ByteBufCodecs.stringUtf8(1024)` bound for one page, in UTF-16 code units.
pub const MAX_EDIT_BOOK_PAGE_LENGTH: usize = 1024;
/// `ByteBufCodecs.stringUtf8(32)` bound for the title, in UTF-16 code units.
pub const MAX_EDIT_BOOK_TITLE_LENGTH: usize = 32;

/// Sent by the client when the player saves or signs a writable book.
///
/// A present `title` means the client is signing: the server then turns the
/// writable book into a written book whose title is that entry.
#[derive(ServerPacket, Debug, Clone, PartialEq, Eq)]
pub struct SEditBook {
    /// Inventory index of the edited stack (`0..=8` or `40`).
    pub slot: i32,
    /// Page texts in the client's edit order.
    pub pages: Vec<String>,
    /// Book title, present only when the client is signing.
    pub title: Option<String>,
}

impl ReadFrom for SEditBook {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        let slot = VarInt::read(data)?.0;

        let page_count = VarInt::read(data)?.0;
        let page_count = usize::try_from(page_count)
            .map_err(|_| Error::new(ErrorKind::InvalidData, "edit book page count is negative"))?;
        if page_count > MAX_EDIT_BOOK_PAGES {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("edit book page count {page_count} exceeds {MAX_EDIT_BOOK_PAGES}"),
            ));
        }

        let mut pages = Vec::with_capacity(page_count);
        for _ in 0..page_count {
            pages.push(read_utf(data, MAX_EDIT_BOOK_PAGE_LENGTH)?);
        }

        let title = if bool::read(data)? {
            Some(read_utf(data, MAX_EDIT_BOOK_TITLE_LENGTH)?)
        } else {
            None
        };

        Ok(Self { slot, pages, title })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use vecta_utils::{
        codec::VarInt,
        serial::{ReadFrom as _, WriteTo as _},
    };

    use super::{
        MAX_EDIT_BOOK_PAGE_LENGTH, MAX_EDIT_BOOK_PAGES, MAX_EDIT_BOOK_TITLE_LENGTH, SEditBook,
    };

    fn utf(value: &str) -> Vec<u8> {
        let mut encoded = Vec::new();
        VarInt(value.len() as i32)
            .write(&mut encoded)
            .expect("test string length should encode");
        encoded.extend_from_slice(value.as_bytes());
        encoded
    }

    fn packet_bytes(slot: i32, pages: &[&str], title: Option<&str>) -> Vec<u8> {
        let mut bytes = Vec::new();
        VarInt(slot).write(&mut bytes).expect("slot should encode");
        VarInt(pages.len() as i32)
            .write(&mut bytes)
            .expect("page count should encode");
        for page in pages {
            bytes.extend_from_slice(&utf(page));
        }
        bytes.push(u8::from(title.is_some()));
        if let Some(title) = title {
            bytes.extend_from_slice(&utf(title));
        }
        bytes
    }

    fn decode(bytes: &[u8]) -> Result<SEditBook, std::io::Error> {
        SEditBook::read(&mut Cursor::new(bytes))
    }

    #[test]
    fn reads_saved_book_without_title() {
        let decoded = decode(&packet_bytes(3, &["first", "second"], None))
            .expect("saved book packet should decode");

        assert_eq!(decoded.slot, 3);
        assert_eq!(decoded.pages, vec!["first".to_owned(), "second".to_owned()]);
        assert_eq!(decoded.title, None);
    }

    #[test]
    fn reads_signed_book_with_title() {
        let decoded = decode(&packet_bytes(40, &["page"], Some("Title")))
            .expect("signed book packet should decode");

        assert_eq!(decoded.slot, 40);
        assert_eq!(decoded.pages, vec!["page".to_owned()]);
        assert_eq!(decoded.title.as_deref(), Some("Title"));
    }

    #[test]
    fn accepts_vanilla_page_and_title_boundaries_in_utf16_units() {
        let page = "a".repeat(MAX_EDIT_BOOK_PAGE_LENGTH);
        let title = "a".repeat(MAX_EDIT_BOOK_TITLE_LENGTH);
        let decoded = decode(&packet_bytes(0, &[page.as_str()], Some(title.as_str())))
            .expect("boundary lengths should decode");

        assert_eq!(
            decoded.pages[0].encode_utf16().count(),
            MAX_EDIT_BOOK_PAGE_LENGTH
        );
        assert_eq!(
            decoded
                .title
                .expect("title should decode")
                .encode_utf16()
                .count(),
            MAX_EDIT_BOOK_TITLE_LENGTH
        );
    }

    #[test]
    fn accepts_multibyte_pages_above_the_character_bound_in_bytes() {
        // Four bytes per character in UTF-8, one UTF-16 surrogate pair each.
        let page = "\u{1F600}".repeat(MAX_EDIT_BOOK_PAGE_LENGTH / 2);
        assert_eq!(page.len(), MAX_EDIT_BOOK_PAGE_LENGTH * 2);

        let decoded = decode(&packet_bytes(0, &[page.as_str()], None))
            .expect("vanilla counts UTF-16 units, not bytes");
        assert_eq!(decoded.pages[0], page);
    }

    #[test]
    fn rejects_pages_above_the_vanilla_character_bound() {
        let page = "a".repeat(MAX_EDIT_BOOK_PAGE_LENGTH + 1);
        assert!(decode(&packet_bytes(0, &[page.as_str()], None)).is_err());
    }

    #[test]
    fn rejects_titles_above_the_vanilla_character_bound() {
        let title = "a".repeat(MAX_EDIT_BOOK_TITLE_LENGTH + 1);
        assert!(decode(&packet_bytes(0, &[], Some(title.as_str()))).is_err());
    }

    #[test]
    fn rejects_more_pages_than_the_vanilla_list_bound() {
        let pages = vec!["page"; MAX_EDIT_BOOK_PAGES + 1];
        assert!(decode(&packet_bytes(0, &pages, None)).is_err());
    }

    #[test]
    fn rejects_a_negative_page_count() {
        let mut bytes = Vec::new();
        VarInt(0).write(&mut bytes).unwrap();
        VarInt(-1).write(&mut bytes).unwrap();

        assert!(decode(&bytes).is_err());
    }
}

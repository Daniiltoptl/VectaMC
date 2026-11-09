//! Player recipe book consumers (vanilla `ServerRecipeBook` network flow).
//!
//! The book state lives in [`Player::recipe_book`]; this module wires it to
//! the recipe-book packets: the initial sync after `CUpdateRecipes` and the
//! two serverbound handlers. Display ids are resolved through a shared
//! [`DisplayIndex`] built from the registered recipes, mirroring vanilla's
//! `RecipeManager`-backed `DisplayResolver`.

use std::sync::LazyLock;

use vecta_protocol::packets::game::{
    CRecipeBookAdd, CRecipeBookSettings, RecipeBookAddEntry, SRecipeBookChangeSettings,
    SRecipeBookSeenRecipe,
};
use vecta_registry::REGISTRY;
use vecta_registry::recipe::DisplayIndex;

use super::Player;

/// Vanilla resolves display entries through the recipe manager; the index is
/// a deterministic function of the registered recipes, so one process-wide
/// instance serves every player until a datapack reload rebuilds the
/// registry.
fn display_index() -> &'static DisplayIndex {
    static DISPLAY_INDEX: LazyLock<DisplayIndex> =
        LazyLock::new(|| DisplayIndex::build(&REGISTRY.recipes));
    &DISPLAY_INDEX
}

impl Player {
    /// Vanilla `ServerRecipeBook.sendInitialRecipeBook`: sends the tab
    /// settings packet, then every known recipe as an add entry with the
    /// highlight flag, replacing whatever the client book held.
    pub(crate) fn send_initial_recipe_book(&self) {
        let entries = {
            let book = self.recipe_book.lock();
            self.send_packet(CRecipeBookSettings {
                book_settings: *book.book_settings(),
            });

            let index = display_index();
            book.known_keys()
                .flat_map(|key| {
                    index
                        .list_displays_for_recipe(key)
                        .map(|info| {
                            RecipeBookAddEntry::new(
                                info.display.clone(),
                                false,
                                book.is_highlighted(key),
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };

        self.send_packet(CRecipeBookAdd {
            entries,
            replace: true,
        });
    }

    /// Vanilla `ServerGamePacketListenerImpl.handleRecipeBookChangeSettingsPacket`.
    pub fn handle_recipe_book_change_settings(&self, packet: SRecipeBookChangeSettings) {
        self.recipe_book.lock().set_book_setting(
            packet.book_type,
            packet.is_open,
            packet.is_filtering,
        );
    }

    /// Vanilla `ServerGamePacketListenerImpl.handleRecipeBookSeenRecipePacket`:
    /// resolves the display id back to its recipe and clears the highlight.
    pub fn handle_recipe_book_seen_recipe(&self, packet: SRecipeBookSeenRecipe) {
        if let Some(info) = display_index().get_recipe_from_display(packet.recipe) {
            self.recipe_book.lock().remove_highlight(info.parent.key());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use text_components::TextComponent;
    use vecta_protocol::packet_traits::{CompressionInfo, EncodedPacket};
    use vecta_protocol::packets::game::{SRecipeBookChangeSettings, SRecipeBookSeenRecipe};
    use vecta_registry::packets::play::{C_RECIPE_BOOK_ADD, C_RECIPE_BOOK_SETTINGS};
    use vecta_registry::recipe::{RecipeBookType, RecipeDisplayId};
    use vecta_registry::{REGISTRY, init_vanilla_registry, vanilla_recipes};
    use vecta_utils::codec::VarInt;
    use vecta_utils::locks::SyncMutex;
    use vecta_utils::serial::ReadFrom as _;

    use super::{Player, display_index};
    use crate::player::PlayerConnection;
    use crate::player::connection::NetworkConnection;
    use crate::test_support::{TestPlayerBuilder, fresh_test_world};

    struct RecordingConnection {
        sent_packets: Arc<SyncMutex<Vec<EncodedPacket>>>,
        closed: AtomicBool,
    }

    impl NetworkConnection for RecordingConnection {
        fn compression(&self) -> Option<CompressionInfo> {
            None
        }

        fn send_encoded(&self, packet: EncodedPacket) {
            self.sent_packets.lock().push(packet);
        }

        fn send_encoded_bundle(&self, packets: Vec<EncodedPacket>) {
            self.sent_packets.lock().extend(packets);
        }

        fn disconnect_with_reason(&self, _reason: TextComponent) {}

        fn tick(&self) {}

        fn latency(&self) -> i32 {
            0
        }

        fn close(&self) {
            self.closed.store(true, Ordering::Release);
        }

        fn closed(&self) -> bool {
            self.closed.load(Ordering::Acquire)
        }
    }

    fn recording_player(name: &'static str) -> (Arc<Player>, Arc<SyncMutex<Vec<EncodedPacket>>>) {
        let sent_packets = Arc::new(SyncMutex::new(Vec::new()));
        let world = fresh_test_world(name).world;
        let connection = Arc::new(PlayerConnection::Other(Box::new(RecordingConnection {
            sent_packets: Arc::clone(&sent_packets),
            closed: AtomicBool::new(false),
        })));
        let player = TestPlayerBuilder::new(world, name, 1)
            .connection(connection)
            .build();
        (player, sent_packets)
    }

    /// Unwraps `[length VarInt][id VarInt][payload]` of an encoded packet.
    fn decode(packet: &EncodedPacket) -> (i32, Vec<u8>) {
        let mut cursor = Cursor::new(packet.encoded_data.as_slice());
        let _length = VarInt::read(&mut cursor).expect("packet length should decode");
        let id = VarInt::read(&mut cursor)
            .expect("packet id should decode")
            .0;
        let payload = cursor.get_ref()[cursor.position() as usize..].to_vec();
        (id, payload)
    }

    fn award_baked_potato(player: &Player) {
        let Some(recipe) = REGISTRY.recipes.by_key(vanilla_recipes::BAKED_POTATO.key()) else {
            panic!("baked potato recipe must be registered");
        };
        let awarded = player.recipe_book.lock().award_recipes([recipe]);
        assert_eq!(awarded.len(), 1);
    }

    #[test]
    fn initial_recipe_book_sends_settings_then_replace_add() {
        init_vanilla_registry();
        let (player, sent_packets) = recording_player("initial_book");
        award_baked_potato(&player);
        sent_packets.lock().clear();

        player.send_initial_recipe_book();

        let sent = sent_packets.lock();
        let decoded: Vec<(i32, Vec<u8>)> = sent.iter().map(decode).collect();
        assert_eq!(
            decoded.len(),
            2,
            "exactly the settings and add packets are sent"
        );
        assert_eq!(decoded[0].0, C_RECIPE_BOOK_SETTINGS);
        assert_eq!(
            decoded[0].1,
            vec![0; 8],
            "default book settings are eight false bools"
        );
        assert_eq!(decoded[1].0, C_RECIPE_BOOK_ADD);

        let add = &decoded[1].1;
        let mut cursor = Cursor::new(add.as_slice());
        let entry_count = VarInt::read(&mut cursor)
            .expect("entry count should decode")
            .0;
        assert_eq!(entry_count, 1, "one known recipe becomes one entry");
        assert_eq!(*add.last().expect("add packet has a replace flag"), 1);
    }

    #[test]
    fn seen_recipe_clears_the_highlight_but_keeps_the_recipe() {
        init_vanilla_registry();
        let (player, _sent_packets) = recording_player("seen_recipe");
        award_baked_potato(&player);
        assert!(
            player
                .recipe_book
                .lock()
                .is_highlighted(vanilla_recipes::BAKED_POTATO.key())
        );

        let display_id = display_index()
            .list_displays_for_recipe(vanilla_recipes::BAKED_POTATO.key())
            .next()
            .expect("baked potato must have a display entry")
            .display
            .id;

        player.handle_recipe_book_seen_recipe(SRecipeBookSeenRecipe {
            recipe: RecipeDisplayId {
                index: display_id.index,
            },
        });

        let book = player.recipe_book.lock();
        assert!(book.contains(vanilla_recipes::BAKED_POTATO.key()));
        assert!(!book.is_highlighted(vanilla_recipes::BAKED_POTATO.key()));
    }

    #[test]
    fn change_settings_updates_the_tab_state() {
        init_vanilla_registry();
        let (player, _sent_packets) = recording_player("book_settings");

        player.handle_recipe_book_change_settings(SRecipeBookChangeSettings {
            book_type: RecipeBookType::Furnace,
            is_open: true,
            is_filtering: false,
        });

        let book = player.recipe_book.lock();
        let settings = book.book_settings();
        assert!(settings.get(RecipeBookType::Furnace).open);
        assert!(!settings.get(RecipeBookType::Furnace).filtering);
        assert!(!settings.get(RecipeBookType::Crafting).open);
    }
}

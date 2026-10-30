use crate::generator_functions::{
    generate_identifier, read_json_asset, sort_contiguous_registry_entries,
};
use heck::{ToShoutySnakeCase, ToUpperCamelCase};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use rustc_hash::FxHashSet;
use serde::Deserialize;
use vecta_utils::Identifier;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtractedRegistry {
    minecraft_version: String,
    entries: Vec<ExtractedType>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtractedType {
    id: usize,
    key: Identifier,
}

pub(crate) fn build() -> TokenStream {
    const ASSET: &str = "build_assets/slot_displays.json";
    let mut registry: ExtractedRegistry = read_json_asset(ASSET);
    let version = env!("CARGO_PKG_VERSION")
        .split_once("+mc")
        .map(|(_, version)| version);
    assert_eq!(
        Some(registry.minecraft_version.as_str()),
        version,
        "Slot display extractor output must match the workspace Minecraft version"
    );
    assert!(
        !registry.entries.is_empty(),
        "Slot display registry must not be empty"
    );
    sort_contiguous_registry_entries(&mut registry.entries, ASSET, |entry| entry.id);

    let mut constants = TokenStream::new();
    let mut registrations = TokenStream::new();
    let mut keys = FxHashSet::default();
    for entry in &registry.entries {
        assert!(
            keys.insert(&entry.key),
            "Duplicate slot display key: {}",
            entry.key
        );
        assert_eq!(
            entry.key.namespace.as_ref(),
            "minecraft",
            "Unsupported extracted display namespace"
        );
        let name = Ident::new(&entry.key.path.to_shouty_snake_case(), Span::call_site());
        let kind = Ident::new(&entry.key.path.to_upper_camel_case(), Span::call_site());
        let key = generate_identifier(&entry.key);
        constants.extend(quote! {
            pub static #name: SlotDisplayType = SlotDisplayType::vanilla(#key, SlotDisplayKind::#kind);
        });
        registrations.extend(quote! { registry.register(&#name); });
    }
    quote! {
        use crate::recipe::{SlotDisplayKind, SlotDisplayType, SlotDisplayTypeRegistry};
        use std::borrow::Cow;
        use vecta_utils::Identifier;

        #constants

        pub fn register_slot_displays(registry: &mut SlotDisplayTypeRegistry) {
            #registrations
        }
    }
}

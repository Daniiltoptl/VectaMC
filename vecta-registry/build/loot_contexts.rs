use crate::generator_functions::{
    generate_identifier, read_json_asset, sort_contiguous_registry_entries,
};
use heck::ToShoutySnakeCase;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Deserialize;
use vecta_utils::Identifier;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtractedContexts {
    minecraft_version: String,
    parameters: Vec<Parameter>,
    sets: Vec<ParameterSet>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Parameter {
    key: Identifier,
    java_type: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ParameterSet {
    id: usize,
    key: Identifier,
    required: Vec<Identifier>,
    allowed: Vec<Identifier>,
}

pub(crate) fn build() -> TokenStream {
    const ASSET: &str = "build_assets/loot_contexts.json";
    let mut extracted: ExtractedContexts = read_json_asset(ASSET);
    assert_eq!(
        Some(extracted.minecraft_version.as_str()),
        env!("CARGO_PKG_VERSION")
            .split_once("+mc")
            .map(|(_, version)| version),
        "Loot context extractor version mismatch"
    );
    sort_contiguous_registry_entries(&mut extracted.sets, ASSET, |entry| entry.id);
    let mut parameters = TokenStream::new();
    let mut imports = FxHashSet::default();
    let mut parameter_ids = FxHashMap::default();
    for parameter in &extracted.parameters {
        let name = Ident::new(
            &parameter.key.path.to_shouty_snake_case(),
            Span::call_site(),
        );
        let key = parameter.key.to_string();
        let family = match parameter.java_type.as_str() {
            "net.minecraft.world.entity.Entity" | "net.minecraft.world.entity.player.Player" => {
                "EntityValue"
            }
            "net.minecraft.world.phys.Vec3" => "OriginValue",
            "net.minecraft.world.level.block.state.BlockState" => "BlockStateValue",
            "net.minecraft.world.level.block.entity.BlockEntity" => "BlockEntityValue",
            "net.minecraft.world.item.ItemInstance" => "ToolValue",
            "net.minecraft.world.damagesource.DamageSource" => "DamageSourceValue",
            "java.lang.Float" => "FloatValue",
            "java.lang.Integer" => "IntegerValue",
            "java.lang.Boolean" => "BooleanValue",
            "net.minecraft.util.Unit" => "UnitValue",
            "net.minecraft.world.entity.SlotProvider" => {
                imports.insert("ContextKeyId");
                imports.insert("ContextValueKind");
                parameters.extend(quote! { pub const #name: ContextKeyId = ContextKeyId::new(#key, ContextValueKind::Container); });
                assert!(
                    parameter_ids
                        .insert(parameter.key.clone(), quote! { params::#name })
                        .is_none(),
                    "Duplicate context parameter"
                );
                continue;
            }
            other => panic!("Unsupported extracted loot parameter type: {other}"),
        };
        imports.insert("ContextKey");
        imports.insert(family);
        let family = Ident::new(family, Span::call_site());
        parameters.extend(quote! { pub const #name: ContextKey<#family> = ContextKey::new(#key); });
        assert!(
            parameter_ids
                .insert(parameter.key.clone(), quote! { params::#name.id() })
                .is_none(),
            "Duplicate context parameter"
        );
    }
    let mut sets = TokenStream::new();
    let mut registrations = TokenStream::new();
    let mut keys = FxHashSet::default();
    for set in &extracted.sets {
        assert!(keys.insert(&set.key), "Duplicate context set key");
        let allowed: FxHashSet<_> = set.allowed.iter().collect();
        assert_eq!(
            allowed.len(),
            set.allowed.len(),
            "Duplicate allowed parameter"
        );
        let required: FxHashSet<_> = set.required.iter().collect();
        assert_eq!(
            required.len(),
            set.required.len(),
            "Duplicate required parameter"
        );
        assert!(
            required.is_subset(&allowed),
            "Required parameters must be allowed"
        );
        let required: Vec<_> = set
            .required
            .iter()
            .map(|key| {
                let Some(parameter) = parameter_ids.get(key) else {
                    panic!("Unknown required parameter: {key}")
                };
                parameter
            })
            .collect();
        let allowed: Vec<_> = set
            .allowed
            .iter()
            .map(|key| {
                let Some(parameter) = parameter_ids.get(key) else {
                    panic!("Unknown allowed parameter: {key}")
                };
                parameter
            })
            .collect();
        let name = Ident::new(&set.key.path.to_shouty_snake_case(), Span::call_site());
        let key = generate_identifier(&set.key);
        sets.extend(quote! { pub static #name: ContextKeySet = ContextKeySet::new(#key, &[#(#required),*], &[#(#allowed),*]); });
        registrations.extend(quote! { registry.register(&#name); });
    }
    let mut imports = imports.into_iter().collect::<Vec<_>>();
    imports.sort_unstable();
    let imports = imports
        .into_iter()
        .map(|name| Ident::new(name, Span::call_site()));
    quote! {
        use std::borrow::Cow;
        use vecta_utils::Identifier;
        use crate::loot_table::{ContextKeySet, ContextKeySetRegistry};
        pub mod params {
            use crate::loot_table::{#(#imports),*};
            #parameters
        }
        #sets
        pub fn register_context_key_sets(registry: &mut ContextKeySetRegistry) { #registrations }
    }
}

//! Compiles the extracted `CONTEXT_FLOAT_PROVIDER` / `CONTEXT_INT_PROVIDER`
//! configured values into typed Rust.
//!
//! The extractor ran the real `VanillaRegistries` bootstrap through
//! `RegistryOps`, so each payload below is the codec shape of a registered
//! provider. Unknown payload types panic at build time: a configured value is
//! never replaced by a guessed constant.

use crate::generator_functions::read_json_asset;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExtractedProviders {
    minecraft_version: String,
    float_values: Vec<Entry>,
    int_values: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    key: String,
    value: Value,
}

/// Which provider family a payload belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
    Float,
    Int,
}

struct Generator {
    family: Family,
    /// Unique prefix so two configured values cannot collide on a static name.
    prefix: String,
    /// Emitted `static` property tables for inline `match_block` conditions.
    statics: Vec<TokenStream>,
    static_index: usize,
}

pub(crate) fn build() -> TokenStream {
    const ASSET: &str = "build_assets/context_number_provider_values.json";
    let extracted: ExtractedProviders = read_json_asset(ASSET);
    assert_eq!(
        Some(extracted.minecraft_version.as_str()),
        env!("CARGO_PKG_VERSION")
            .split_once("+mc")
            .map(|(_, version)| version),
        "Number provider extractor version mismatch"
    );

    let mut float_registrations = TokenStream::new();
    for (index, entry) in extracted.float_values.iter().enumerate() {
        let mut generator = Generator {
            family: Family::Float,
            prefix: format!("FLOAT_{index}"),
            statics: Vec::new(),
            static_index: 0,
        };
        let key = identifier_tokens(&entry.key);
        let provider = generator.provider(&entry.value);
        let statics = generator.statics_tokens();
        float_registrations.extend(quote! {
            #statics
            registry.register(#key, #provider);
        });
    }

    let mut int_registrations = TokenStream::new();
    for (index, entry) in extracted.int_values.iter().enumerate() {
        let mut generator = Generator {
            family: Family::Int,
            prefix: format!("INT_{index}"),
            statics: Vec::new(),
            static_index: 0,
        };
        let key = identifier_tokens(&entry.key);
        let provider = generator.provider(&entry.value);
        let statics = generator.statics_tokens();
        int_registrations.extend(quote! {
            #statics
            registry.register(#key, #provider);
        });
    }

    let float_count = extracted.float_values.len();
    let int_count = extracted.int_values.len();
    quote! {
        //! Generated from `build_assets/context_number_provider_values.json`.

        use crate::loot_table::{
            ConditionRef, FloatProvider, FloatProviderRegistry, IntProvider, IntProviderRegistry,
            LootCondition, ProviderRef, PropertyCheck, WeightedList, WeightedValue,
        };
        use vecta_utils::Identifier;

        /// Registers every configured `minecraft:context_float_provider`.
        pub fn register_context_float_providers(registry: &mut FloatProviderRegistry) {
            #float_registrations
        }

        /// Registers every configured `minecraft:context_int_provider`.
        pub fn register_context_int_providers(registry: &mut IntProviderRegistry) {
            #int_registrations
        }

        /// Number of configured float providers compiled into this build.
        pub const FLOAT_PROVIDER_COUNT: usize = #float_count;

        /// Number of configured int providers compiled into this build.
        pub const INT_PROVIDER_COUNT: usize = #int_count;
    }
}

fn identifier_tokens(value: &str) -> TokenStream {
    let (namespace, path) = value
        .split_once(':')
        .unwrap_or_else(|| panic!("number provider key without a namespace: {value}"));
    quote! { Identifier::new_static(#namespace, #path) }
}

impl Generator {
    fn statics_tokens(&self) -> TokenStream {
        let statics = self.statics.iter();
        quote! { #(#statics)* }
    }

    /// Builds a bare provider value (the registered entry itself).
    fn provider(&mut self, value: &Value) -> TokenStream {
        match value {
            Value::Number(number) => self.constant(number),
            Value::Object(_) => self.object(value),
            other => panic!("expected an inline number provider payload, got {other}"),
        }
    }

    /// Builds a provider slot: a registry key or an inline payload.
    fn provider_ref(&mut self, value: &Value) -> TokenStream {
        match value {
            Value::String(reference) => {
                let key = identifier_tokens(reference);
                quote! { ProviderRef::Reference(#key) }
            }
            other => {
                let provider = self.provider(other);
                quote! { ProviderRef::inline(#provider) }
            }
        }
    }

    /// Builds a list of provider slots (`inputs`).
    fn provider_refs(&mut self, value: &Value) -> TokenStream {
        let list = value
            .as_array()
            .unwrap_or_else(|| panic!("expected a provider list, got {value}"));
        let mut refs = TokenStream::new();
        for element in list {
            let element = self.provider_ref(element);
            refs.extend(quote! { #element, });
        }
        quote! { vec![#refs] }
    }

    fn constant(&self, number: &serde_json::Number) -> TokenStream {
        match self.family {
            Family::Float => {
                let value = number
                    .as_f64()
                    .unwrap_or_else(|| panic!("float provider payload is not numeric: {number}"));
                let value = value as f32;
                quote! { FloatProvider::Constant(#value) }
            }
            Family::Int => {
                let value = number
                    .as_i64()
                    .unwrap_or_else(|| panic!("int provider payload is not an integer: {number}"));
                let value = i32::try_from(value)
                    .unwrap_or_else(|_| panic!("int provider payload out of range: {value}"));
                quote! { IntProvider::Constant(#value) }
            }
        }
    }

    fn object(&mut self, value: &Value) -> TokenStream {
        let provider_type = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("number provider payload without a type: {value}"));
        match (provider_type, self.family) {
            ("minecraft:constant", _) => {
                let inner = value
                    .get("value")
                    .unwrap_or_else(|| panic!("constant provider without a value: {value}"));
                let number = inner
                    .as_number()
                    .unwrap_or_else(|| panic!("constant provider value is not numeric: {inner}"));
                self.constant(number)
            }
            ("minecraft:conditional", Family::Float) => self.conditional(value, |condition, on_true, on_false| {
                quote! { FloatProvider::Conditional { condition: #condition, on_true: #on_true, on_false: #on_false } }
            }),
            ("minecraft:conditional", Family::Int) => self.conditional(value, |condition, on_true, on_false| {
                quote! { IntProvider::Conditional { condition: #condition, on_true: #on_true, on_false: #on_false } }
            }),
            ("minecraft:number_dispatcher", Family::Float) => self.dispatcher(value, |cases, default| {
                quote! { FloatProvider::NumberDispatcher { cases: #cases, default: #default } }
            }),
            ("minecraft:number_dispatcher", Family::Int) => self.dispatcher(value, |cases, default| {
                quote! { IntProvider::NumberDispatcher { cases: #cases, default: #default } }
            }),
            ("minecraft:weighted_list", Family::Float) => self.weighted(value, |entries| {
                quote! { FloatProvider::WeightedList(WeightedList::new(#entries)) }
            }),
            ("minecraft:weighted_list", Family::Int) => self.weighted(value, |entries| {
                quote! { IntProvider::WeightedList(WeightedList::new(#entries)) }
            }),
            ("minecraft:add", Family::Float) => self.aggregate(value, |inputs| {
                quote! { FloatProvider::Add(#inputs) }
            }),
            ("minecraft:add", Family::Int) => self.aggregate(value, |inputs| {
                quote! { IntProvider::Add(#inputs) }
            }),
            ("minecraft:avg", Family::Float) => self.aggregate(value, |inputs| {
                quote! { FloatProvider::Average(#inputs) }
            }),
            ("minecraft:avg", Family::Int) => self.aggregate(value, |inputs| {
                quote! { IntProvider::Average(#inputs) }
            }),
            ("minecraft:mul", Family::Float) => self.aggregate(value, |inputs| {
                quote! { FloatProvider::Mul(#inputs) }
            }),
            ("minecraft:mul", Family::Int) => self.aggregate(value, |inputs| {
                quote! { IntProvider::Mul(#inputs) }
            }),
            ("minecraft:max", Family::Float) => self.aggregate(value, |inputs| {
                quote! { FloatProvider::Max(#inputs) }
            }),
            ("minecraft:max", Family::Int) => self.aggregate(value, |inputs| {
                quote! { IntProvider::Max(#inputs) }
            }),
            ("minecraft:min", Family::Float) => self.aggregate(value, |inputs| {
                quote! { FloatProvider::Min(#inputs) }
            }),
            ("minecraft:min", Family::Int) => self.aggregate(value, |inputs| {
                quote! { IntProvider::Min(#inputs) }
            }),
            ("minecraft:div", Family::Float) => self.binary(value, |left, right| {
                quote! { FloatProvider::Div(#left, #right) }
            }),
            ("minecraft:div", Family::Int) => self.binary(value, |left, right| {
                quote! { IntProvider::Div(#left, #right) }
            }),
            (other, _) => panic!("unsupported configured number provider type: {other}"),
        }
    }

    fn conditional(
        &mut self,
        value: &Value,
        build: impl FnOnce(TokenStream, TokenStream, TokenStream) -> TokenStream,
    ) -> TokenStream {
        let condition = value
            .get("condition")
            .unwrap_or_else(|| panic!("conditional provider without a condition: {value}"));
        let condition = self.condition(condition);
        let on_true = value
            .get("on_true")
            .unwrap_or_else(|| panic!("conditional provider without on_true: {value}"));
        let on_true = self.provider_ref(on_true);
        let on_false = value.get("on_false");
        let on_false = match on_false {
            Some(on_false) => self.provider_ref(on_false),
            // `ContextFloatProviders.exactly(0.0F)` / `ContextIntProviders.exactly(0)`.
            None => self.default_zero(),
        };
        build(condition, on_true, on_false)
    }

    fn default_zero(&self) -> TokenStream {
        match self.family {
            Family::Float => quote! { ProviderRef::inline(FloatProvider::Constant(0.0)) },
            Family::Int => quote! { ProviderRef::inline(IntProvider::Constant(0)) },
        }
    }

    fn dispatcher(
        &mut self,
        value: &Value,
        build: impl FnOnce(TokenStream, TokenStream) -> TokenStream,
    ) -> TokenStream {
        let cases = value
            .get("cases")
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("number_dispatcher without cases: {value}"));
        let mut case_tokens = TokenStream::new();
        for case in cases {
            let condition = case
                .get("condition")
                .unwrap_or_else(|| panic!("dispatcher case without a condition: {case}"));
            let condition = self.condition(condition);
            let case_value = case
                .get("value")
                .unwrap_or_else(|| panic!("dispatcher case without a value: {case}"));
            let case_value = self.provider_ref(case_value);
            case_tokens.extend(quote! { (#condition, #case_value), });
        }
        let default = value.get("default");
        let default = match default {
            Some(default) => self.provider_ref(default),
            None => self.default_zero(),
        };
        build(quote! { vec![#case_tokens] }, default)
    }

    fn weighted(
        &mut self,
        value: &Value,
        build: impl FnOnce(TokenStream) -> TokenStream,
    ) -> TokenStream {
        let distribution = value
            .get("distribution")
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("weighted_list without a distribution: {value}"));
        let mut entries = TokenStream::new();
        for entry in distribution {
            let weight = entry
                .get("weight")
                .and_then(Value::as_i64)
                .unwrap_or_else(|| panic!("weighted entry without a weight: {entry}"));
            let weight = i32::try_from(weight)
                .unwrap_or_else(|_| panic!("weighted entry weight out of range: {weight}"));
            let data = entry
                .get("data")
                .unwrap_or_else(|| panic!("weighted entry without data: {entry}"));
            let data = self.provider_ref(data);
            entries.extend(quote! { WeightedValue::new(#data, #weight), });
        }
        build(quote! { vec![#entries] })
    }

    fn aggregate(
        &mut self,
        value: &Value,
        build: impl FnOnce(TokenStream) -> TokenStream,
    ) -> TokenStream {
        let inputs = value
            .get("inputs")
            .unwrap_or_else(|| panic!("aggregate provider without inputs: {value}"));
        let inputs = self.provider_refs(inputs);
        build(inputs)
    }

    fn binary(
        &mut self,
        value: &Value,
        build: impl FnOnce(TokenStream, TokenStream) -> TokenStream,
    ) -> TokenStream {
        let left = value
            .get("left")
            .unwrap_or_else(|| panic!("binary provider without left: {value}"));
        let left = self.provider_ref(left);
        let right = value
            .get("right")
            .unwrap_or_else(|| panic!("binary provider without right: {value}"));
        let right = self.provider_ref(right);
        build(left, right)
    }

    fn condition(&mut self, value: &Value) -> TokenStream {
        match value {
            Value::String(reference) => {
                let key = identifier_tokens(reference);
                quote! { ConditionRef::Reference(#key) }
            }
            Value::Object(_) => {
                let condition = self.inline_condition(value);
                quote! { ConditionRef::Inline(Box::new(#condition)) }
            }
            other => panic!("unsupported condition payload: {other}"),
        }
    }

    fn inline_condition(&mut self, value: &Value) -> TokenStream {
        let condition_type = value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("condition payload without a type: {value}"));
        match condition_type {
            "minecraft:match_block" => {
                let blocks = value
                    .get("blocks")
                    .unwrap_or_else(|| panic!("match_block without blocks: {value}"));
                let block = blocks
                    .as_str()
                    .unwrap_or_else(|| panic!("match_block blocks must be one id here: {value}"));
                let block = identifier_tokens(block);
                let properties = self.property_table(value.get("state"));
                quote! { LootCondition::MatchBlock { block: #block, properties: #properties } }
            }
            other => panic!("unsupported inline condition type: {other}"),
        }
    }

    fn property_table(&mut self, state: Option<&Value>) -> TokenStream {
        let Some(state) = state else {
            return quote! { &[] };
        };
        let state = state
            .as_object()
            .unwrap_or_else(|| panic!("match_block state must be an object: {state}"));
        let mut entries = TokenStream::new();
        for (name, value) in state {
            let value = value
                .as_str()
                .unwrap_or_else(|| panic!("match_block property value must be a string: {value}"));
            entries.extend(quote! { PropertyCheck { name: #name, value: #value }, });
        }
        let ident = Ident::new(
            &format!("{}_{}", self.prefix, self.static_index),
            Span::call_site(),
        );
        self.static_index += 1;
        self.statics.push(quote! {
            static #ident: &[PropertyCheck] = &[#entries];
        });
        quote! { #ident }
    }
}

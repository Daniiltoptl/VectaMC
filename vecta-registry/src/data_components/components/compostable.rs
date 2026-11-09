//! Vanilla `minecraft:compostable` item component.

use std::io::{Cursor, Result, Write};

use simdnbt::owned::{NbtCompound, NbtTag};
use simdnbt::{FromNbtTag, ToNbtTag};
use vecta_utils::Identifier;
use vecta_utils::hash::{ComponentHasher, HashComponent, HashEntry};
use vecta_utils::serial::{ReadFrom, WriteTo};

use super::resolvable::ResolvableInt;

/// How many compost layers an item adds: an inline constant or a reference
/// to a `minecraft:context_int_provider`, mirroring the vanilla
/// `ResolvableInt` field of `net.minecraft.world.item.component.Compostable`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compostable {
    pub layers: ResolvableInt,
}

impl Compostable {
    /// Vanilla `Compostable(ResourceKey<ContextIntProvider> layers)`.
    /// The generated item data uses the reference form.
    #[must_use]
    pub const fn new(layers: Identifier) -> Self {
        Self {
            layers: ResolvableInt::from_key(layers),
        }
    }

    /// Vanilla `new Compostable(ResolvableInt.Constant(value))` for inline
    /// constants written by datapacks or commands.
    #[must_use]
    pub const fn constant_layers(layers: i32) -> Self {
        Self {
            layers: ResolvableInt::Constant(layers),
        }
    }
}

impl WriteTo for Compostable {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.layers.write(writer)
    }
}

impl ReadFrom for Compostable {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        Ok(Self {
            layers: ResolvableInt::read(data)?,
        })
    }
}

impl ToNbtTag for Compostable {
    fn to_nbt_tag(self) -> NbtTag {
        let mut compound = NbtCompound::new();
        compound.insert("layers", self.layers.to_nbt_tag());
        NbtTag::Compound(compound)
    }
}

impl FromNbtTag for Compostable {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag) -> Option<Self> {
        let compound = tag.compound()?;
        let layers = ResolvableInt::from_nbt_tag(compound.get("layers")?)?;
        Some(Self { layers })
    }
}

impl HashComponent for Compostable {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        let mut key_hasher = ComponentHasher::new();
        key_hasher.put_string("layers");
        let mut value_hasher = ComponentHasher::new();
        self.layers.hash_component(&mut value_hasher);
        let entry = HashEntry::new(key_hasher, value_hasher);

        hasher.start_map();
        hasher.put_raw_bytes(&entry.key_bytes);
        hasher.put_raw_bytes(&entry.value_bytes);
        hasher.end_map();
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use simdnbt::FromNbtTag as _;
    use simdnbt::ToNbtTag as _;
    use simdnbt::borrow::read_tag;
    use simdnbt::owned::{NbtCompound, NbtTag};
    use vecta_utils::Identifier;
    use vecta_utils::serial::{ReadFrom as _, WriteTo as _};

    use super::Compostable;
    use crate::data_components::components::resolvable::ResolvableInt;

    fn parse(tag: NbtTag) -> Option<Compostable> {
        let mut bytes = Vec::new();
        tag.write(&mut bytes);
        let borrowed = read_tag(&mut Cursor::new(bytes.as_slice())).ok()?;
        Compostable::from_nbt_tag(borrowed.as_tag())
    }

    #[test]
    fn round_trips_both_codecs() {
        let value = Compostable::new(Identifier::vanilla_static("compostable/low"));

        let nbt = value.clone().to_nbt_tag();
        assert_eq!(parse(nbt), Some(value.clone()));

        let mut network = Vec::new();
        value
            .write(&mut network)
            .expect("compostable should encode");
        assert_eq!(
            Compostable::read(&mut Cursor::new(network.as_slice()))
                .expect("compostable should decode"),
            value
        );
    }

    #[test]
    fn inline_constant_round_trips_both_codecs() {
        let value = Compostable::constant_layers(3);

        // NBT: `layers` is a plain int tag.
        let NbtTag::Compound(compound) = value.clone().to_nbt_tag() else {
            panic!("compostable NBT is a compound");
        };
        assert_eq!(compound.get("layers"), Some(&NbtTag::Int(3)));
        assert_eq!(parse(NbtTag::Compound(compound)), Some(value.clone()));

        // Wire: bool prefix, then the big-endian int.
        let mut network = Vec::new();
        value
            .write(&mut network)
            .expect("compostable should encode");
        assert_eq!(network[0], 1);
        assert_eq!(&network[1..], &3i32.to_be_bytes());
        assert_eq!(
            Compostable::read(&mut Cursor::new(network.as_slice()))
                .expect("compostable should decode"),
            value
        );
    }

    #[test]
    fn parses_a_datapack_constant_form() {
        let mut compound = NbtCompound::new();
        compound.insert("layers", 5);
        let parsed = parse(NbtTag::Compound(compound)).expect("constant form should parse");
        assert_eq!(parsed.layers, ResolvableInt::Constant(5));
    }
}

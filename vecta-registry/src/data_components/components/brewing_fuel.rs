//! Vanilla `minecraft:brewing_fuel` item component.

use std::io::{Cursor, Result, Write};

use simdnbt::owned::{NbtCompound, NbtTag};
use simdnbt::{FromNbtTag, ToNbtTag};
use vecta_utils::Identifier;
use vecta_utils::hash::{ComponentHasher, HashComponent, HashEntry, sort_map_entries};
use vecta_utils::serial::{ReadFrom, WriteTo};

use super::resolvable::{ResolvableFloat, ResolvableInt};

/// Number of uses and brewing-speed multiplier of a brewing fuel item:
/// inline constants or references to `minecraft:context_int_provider` /
/// `minecraft:context_float_provider`, mirroring the vanilla
/// `ResolvableInt`/`ResolvableFloat` fields of
/// `net.minecraft.world.item.component.BrewingFuel`.
#[derive(Debug, Clone, PartialEq)]
pub struct BrewingFuel {
    pub uses: ResolvableInt,
    pub speed_multiplier: ResolvableFloat,
}

impl BrewingFuel {
    /// Vanilla `BrewingFuel(ResourceKey, ResourceKey)`. The generated item
    /// data uses the reference form.
    #[must_use]
    pub const fn new(uses: Identifier, speed_multiplier: Identifier) -> Self {
        Self {
            uses: ResolvableInt::from_key(uses),
            speed_multiplier: ResolvableFloat::from_key(speed_multiplier),
        }
    }

    /// Vanilla inline-constant constructor for datapacks and commands.
    #[must_use]
    pub const fn constant(uses: i32, speed_multiplier: f32) -> Self {
        Self {
            uses: ResolvableInt::Constant(uses),
            speed_multiplier: ResolvableFloat::Constant(speed_multiplier),
        }
    }
}

impl WriteTo for BrewingFuel {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.uses.write(writer)?;
        self.speed_multiplier.write(writer)
    }
}

impl ReadFrom for BrewingFuel {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        Ok(Self {
            uses: ResolvableInt::read(data)?,
            speed_multiplier: ResolvableFloat::read(data)?,
        })
    }
}

impl ToNbtTag for BrewingFuel {
    fn to_nbt_tag(self) -> NbtTag {
        let mut compound = NbtCompound::new();
        compound.insert("uses", self.uses.to_nbt_tag());
        compound.insert("speed_multiplier", self.speed_multiplier.to_nbt_tag());
        NbtTag::Compound(compound)
    }
}

impl FromNbtTag for BrewingFuel {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag) -> Option<Self> {
        let compound = tag.compound()?;
        let uses = ResolvableInt::from_nbt_tag(compound.get("uses")?)?;
        let speed_multiplier = ResolvableFloat::from_nbt_tag(compound.get("speed_multiplier")?)?;
        Some(Self {
            uses,
            speed_multiplier,
        })
    }
}

impl HashComponent for BrewingFuel {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        let mut entries = Vec::with_capacity(2);
        push_hash_entry(&mut entries, "uses", &self.uses);
        push_hash_entry(&mut entries, "speed_multiplier", &self.speed_multiplier);
        sort_map_entries(&mut entries);
        hasher.start_map();
        for entry in entries {
            hasher.put_raw_bytes(&entry.key_bytes);
            hasher.put_raw_bytes(&entry.value_bytes);
        }
        hasher.end_map();
    }
}

fn push_hash_entry<T: HashComponent + ?Sized>(entries: &mut Vec<HashEntry>, key: &str, value: &T) {
    let mut key_hasher = ComponentHasher::new();
    key_hasher.put_string(key);
    let mut value_hasher = ComponentHasher::new();
    value.hash_component(&mut value_hasher);
    entries.push(HashEntry::new(key_hasher, value_hasher));
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

    use super::BrewingFuel;
    use crate::data_components::components::resolvable::{ResolvableFloat, ResolvableInt};

    fn parse(tag: NbtTag) -> Option<BrewingFuel> {
        let mut bytes = Vec::new();
        tag.write(&mut bytes);
        let borrowed = read_tag(&mut Cursor::new(bytes.as_slice())).ok()?;
        BrewingFuel::from_nbt_tag(borrowed.as_tag())
    }

    #[test]
    fn round_trips_both_codecs() {
        let value = BrewingFuel::new(
            Identifier::vanilla_static("brewing/uses_default"),
            Identifier::vanilla_static("brewing/speed_default"),
        );

        let nbt = value.clone().to_nbt_tag();
        assert_eq!(parse(nbt), Some(value.clone()));

        let mut network = Vec::new();
        value.write(&mut network).expect("fuel should encode");
        assert_eq!(
            BrewingFuel::read(&mut Cursor::new(network.as_slice())).expect("fuel should decode"),
            value
        );
    }

    #[test]
    fn inline_constants_round_trip_both_codecs() {
        let value = BrewingFuel::constant(20, 1.0);

        let NbtTag::Compound(compound) = value.clone().to_nbt_tag() else {
            panic!("brewing fuel NBT is a compound");
        };
        assert_eq!(compound.get("uses"), Some(&NbtTag::Int(20)));
        assert_eq!(compound.get("speed_multiplier"), Some(&NbtTag::Float(1.0)));
        assert_eq!(parse(NbtTag::Compound(compound)), Some(value.clone()));

        let mut network = Vec::new();
        value.write(&mut network).expect("fuel should encode");
        assert_eq!(
            BrewingFuel::read(&mut Cursor::new(network.as_slice())).expect("fuel should decode"),
            value
        );
    }

    #[test]
    fn parses_a_mixed_datapack_form() {
        let mut compound = NbtCompound::new();
        compound.insert("uses", 40);
        compound.insert("speed_multiplier", "minecraft:brewing/speed_default");
        let parsed = parse(NbtTag::Compound(compound)).expect("mixed form should parse");
        assert_eq!(parsed.uses, ResolvableInt::Constant(40));
        assert_eq!(
            parsed.speed_multiplier,
            ResolvableFloat::from_key(Identifier::vanilla_static("brewing/speed_default"))
        );
    }
}

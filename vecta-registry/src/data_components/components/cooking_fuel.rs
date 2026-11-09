//! Vanilla `minecraft:cooking_fuel` item component.

use std::io::{Cursor, Result, Write};

use simdnbt::owned::{NbtCompound, NbtTag};
use simdnbt::{FromNbtTag, ToNbtTag};
use vecta_utils::Identifier;
use vecta_utils::hash::{ComponentHasher, HashComponent, HashEntry, sort_map_entries};
use vecta_utils::serial::{ReadFrom, WriteTo};

use super::resolvable::{ResolvableFloat, ResolvableInt};

/// Burn time and cooking-speed multiplier of a fuel item: inline constants
/// or references to `minecraft:context_int_provider` /
/// `minecraft:context_float_provider`, mirroring the vanilla
/// `ResolvableInt`/`ResolvableFloat` fields of
/// `net.minecraft.world.item.component.CookingFuel`.
#[derive(Debug, Clone, PartialEq)]
pub struct CookingFuel {
    pub burn_time: ResolvableInt,
    pub speed_multiplier: ResolvableFloat,
}

impl CookingFuel {
    /// Vanilla `CookingFuel(ResourceKey, ResourceKey)`. The generated item
    /// data uses the reference form.
    #[must_use]
    pub const fn new(burn_time: Identifier, speed_multiplier: Identifier) -> Self {
        Self {
            burn_time: ResolvableInt::from_key(burn_time),
            speed_multiplier: ResolvableFloat::from_key(speed_multiplier),
        }
    }

    /// Vanilla inline-constant constructor for datapacks and commands.
    #[must_use]
    pub const fn constant(burn_time: i32, speed_multiplier: f32) -> Self {
        Self {
            burn_time: ResolvableInt::Constant(burn_time),
            speed_multiplier: ResolvableFloat::Constant(speed_multiplier),
        }
    }
}

impl WriteTo for CookingFuel {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        self.burn_time.write(writer)?;
        self.speed_multiplier.write(writer)
    }
}

impl ReadFrom for CookingFuel {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        Ok(Self {
            burn_time: ResolvableInt::read(data)?,
            speed_multiplier: ResolvableFloat::read(data)?,
        })
    }
}

impl ToNbtTag for CookingFuel {
    fn to_nbt_tag(self) -> NbtTag {
        let mut compound = NbtCompound::new();
        compound.insert("burn_time", self.burn_time.to_nbt_tag());
        compound.insert("speed_multiplier", self.speed_multiplier.to_nbt_tag());
        NbtTag::Compound(compound)
    }
}

impl FromNbtTag for CookingFuel {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag) -> Option<Self> {
        let compound = tag.compound()?;
        let burn_time = ResolvableInt::from_nbt_tag(compound.get("burn_time")?)?;
        let speed_multiplier = ResolvableFloat::from_nbt_tag(compound.get("speed_multiplier")?)?;
        Some(Self {
            burn_time,
            speed_multiplier,
        })
    }
}

impl HashComponent for CookingFuel {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        let mut entries = Vec::with_capacity(2);
        push_hash_entry(&mut entries, "burn_time", &self.burn_time);
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

    use super::CookingFuel;
    use crate::data_components::components::resolvable::{ResolvableFloat, ResolvableInt};

    fn parse(tag: NbtTag) -> Option<CookingFuel> {
        let mut bytes = Vec::new();
        tag.write(&mut bytes);
        let borrowed = read_tag(&mut Cursor::new(bytes.as_slice())).ok()?;
        CookingFuel::from_nbt_tag(borrowed.as_tag())
    }

    #[test]
    fn round_trips_both_codecs() {
        let value = CookingFuel::new(
            Identifier::vanilla_static("cooking/time_wood_blocks"),
            Identifier::vanilla_static("cooking/speed_default"),
        );

        let nbt = value.clone().to_nbt_tag();
        assert_eq!(parse(nbt), Some(value.clone()));

        let mut network = Vec::new();
        value.write(&mut network).expect("fuel should encode");
        assert_eq!(
            CookingFuel::read(&mut Cursor::new(network.as_slice())).expect("fuel should decode"),
            value
        );
    }

    #[test]
    fn inline_constants_round_trip_both_codecs() {
        let value = CookingFuel::constant(200, 2.0);

        // NBT: numeric tags for both fields.
        let NbtTag::Compound(compound) = value.clone().to_nbt_tag() else {
            panic!("cooking fuel NBT is a compound");
        };
        assert_eq!(compound.get("burn_time"), Some(&NbtTag::Int(200)));
        assert_eq!(compound.get("speed_multiplier"), Some(&NbtTag::Float(2.0)));
        assert_eq!(parse(NbtTag::Compound(compound)), Some(value.clone()));

        // Wire: bool prefix `true`, big-endian int, bool prefix `true`, f32.
        let mut network = Vec::new();
        value.write(&mut network).expect("fuel should encode");
        assert_eq!(network[0], 1);
        assert_eq!(&network[1..5], &200i32.to_be_bytes());
        assert_eq!(network[5], 1);
        assert_eq!(&network[6..], &2.0f32.to_be_bytes());
        assert_eq!(
            CookingFuel::read(&mut Cursor::new(network.as_slice())).expect("fuel should decode"),
            value
        );
    }

    #[test]
    fn parses_a_mixed_datapack_form() {
        // Vanilla allows mixing: an inline constant burn time with a
        // referenced speed multiplier.
        let mut compound = NbtCompound::new();
        compound.insert("burn_time", 100);
        compound.insert("speed_multiplier", "minecraft:cooking/speed_default");
        let parsed = parse(NbtTag::Compound(compound)).expect("mixed form should parse");
        assert_eq!(parsed.burn_time, ResolvableInt::Constant(100));
        assert_eq!(
            parsed.speed_multiplier,
            ResolvableFloat::from_key(Identifier::vanilla_static("cooking/speed_default"))
        );
    }
}

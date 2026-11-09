//! Vanilla `ResolvableInt` and `ResolvableFloat`
//! (`net.minecraft.world.level.storage.loot.providers.number.ints.ResolvableInt`,
//! `...floats.ResolvableFloat`).
//!
//! A resolvable number is either an inline constant or a reference to a
//! configured `minecraft:context_int_provider` / `minecraft:context_float_provider`.
//! Wire form: a boolean picks the branch (`true` = constant, `false` =
//! reference), matching `ByteBufCodecs.either`; the constant integer is a
//! big-endian `int` (`ByteBufCodecs.INT`), the constant float a big-endian
//! `float` (`ByteBufCodecs.FLOAT`). NBT/JSON form: a plain number or a
//! resource-key string.

use std::io::{Cursor, Result, Write};

use simdnbt::owned::NbtTag;
use simdnbt::{FromNbtTag, ToNbtTag};
use vecta_utils::Identifier;
use vecta_utils::hash::{ComponentHasher, HashComponent};
use vecta_utils::nbt::NbtNumeric as _;
use vecta_utils::serial::{ReadFrom, WriteTo};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvableInt {
    /// Inline `ResolvableInt.Constant(value)`.
    Constant(i32),
    /// `ResolvableInt.Reference(key)` into `minecraft:context_int_provider`.
    Reference(Identifier),
}

impl ResolvableInt {
    /// Vanilla `ResolvableInt.fromKey(key)`.
    #[must_use]
    pub const fn from_key(key: Identifier) -> Self {
        Self::Reference(key)
    }

    /// Vanilla `get(context, defaultValue)`: a constant returns itself; a
    /// reference falls back to `default` when the provider is not registered.
    #[must_use]
    pub fn get_or_default(
        &self,
        lookup: impl FnOnce(&Identifier) -> Option<i32>,
        default: i32,
    ) -> i32 {
        match self {
            Self::Constant(value) => *value,
            Self::Reference(key) => lookup(key).unwrap_or(default),
        }
    }
}

impl WriteTo for ResolvableInt {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        match self {
            Self::Constant(value) => {
                true.write(writer)?;
                value.write(writer)
            }
            Self::Reference(key) => {
                false.write(writer)?;
                key.write(writer)
            }
        }
    }
}

impl ReadFrom for ResolvableInt {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        if bool::read(data)? {
            Ok(Self::Constant(i32::read(data)?))
        } else {
            Ok(Self::Reference(Identifier::read(data)?))
        }
    }
}

impl ToNbtTag for ResolvableInt {
    fn to_nbt_tag(self) -> NbtTag {
        match self {
            Self::Constant(value) => NbtTag::Int(value),
            Self::Reference(key) => key.to_nbt_tag(),
        }
    }
}

impl FromNbtTag for ResolvableInt {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag<'_, '_>) -> Option<Self> {
        if let Some(value) = tag.codec_i32() {
            return Some(Self::Constant(value));
        }
        let key = Identifier::from_nbt_tag(tag)?;
        Some(Self::Reference(key))
    }
}

impl HashComponent for ResolvableInt {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        match self {
            Self::Constant(value) => hasher.put_int(*value),
            Self::Reference(key) => key.hash_component(hasher),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvableFloat {
    /// Inline `ResolvableFloat.Constant(value)`.
    Constant(f32),
    /// `ResolvableFloat.Reference(key)` into `minecraft:context_float_provider`.
    Reference(Identifier),
}

impl ResolvableFloat {
    /// Vanilla `ResolvableFloat.fromKey(key)`.
    #[must_use]
    pub const fn from_key(key: Identifier) -> Self {
        Self::Reference(key)
    }

    /// Vanilla `get(context, defaultValue)`.
    #[must_use]
    pub fn get_or_default(
        &self,
        lookup: impl FnOnce(&Identifier) -> Option<f32>,
        default: f32,
    ) -> f32 {
        match self {
            Self::Constant(value) => *value,
            Self::Reference(key) => lookup(key).unwrap_or(default),
        }
    }
}

impl WriteTo for ResolvableFloat {
    fn write(&self, writer: &mut impl Write) -> Result<()> {
        match self {
            Self::Constant(value) => {
                true.write(writer)?;
                value.write(writer)
            }
            Self::Reference(key) => {
                false.write(writer)?;
                key.write(writer)
            }
        }
    }
}

impl ReadFrom for ResolvableFloat {
    fn read(data: &mut Cursor<&[u8]>) -> Result<Self> {
        if bool::read(data)? {
            Ok(Self::Constant(f32::read(data)?))
        } else {
            Ok(Self::Reference(Identifier::read(data)?))
        }
    }
}

impl ToNbtTag for ResolvableFloat {
    fn to_nbt_tag(self) -> NbtTag {
        match self {
            Self::Constant(value) => NbtTag::Float(value),
            Self::Reference(key) => key.to_nbt_tag(),
        }
    }
}

impl FromNbtTag for ResolvableFloat {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag<'_, '_>) -> Option<Self> {
        if let Some(value) = tag.codec_f32() {
            return Some(Self::Constant(value));
        }
        let key = Identifier::from_nbt_tag(tag)?;
        Some(Self::Reference(key))
    }
}

impl HashComponent for ResolvableFloat {
    fn hash_component(&self, hasher: &mut ComponentHasher) {
        match self {
            Self::Constant(value) => hasher.put_float(*value),
            Self::Reference(key) => key.hash_component(hasher),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use simdnbt::FromNbtTag as _;
    use simdnbt::ToNbtTag as _;
    use simdnbt::borrow::read_tag;
    use simdnbt::owned::NbtTag;
    use vecta_utils::Identifier;
    use vecta_utils::hash::{ComponentHasher, HashComponent as _};
    use vecta_utils::serial::{ReadFrom as _, WriteTo as _};

    use super::{ResolvableFloat, ResolvableInt};

    fn parse_int(tag: NbtTag) -> Option<ResolvableInt> {
        let mut bytes = Vec::new();
        tag.write(&mut bytes);
        let borrowed = read_tag(&mut Cursor::new(bytes.as_slice())).ok()?;
        ResolvableInt::from_nbt_tag(borrowed.as_tag())
    }

    fn parse_float(tag: NbtTag) -> Option<ResolvableFloat> {
        let mut bytes = Vec::new();
        tag.write(&mut bytes);
        let borrowed = read_tag(&mut Cursor::new(bytes.as_slice())).ok()?;
        ResolvableFloat::from_nbt_tag(borrowed.as_tag())
    }

    #[test]
    fn ints_round_trip_both_branches_over_both_codecs() {
        let constant = ResolvableInt::Constant(3);
        let reference = ResolvableInt::from_key(Identifier::vanilla_static("compostable/low"));

        for value in [constant, reference] {
            let mut network = Vec::new();
            value
                .write(&mut network)
                .expect("resolvable int should encode");
            assert_eq!(
                ResolvableInt::read(&mut Cursor::new(network.as_slice()))
                    .expect("resolvable int should decode"),
                value
            );

            let nbt = value.clone().to_nbt_tag();
            assert_eq!(parse_int(nbt), Some(value));
        }
    }

    #[test]
    fn floats_round_trip_both_branches_over_both_codecs() {
        let constant = ResolvableFloat::Constant(1.5);
        let reference = ResolvableFloat::from_key(Identifier::vanilla_static("cooking/fast"));

        for value in [constant, reference] {
            let mut network = Vec::new();
            value
                .write(&mut network)
                .expect("resolvable float should encode");
            assert_eq!(
                ResolvableFloat::read(&mut Cursor::new(network.as_slice()))
                    .expect("resolvable float should decode"),
                value
            );

            let nbt = value.clone().to_nbt_tag();
            assert_eq!(parse_float(nbt), Some(value));
        }
    }

    #[test]
    fn wire_branch_prefixes_match_byte_buf_codecs_either() {
        // `true` = left branch (constant), `false` = right branch (reference).
        let mut encoded = Vec::new();
        ResolvableInt::Constant(7)
            .write(&mut encoded)
            .expect("constant should encode");
        assert_eq!(encoded[0], 1);
        assert_eq!(&encoded[1..], &7i32.to_be_bytes());

        let mut encoded = Vec::new();
        ResolvableFloat::from_key(Identifier::vanilla_static("minecraft:x"))
            .write(&mut encoded)
            .expect("reference should encode");
        assert_eq!(encoded[0], 0);
    }

    #[test]
    fn get_or_default_falls_back_only_for_missing_references() {
        assert_eq!(ResolvableInt::Constant(5).get_or_default(|_| Some(1), 0), 5);
        assert_eq!(ResolvableInt::Constant(5).get_or_default(|_| None, 0), 5);
        let key = Identifier::vanilla_static("compostable/low");
        assert_eq!(
            ResolvableInt::from_key(key.clone()).get_or_default(|_| Some(1), 0),
            1
        );
        assert_eq!(ResolvableInt::from_key(key).get_or_default(|_| None, 0), 0);

        assert_eq!(
            ResolvableFloat::Constant(2.0).get_or_default(|_| None, 1.0),
            2.0
        );
    }

    #[test]
    fn constants_and_references_hash_differently() {
        let hash = |value: &ResolvableInt| {
            let mut hasher = ComponentHasher::new();
            value.hash_component(&mut hasher);
            hasher.finish()
        };
        let key = Identifier::vanilla_static("compostable/low");
        assert_ne!(
            hash(&ResolvableInt::Constant(1)),
            hash(&ResolvableInt::from_key(key))
        );
    }
}

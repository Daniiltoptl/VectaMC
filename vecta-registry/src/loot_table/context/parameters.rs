//! Typed, borrowed loot parameters. Key identity includes the value family, so
//! reusing a name with another type cannot overwrite or satisfy the typed key.

use std::marker::PhantomData;

use vecta_utils::{BlockStateId, Downcast as _, DowncastType, DowncastTypeKey, ErasedType};

use super::{BlockEntityRef, DamageSourceInfo, EntityRef, ItemStack};

/// Value identity is independent of process-local Rust `TypeId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextValueKind {
    Entity,
    BlockState,
    BlockEntity,
    Tool,
    Origin,
    DamageSource,
    Float,
    Integer,
    Boolean,
    Unit,
    /// Registry metadata only until a real `SlotProvider` consumer is implemented.
    Container,
    Extension(DowncastTypeKey),
}

/// An erased key usable in parameter-set validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContextKeyId {
    name: &'static str,
    kind: ContextValueKind,
}

impl ContextKeyId {
    #[must_use]
    pub const fn new(name: &'static str, kind: ContextValueKind) -> Self {
        Self { name, kind }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }
}

/// A key fixes the value's lifetime-aware Rust type at the call site.
pub struct ContextKey<T: ContextValueType> {
    id: ContextKeyId,
    value_type: PhantomData<fn() -> T>,
}

impl<T: ContextValueType> Copy for ContextKey<T> {}
impl<T: ContextValueType> Clone for ContextKey<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ContextValueType> ContextKey<T> {
    /// Extension owners must use their own namespace and reuse a single key.
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self {
            id: ContextKeyId::new(name, T::KIND),
            value_type: PhantomData,
        }
    }

    #[must_use]
    pub const fn id(self) -> ContextKeyId {
        self.id
    }
}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub enum ContextValue<'a> {
    Entity(EntityRef<'a>),
    BlockState(BlockStateId),
    BlockEntity(BlockEntityRef<'a>),
    Tool(&'a ItemStack),
    Origin((f64, f64, f64)),
    DamageSource(DamageSourceInfo<'a>),
    Float(f32),
    Integer(i32),
    Boolean(bool),
    Unit(()),
    Extension(&'a (dyn ErasedType + Send + Sync)),
}

mod private {
    pub trait Sealed {}
}

/// Built-in borrowed families and keyed extension-owned values share one map.
pub trait ContextValueType: private::Sealed {
    type Value<'a>;
    const KIND: ContextValueKind;
    #[doc(hidden)]
    fn encode(value: Self::Value<'_>) -> ContextValue<'_>;
    #[doc(hidden)]
    fn decode(value: ContextValue<'_>) -> Option<Self::Value<'_>>;
}

macro_rules! value_family {
    ($name:ident, $value:ty, $variant:ident) => {
        pub struct $name;
        impl private::Sealed for $name {}
        impl ContextValueType for $name {
            type Value<'a> = $value;
            const KIND: ContextValueKind = ContextValueKind::$variant;
            fn encode(value: Self::Value<'_>) -> ContextValue<'_> {
                ContextValue::$variant(value)
            }
            fn decode(value: ContextValue<'_>) -> Option<Self::Value<'_>> {
                if let ContextValue::$variant(value) = value {
                    Some(value)
                } else {
                    None
                }
            }
        }
    };
}

value_family!(EntityValue, EntityRef<'a>, Entity);
value_family!(BlockStateValue, BlockStateId, BlockState);
value_family!(BlockEntityValue, BlockEntityRef<'a>, BlockEntity);
value_family!(ToolValue, &'a ItemStack, Tool);
value_family!(OriginValue, (f64, f64, f64), Origin);
value_family!(DamageSourceValue, DamageSourceInfo<'a>, DamageSource);
value_family!(FloatValue, f32, Float);
value_family!(IntegerValue, i32, Integer);
value_family!(BooleanValue, bool, Boolean);
value_family!(UnitValue, (), Unit);

/// Borrows an owner's concrete value; it need not be copied into a snapshot.
pub struct ExtensionValue<T: DowncastType + Send + Sync>(PhantomData<fn() -> T>);
impl<T: DowncastType + Send + Sync> private::Sealed for ExtensionValue<T> {}
impl<T: DowncastType + Send + Sync> ContextValueType for ExtensionValue<T> {
    type Value<'a> = &'a T;
    const KIND: ContextValueKind = ContextValueKind::Extension(T::TYPE_KEY);
    fn encode(value: Self::Value<'_>) -> ContextValue<'_> {
        ContextValue::Extension(value)
    }
    fn decode(value: ContextValue<'_>) -> Option<Self::Value<'_>> {
        let ContextValue::Extension(value) = value else {
            return None;
        };
        value.downcast_ref::<T>()
    }
}

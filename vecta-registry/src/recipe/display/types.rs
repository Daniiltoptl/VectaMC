use std::fmt::{self, Debug, Formatter};
use std::io::{Cursor, Error, Result, Write};

use rustc_hash::FxHashMap;
use vecta_utils::{Downcast as _, DowncastType, DowncastTypeKey, ErasedType, Identifier};

use super::{SlotDisplay, SlotDisplayKind};

/// Payload codec for an extension-owned slot display implementation.
pub trait SlotDisplayCodec:
    DowncastType + Clone + Debug + PartialEq + Send + Sync + 'static
{
    fn read_network(data: &mut Cursor<&[u8]>) -> Result<Self>;
    fn write_network(&self, writer: &mut Vec<u8>) -> Result<()>;
}

trait ErasedSlotDisplay: ErasedType + Debug + Send + Sync {
    fn clone_display(&self) -> Box<dyn ErasedSlotDisplay>;
    fn display_eq(&self, other: &dyn ErasedSlotDisplay) -> bool;
}

impl<T: SlotDisplayCodec> ErasedSlotDisplay for T {
    fn clone_display(&self) -> Box<dyn ErasedSlotDisplay> {
        Box::new(self.clone())
    }

    fn display_eq(&self, other: &dyn ErasedSlotDisplay) -> bool {
        other.downcast_ref::<T>() == Some(self)
    }
}

type NetworkReader = fn(&mut Cursor<&[u8]>) -> Result<Box<dyn ErasedSlotDisplay>>;
type NetworkWriter = fn(&dyn ErasedSlotDisplay, &mut Vec<u8>) -> Result<()>;

enum PayloadCodec {
    Vanilla(SlotDisplayKind),
    Custom {
        type_key: DowncastTypeKey,
        read: NetworkReader,
        write: NetworkWriter,
    },
}

/// Registered slot display type. Its numeric ID is assigned by the registry,
/// independently of the Rust payload kind and of extension-owned type keys.
pub struct SlotDisplayType {
    pub key: Identifier,
    codec: PayloadCodec,
}

impl SlotDisplayType {
    pub(crate) const fn vanilla(key: Identifier, kind: SlotDisplayKind) -> Self {
        Self {
            key,
            codec: PayloadCodec::Vanilla(kind),
        }
    }

    #[must_use]
    pub const fn of<T: SlotDisplayCodec>(key: Identifier) -> Self {
        Self {
            key,
            codec: PayloadCodec::Custom {
                type_key: T::TYPE_KEY,
                read: read_network::<T>,
                write: write_network::<T>,
            },
        }
    }

    pub(super) fn read_payload(&'static self, data: &mut Cursor<&[u8]>) -> Result<SlotDisplay> {
        match &self.codec {
            PayloadCodec::Vanilla(kind) => SlotDisplay::read_vanilla_payload(*kind, data),
            PayloadCodec::Custom { read, .. } => Ok(SlotDisplay::Custom(CustomSlotDisplay {
                display_type: self,
                value: read(data)?,
            })),
        }
    }

    pub(super) fn write_payload(
        &self,
        display: &SlotDisplay,
        writer: &mut impl Write,
    ) -> Result<()> {
        match &self.codec {
            PayloadCodec::Vanilla(kind) if display.kind() == Some(*kind) => {
                display.write_vanilla_payload(writer)
            }
            PayloadCodec::Custom { write, .. } => {
                let SlotDisplay::Custom(display) = display else {
                    return Err(Error::other("Vanilla display passed to a custom codec"));
                };
                if !std::ptr::eq(self, display.display_type) {
                    return Err(Error::other(
                        "Custom display has a different registered type",
                    ));
                }
                let mut bytes = Vec::new();
                write(display.value.as_ref(), &mut bytes)?;
                writer.write_all(&bytes)
            }
            PayloadCodec::Vanilla(_) => Err(Error::other("Slot display payload kind mismatch")),
        }
    }
}

impl Debug for SlotDisplayType {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SlotDisplayType")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

pub type SlotDisplayTypeRef = &'static SlotDisplayType;

/// Owned, keyed payload for a registered extension display.
pub struct CustomSlotDisplay {
    display_type: SlotDisplayTypeRef,
    value: Box<dyn ErasedSlotDisplay>,
}

impl CustomSlotDisplay {
    pub fn new<T: SlotDisplayCodec>(display_type: SlotDisplayTypeRef, value: T) -> Result<Self> {
        if !matches!(&display_type.codec, PayloadCodec::Custom { type_key, .. } if *type_key == T::TYPE_KEY)
        {
            return Err(Error::other(
                "Display payload does not match its registered codec",
            ));
        }
        Ok(Self {
            display_type,
            value: Box::new(value),
        })
    }

    #[must_use]
    pub const fn display_type(&self) -> SlotDisplayTypeRef {
        self.display_type
    }

    #[must_use]
    pub fn downcast_ref<T: DowncastType>(&self) -> Option<&T> {
        self.value.downcast_ref::<T>()
    }
}

impl Clone for CustomSlotDisplay {
    fn clone(&self) -> Self {
        Self {
            display_type: self.display_type,
            value: self.value.clone_display(),
        }
    }
}

impl Debug for CustomSlotDisplay {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CustomSlotDisplay")
            .field("display_type", &self.display_type.key)
            .field("value", &self.value)
            .finish()
    }
}

impl PartialEq for CustomSlotDisplay {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.display_type, other.display_type)
            && self.value.display_eq(other.value.as_ref())
    }
}

fn read_network<T: SlotDisplayCodec>(
    data: &mut Cursor<&[u8]>,
) -> Result<Box<dyn ErasedSlotDisplay>> {
    Ok(Box::new(T::read_network(data)?))
}

fn write_network<T: SlotDisplayCodec>(
    value: &dyn ErasedSlotDisplay,
    writer: &mut Vec<u8>,
) -> Result<()> {
    let value = value
        .downcast_ref::<T>()
        .ok_or_else(|| Error::other("Slot display codec received a different payload type"))?;
    value.write_network(writer)
}

pub struct SlotDisplayTypeRegistry {
    slot_displays_by_id: Vec<SlotDisplayTypeRef>,
    slot_displays_by_key: FxHashMap<Identifier, usize>,
    allows_registering: bool,
}

impl SlotDisplayTypeRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            slot_displays_by_id: Vec::new(),
            slot_displays_by_key: FxHashMap::default(),
            allows_registering: true,
        }
    }

    pub(super) fn registered_entry_with_id(
        &self,
        entry: SlotDisplayTypeRef,
    ) -> Option<(usize, SlotDisplayTypeRef)> {
        let id = self.slot_displays_by_key.get(&entry.key).copied()?;
        let registered = self.slot_displays_by_id.get(id).copied()?;
        std::ptr::eq(registered, entry).then_some((id, registered))
    }
}

crate::impl_standard_methods!(
    SlotDisplayTypeRegistry,
    SlotDisplayTypeRef,
    slot_displays_by_id,
    slot_displays_by_key,
    allows_registering,
    "Cannot register duplicate slot display key: {}"
);
crate::impl_registry!(
    SlotDisplayTypeRegistry,
    SlotDisplayType,
    slot_displays_by_id,
    slot_displays_by_key,
    slot_displays
);

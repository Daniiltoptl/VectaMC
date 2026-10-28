use crate::inventory::menu::MenuKind;

/// A menu kind with all-default handling and no special behavior.
#[derive(Debug)]
pub struct BasicKind;

// SAFETY: This Vecta-owned key uniquely identifies the concrete menu kind
// within the process.
unsafe impl vecta_utils::DowncastType for BasicKind {
    const TYPE_KEY: vecta_utils::DowncastTypeKey =
        vecta_utils::DowncastTypeKey::new("vecta:menu/basic");
}

impl MenuKind for BasicKind {}

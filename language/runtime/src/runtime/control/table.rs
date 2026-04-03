use std::collections::HashMap;
use std::sync::OnceLock;

use parking_lot::Mutex;

use super::handle::{ControlEntry, ControlHandleId};
use super::object::ControlObject;

/// Process-global metadata for externally visible runtime control objects.
#[derive(Debug, Default)]
pub(crate) struct Control {
    /// Registered control handles.
    pub(super) handles: HashMap<ControlHandleId, ControlEntry>,
    /// Stored control objects keyed by their opaque handle id.
    pub(super) objects: HashMap<ControlHandleId, ControlObject>,
}

static CONTROL: OnceLock<Mutex<Control>> = OnceLock::new();

/// Return the process-global runtime control table.
pub(crate) fn control() -> &'static Mutex<Control> {
    CONTROL.get_or_init(|| Mutex::new(Control::default()))
}

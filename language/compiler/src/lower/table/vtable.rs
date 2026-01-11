/// VTable layout for class method dispatch.
///
/// Each class with virtual methods gets a vtable containing function pointers
/// for all overridable methods in inheritance order.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct VTable {
    // TODO #Incomplete: method slots, parent offsets, etc.
}

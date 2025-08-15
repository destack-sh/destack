//! destack.core.common.change@2025.08.15.1

#![destack::partial(destack.core.common.change, file)]

use crate::Value;

#[destack::generated(EditOperation, -, block)]
/// A specific Edit of an Entity.
pub struct EditOperation {
    pub r#type: EditOperationType,
    pub property_id: u8,
    pub custom_property_name: Option<String>,
    pub key: Option<Value>,
    pub value: Option<Value>,
    pub reverse_operation: Box<Option<EditOperation>>,
    pub reverse_value: Option<Value>,
}

#[destack::generated(ChangeType, -, block)]
/// The type of Change.
pub enum ChangeType {
    /// Create a new Entity
    Create = 20,
    /// Upsert an Entity (create if not exists, update if exists)
    Upsert = 21,
    /// Update an existing Entity
    Update = 30,
    /// Move an Entity to a new parent Entity (or detach)
    Move = 31,
    /// Delete an Entity (and its descendants)
    Delete = 40,
    /// Restore a deleted Entity (and its descendants)
    Restore = 41,
}

#[destack::generated(EditOperationType, -, block)]
/// The update operation to perform on a Node.
pub enum EditOperationType {
    /// Set a Property to a value (may be an empty value)
    Set = 1,
}

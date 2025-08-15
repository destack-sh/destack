//! destack.core.common.change@2025.08.15.1

#![destack::partial(destack.core.common.change, file)]

#[destack::generated(EditOperation, struct, block)]
/// A specific Edit of an Entity.
pub struct EditOperation {}

#[destack::generated(ChangeType, enum, block)]
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

#[destack::generated(EditOperationType, enum, block)]
/// The update operation to perform on a Node.
pub enum EditOperationType {
    /// Set a Property to a value (may be an empty value)
    Set = 1,
}

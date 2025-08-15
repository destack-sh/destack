//! destack.core.common.change@2025.08.14.0

#![destack::partial(destack.core.common.change, file)]

#[destack::generated(EditOperation, struct, block)]
/// A specific Edit of an Entity.
pub struct EditOperation {

}

#[destack::generated(ChangeType, enum, block)]
/// The type of Change.
pub enum ChangeType {
    /// Create a new Entity
    CREATE = 20,
    /// Upsert an Entity (create if not exists, update if exists)
    UPSERT = 21,
    /// Update an existing Entity
    UPDATE = 30,
    /// Move an Entity to a new parent Entity (or detach)
    MOVE = 31,
    /// Delete an Entity (and its descendants)
    DELETE = 40,
    /// Restore a deleted Entity (and its descendants)
    RESTORE = 41
}

#[destack::generated(EditOperationType, enum, block)]
/// The update operation to perform on a Node.
pub enum EditOperationType {
    /// Set a Property to a value (may be an empty value)
    SET = 1
}
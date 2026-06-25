use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Access, GenericArgumentBinding, GlobalSymbolId, GlobalTypeId, ScalarLiteral, StaticKey,
};

/// Value projection selected during checking.
///
/// Examples:
/// ```ds
/// value                 // Identity
/// point.x               // FieldGet
/// values.length         // SliceLength
/// dynamic.payload       // DynamicPayload
/// dynamic.type          // DynamicType
/// Shape.Circle(radius)  // VariantTag and VariantPayload
/// UserId(raw)           // NewtypePayload
/// &value                // Borrow
/// ^value                // Move
/// *box                  // Dereference
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Projection {
    /// Keep the same runtime value and view it as a narrower type.
    ///
    /// Examples:
    /// ```ds
    /// value is string  // same value, narrower checked type
    /// ```
    Identity {
        /// The projected value type.
        ty: GlobalTypeId,
    },
    /// Extract one static layout field from an aggregate value.
    ///
    /// Examples:
    /// ```ds
    /// point.x
    /// tuple[0]
    /// user[uniqueName]
    /// ```
    FieldGet {
        /// The selected field.
        field: ProjectionField,
        /// The projected value type.
        ty: GlobalTypeId,
    },
    /// Read the runtime length from a slice descriptor.
    ///
    /// Examples:
    /// ```ds
    /// values.length
    /// ```
    SliceLength {
        /// The projected length type.
        ty: GlobalTypeId,
    },
    /// Read the erased payload from a dynamic value.
    ///
    /// Examples:
    /// ```ds
    /// dynamic.payload
    /// ```
    DynamicPayload {
        /// The projected payload type.
        ty: GlobalTypeId,
    },
    /// Read the concrete type id from a dynamic value.
    ///
    /// Examples:
    /// ```ds
    /// dynamic.type
    /// ```
    DynamicType {
        /// The projected type descriptor type.
        ty: GlobalTypeId,
    },
    /// Read the active tag from a physical tagged sum value.
    ///
    /// Examples:
    /// ```ds
    /// shape is Shape.Circle
    /// match shape { Shape.Circle(radius) => radius }
    /// ```
    VariantTag {
        /// The projected tag type.
        ty: GlobalTypeId,
    },
    /// Extract the payload selected by one concrete variant tag.
    ///
    /// Examples:
    /// ```ds
    /// match shape {
    ///     Shape.Circle(radius) => radius
    /// }
    /// ```
    VariantPayload {
        /// The selected variant family symbol.
        owner: GlobalSymbolId,
        /// The selected variant member symbol.
        member: GlobalSymbolId,
        /// The selected generic argument bindings for the selected owner.
        generic_arguments: Vec<GenericArgumentBinding>,
        /// The discriminant value tested at runtime.
        discriminant: ScalarLiteral,
        /// The projected payload type.
        ty: GlobalTypeId,
    },
    /// Unwrap one newtype payload.
    ///
    /// Examples:
    /// ```ds
    /// newtype UserId = string;
    /// match id { UserId(raw) => raw }
    /// ```
    NewtypePayload {
        /// The selected newtype symbol.
        symbol: GlobalSymbolId,
        /// The selected generic argument bindings for the selected newtype.
        generic_arguments: Vec<GenericArgumentBinding>,
        /// The projected payload type.
        ty: GlobalTypeId,
    },
    /// Borrow the input before matching it.
    ///
    /// Examples:
    /// ```ds
    /// match &value { Pattern => ... }
    /// ```
    Borrow {
        /// The requested borrow access, if source explicit.
        access: Option<Access>,
        /// The projected borrow type.
        ty: GlobalTypeId,
    },
    /// Move the input before matching it.
    ///
    /// Examples:
    /// ```ds
    /// match ^value { Pattern => ... }
    /// ```
    Move {
        /// The requested move access, if source explicit.
        access: Option<Access>,
        /// The projected moved type.
        ty: GlobalTypeId,
    },
    /// Dereference the input before matching it.
    ///
    /// Examples:
    /// ```ds
    /// match *box { Point { x, y } => ... }
    /// ```
    Dereference {
        /// The projected pointee type.
        ty: GlobalTypeId,
    },
}

/// Static field selected by one projection.
///
/// Examples:
/// ```ds
/// point.x                 // Key("x") for structural objects
/// tuple[0]                // Key(0) for tuple-like layout
/// user.name               // Member(User.name) for nominal stored fields
/// object[Symbol.for("x")] // Key(Symbol.for("x")) for structural symbol keys
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ProjectionField {
    /// Structural field key.
    ///
    /// Examples:
    /// ```ds
    /// declare const point: { x: int32 };
    /// point.x
    ///
    /// declare const object: { [Symbol.for("tag")]: string };
    /// object[Symbol.for("tag")]
    /// ```
    Key(StaticKey),
    /// Declaration-backed nominal stored member.
    ///
    /// Examples:
    /// ```ds
    /// struct Point { x: int32; y: int32 }
    /// point.x // selects the Point.x field symbol, not only the key "x"
    /// ```
    Member(GlobalSymbolId),
}

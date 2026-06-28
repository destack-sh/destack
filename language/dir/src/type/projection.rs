use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Access, CallResolution, GenericArgumentBinding, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId,
    MemberResolution, ScalarLiteral, StaticKey,
};

/// Value projection selected during checking.
///
/// Examples:
/// ```ds
/// point.x               // FieldGet
/// user.name             // PropertyGet, when backed by a getter
/// bag[key]              // SubscriptGet
/// values[0]             // Call, when selected through Sequence.index
/// values[start..]       // Call, when selected through Sequence.rest
/// { ...rest }           // ObjectRest
/// values.length         // SliceLength
/// dynamic.payload       // DynamicPayload
/// dynamic.type          // DynamicType
/// Shape.Circle(radius)  // VariantTag and VariantPayload
/// UserId(raw)           // NewtypePayload
/// &value                // Borrow
/// ^value                // Move
/// *box                  // Dereference
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum Projection {
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
    /// Read one accessor-backed property value.
    ///
    /// Examples:
    /// ```ds
    /// user.name // selects get name()
    /// ```
    PropertyGet {
        /// The selected getter member.
        read: MemberResolution,
        /// The projected value type.
        ty: GlobalTypeId,
    },
    /// Read one dynamically selected subscript value.
    ///
    /// Examples:
    /// ```ds
    /// const { [key]: value } = object;
    /// ```
    SubscriptGet {
        /// The source node providing the subscript key.
        index: GlobalNodeIdAny,
        /// The selected subscript operation.
        read: SubscriptOperation,
        /// The projected value type.
        ty: GlobalTypeId,
    },
    /// Read one value through a selected call.
    ///
    /// Examples:
    /// ```ds
    /// const [head] = values; // selected Sequence.index call
    /// const [head, ...tail] = values; // selected Sequence.rest call
    /// ```
    Call {
        /// The selected call operation.
        call: CallResolution,
        /// The returned value type.
        ty: GlobalTypeId,
    },
    /// Materialize one object rest value from selected fields.
    ///
    /// Examples:
    /// ```ds
    /// const { name, ...rest } = user;
    /// ```
    ObjectRest {
        /// The selected source field projections.
        fields: Vec<ObjectRestField>,
        /// The materialized rest value type.
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
        /// The selected dereference operation.
        read: DereferenceOperation,
        /// The projected pointee type.
        ty: GlobalTypeId,
    },
}

impl Projection {
    /// Return the projected value type.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::FieldGet { ty, .. }
            | Self::PropertyGet { ty, .. }
            | Self::SubscriptGet { ty, .. }
            | Self::Call { ty, .. }
            | Self::ObjectRest { ty, .. }
            | Self::SliceLength { ty }
            | Self::DynamicPayload { ty }
            | Self::DynamicType { ty }
            | Self::VariantTag { ty }
            | Self::VariantPayload { ty, .. }
            | Self::NewtypePayload { ty, .. }
            | Self::Borrow { ty, .. }
            | Self::Move { ty, .. }
            | Self::Dereference { ty, .. } => *ty,
        }
    }
}

/// One source field used to materialize an object rest value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ObjectRestField {
    /// The materialized field key.
    pub key: StaticKey,
    /// The selected source projection.
    pub projection: Projection,
}

/// Subscript operation selected by one projection or place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum SubscriptOperation {
    /// Structural tuple, field, or index-signature selection.
    Member(MemberResolution),
    /// Protocol-backed subscript call.
    Call(CallResolution),
}

/// Dereference operation selected by one projection or place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum DereferenceOperation {
    /// Direct dereference of a physical reference or pointer form.
    Direct,
    /// Protocol-backed dereference call.
    Call(CallResolution),
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

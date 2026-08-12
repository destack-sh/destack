use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Access, Call, CallDecision, Dereference, DereferenceResolution, FieldResolution,
    GenericArgumentBinding, GlobalSymbolId, GlobalTypeId, MemberAccess, MemberDecision,
    OperationResolution, ScalarLiteral, StaticKey, Subscript, SubscriptDecision, TypeFold,
    VariantCase,
};

/// Value projection selected during checking.
///
/// Examples:
/// ```ds
/// point.x               // Field
/// user.name             // Call, when backed by a getter
/// bag[key]              // Subscript
/// values[0]             // Call, when selected through Sequence.index
/// values[start..]       // Call, when selected through Sequence.rest
/// { missing = value }   // Absent, when the source has no such field
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub enum Projection {
    /// Produce `undefined` for one statically absent destructuring field.
    ///
    /// Examples:
    /// ```ds
    /// const { missing = fallback } = {};
    /// ```
    Absent {
        /// The projected `undefined` type.
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
    Field(FieldResolution),
    /// Read one dynamically selected subscript value.
    ///
    /// Examples:
    /// ```ds
    /// const { [key]: value } = object;
    /// ```
    Subscript(Box<Subscript>),
    /// Read one value through a selected call.
    ///
    /// Examples:
    /// ```ds
    /// const [head] = values; // selected Sequence.index call
    /// const [head, ...tail] = values; // selected Sequence.rest call
    /// ```
    Call(Box<Call>),
    /// Read through one selected member access.
    ///
    /// Examples:
    /// ```ds
    /// declare const value: { item: Readable } & { item: Writable };
    /// const { item } = value;
    /// ```
    Member(Box<MemberAccess>),
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
        /// The checked variant carrier type.
        carrier: GlobalTypeId,
        /// The selected discriminator field.
        discriminator: StaticKey,
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
        /// The selected tagged case.
        case: VariantCase,
        /// The instantiated backing arm.
        backing: GlobalTypeId,
        /// The selected discriminator field.
        discriminator: StaticKey,
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
    Dereference(Dereference),
    /// Duplicate one copyable value out of a place or view.
    ///
    /// Examples:
    /// ```ds
    /// takeHandle(view.handle) // copies the handle out of the view
    /// value as float64        // copies the scalar it converts
    /// ```
    Copy {
        /// The duplicated value type.
        ty: GlobalTypeId,
    },
}

/// Projection selected for one value or every runtime union arm.
///
/// Examples:
/// ```ds
/// const { x } = value; // Union when value is a union
/// ```
pub type ProjectionResolution = OperationResolution<Projection>;

impl Projection {
    /// Return the projected value type.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::Field(field) => field.ty,
            Self::Absent { ty }
            | Self::ObjectRest { ty, .. }
            | Self::SliceLength { ty }
            | Self::DynamicPayload { ty }
            | Self::DynamicType { ty }
            | Self::VariantTag { ty, .. }
            | Self::VariantPayload { ty, .. }
            | Self::NewtypePayload { ty, .. }
            | Self::Borrow { ty, .. }
            | Self::Move { ty, .. }
            | Self::Copy { ty } => *ty,
            Self::Subscript(read) => read.ty,
            Self::Call(call) => call.return_type,
            Self::Member(access) => access.ty,
            Self::Dereference(read) => read.ty,
        }
    }
}

impl OperationResolution<Projection> {
    /// Return the projected value type.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::One(projection) => projection.ty(),
            Self::Union { ty, .. } => *ty,
        }
    }
}

impl From<CallDecision> for ProjectionResolution {
    /// Convert one call resolution into the corresponding projection resolution.
    fn from(resolution: CallDecision) -> Self {
        match resolution {
            OperationResolution::One(call) => Self::One(Projection::Call(Box::new(call))),
            OperationResolution::Union { arms, ty } => {
                let arms = arms
                    .into_iter()
                    .map(|call| Projection::Call(Box::new(call)))
                    .collect();

                Self::Union { arms, ty }
            }
        }
    }
}

impl From<SubscriptDecision> for ProjectionResolution {
    /// Convert one subscript resolution into the corresponding projection resolution.
    fn from(resolution: SubscriptDecision) -> Self {
        match resolution {
            OperationResolution::One(subscript) => {
                Self::One(Projection::Subscript(Box::new(subscript)))
            }
            OperationResolution::Union { arms, ty } => {
                let arms = arms
                    .into_iter()
                    .map(|subscript| Projection::Subscript(Box::new(subscript)))
                    .collect();

                Self::Union { arms, ty }
            }
        }
    }
}

impl From<DereferenceResolution> for ProjectionResolution {
    /// Convert one dereference resolution into the corresponding projection resolution.
    fn from(resolution: DereferenceResolution) -> Self {
        match resolution {
            OperationResolution::One(dereference) => {
                Self::One(Projection::Dereference(dereference))
            }
            OperationResolution::Union { arms, ty } => {
                let arms = arms.into_iter().map(Projection::Dereference).collect();

                Self::Union { arms, ty }
            }
        }
    }
}

impl From<MemberDecision> for ProjectionResolution {
    /// Convert one member resolution into the corresponding projection resolution.
    fn from(resolution: MemberDecision) -> Self {
        match resolution {
            OperationResolution::One(access) => Self::One(Projection::Member(Box::new(access))),
            OperationResolution::Union { arms, ty } => Self::Union {
                arms: arms
                    .into_iter()
                    .map(|access| Projection::Member(Box::new(access)))
                    .collect(),
                ty,
            },
        }
    }
}

/// One source field used to materialize an object rest value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold)]
pub struct ObjectRestField {
    /// The materialized field key.
    pub key: StaticKey,
    /// The selected source projection.
    pub projection: Projection,
}

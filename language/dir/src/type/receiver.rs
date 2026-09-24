use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Dereference, GlobalSymbolId, GlobalTypeId, InstanceKey, InstanceKeyVisit, TypeFold};

/// Receiver selected by contextual lookup, such as `this` or `super`.
///
/// Examples:
/// ```ds
/// this.name      // declaration: the enclosing class, ty: its instance type
/// super.render() // declaration: the enclosing class, ty: its superclass type
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct ReceiverDecision {
    /// The receiver syntax kind.
    pub kind: ReceiverKind,
    /// The declaration that introduces the receiver.
    pub declaration: GlobalSymbolId,
    /// The receiver type after inference.
    pub ty: GlobalTypeId,
}

/// Receiver syntax resolved by contextual lookup.
///
/// Examples:
/// ```ds
/// this
/// super
/// ```
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Reflect,
    TypeFold,
    InstanceKeyVisit,
)]
pub enum ReceiverKind {
    /// The active `this` receiver.
    ///
    /// Examples:
    /// ```ds
    /// this.name
    /// ```
    This,
    /// The active superclass receiver.
    ///
    /// Examples:
    /// ```ds
    /// super.render()
    /// ```
    Super,
}

/// One receiver and its ordered implicit transformations.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct AdjustedReceiver {
    /// The receiver type before adjustment.
    pub source: GlobalTypeId,
    /// The ordered receiver adjustments.
    pub adjustments: Vec<ReceiverAdjustment>,
}

impl AdjustedReceiver {
    /// Create an unadjusted receiver.
    pub fn direct(source: GlobalTypeId) -> Self {
        Self {
            source,
            adjustments: Vec::new(),
        }
    }

    /// Return the receiver type after every adjustment.
    pub fn ty(&self) -> GlobalTypeId {
        self.adjustments
            .last()
            .map(ReceiverAdjustment::ty)
            .unwrap_or(self.source)
    }

    /// Prepend one adjustment performed before the existing adjustments.
    pub fn prepend(&mut self, adjustment: ReceiverAdjustment) {
        self.adjustments.insert(0, adjustment);
    }
}

/// Receiver selected for one member access.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub enum MemberReceiver {
    /// Member selected directly from the adjusted receiver.
    Direct(AdjustedReceiver),
    /// Member selected through an erased interface dispatch table.
    Dynamic(DynamicDispatch),
}

impl MemberReceiver {
    /// Create an unadjusted direct member receiver.
    pub fn direct(source: GlobalTypeId) -> Self {
        Self::Direct(AdjustedReceiver::direct(source))
    }

    /// Return the receiver type before adjustment.
    pub fn source(&self) -> GlobalTypeId {
        match self {
            Self::Direct(receiver) => receiver.source,
            Self::Dynamic(dispatch) => dispatch.receiver.source,
        }
    }

    /// Return the receiver type after every adjustment.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::Direct(receiver) => receiver.ty(),
            Self::Dynamic(dispatch) => dispatch.receiver.ty(),
        }
    }

    /// Return the adjusted receiver.
    pub fn adjusted(&self) -> &AdjustedReceiver {
        match self {
            Self::Direct(receiver) => receiver,
            Self::Dynamic(dispatch) => &dispatch.receiver,
        }
    }

    /// Return the adjusted receiver mutably.
    pub fn adjusted_mut(&mut self) -> &mut AdjustedReceiver {
        match self {
            Self::Direct(receiver) => receiver,
            Self::Dynamic(dispatch) => &mut dispatch.receiver,
        }
    }

    /// Prepend one adjustment performed before member selection.
    pub fn prepend(&mut self, adjustment: ReceiverAdjustment) {
        match self {
            Self::Direct(receiver) => receiver.prepend(adjustment),
            Self::Dynamic(dispatch) => dispatch.receiver.prepend(adjustment),
        }
    }
}

/// Erased receiver selected for dynamic dispatch.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub struct DynamicDispatch {
    /// The erased receiver.
    pub receiver: AdjustedReceiver,
    /// The interface constraint declaring the dispatch member.
    pub constraint: GlobalTypeId,
}

/// One implicit transformation applied before receiver selection.
///
/// Examples:
/// ```ds
/// value.method()       // Borrow, when `this` expects a borrowed receiver
/// box.value            // Dereference, when `Box<T>` exposes members of `T`
/// userId.length        // NewtypePayload, when the backing string exposes `length`
/// shape.radius         // UnionPayload, after narrowing selects one union arm
/// circle.area()        // Upcast, when `Circle` inherits `area` from `Shape`
/// ```
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, TypeFold, InstanceKeyVisit,
)]
pub enum ReceiverAdjustment {
    /// Borrow the receiver for one method call.
    Borrow {
        /// The adjusted receiver type.
        ty: GlobalTypeId,
    },
    /// Dereference the receiver before lookup.
    Dereference(Dereference),
    /// Project the payload of one newtype receiver.
    NewtypePayload {
        /// The selected newtype declaration and its generic arguments.
        key: InstanceKey,
        /// The adjusted receiver type.
        ty: GlobalTypeId,
    },
    /// Project the payload selected by one precise union arm.
    UnionPayload {
        /// The union whose arm order defines the payload position.
        union: GlobalTypeId,
        /// The selected union arm.
        arm: GlobalTypeId,
        /// The adjusted receiver type.
        ty: GlobalTypeId,
    },
    /// Reinterpret the receiver at the base class declaring the member.
    Upcast {
        /// The adjusted receiver type.
        ty: GlobalTypeId,
    },
}

impl ReceiverAdjustment {
    /// Return the adjusted receiver type.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::Borrow { ty }
            | Self::NewtypePayload { ty, .. }
            | Self::UnionPayload { ty, .. }
            | Self::Upcast { ty } => *ty,
            Self::Dereference(dereference) => dereference.ty,
        }
    }
}

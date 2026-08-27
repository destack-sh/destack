use destack_core::StringId;

use crate::{
    Access, AttributeIdentifier, Constant, Copy, FloatType, FloatValue, Lifetime,
    LifetimeParameter, Multiplicity, Nullability, ReferenceKind, Storage, TypeId,
};

/// A MIR value that embeds canonical types.
pub trait TypeFold {
    /// Map every embedded type.
    fn map_types(&mut self, map: &mut impl FnMut(TypeId) -> TypeId);
}

impl TypeFold for TypeId {
    fn map_types(&mut self, map: &mut impl FnMut(TypeId) -> TypeId) {
        *self = map(*self);
    }
}

impl<T: TypeFold> TypeFold for Vec<T> {
    fn map_types(&mut self, map: &mut impl FnMut(TypeId) -> TypeId) {
        for value in self {
            value.map_types(map);
        }
    }
}

impl<T: TypeFold> TypeFold for Option<T> {
    fn map_types(&mut self, map: &mut impl FnMut(TypeId) -> TypeId) {
        if let Some(value) = self {
            value.map_types(map);
        }
    }
}

/// Implement empty folds for values that contain no types.
macro_rules! type_fold_leaves {
    ($($ty:ty),* $(,)?) => {
        $(
            impl TypeFold for $ty {
                fn map_types(&mut self, _map: &mut impl FnMut(TypeId) -> TypeId) {}
            }
        )*
    };
}

type_fold_leaves!(
    bool,
    i128,
    u16,
    u32,
    u64,
    StringId,
    Access,
    AttributeIdentifier,
    Constant,
    Copy,
    FloatType,
    FloatValue,
    Lifetime,
    LifetimeParameter,
    Multiplicity,
    Nullability,
    ReferenceKind,
    Storage,
);

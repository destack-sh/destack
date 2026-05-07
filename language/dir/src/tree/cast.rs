use serde::{Deserialize, Serialize};

/// The operator used to perform a cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CastOperator {
    /// Preserve the representation without conversion.
    Identity,
    /// Widen an integer type to a larger width.
    IntWiden,
    /// Narrow an integer type to a smaller width.
    IntNarrow,
    /// Change integer signedness at the same width.
    IntSignChange,
    /// Widen a float type to a larger width.
    FloatWiden,
    /// Narrow a float type to a smaller width.
    FloatNarrow,
    /// Convert an integer to a float.
    IntToFloat,
    /// Convert a float to an integer.
    FloatToInt,
    /// Convert a pointer to an integer.
    PointerToInt,
    /// Convert an integer to a pointer.
    IntToPointer,
    /// Convert one pointer type to another.
    PointerCast,
    /// Convert a sized array to a slice view.
    ArraySizedToSlice,
    /// Convert an enum to an integer.
    EnumToInt,
    /// Convert an integer to an enum.
    IntToEnum,
    /// Convert an enum to a string.
    EnumToString,
    /// Convert a string to an enum.
    StringToEnum,
    /// Upcast into a wider union.
    UnionUpcast,
    /// Downcast from a union.
    UnionDowncast,
    /// Upcast into an instance type.
    InstanceUpcast,
    /// Downcast from an instance type.
    InstanceDowncast,
    /// Upcast into a nullable type.
    NullableUpcast,
    /// Downcast from a nullable type.
    NullableDowncast,
    /// Downcast from unknown.
    UnknownDowncast,
    /// Downcast from any.
    AnyDowncast,
    /// Upcast into any.
    AnyUpcast,
    /// Upcast into unknown.
    UnknownUpcast,
    /// Upcast into object (from non-primitive).
    ObjectUpcast,
    /// Downcast from object to specific non-primitive.
    ObjectDowncast,
}

/// How one cast expression entered the DIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CastOrigin {
    /// Casts written explicitly in source.
    Explicit,
    /// Casts inserted implicitly at type boundaries.
    Implicit,
}

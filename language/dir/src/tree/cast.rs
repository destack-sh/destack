/// The kind of cast to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastKind {
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
    /// Upcast into a wider union.
    UnionUpcast,
    /// Downcast from a union with a runtime check.
    UnionDowncast,
    /// Upcast into an instance type.
    InstanceUpcast,
    /// Downcast from an instance type with a runtime check.
    InstanceDowncast,
    /// Upcast into a nullable type.
    NullableUpcast,
    /// Downcast from a nullable type with a runtime check.
    NullableDowncast,
}

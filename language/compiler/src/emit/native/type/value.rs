use cranelift_codegen::ir as cir;
use smallvec::{SmallVec, smallvec};
use tspp_mir as mir;

use crate::EmitError;

use super::TypeEmitter;

/// One scalar field inside a native scalar pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::emit::native) struct ScalarField {
    /// Cranelift scalar type.
    pub(in crate::emit::native) ty: cir::Type,
    /// Canonical byte offset.
    pub(in crate::emit::native) offset: u32,
}

/// One native value representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::emit::native) enum ValueType {
    /// One value held directly in Cranelift SSA.
    Direct {
        /// The Cranelift value type.
        ty: cir::Type,
        /// The canonical byte length.
        byte_len: u32,
        /// The canonical byte alignment.
        alignment: u32,
    },
    /// Two scalar fields held directly in machine registers.
    ScalarPair {
        /// Physical scalar fields in canonical byte order.
        fields: [ScalarField; 2],
        /// Canonical byte length.
        byte_len: u32,
        /// Canonical byte alignment.
        alignment: u32,
    },
    /// One value passed by address in canonical MIR layout.
    Indirect {
        /// The canonical byte length.
        byte_len: u32,
        /// The canonical byte alignment.
        alignment: u32,
    },
}

impl ValueType {
    /// Return the Cranelift type when this value is direct.
    pub(in crate::emit::native) const fn direct(self) -> Option<cir::Type> {
        match self {
            Self::Direct { ty, .. } => Some(ty),
            Self::ScalarPair { .. } | Self::Indirect { .. } => None,
        }
    }

    /// Return the scalar fields when this value is a scalar pair.
    pub(in crate::emit::native) const fn scalar_pair(self) -> Option<[ScalarField; 2]> {
        match self {
            Self::ScalarPair { fields, .. } => Some(fields),
            Self::Direct { .. } | Self::Indirect { .. } => None,
        }
    }

    /// Return the number of Program words occupied at the native entry boundary.
    pub(in crate::emit::native) fn word_count(self) -> u32 {
        let byte_len = match self {
            Self::Direct { byte_len, .. }
            | Self::ScalarPair { byte_len, .. }
            | Self::Indirect { byte_len, .. } => byte_len,
        };

        byte_len.div_ceil(8)
    }

    /// Return the canonical byte length.
    pub(in crate::emit::native) fn byte_len(self) -> u32 {
        match self {
            Self::Direct { byte_len, .. }
            | Self::ScalarPair { byte_len, .. }
            | Self::Indirect { byte_len, .. } => byte_len,
        }
    }

    /// Return the canonical byte alignment.
    pub(in crate::emit::native) fn alignment(self) -> u32 {
        match self {
            Self::Direct { alignment, .. }
            | Self::ScalarPair { alignment, .. }
            | Self::Indirect { alignment, .. } => alignment,
        }
    }

    /// Return whether values use indirect canonical storage.
    pub(in crate::emit::native) const fn is_indirect(self) -> bool {
        matches!(self, Self::Indirect { .. })
    }

    /// Return the number of flattened ABI parameters used by this value.
    pub(in crate::emit::native) const fn abi_parameter_count(self) -> usize {
        match self {
            Self::Direct { .. } | Self::Indirect { .. } => 1,
            Self::ScalarPair { .. } => 2,
        }
    }

    /// Return the Cranelift ABI representation of this value.
    pub(in crate::emit::native) fn abi(self, pointer: cir::Type) -> SmallVec<[cir::AbiParam; 2]> {
        match self {
            Self::Direct { ty, .. } => smallvec![cir::AbiParam::new(ty)],
            Self::ScalarPair { fields, .. } => {
                fields.map(|field| cir::AbiParam::new(field.ty)).into()
            }
            Self::Indirect { .. } => smallvec![cir::AbiParam::new(pointer)],
        }
    }
}

impl TypeEmitter<'_> {
    /// Return one function result's native representation.
    pub(in crate::emit::native) fn result(
        &self,
        id: mir::TypeId,
    ) -> Result<Option<ValueType>, EmitError> {
        match self.optimized.tree.type_definition(id) {
            mir::Type::Never | mir::Type::Void | mir::Type::Null => Ok(None),
            _ => self.value(id).map(Some),
        }
    }

    /// Return one MIR value's native representation.
    pub(in crate::emit::native) fn value(&self, id: mir::TypeId) -> Result<ValueType, EmitError> {
        // read the physical value layout
        let layout = self
            .optimized
            .layouts
            .type_layout(id)
            .ok_or_else(|| self.unsupported("native value has no physical layout"))?;
        let value_type = match layout.representation {
            mir::Representation::Scalar(scalar) => ValueType::Direct {
                ty: self.scalar(scalar)?,
                byte_len: layout.size,
                alignment: layout.alignment,
            },
            mir::Representation::ScalarPair(fields) => ValueType::ScalarPair {
                fields: [
                    ScalarField {
                        ty: self.scalar(fields[0].scalar)?,
                        offset: fields[0].offset,
                    },
                    ScalarField {
                        ty: self.scalar(fields[1].scalar)?,
                        offset: fields[1].offset,
                    },
                ],
                byte_len: layout.size,
                alignment: layout.alignment,
            },
            mir::Representation::Vector(vector) => ValueType::Direct {
                ty: self.vector(vector)?,
                byte_len: layout.size,
                alignment: layout.alignment,
            },
            mir::Representation::Memory => ValueType::Indirect {
                byte_len: layout.size,
                alignment: layout.alignment,
            },
        };

        Ok(value_type)
    }

    /// Return one Cranelift scalar type.
    fn scalar(&self, scalar: mir::Scalar) -> Result<cir::Type, EmitError> {
        match scalar.primitive {
            mir::Primitive::Integer { width } => self
                .integer(width)
                .ok_or_else(|| self.unsupported("native integer width is unavailable")),
            mir::Primitive::Float(format) => self.float(format),
            mir::Primitive::Pointer { width } if width == self.layout.pointer_bits() => {
                Ok(self.pointer())
            }
            mir::Primitive::Pointer { .. } => {
                Err(self.unsupported("native pointer width disagrees with the target"))
            }
        }
    }

    /// Return one Cranelift fixed vector type.
    fn vector(&self, vector: mir::Vector) -> Result<cir::Type, EmitError> {
        let lane = self.scalar(vector.element)?;

        lane.by(vector.lanes)
            .ok_or_else(|| self.unsupported("native vector shape is unavailable"))
    }

    /// Return one integer register type.
    fn integer(&self, width: u16) -> Option<cir::Type> {
        match width {
            8 => Some(cir::types::I8),
            16 => Some(cir::types::I16),
            32 => Some(cir::types::I32),
            64 => Some(cir::types::I64),
            128 => Some(cir::types::I128),
            _ => None,
        }
    }

    /// Return one floating point register type.
    fn float(&self, format: mir::FloatType) -> Result<cir::Type, EmitError> {
        match format {
            mir::FloatType::Float32 => Ok(cir::types::F32),
            mir::FloatType::Float64 => Ok(cir::types::F64),
        }
    }
}

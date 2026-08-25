use destack_bytecode as bytecode;
use destack_mir as mir;

use crate::EmitError;

use super::TypeEmitter;

impl TypeEmitter<'_> {
    /// Return the bytecode register representation for one MIR type.
    pub(crate) fn register_type(&self, ty: mir::TypeId) -> Result<bytecode::ValueType, EmitError> {
        // resolve the storage type behind the MIR type and its layout
        let representation = self.optimized.tree.storage_type(ty);
        let definition = self.optimized.tree.get(representation);
        let layout = self.layout(representation)?;

        // map each MIR type onto its bytecode register representation
        let value_type = match definition {
            mir::Type::Never | mir::Type::Void => return Err(self.unsupported_type()),
            mir::Type::Boolean => bytecode::ValueType::scalar(bytecode::Scalar::Boolean),
            mir::Type::Character => bytecode::ValueType::scalar(bytecode::Scalar::Uint32),
            mir::Type::Int { is_signed, .. } => self.integer(layout, *is_signed)?,
            mir::Type::Isize => bytecode::ValueType::scalar(self.scalar(representation)?),
            mir::Type::Usize | mir::Type::TypeDescriptor => {
                bytecode::ValueType::scalar(self.scalar(representation)?)
            }
            mir::Type::Float(format) => bytecode::ValueType::scalar(self.float(*format)),
            mir::Type::TypeId => bytecode::ValueType::type_id(),
            mir::Type::Reference { kind, storage, .. } => {
                bytecode::ValueType::reference(self.reference_kind(*kind), self.storage(*storage))
            }
            mir::Type::Pointer { .. } => bytecode::ValueType::pointer(),
            mir::Type::Slice {
                kind,
                element,
                storage,
                ..
            } => bytecode::ValueType::slice(
                self.type_id(*element)?,
                self.reference_kind(*kind),
                self.storage(*storage),
            ),
            mir::Type::Uninit { value } => self.uninitialized(*value)?,
            mir::Type::Dynamic {
                kind,
                constraint,
                storage,
                ..
            } => bytecode::ValueType::dynamic(
                self.type_id(*constraint)?,
                bytecode::ReferenceType::new(self.reference_kind(*kind), self.storage(*storage)),
            ),
            mir::Type::Vector { element, .. } => {
                let mir::Representation::Vector(vector) = layout.representation else {
                    return Err(self.internal("MIR vector has no vector representation"));
                };
                let scalar = self.scalar(*element)?;
                let lane_count =
                    u16::try_from(vector.lanes).map_err(|_| self.unsupported_type())?;

                bytecode::ValueType::vector(bytecode::VectorType::new(scalar, lane_count))
            }
            mir::Type::Function { kind, storage, .. } => bytecode::ValueType::function(
                bytecode::ReferenceType::new(self.reference_kind(*kind), self.storage(*storage)),
            ),
            mir::Type::FunctionPointer { .. } => bytecode::ValueType::function_pointer(),
            mir::Type::FixedArray { .. }
            | mir::Type::Tuple { .. }
            | mir::Type::Struct { .. }
            | mir::Type::Variant { .. }
            | mir::Type::Newtype { .. } => {
                let word_count = self.word_count(representation)?;
                if word_count == 0 {
                    return Err(self.unsupported_type());
                }

                bytecode::ValueType::indexed(self.type_id(representation)?, word_count)
            }
            mir::Type::Atomic { .. }
            | mir::Type::ManuallyDrop { .. }
            | mir::Type::Application { .. }
            | mir::Type::Error
            | mir::Type::FunctionSignature { .. } => return Err(self.unsupported_type()),
        };

        // require semantic bytecode types to preserve the canonical MIR width
        let word_count = self.word_count(representation)?;
        if value_type.word_count() != word_count {
            return Err(self.internal("bytecode value width disagrees with its MIR layout"));
        }

        Ok(value_type)
    }

    /// Return the scalar bytecode representation for one MIR type.
    pub(crate) fn scalar(&self, ty: mir::TypeId) -> Result<bytecode::Scalar, EmitError> {
        // require the storage type to be laid out as a scalar
        let storage = self.optimized.tree.storage_type(ty);
        let layout = self.layout(storage)?;
        let scalar = match layout.representation {
            mir::Representation::Scalar(scalar) => scalar,
            _ => return Err(self.unsupported_type()),
        };

        // pick the signedness and width from the MIR type
        match self.optimized.tree.get(storage) {
            mir::Type::Boolean => Ok(bytecode::Scalar::Boolean),
            mir::Type::Character => Ok(bytecode::Scalar::Uint32),
            mir::Type::Int { is_signed, .. } => self.integer_scalar(scalar.bit_width(), *is_signed),
            mir::Type::Isize => self.integer_scalar(scalar.bit_width(), true),
            mir::Type::Usize | mir::Type::TypeDescriptor => {
                self.integer_scalar(scalar.bit_width(), false)
            }
            mir::Type::Float(format) => Ok(self.float(*format)),
            mir::Type::TypeId => Ok(bytecode::Scalar::Uint32),
            _ => Err(self.unsupported_type()),
        }
    }

    /// Return one uninitialized bytecode representation.
    fn uninitialized(&self, ty: mir::TypeId) -> Result<bytecode::ValueType, EmitError> {
        match self
            .optimized
            .tree
            .get(self.optimized.tree.storage_type(ty))
        {
            mir::Type::Reference { kind, storage, .. } => {
                Ok(bytecode::ValueType::uninit_reference(
                    self.reference_kind(*kind),
                    self.storage(*storage),
                ))
            }
            mir::Type::Slice {
                kind,
                storage,
                element,
                ..
            } => Ok(bytecode::ValueType::uninit_slice(
                self.type_id(*element)?,
                self.reference_kind(*kind),
                self.storage(*storage),
            )),
            _ => Err(self.unsupported_type()),
        }
    }

    /// Return the exact bytecode integer representation.
    fn integer(
        &self,
        layout: &mir::Layout,
        is_signed: bool,
    ) -> Result<bytecode::ValueType, EmitError> {
        let mir::Representation::Scalar(scalar) = layout.representation else {
            return Err(self.unsupported_type());
        };

        let width = scalar.bit_width();

        // widest integers get their own bytecode representations
        if width == 128 && is_signed {
            Ok(bytecode::ValueType::int128())
        }
        // unsigned counterpart
        else if width == 128 {
            Ok(bytecode::ValueType::uint128())
        }
        // everything narrower fits one scalar
        else {
            Ok(bytecode::ValueType::scalar(
                self.integer_scalar(width, is_signed)?,
            ))
        }
    }

    /// Return one scalar integer representation.
    pub(crate) fn integer_scalar(
        &self,
        width: u16,
        is_signed: bool,
    ) -> Result<bytecode::Scalar, EmitError> {
        match (width, is_signed) {
            (1..=8, true) => Ok(bytecode::Scalar::Int8),
            (1..=8, false) => Ok(bytecode::Scalar::Uint8),
            (9..=16, true) => Ok(bytecode::Scalar::Int16),
            (9..=16, false) => Ok(bytecode::Scalar::Uint16),
            (17..=32, true) => Ok(bytecode::Scalar::Int32),
            (17..=32, false) => Ok(bytecode::Scalar::Uint32),
            (33..=64, true) => Ok(bytecode::Scalar::Int64),
            (33..=64, false) => Ok(bytecode::Scalar::Uint64),
            _ => Err(self.unsupported_type()),
        }
    }

    /// Return one bytecode floating point representation.
    fn float(&self, format: mir::FloatType) -> bytecode::Scalar {
        match format {
            mir::FloatType::Float32 => bytecode::Scalar::Float32,
            mir::FloatType::Float64 => bytecode::Scalar::Float64,
        }
    }

    /// Return one bytecode reference ownership.
    fn reference_kind(&self, kind: mir::ReferenceKind) -> bytecode::ReferenceKind {
        match kind {
            mir::ReferenceKind::Managed => bytecode::ReferenceKind::MANAGED,
            mir::ReferenceKind::Unique => bytecode::ReferenceKind::UNIQUE,
            mir::ReferenceKind::Borrowed => bytecode::ReferenceKind::BORROWED,
        }
    }

    /// Return one bytecode reference storage.
    fn storage(&self, storage: mir::Storage) -> bytecode::Storage {
        match storage {
            mir::Storage::LocalHeap => bytecode::Storage::LOCAL,
            mir::Storage::SharedHeap => bytecode::Storage::SHARED,
            mir::Storage::Constant => bytecode::Storage::CONSTANT,
            mir::Storage::Frame => bytecode::Storage::FRAME,
            mir::Storage::LocalStatic => bytecode::Storage::LOCAL_GLOBAL,
            mir::Storage::SharedStatic => bytecode::Storage::SHARED_GLOBAL,
        }
    }
}

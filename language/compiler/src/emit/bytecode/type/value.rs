use tspp_bytecode as bytecode;
use tspp_mir as mir;

use crate::EmitError;

use super::TypeEmitter;

impl TypeEmitter<'_> {
    /// Return the bytecode register representation for one MIR type.
    pub(crate) fn register_type(&self, ty: mir::TypeId) -> Result<bytecode::ValueType, EmitError> {
        // resolve the storage type behind the MIR type and its layout
        let representation = self.optimized.tree.storage_type(ty);
        let definition = self.optimized.tree.type_definition(representation);
        let layout = self.layout(representation)?;

        // map each MIR type onto its bytecode register representation
        let value_type = match definition {
            mir::Type::Never
            | mir::Type::Void
            | mir::Type::Null
            | mir::Type::Parameter { .. }
            | mir::Type::Witness { .. }
            | mir::Type::Declaration { .. } => {
                return Err(self.unsupported_type());
            }
            mir::Type::Boolean => bytecode::ValueType::scalar(bytecode::Scalar::Boolean),
            mir::Type::Character => bytecode::ValueType::scalar(bytecode::Scalar::Uint32),
            mir::Type::Int { is_signed, .. } => self.integer(layout, *is_signed)?,
            mir::Type::Isize => bytecode::ValueType::scalar(self.scalar(representation)?),
            mir::Type::Usize => bytecode::ValueType::scalar(self.scalar(representation)?),
            mir::Type::Float(format) => bytecode::ValueType::scalar(self.float(*format)),
            mir::Type::TypeId => bytecode::ValueType::type_id(),
            mir::Type::Reference { kind, .. } => bytecode::ValueType::reference(
                self.reference_kind(*kind),
                self.reference_storage(*kind),
            ),
            mir::Type::Pointer { .. } => bytecode::ValueType::pointer(),
            mir::Type::Slice { kind, element, .. } => bytecode::ValueType::slice(
                self.type_id(*element)?,
                self.reference_kind(*kind),
                self.reference_storage(*kind),
            ),
            mir::Type::Uninit { value } => self.uninitialized(*value)?,
            mir::Type::Dynamic {
                kind, constraint, ..
            } => bytecode::ValueType::dynamic(
                self.type_id(*constraint)?,
                bytecode::ReferenceType::new(
                    self.reference_kind(*kind),
                    self.reference_storage(*kind),
                ),
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
            mir::Type::Function { kind, .. } => {
                bytecode::ValueType::function(bytecode::ReferenceType::new(
                    self.reference_kind(*kind),
                    self.reference_storage(*kind),
                ))
            }
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
            mir::Type::ManuallyDrop { .. }
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
        match self.optimized.tree.type_definition(storage) {
            mir::Type::Boolean => Ok(bytecode::Scalar::Boolean),
            mir::Type::Character => Ok(bytecode::Scalar::Uint32),
            mir::Type::Int { is_signed, .. } => self.integer_scalar(scalar.bit_width(), *is_signed),
            mir::Type::Isize => self.integer_scalar(scalar.bit_width(), true),
            mir::Type::Usize => self.integer_scalar(scalar.bit_width(), false),
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
            mir::Type::Reference { kind, .. } => Ok(bytecode::ValueType::uninit_reference(
                self.reference_kind(*kind),
                self.reference_storage(*kind),
            )),
            mir::Type::Slice { kind, element, .. } => Ok(bytecode::ValueType::uninit_slice(
                self.type_id(*element)?,
                self.reference_kind(*kind),
                self.reference_storage(*kind),
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
        // require a scalar layout for the integer
        let mir::Representation::Scalar(scalar) = layout.representation else {
            return Err(self.unsupported_type());
        };

        let width = scalar.bit_width();

        // select the wide signed integer representation
        if width == 128 && is_signed {
            Ok(bytecode::ValueType::int128())
        }
        // select the wide unsigned integer representation
        else if width == 128 {
            Ok(bytecode::ValueType::uint128())
        }
        // select a scalar for narrower integers
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
    fn reference_kind(&self, kind: mir::Reference) -> bytecode::ReferenceKind {
        match kind {
            mir::Reference::Managed(_) => bytecode::ReferenceKind::MANAGED,
            mir::Reference::Unique => bytecode::ReferenceKind::UNIQUE,
            mir::Reference::Borrowed => bytecode::ReferenceKind::BORROWED,
            mir::Reference::Raw => bytecode::ReferenceKind::RAW,
        }
    }

    /// Return the bytecode storage one reference addresses, a borrow resolving it by address.
    fn reference_storage(&self, kind: mir::Reference) -> bytecode::Storage {
        kind.storage().map_or(bytecode::Storage::ANY, Self::storage)
    }

    /// Return one bytecode storage class.
    fn storage(storage: mir::Storage) -> bytecode::Storage {
        match storage {
            mir::Storage::Heap(mir::Space::Local) => bytecode::Storage::LOCAL,
            mir::Storage::Heap(mir::Space::Shared) => bytecode::Storage::SHARED,
            mir::Storage::Heap(mir::Space::Constant)
            | mir::Storage::Static(mir::Space::Constant) => bytecode::Storage::CONSTANT,
            mir::Storage::Frame => bytecode::Storage::FRAME,
            mir::Storage::Static(mir::Space::Local) => bytecode::Storage::LOCAL_GLOBAL,
            mir::Storage::Static(mir::Space::Shared) => bytecode::Storage::SHARED_GLOBAL,
        }
    }
}

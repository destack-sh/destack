use destack_artifact::MirOptimized;
use destack_bytecode as bytecode;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::{EmitError, ObjectEmitter};

/// Bytecode value representations and object-local type identities.
#[derive(Debug)]
pub(crate) struct TypeEmitter<'a> {
    /// Optimized MIR being emitted.
    optimized: &'a MirOptimized,
    /// Module owning the emitted MIR.
    module: ModuleId,
    /// Common object identity assignments.
    object: &'a ObjectEmitter,
}

impl<'a> TypeEmitter<'a> {
    /// Create one bytecode type emitter over common object identities.
    pub(crate) fn new(
        module: ModuleId,
        optimized: &'a MirOptimized,
        object: &'a ObjectEmitter,
    ) -> Self {
        Self {
            optimized,
            module,
            object,
        }
    }

    /// Return the object-local bytecode type id for one MIR type.
    pub(crate) fn type_id(&self, ty: mir::TypeId) -> Result<bytecode::TypeId, EmitError> {
        let index = self
            .object
            .type_index(ty)
            .ok_or_else(|| self.missing_type())?;

        Ok(bytecode::TypeId(index as u32))
    }

    /// Return the object-local bytecode function id for one MIR function.
    pub(crate) fn function_id(
        &self,
        function: mir::FunctionId,
    ) -> Result<bytecode::FunctionId, EmitError> {
        let index = self
            .object
            .function_index(function)
            .ok_or_else(|| self.missing("function"))?;

        Ok(bytecode::FunctionId(index as u32))
    }

    /// Return the object-local bytecode global id for one MIR global.
    pub(crate) fn global_id(&self, global: mir::GlobalId) -> Result<bytecode::GlobalId, EmitError> {
        let index = self
            .object
            .global_index(global)
            .ok_or_else(|| self.missing("global"))?;

        Ok(bytecode::GlobalId(index as u32))
    }

    /// Return the concrete layout for one MIR type.
    pub(crate) fn layout(&self, ty: mir::TypeId) -> Result<&mir::Layout, EmitError> {
        self.optimized
            .layouts
            .type_layout(ty)
            .ok_or_else(|| self.missing_type())
    }

    /// Return the exact runtime byte length of one MIR type.
    pub(crate) fn byte_len(&self, ty: mir::TypeId) -> Result<u32, EmitError> {
        Ok(self.layout(ty)?.size)
    }

    /// Return one source-ordered field layout.
    pub(crate) fn field(
        &self,
        ty: mir::TypeId,
        index: u32,
    ) -> Result<&mir::LayoutField, EmitError> {
        self.layout(ty)?
            .source_field(index)
            .ok_or_else(|| self.missing("field layout"))
    }

    /// Return one source-ordered aggregate placement.
    pub(crate) fn placement(&self, ty: mir::TypeId, index: u32) -> Result<(u32, u32), EmitError> {
        let layout = self.layout(ty)?;

        // project named and positional fields by source order
        if let Some(field) = layout.source_field(index) {
            return Ok((field.offset, field.size));
        }

        // project fixed indexed storage by its physical stride
        if layout.element().is_some() {
            return self.element(ty, index);
        }

        // transparent wrappers place their one backing value at byte zero
        if let mir::LayoutShape::Newtype(backing) = &layout.shape {
            if index != 0 {
                return Err(self.missing("newtype placement"));
            }
            let byte_len = self.byte_len(backing.backing_type)?;

            return Ok((0, byte_len));
        }

        Err(self.missing("aggregate placement"))
    }

    /// Return one fixed element's byte offset and stored byte length.
    pub(crate) fn element(&self, ty: mir::TypeId, index: u32) -> Result<(u32, u32), EmitError> {
        let element = self
            .layout(ty)?
            .element()
            .ok_or_else(|| self.missing("element layout"))?;
        let byte_offset = element.stride * index;
        let byte_len = self.byte_len(element.element)?;

        Ok((byte_offset, byte_len))
    }

    /// Return one variant case payload's byte offset and stored byte length.
    pub(crate) fn variant(&self, ty: mir::TypeId, case: u32) -> Result<(u32, u32), EmitError> {
        let mir::LayoutShape::Variant(layout) = &self.layout(ty)?.shape else {
            return Err(self.missing("variant layout"));
        };
        let case = layout
            .cases
            .get(case as usize)
            .ok_or_else(|| self.missing("variant case layout"))?;
        let byte_len = self.byte_len(case.ty)?;

        Ok((case.payload_offset, byte_len))
    }

    /// Return the bytecode register representation for one MIR type.
    pub(crate) fn register_type(&self, ty: mir::TypeId) -> Result<bytecode::ValueType, EmitError> {
        let representation = self.storage_type(ty);
        let definition = self.optimized.tree.get(representation);
        let value_type = match definition {
            mir::Type::Never | mir::Type::Void => return Err(self.unsupported_type()),
            mir::Type::Boolean => bytecode::ValueType::scalar(bytecode::Scalar::Boolean),
            mir::Type::Character => bytecode::ValueType::scalar(bytecode::Scalar::Uint32),
            mir::Type::Int { width, is_signed } => self.integer(*width, *is_signed)?,
            mir::Type::Isize => bytecode::ValueType::scalar(bytecode::Scalar::Int64),
            mir::Type::Usize
            | mir::Type::TypeDescriptor
            | mir::Type::Continuation { .. }
            | mir::Type::Waiter { .. } => bytecode::ValueType::scalar(bytecode::Scalar::Uint64),
            mir::Type::Float(format) => bytecode::ValueType::scalar(self.float(*format)),
            mir::Type::TypeId => bytecode::ValueType::type_id(),
            mir::Type::Reference { kind, storage, .. } => {
                bytecode::ValueType::reference(self.reference_kind(*kind)?, self.storage(*storage))
            }
            mir::Type::Slice {
                kind,
                element,
                storage,
                ..
            } => bytecode::ValueType::slice(
                self.type_id(*element)?,
                self.reference_kind(*kind)?,
                self.storage(*storage),
            ),
            mir::Type::Uninit { value } => self.uninitialized(*value)?,
            mir::Type::Dynamic { constraint, space } => {
                bytecode::ValueType::dynamic(self.type_id(*constraint)?, self.space(*space))
            }
            mir::Type::Vector { element, lanes, .. } => {
                let scalar = self.scalar(*element)?;
                let lane_count = u16::try_from(*lanes).map_err(|_| self.unsupported_type())?;

                bytecode::ValueType::vector(bytecode::VectorType::new(scalar, lane_count))
            }
            mir::Type::Tensor { element, space, .. } => bytecode::ValueType::tensor(
                self.scalar(*element)?,
                self.type_id(representation)?,
                self.space(*space),
            ),
            mir::Type::TensorView {
                kind,
                element,
                storage,
                shape,
                ..
            } => {
                let rank = u16::try_from(shape.len()).map_err(|_| self.unsupported_type())?;
                let word_count = bytecode::ValueType::tensor_view_word_count(rank)
                    .ok_or_else(|| self.unsupported_type())?;

                bytecode::ValueType::tensor_view(
                    self.scalar(*element)?,
                    self.type_id(representation)?,
                    bytecode::ReferenceType::new(
                        self.reference_kind(*kind)?,
                        self.storage(*storage),
                    ),
                    word_count,
                )
            }
            mir::Type::Function { environment, .. } => {
                let environment = self.register_type(*environment)?;
                let reference = environment
                    .reference_type()
                    .ok_or_else(|| self.unsupported_type())?;

                bytecode::ValueType::function(reference)
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
            mir::Type::Atomic { .. }
            | mir::Type::ManuallyDrop { .. }
            | mir::Type::WithLifetimes { .. }
            | mir::Type::Error
            | mir::Type::FunctionSignature { .. } => return Err(self.unsupported_type()),
        };

        Ok(value_type)
    }

    /// Return the scalar bytecode representation for one MIR type.
    pub(crate) fn scalar(&self, ty: mir::TypeId) -> Result<bytecode::Scalar, EmitError> {
        let storage = self.storage_type(ty);
        match self.optimized.tree.get(storage) {
            mir::Type::Boolean => Ok(bytecode::Scalar::Boolean),
            mir::Type::Character => Ok(bytecode::Scalar::Uint32),
            mir::Type::Int { width, is_signed } => self.integer_scalar(*width, *is_signed),
            mir::Type::Isize => Ok(bytecode::Scalar::Int64),
            mir::Type::Usize | mir::Type::TypeDescriptor => Ok(bytecode::Scalar::Uint64),
            mir::Type::Float(format) => Ok(self.float(*format)),
            mir::Type::TypeId => Ok(bytecode::Scalar::Uint32),
            _ => Err(self.unsupported_type()),
        }
    }

    /// Return the transparent storage type for one MIR type.
    fn storage_type(&self, mut ty: mir::TypeId) -> mir::TypeId {
        loop {
            match self.optimized.tree.get(ty) {
                mir::Type::Atomic { value }
                | mir::Type::ManuallyDrop { value }
                | mir::Type::WithLifetimes { base: value, .. }
                | mir::Type::Newtype { inner: value, .. } => ty = *value,
                _ => return ty,
            }
        }
    }

    /// Return one uninitialized bytecode representation.
    fn uninitialized(&self, ty: mir::TypeId) -> Result<bytecode::ValueType, EmitError> {
        match self.optimized.tree.get(self.storage_type(ty)) {
            mir::Type::Reference { kind, storage, .. } => {
                Ok(bytecode::ValueType::uninit_reference(
                    self.reference_kind(*kind)?,
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
                self.reference_kind(*kind)?,
                self.storage(*storage),
            )),
            _ => Err(self.unsupported_type()),
        }
    }

    /// Return the exact bytecode integer representation.
    fn integer(&self, width: u16, is_signed: bool) -> Result<bytecode::ValueType, EmitError> {
        if width == 128 && is_signed {
            Ok(bytecode::ValueType::int128())
        } else if width == 128 {
            Ok(bytecode::ValueType::uint128())
        } else {
            Ok(bytecode::ValueType::scalar(
                self.integer_scalar(width, is_signed)?,
            ))
        }
    }

    /// Return one scalar integer representation.
    fn integer_scalar(&self, width: u16, is_signed: bool) -> Result<bytecode::Scalar, EmitError> {
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
            mir::FloatType::Float16 => bytecode::Scalar::Float16,
            mir::FloatType::Bfloat16 => bytecode::Scalar::Bfloat16,
            mir::FloatType::Float32 => bytecode::Scalar::Float32,
            mir::FloatType::Float64 => bytecode::Scalar::Float64,
        }
    }

    /// Return one bytecode reference ownership.
    fn reference_kind(
        &self,
        kind: mir::ReferenceKind,
    ) -> Result<bytecode::ReferenceKind, EmitError> {
        match kind {
            mir::ReferenceKind::Managed => Ok(bytecode::ReferenceKind::MANAGED),
            mir::ReferenceKind::Unique => Ok(bytecode::ReferenceKind::UNIQUE),
            mir::ReferenceKind::Borrowed => Ok(bytecode::ReferenceKind::BORROWED),
            mir::ReferenceKind::Raw => Ok(bytecode::ReferenceKind::RAW),
        }
    }

    /// Return one bytecode heap ownership domain.
    fn space(&self, space: mir::Space) -> bytecode::Space {
        match space {
            mir::Space::Local => bytecode::Space::LOCAL,
            mir::Space::Shared => bytecode::Space::SHARED,
        }
    }

    /// Return one bytecode reference storage.
    fn storage(&self, storage: mir::Storage) -> bytecode::Storage {
        match storage {
            mir::Storage::Heap(mir::Space::Local) => bytecode::Storage::LOCAL,
            mir::Storage::Heap(mir::Space::Shared) => bytecode::Storage::SHARED,
            mir::Storage::Frame => bytecode::Storage::FRAME,
            mir::Storage::Global(mir::GlobalStorage::Constant) => bytecode::Storage::CONSTANT,
            mir::Storage::Global(mir::GlobalStorage::Local) => bytecode::Storage::LOCAL_GLOBAL,
            mir::Storage::Global(mir::GlobalStorage::Shared) => bytecode::Storage::SHARED_GLOBAL,
        }
    }

    /// Return the register word width of one aggregate layout.
    fn word_count(&self, ty: mir::TypeId) -> Result<u16, EmitError> {
        let layout = self
            .optimized
            .layouts
            .type_layout(ty)
            .ok_or_else(|| self.missing_type())?;
        let words = layout.size.div_ceil(size_of::<u64>() as u32);

        u16::try_from(words).map_err(|_| self.unsupported_type())
    }

    /// Build one missing type diagnostic.
    fn missing_type(&self) -> EmitError {
        EmitError::MissingType {
            anchor: self.module.into(),
            module: self.module,
        }
    }

    /// Build one missing common object item diagnostic.
    fn missing(&self, item: &str) -> EmitError {
        EmitError::UnexpectedConstruct {
            anchor: self.module.into(),
            module: self.module,
            message: format!("{item} is absent from the common object"),
        }
    }

    /// Build one unsupported type diagnostic.
    fn unsupported_type(&self) -> EmitError {
        EmitError::UnsupportedType {
            anchor: self.module.into(),
            module: self.module,
        }
    }
}

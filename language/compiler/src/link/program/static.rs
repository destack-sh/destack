use std::collections::HashMap;

use destack_core::{SectionPacker, float_from_bits, float_to_bits};
use destack_mir as mir;
use destack_program::vm::FunctionPointer;
use destack_program::{Global, GlobalAllocator, GlobalLocation, GlobalTable, StaticImage};

use crate::LinkResult;

use super::ProgramLinker;
use super::vm::StorageLayout;

/// Linked static memory spaces for one program.
#[derive(Debug)]
pub(crate) struct ProgramStatics {
    /// Executable global metadata.
    pub(crate) globals: GlobalTable,
    /// Immutable program constants.
    pub(crate) constants: StaticImage,
    /// Shared mutable program statics.
    pub(crate) shared: StaticImage,
    /// Local mutable program statics.
    pub(crate) local: StaticImage,
}

/// Link MIR globals into program static spaces.
#[derive(Debug)]
pub(crate) struct StaticLinker<'a> {
    /// MIR tree being linked.
    tree: &'a mir::Tree,
    /// Target ABI layout for static scalar encoding.
    target_layout: &'a mir::TargetLayout,
    /// Program linker owning dense program id projection.
    program: &'a ProgramLinker,
    /// Executable value storage layouts by MIR type id.
    layouts: &'a HashMap<mir::TypeId, StorageLayout>,
}

/// One initialized byte range inside a global payload.
#[derive(Clone, Copy, Debug)]
struct InitializerRange {
    /// The value type for this range.
    ty: mir::TypeId,
    /// The byte offset inside the payload.
    offset: usize,
    /// The byte width of this range.
    byte_len: usize,
}

impl<'a> StaticLinker<'a> {
    /// Create one static linker.
    pub(crate) fn new(
        tree: &'a mir::Tree,
        target_layout: &'a mir::TargetLayout,
        program: &'a ProgramLinker,
        layouts: &'a HashMap<mir::TypeId, StorageLayout>,
    ) -> Self {
        Self {
            tree,
            target_layout,
            program,
            layouts,
        }
    }

    /// Link constant, shared static, and local static spaces.
    pub(crate) fn link(&self, sections: &mut SectionPacker) -> LinkResult<ProgramStatics> {
        let mut constants = GlobalAllocator::new();
        let mut shared = GlobalAllocator::new();
        let mut local = GlobalAllocator::new();
        let mut globals = Vec::new();

        // split globals by MIR placement
        for (global_id, global) in self.tree.iter_nodes::<mir::Global>() {
            if global.is_import() {
                continue;
            }

            let ty = global.ty;
            let layout = self.layouts.get(&ty).ok_or_else(|| {
                self.program
                    .type_mismatch("program global layout", format!("{ty:?}"))
            })?;
            let bytes = match global.initializer.as_ref() {
                Some(initializer) => self.initializer_bytes(initializer, ty)?,
                None => vec![0; layout.byte_len],
            };

            match global.space {
                mir::Space::Static => {
                    self.define_global_bytes(
                        &mut constants,
                        &mut globals,
                        GlobalLocation::Constant,
                        global_id,
                        ty,
                        layout.alignment(),
                        false,
                        &bytes,
                    )?;
                }
                mir::Space::Shared => {
                    self.define_global_bytes(
                        &mut shared,
                        &mut globals,
                        GlobalLocation::SharedStatic,
                        global_id,
                        ty,
                        layout.alignment(),
                        global.is_mutable(),
                        &bytes,
                    )?;
                }
                mir::Space::Local => {
                    self.define_global_bytes(
                        &mut local,
                        &mut globals,
                        GlobalLocation::LocalStatic,
                        global_id,
                        ty,
                        layout.alignment(),
                        global.is_mutable(),
                        &bytes,
                    )?;
                }
                mir::Space::Frame => {
                    return Err(self
                        .program
                        .invalid_input(format!("global {global_id:?} cannot use frame storage")));
                }
            }
        }

        Ok(ProgramStatics {
            globals: GlobalTable::pack(sections, globals),
            constants: constants.finish(sections),
            shared: shared.finish(sections),
            local: local.finish(sections),
        })
    }

    /// Encode one static initializer into bytes.
    fn initializer_bytes(
        &self,
        initializer: &mir::GlobalInitializer,
        ty: mir::TypeId,
    ) -> LinkResult<Vec<u8>> {
        let layout = self.layouts.get(&ty).ok_or_else(|| {
            self.program
                .type_mismatch("program initializer layout", format!("{ty:?}"))
        })?;

        if layout.is_scalar() {
            return self.scalar_initializer_bytes(initializer, ty, layout.byte_len);
        }

        match initializer {
            mir::GlobalInitializer::Zero => {
                self.validate_zero_initializer(ty)?;

                Ok(vec![0; layout.byte_len])
            }
            mir::GlobalInitializer::Bytes(bytes) => {
                if bytes.len() != layout.byte_len {
                    return Err(self.program.type_mismatch(
                        format!("{} initializer bytes", layout.byte_len),
                        format!("{} initializer bytes", bytes.len()),
                    ));
                }

                Ok(bytes.clone())
            }
            mir::GlobalInitializer::Aggregate(elements) => {
                self.payload_initializer_bytes(elements, ty)
            }
            mir::GlobalInitializer::Scalar(_) | mir::GlobalInitializer::FunctionAddress(_) => {
                Err(self
                    .program
                    .type_mismatch("payload initializer", "scalar initializer"))
            }
        }
    }

    /// Encode one scalar initializer into bytes.
    fn scalar_initializer_bytes(
        &self,
        initializer: &mir::GlobalInitializer,
        ty: mir::TypeId,
        byte_len: usize,
    ) -> LinkResult<Vec<u8>> {
        match initializer {
            mir::GlobalInitializer::Zero => {
                self.validate_zero_scalar_type(ty)?;

                Ok(vec![0; byte_len])
            }
            mir::GlobalInitializer::Bytes(bytes) => {
                if bytes.len() != byte_len {
                    return Err(self.program.type_mismatch(
                        format!("{byte_len} initializer bytes"),
                        format!("{} initializer bytes", bytes.len()),
                    ));
                }

                Ok(bytes.clone())
            }
            mir::GlobalInitializer::Scalar(constant) => {
                let bytes = self.constant_scalar_bytes(constant, ty, byte_len)?;

                Ok(bytes)
            }
            mir::GlobalInitializer::FunctionAddress(function) => {
                let bytes = self.function_address_initializer_bytes(*function, ty, byte_len)?;

                Ok(bytes)
            }
            mir::GlobalInitializer::Aggregate(_) => Err(self
                .program
                .type_mismatch("scalar initializer", format!("{ty:?}"))),
        }
    }

    /// Encode one function address initializer as bytes.
    fn function_address_initializer_bytes(
        &self,
        function: mir::FunctionId,
        ty: mir::TypeId,
        byte_len: usize,
    ) -> LinkResult<Vec<u8>> {
        let ty_node = self.tree.get(self.tree.repr_type(ty));
        let mir::Type::FunctionPointer { .. } = ty_node else {
            return Err(self
                .program
                .type_mismatch("function pointer initializer", format!("{ty_node:?}")));
        };

        let expected_byte_len = usize::from(self.target_layout.pointer_bytes());
        if byte_len != expected_byte_len {
            return Err(self.program.type_mismatch(
                format!("{expected_byte_len} byte function pointer initializer"),
                format!("{byte_len} byte initializer"),
            ));
        }

        let function = FunctionPointer::from(self.program.function_id(function));
        let bytes = self.unsigned_bytes(function.bits() as u128, byte_len);

        Ok(bytes)
    }

    /// Encode one scalar initializer according to the declared type.
    fn constant_scalar_bytes(
        &self,
        constant: &mir::Constant,
        ty: mir::TypeId,
        byte_len: usize,
    ) -> LinkResult<Vec<u8>> {
        let ty = self.tree.repr_type(ty);
        let ty_node = self.tree.get(ty);

        match ty_node {
            mir::Type::Boolean => self.boolean_constant_bytes(constant, byte_len),
            mir::Type::Int { width, is_signed } => {
                self.integer_constant_bytes(constant, *width, *is_signed, byte_len)
            }
            mir::Type::Isize => {
                self.validate_pointer_byte_len(byte_len)?;
                let width = self.target_layout.pointer_bits();

                self.integer_constant_bytes(constant, width, true, byte_len)
            }
            mir::Type::Usize | mir::Type::TypeId => {
                self.validate_pointer_byte_len(byte_len)?;
                let width = self.target_layout.pointer_bits();

                self.integer_constant_bytes(constant, width, false, byte_len)
            }
            mir::Type::Float(format) => self.float_constant_bytes(constant, *format, byte_len),
            mir::Type::Reference { nullability, .. } => {
                self.reference_constant_bytes(constant, *nullability, byte_len)
            }
            _ => Err(self
                .program
                .type_mismatch("scalar initializer type", format!("{ty_node:?}"))),
        }
    }

    /// Encode one boolean constant.
    fn boolean_constant_bytes(
        &self,
        constant: &mir::Constant,
        byte_len: usize,
    ) -> LinkResult<Vec<u8>> {
        let mir::Constant::Boolean { value } = constant else {
            return Err(self
                .program
                .type_mismatch("boolean initializer", format!("{constant:?}")));
        };

        if byte_len != 1 {
            return Err(self
                .program
                .type_mismatch("1 byte boolean initializer", format!("{byte_len} bytes")));
        }

        Ok(vec![u8::from(*value)])
    }

    /// Encode one integer-like constant.
    fn integer_constant_bytes(
        &self,
        constant: &mir::Constant,
        width: u16,
        is_signed: bool,
        byte_len: usize,
    ) -> LinkResult<Vec<u8>> {
        self.validate_integer_byte_len(width, byte_len)?;

        if is_signed {
            let value = self.signed_constant_value(constant, width)?;

            return Ok(self.signed_bytes(value, byte_len));
        }

        let value = self.unsigned_constant_value(constant, width)?;

        Ok(self.unsigned_bytes(value, byte_len))
    }

    /// Return one signed integer constant after declared-type range checks.
    fn signed_constant_value(&self, constant: &mir::Constant, width: u16) -> LinkResult<i128> {
        let value = match constant {
            mir::Constant::Int { value, .. } => *value,
            mir::Constant::UInt { value, .. } => i128::try_from(*value).map_err(|_| {
                self.program.type_mismatch(
                    format!("signed {width} bit initializer"),
                    format!("{value}"),
                )
            })?,
            mir::Constant::Char { value } => i128::from(*value as u32),
            _ => {
                return Err(self.program.type_mismatch(
                    format!("signed {width} bit initializer"),
                    format!("{constant:?}"),
                ));
            }
        };

        if !Self::signed_value_fits(value, width) {
            return Err(self.program.type_mismatch(
                format!("signed {width} bit initializer"),
                format!("{value}"),
            ));
        }

        Ok(value)
    }

    /// Return one unsigned integer constant after declared-type range checks.
    fn unsigned_constant_value(&self, constant: &mir::Constant, width: u16) -> LinkResult<u128> {
        let value = match constant {
            mir::Constant::UInt { value, .. } => *value,
            mir::Constant::Int { value, .. } if *value >= 0 => *value as u128,
            mir::Constant::Char { value } => u128::from(*value as u32),
            _ => {
                return Err(self.program.type_mismatch(
                    format!("unsigned {width} bit initializer"),
                    format!("{constant:?}"),
                ));
            }
        };

        if !Self::unsigned_value_fits(value, width) {
            return Err(self.program.type_mismatch(
                format!("unsigned {width} bit initializer"),
                format!("{value}"),
            ));
        }

        Ok(value)
    }

    /// Encode one floating-point constant.
    fn float_constant_bytes(
        &self,
        constant: &mir::Constant,
        format: mir::FloatType,
        byte_len: usize,
    ) -> LinkResult<Vec<u8>> {
        let mir::Constant::Float {
            bits,
            format: source,
        } = constant
        else {
            return Err(self.program.type_mismatch(
                format!("{} initializer", format.label()),
                format!("{constant:?}"),
            ));
        };

        self.validate_float_byte_len(format, byte_len)?;

        let raw = if *source == format {
            *bits
        } else {
            let value = float_from_bits(source.format(), *bits);

            float_to_bits(format.format(), value)
        };

        Ok(self.unsigned_bytes(raw.into(), byte_len))
    }

    /// Encode one reference constant.
    fn reference_constant_bytes(
        &self,
        constant: &mir::Constant,
        nullability: mir::Nullability,
        byte_len: usize,
    ) -> LinkResult<Vec<u8>> {
        let mir::Constant::Null = constant else {
            return Err(self
                .program
                .type_mismatch("reference initializer", format!("{constant:?}")));
        };

        if !nullability.allows_null() {
            return Err(self
                .program
                .unsupported_zero_initializer("non-null reference"));
        }

        Ok(vec![0; byte_len])
    }

    /// Validate integer initializer storage width.
    fn validate_integer_byte_len(&self, width: u16, byte_len: usize) -> LinkResult<()> {
        let expected_byte_len = self.integer_byte_len(width)?;
        if byte_len != expected_byte_len {
            return Err(self.program.type_mismatch(
                format!("{expected_byte_len} byte integer initializer"),
                format!("{byte_len} byte initializer"),
            ));
        }

        Ok(())
    }

    /// Validate pointer-sized initializer storage width.
    fn validate_pointer_byte_len(&self, byte_len: usize) -> LinkResult<()> {
        let expected_byte_len = usize::from(self.target_layout.pointer_bytes());
        if byte_len != expected_byte_len {
            return Err(self.program.type_mismatch(
                format!("{expected_byte_len} byte pointer initializer"),
                format!("{byte_len} byte initializer"),
            ));
        }

        Ok(())
    }

    /// Validate floating-point initializer storage width.
    fn validate_float_byte_len(&self, format: mir::FloatType, byte_len: usize) -> LinkResult<()> {
        let expected_byte_len = self.integer_byte_len(format.width())?;
        if byte_len != expected_byte_len {
            return Err(self.program.type_mismatch(
                format!("{expected_byte_len} byte {} initializer", format.label()),
                format!("{byte_len} byte initializer"),
            ));
        }

        Ok(())
    }

    /// Return the byte width for one scalar bit width.
    fn integer_byte_len(&self, width: u16) -> LinkResult<usize> {
        if width == 0 {
            return Err(self.program.type_mismatch("nonzero scalar width", "0 bits"));
        }

        let width = usize::from(width);

        Ok(width.div_ceil(8))
    }

    /// Return whether one signed value fits the requested bit width.
    fn signed_value_fits(value: i128, width: u16) -> bool {
        if width == 0 {
            return value == 0;
        }

        if width >= 128 {
            return true;
        }

        let min = -(1i128 << (width - 1));
        let max = (1i128 << (width - 1)) - 1;

        value >= min && value <= max
    }

    /// Return whether one unsigned value fits the requested bit width.
    fn unsigned_value_fits(value: u128, width: u16) -> bool {
        if width == 0 {
            return value == 0;
        }

        if width >= 128 {
            return true;
        }

        value <= ((1u128 << width) - 1)
    }

    /// Encode one signed integer in target byte order.
    fn signed_bytes(&self, value: i128, byte_len: usize) -> Vec<u8> {
        let mut bytes = vec![if value < 0 { 0xff } else { 0 }; byte_len];
        let source = match self.target_layout.endian {
            mir::Endian::Little => value.to_le_bytes(),
            mir::Endian::Big => value.to_be_bytes(),
        };
        let copied = source.len().min(byte_len);
        match self.target_layout.endian {
            mir::Endian::Little => bytes[..copied].copy_from_slice(&source[..copied]),
            mir::Endian::Big => {
                let source_start = source.len() - copied;
                let target_start = byte_len - copied;
                bytes[target_start..].copy_from_slice(&source[source_start..]);
            }
        }

        bytes
    }

    /// Encode one unsigned integer in target byte order.
    fn unsigned_bytes(&self, value: u128, byte_len: usize) -> Vec<u8> {
        let mut bytes = vec![0; byte_len];
        let source = match self.target_layout.endian {
            mir::Endian::Little => value.to_le_bytes(),
            mir::Endian::Big => value.to_be_bytes(),
        };
        let copied = source.len().min(byte_len);
        match self.target_layout.endian {
            mir::Endian::Little => bytes[..copied].copy_from_slice(&source[..copied]),
            mir::Endian::Big => {
                let source_start = source.len() - copied;
                let target_start = byte_len - copied;
                bytes[target_start..].copy_from_slice(&source[source_start..]);
            }
        }

        bytes
    }

    /// Validate one zero initializer against the declared type.
    fn validate_zero_initializer(&self, ty: mir::TypeId) -> LinkResult<()> {
        let layout = self.layouts.get(&ty).ok_or_else(|| {
            self.program
                .type_mismatch("program initializer layout", format!("{ty:?}"))
        })?;

        if layout.is_scalar() {
            self.validate_zero_scalar_type(ty)?;

            return Ok(());
        }

        for range in self.initializer_ranges(ty)? {
            self.validate_zero_initializer(range.ty)?;
        }

        Ok(())
    }

    /// Validate whether one scalar type accepts a zero initializer.
    fn validate_zero_scalar_type(&self, ty: mir::TypeId) -> LinkResult<()> {
        let ty = self.tree.repr_type(ty);
        let ty_node = self.tree.get(ty).clone();

        match ty_node {
            mir::Type::Void
            | mir::Type::Int { .. }
            | mir::Type::Isize
            | mir::Type::Usize
            | mir::Type::Float(_)
            | mir::Type::Boolean
            | mir::Type::TypeId => Ok(()),
            mir::Type::Reference {
                kind: _,
                space: _,
                access: _,
                nullability,
                ..
            } => {
                if !nullability.allows_null() {
                    return Err(self
                        .program
                        .unsupported_zero_initializer(format!("{ty_node:?}")));
                }

                Ok(())
            }
            _ => Err(self
                .program
                .unsupported_zero_initializer(format!("{ty_node:?}"))),
        }
    }

    /// Encode one payload initializer into bytes.
    fn payload_initializer_bytes(
        &self,
        elements: &[mir::GlobalInitializer],
        ty: mir::TypeId,
    ) -> LinkResult<Vec<u8>> {
        let layout = self.layouts.get(&ty).ok_or_else(|| {
            self.program
                .type_mismatch("program payload layout", format!("{ty:?}"))
        })?;
        let ranges = self.initializer_ranges(ty)?;
        if elements.len() != ranges.len() {
            return Err(self.program.type_mismatch(
                format!("{} initializer elements", ranges.len()),
                format!("{} initializer elements", elements.len()),
            ));
        }

        let mut bytes = vec![0u8; layout.byte_len];
        for (element, range) in elements.iter().zip(ranges) {
            let value_bytes = self.initializer_bytes(element, range.ty)?;
            if value_bytes.len() != range.byte_len {
                return Err(self.program.type_mismatch(
                    format!("{} initializer bytes", range.byte_len),
                    format!("{} initializer bytes", value_bytes.len()),
                ));
            }

            let end = range
                .offset
                .checked_add(range.byte_len)
                .ok_or_else(|| self.program.layout_overflow("initializer byte range"))?;
            let target = bytes
                .get_mut(range.offset..end)
                .ok_or_else(|| self.program.layout_overflow("initializer byte range"))?;
            target.copy_from_slice(&value_bytes);
        }

        Ok(bytes)
    }

    /// Return initializer byte ranges for one payload type.
    fn initializer_ranges(&self, ty: mir::TypeId) -> LinkResult<Vec<InitializerRange>> {
        let layout = self.layouts.get(&ty).ok_or_else(|| {
            self.program
                .type_mismatch("program payload layout", format!("{ty:?}"))
        })?;

        if let Some(field_count) = layout.field_count() {
            let mut ranges = Vec::with_capacity(field_count);
            for index in 0..field_count {
                let field = layout
                    .field(index as u32)
                    .ok_or_else(|| self.program.invalid_field_access(index as u32, field_count))?;
                ranges.push(InitializerRange {
                    ty: field.ty,
                    offset: field.offset,
                    byte_len: field.byte_len,
                });
            }

            return Ok(ranges);
        }

        let element = layout.element().ok_or_else(|| {
            self.program
                .type_mismatch("indexed initializer layout", format!("{ty:?}"))
        })?;
        let element_count = layout.element_count().ok_or_else(|| {
            self.program
                .invalid_input("indexed initializer element count")
        })?;
        let mut ranges = Vec::with_capacity(element_count);
        for index in 0..element_count {
            let offset = element
                .stride
                .checked_mul(index)
                .ok_or_else(|| self.program.layout_overflow("initializer element offset"))?;
            ranges.push(InitializerRange {
                ty: element.ty,
                offset,
                byte_len: element.byte_len,
            });
        }

        Ok(ranges)
    }

    /// Define one global byte region in program static memory.
    fn define_global_bytes(
        &self,
        allocator: &mut GlobalAllocator,
        globals: &mut Vec<Option<Global>>,
        location: GlobalLocation,
        global: mir::GlobalId,
        ty: mir::TypeId,
        alignment: usize,
        is_mutable: bool,
        bytes: &[u8],
    ) -> LinkResult<()> {
        let global_id = self.program.global_id(global);
        let index = global_id.index();
        if globals.len() <= index {
            globals.resize(index + 1, None);
        }
        if globals[index].is_some() {
            return Err(self
                .program
                .internal(format!("duplicate program global {global:?}")));
        }

        let (offset, byte_len) = allocator.allocate(alignment, bytes);
        globals[index] = Some(Global::new(
            location,
            offset,
            byte_len,
            self.program.type_id(ty),
            is_mutable,
        ));

        Ok(())
    }
}

use std::collections::HashMap;
use std::sync::Arc;

use tspp_core::{StringId, float_from_bits, float_to_bits};
use tspp_mir as mir;
use tspp_program as program;
use tspp_program::{
    BIGINT_ITEM, BIGINT_LENGTH_FIELD, BIGINT_LIMBS_FIELD, BIGINT_SIGN_FIELD, Global,
    GlobalAllocator, GlobalId, GlobalLocation, GlobalTableBuilder, Object, STRING_ITEM,
    STRING_UNITS_FIELD, StaticBytes, Symbol, TypeId,
};
use tspp_source::ModuleId;

use crate::CompilerResult;

use super::{ProgramLinker, TypeLinker};
use crate::{invalid_program_input, layout_overflow, type_mismatch, unsupported_zero_initializer};

/// Linked static memory spaces for one program.
#[derive(Debug)]
pub(crate) struct ProgramStatics {
    /// Program global table.
    pub(crate) globals: GlobalTableBuilder,
    /// Immutable program constants.
    pub(crate) constants: StaticBytes,
    /// Shared mutable program statics.
    pub(crate) shared: StaticBytes,
    /// Local mutable program statics.
    pub(crate) local: StaticBytes,
}

/// Link MIR globals into program static spaces.
#[derive(Debug)]
pub(crate) struct StaticLinker<'a> {
    /// Program linker owning dense program id projection.
    program: &'a ProgramLinker<'a>,
}

/// Link one module's MIR globals into program static spaces.
#[derive(Debug)]
struct GlobalLinker<'a> {
    /// Module that owns the MIR identities being linked.
    module: ModuleId,
    /// Object containing the global and type declarations being linked.
    object: &'a Object,
    /// Target ABI layout for static scalar encoding.
    target_layout: mir::TargetLayout,
    /// Program linker owning dense program id projection.
    program: &'a ProgramLinker<'a>,
    /// Complete MIR target layouts.
    layouts: &'a mir::LayoutTable,
}

/// Rendering state threaded through one global's initializer encoding.
struct GlobalRender<'a, 'b> {
    /// The space of the global being rendered.
    space: mir::Space,
    /// Every placed program global in dense id order.
    placements: &'a [(Symbol, Global)],
    /// Static address words rebased at materialization.
    relocations: &'b mut Vec<(usize, GlobalLocation)>,
}

impl GlobalRender<'_, '_> {
    /// Reborrow this render state for one nested range.
    fn reborrow(&mut self) -> GlobalRender<'_, '_> {
        GlobalRender {
            space: self.space,
            placements: self.placements,
            relocations: self.relocations,
        }
    }
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
    pub(crate) fn new(program: &'a ProgramLinker<'a>) -> Self {
        Self { program }
    }

    /// Link constant, shared static, and local static spaces.
    pub(crate) fn link(&self) -> CompilerResult<ProgramStatics> {
        let mut constants = GlobalAllocator::new();
        let mut shared = GlobalAllocator::new();
        let mut local = GlobalAllocator::new();
        let mut globals = Vec::new();

        // place every canonical global definition in its selected static region
        for &(module, global_id) in self.program.globals_by_id() {
            let object = self.program.object(module);
            let global = object
                .global(global_id)
                .ok_or_else(|| invalid_program_input(format!("missing global {global_id:?}")))?;
            let symbol = Symbol::from_raw(global.symbol.raw());

            let linker = GlobalLinker::new(
                module,
                object,
                object.target(),
                self.program,
                object.layouts(),
            );
            let global = linker.place_global(global, &mut constants, &mut shared, &mut local)?;
            globals.push((symbol, global));
        }

        // render initializer bytes into the placed ranges with cross-global addresses
        for (index, &(module, global_id)) in self.program.globals_by_id().iter().enumerate() {
            let object = self.program.object(module);
            let global = object
                .global(global_id)
                .ok_or_else(|| invalid_program_input(format!("missing global {global_id:?}")))?;

            let linker = GlobalLinker::new(
                module,
                object,
                object.target(),
                self.program,
                object.layouts(),
            );
            linker.render_global(
                global,
                &globals[index].1,
                &globals,
                &mut constants,
                &mut shared,
                &mut local,
            )?;
        }

        Ok(ProgramStatics {
            globals: GlobalTableBuilder::new().globals(globals),
            constants: constants.build(),
            shared: shared.build(),
            local: local.build(),
        })
    }
}

impl ProgramStatics {
    /// Return one linked global by final dense id.
    pub(crate) fn global(&self, global: GlobalId) -> Option<&Global> {
        self.globals.get(global)
    }
}

impl<'a> GlobalLinker<'a> {
    /// Create one global linker.
    fn new(
        module: ModuleId,
        object: &'a Object,
        target_layout: mir::TargetLayout,
        program: &'a ProgramLinker<'a>,
        layouts: &'a mir::LayoutTable,
    ) -> Self {
        Self {
            module,
            object,
            target_layout,
            program,
            layouts,
        }
    }

    /// Place one defined MIR global into its program static region.
    fn place_global(
        &self,
        global: &program::object::Global,
        constants: &mut GlobalAllocator,
        shared: &mut GlobalAllocator,
        local: &mut GlobalAllocator,
    ) -> CompilerResult<Global> {
        let ty = global.ty;
        let layout = self.layout(ty)?;
        let alignment = layout.alignment as usize;
        let byte_len = layout.byte_len();

        // reserve a zeroed range in the selected region
        let (allocator, is_mutable) = match global.space {
            mir::Space::Constant => (constants, false),
            mir::Space::Shared => (shared, global.is_mutable()),
            mir::Space::Local => (local, global.is_mutable()),
        };
        let offset = allocator.reserve(alignment, byte_len);

        Ok(Global::new(
            Self::location(global.space),
            offset,
            byte_len,
            self.program.type_id(self.module, ty),
            is_mutable,
        ))
    }

    /// Render one placed global's initializer bytes into its region.
    fn render_global(
        &self,
        global: &program::object::Global,
        placed: &Global,
        placements: &[(Symbol, Global)],
        constants: &mut GlobalAllocator,
        shared: &mut GlobalAllocator,
        local: &mut GlobalAllocator,
    ) -> CompilerResult<()> {
        // reserved ranges stay zeroed without an initializer
        let Some(initializer) = global.initializer.as_ref() else {
            return Ok(());
        };

        // encode value constants into the constant region directly
        if matches!(
            initializer,
            mir::GlobalInitializer::String(_) | mir::GlobalInitializer::BigInt(_)
        ) {
            return self.render_value_constant(global, placed, initializer, constants);
        }

        // render the payload with region-relative address words
        let mut relocations = Vec::new();
        let render = GlobalRender {
            space: global.space,
            placements,
            relocations: &mut relocations,
        };
        let bytes = self.initializer_bytes(initializer, global.ty, placed.offset(), render)?;

        // write the payload into its placed range
        let allocator = match global.space {
            mir::Space::Constant => constants,
            mir::Space::Shared => shared,
            mir::Space::Local => local,
        };
        allocator.write(placed.offset(), &bytes);

        // record address words in their source image
        for (byte_offset, location) in relocations {
            allocator.relocate(byte_offset, location);
        }

        Ok(())
    }

    /// Encode one value constant and its payload into the constant region.
    fn render_value_constant(
        &self,
        global: &program::object::Global,
        placed: &Global,
        initializer: &mir::GlobalInitializer,
        constants: &mut GlobalAllocator,
    ) -> CompilerResult<()> {
        let word_len = usize::from(self.target_layout.pointer_bytes());

        // select the language item key and payload units for this value form
        let (key, sign, units, unit_alignment) = match initializer {
            mir::GlobalInitializer::String(value) => {
                let text = self.program.strings().get(*value).to_string();
                let units: Vec<u8> = text
                    .encode_utf16()
                    .flat_map(|unit| unit.to_le_bytes())
                    .collect();

                (STRING_ITEM, None, units, 2)
            }
            mir::GlobalInitializer::BigInt(value) => {
                let limbs: Vec<u8> = match value {
                    0 => Vec::new(),
                    value => value.unsigned_abs().to_le_bytes().to_vec(),
                };

                (BIGINT_ITEM, Some(value.signum() as i8), limbs, 8)
            }
            _ => {
                return Err(type_mismatch("value initializer", "byte initializer"));
            }
        };

        // require the representation to declare the value form's language item
        let tagged = self
            .object
            .ty(global.ty)
            .and_then(|entry| entry.language_item)
            .is_some_and(|item| item == StringId::for_text(key));
        if !tagged {
            return Err(invalid_program_input(format!(
                "a value constant's representation does not declare '{key}'"
            )));
        }

        // place the payload units as anonymous constant bytes
        let length = (units.len() / unit_alignment) as u128;
        let units_offset = constants.reserve(unit_alignment, units.len());
        constants.write(units_offset, &units);

        // read the representation's fields
        let layout = self.layout(global.ty)?;
        let (fields, is_class) = match &layout.shape {
            mir::LayoutShape::Struct(shape) => (&shape.fields, false),
            mir::LayoutShape::Class(shape) => (&shape.fields, true),
            _ => {
                return Err(type_mismatch(
                    "struct or class representation",
                    format!("{:?}", global.ty),
                ));
            }
        };
        let mut bytes = vec![0; layout.byte_len()];
        let mut relocations = Vec::new();

        // write the virtual table id of a class object
        if is_class {
            let table = self
                .program
                .virtual_table_id(self.module, global.ty)
                .ok_or_else(|| {
                    invalid_program_input("constant class object has no virtual table")
                })?;
            let start = mir::VIRTUAL_TABLE_ID_OFFSET as usize;
            let end = start + mir::VIRTUAL_TABLE_ID_BYTES as usize;
            bytes[start..end].copy_from_slice(&table.0.to_le_bytes());
        }

        // write each representation field at its layout offset
        for field in fields {
            let name = field.name.map(StringId::raw);
            let offset = field.offset as usize;
            // write the payload address word and its unit count
            if name == Some(StringId::for_text(STRING_UNITS_FIELD).raw())
                || name == Some(StringId::for_text(BIGINT_LIMBS_FIELD).raw())
            {
                let address = self.unsigned_bytes(units_offset as u128, word_len);
                bytes[offset..offset + word_len].copy_from_slice(&address);
                relocations.push((placed.offset() + offset, GlobalLocation::Constant));
                let count = self.unsigned_bytes(length, word_len);
                bytes[offset + word_len..offset + 2 * word_len].copy_from_slice(&count);
            }
            // write the sign as one signed byte
            else if name == Some(StringId::for_text(BIGINT_SIGN_FIELD).raw()) {
                let Some(sign) = sign else {
                    return Err(type_mismatch(
                        "bigint representation",
                        "sign on a string representation",
                    ));
                };
                bytes[offset] = sign as u8;
            }
            // write the unit count on its own
            else if name == Some(StringId::for_text(BIGINT_LENGTH_FIELD).raw()) {
                let count = self.unsigned_bytes(length, word_len);
                bytes[offset..offset + word_len].copy_from_slice(&count);
            }
            // refuse representations with fields the encoder does not know
            else {
                return Err(invalid_program_input(
                    "a value representation declares an unencoded field".to_string(),
                ));
            }
        }

        // write the representation and record its address words
        constants.write(placed.offset(), &bytes);
        for (byte_offset, location) in relocations {
            constants.relocate(byte_offset, location);
        }

        Ok(())
    }

    /// Encode one static initializer into bytes.
    fn initializer_bytes(
        &self,
        initializer: &mir::GlobalInitializer,
        ty: mir::TypeId,
        offset: usize,
        render: GlobalRender<'_, '_>,
    ) -> CompilerResult<Vec<u8>> {
        let layout = self.layout(ty)?;

        // encode scalars through the scalar encoder
        if self.is_scalar(ty) {
            return self.scalar_initializer_bytes(initializer, ty, offset, render);
        }

        // encode slice headers as one address word and one length word
        if matches!(layout.shape, mir::LayoutShape::Slice) {
            return self.slice_initializer_bytes(initializer, offset, render, layout.byte_len());
        }

        match initializer {
            // value constants render through the constant encoder before this path
            mir::GlobalInitializer::String(_) | mir::GlobalInitializer::BigInt(_) => Err(
                invalid_program_input("a value initializer reached the byte renderer".to_string()),
            ),
            mir::GlobalInitializer::Zero => {
                self.validate_zero_initializer(ty)?;

                Ok(vec![0; layout.byte_len()])
            }
            mir::GlobalInitializer::Bytes(bytes) => {
                if bytes.len() != layout.byte_len() {
                    return Err(type_mismatch(
                        format!("{} initializer bytes", layout.byte_len()),
                        format!("{} initializer bytes", bytes.len()),
                    ));
                }

                Ok(bytes.clone())
            }
            mir::GlobalInitializer::Aggregate(elements) => {
                self.payload_initializer_bytes(elements, ty, offset, render)
            }
            mir::GlobalInitializer::Variant { case, payload } => {
                self.variant_initializer_bytes(*case, payload.as_deref(), ty, offset, render)
            }
            mir::GlobalInitializer::Scalar(_)
            | mir::GlobalInitializer::FunctionAddress(_)
            | mir::GlobalInitializer::GlobalAddress(_) => {
                Err(type_mismatch("payload initializer", "scalar initializer"))
            }
        }
    }

    /// Encode one scalar initializer into bytes.
    fn scalar_initializer_bytes(
        &self,
        initializer: &mir::GlobalInitializer,
        ty: mir::TypeId,
        offset: usize,
        render: GlobalRender<'_, '_>,
    ) -> CompilerResult<Vec<u8>> {
        let byte_len = self.layout(ty)?.byte_len();

        match initializer {
            // value constants render through the constant encoder before this path
            mir::GlobalInitializer::String(_) | mir::GlobalInitializer::BigInt(_) => Err(
                invalid_program_input("a value initializer reached the byte renderer".to_string()),
            ),
            mir::GlobalInitializer::Zero => {
                self.validate_zero_scalar_type(ty)?;

                Ok(vec![0; byte_len])
            }
            mir::GlobalInitializer::Bytes(bytes) => {
                if bytes.len() != byte_len {
                    return Err(type_mismatch(
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
            mir::GlobalInitializer::GlobalAddress(target) => {
                let ty_node = self.storage_type(ty)?;
                if !matches!(
                    ty_node,
                    mir::Type::Reference { .. } | mir::Type::Pointer { .. }
                ) {
                    return Err(type_mismatch(
                        "reference initializer type",
                        format!("{ty_node:?}"),
                    ));
                }

                self.global_address_bytes(*target, offset, render, byte_len)
            }
            mir::GlobalInitializer::Aggregate(_) | mir::GlobalInitializer::Variant { .. } => {
                Err(type_mismatch("scalar initializer", format!("{ty:?}")))
            }
        }
    }

    /// Encode one slice header initializer as an address word and a length word.
    fn slice_initializer_bytes(
        &self,
        initializer: &mir::GlobalInitializer,
        offset: usize,
        mut render: GlobalRender<'_, '_>,
        byte_len: usize,
    ) -> CompilerResult<Vec<u8>> {
        let mir::GlobalInitializer::Aggregate(elements) = initializer else {
            return Err(type_mismatch(
                "slice header initializer",
                format!("{initializer:?}"),
            ));
        };
        let [address, length] = elements.as_slice() else {
            return Err(type_mismatch(
                "2 slice header elements",
                format!("{} elements", elements.len()),
            ));
        };
        let word_len = usize::from(self.target_layout.pointer_bytes());
        if byte_len != 2 * word_len {
            return Err(type_mismatch(
                format!("{} slice header bytes", 2 * word_len),
                format!("{byte_len} bytes"),
            ));
        }

        // render the data address word
        let mir::GlobalInitializer::GlobalAddress(target) = address else {
            return Err(type_mismatch(
                "global address slice data",
                format!("{address:?}"),
            ));
        };
        let mut bytes = self.global_address_bytes(*target, offset, render.reborrow(), word_len)?;

        // render the element length word
        let mir::GlobalInitializer::Scalar(constant) = length else {
            return Err(type_mismatch("scalar slice length", format!("{length:?}")));
        };
        let length = self.unsigned_constant_value(constant, self.target_layout.pointer_bits())?;
        bytes.extend_from_slice(&self.unsigned_bytes(length, word_len));

        Ok(bytes)
    }

    /// Encode one global address as a target-relative word.
    fn global_address_bytes(
        &self,
        target: mir::GlobalId,
        offset: usize,
        render: GlobalRender<'_, '_>,
        byte_len: usize,
    ) -> CompilerResult<Vec<u8>> {
        // resolve the placed target
        let target_id = self.program.global_id(self.module, target);
        let Some((_, placed)) = render.placements.get(target_id.index()) else {
            return Err(invalid_program_input(format!(
                "missing placed global {target_id:?}"
            )));
        };

        // reject runtime-owned images that address worker-owned storage
        if render.space != mir::Space::Local && placed.location == GlobalLocation::LocalStatic {
            return Err(invalid_program_input(
                "a runtime static initializer cannot reference local static storage".to_string(),
            ));
        }

        // record the target location and write its region-relative offset
        render.relocations.push((offset, placed.location));

        Ok(self.unsigned_bytes(placed.offset() as u128, byte_len))
    }

    /// Return the Program location for one MIR global storage class.
    const fn location(space: mir::Space) -> GlobalLocation {
        match space {
            mir::Space::Constant => GlobalLocation::Constant,
            mir::Space::Shared => GlobalLocation::SharedStatic,
            mir::Space::Local => GlobalLocation::LocalStatic,
        }
    }

    /// Encode one function address initializer as bytes.
    fn function_address_initializer_bytes(
        &self,
        function: mir::FunctionId,
        ty: mir::TypeId,
        byte_len: usize,
    ) -> CompilerResult<Vec<u8>> {
        let ty_node = self.storage_type(ty)?;
        let mir::Type::FunctionPointer { .. } = ty_node else {
            return Err(type_mismatch(
                "function pointer initializer",
                format!("{ty_node:?}"),
            ));
        };

        let expected_byte_len = usize::from(self.target_layout.pointer_bytes());
        if byte_len != expected_byte_len {
            return Err(type_mismatch(
                format!("{expected_byte_len} byte function pointer initializer"),
                format!("{byte_len} byte initializer"),
            ));
        }

        let function = self.program.function_id(self.module, function);
        let bytes = self.unsigned_bytes(u128::from(function.word().bits()), byte_len);

        Ok(bytes)
    }

    /// Encode one scalar initializer according to the declared type.
    fn constant_scalar_bytes(
        &self,
        constant: &mir::Constant,
        ty: mir::TypeId,
        byte_len: usize,
    ) -> CompilerResult<Vec<u8>> {
        let ty_node = self.storage_type(ty)?;

        match ty_node {
            mir::Type::Boolean => self.boolean_constant_bytes(constant, byte_len),
            mir::Type::Character => self.character_constant_bytes(constant, byte_len),
            mir::Type::Int { width, is_signed } => {
                self.integer_constant_bytes(constant, *width, *is_signed, byte_len)
            }
            mir::Type::Isize => {
                self.validate_pointer_byte_len(byte_len)?;
                let width = self.target_layout.pointer_bits();

                self.integer_constant_bytes(constant, width, true, byte_len)
            }
            mir::Type::Usize => {
                self.validate_pointer_byte_len(byte_len)?;
                let width = self.target_layout.pointer_bits();

                self.integer_constant_bytes(constant, width, false, byte_len)
            }
            mir::Type::TypeId => {
                self.integer_constant_bytes(constant, u32::BITS as u16, false, byte_len)
            }
            mir::Type::Float(format) => self.float_constant_bytes(constant, *format, byte_len),
            mir::Type::Pointer { .. } => self.pointer_constant_bytes(constant, byte_len),
            _ => Err(type_mismatch(
                "scalar initializer type",
                format!("{ty_node:?}"),
            )),
        }
    }

    /// Encode one boolean constant.
    fn boolean_constant_bytes(
        &self,
        constant: &mir::Constant,
        byte_len: usize,
    ) -> CompilerResult<Vec<u8>> {
        let mir::Constant::Boolean { value } = constant else {
            return Err(type_mismatch(
                "boolean initializer",
                format!("{constant:?}"),
            ));
        };

        if byte_len != 1 {
            return Err(type_mismatch(
                "1 byte boolean initializer",
                format!("{byte_len} bytes"),
            ));
        }

        Ok(vec![u8::from(*value)])
    }

    /// Encode one character constant.
    fn character_constant_bytes(
        &self,
        constant: &mir::Constant,
        byte_len: usize,
    ) -> CompilerResult<Vec<u8>> {
        let mir::Constant::Char { value } = constant else {
            return Err(type_mismatch(
                "character initializer",
                format!("{constant:?}"),
            ));
        };
        self.validate_integer_byte_len(u32::BITS as u16, byte_len)?;

        Ok(self.unsigned_bytes(u128::from(*value as u32), byte_len))
    }

    /// Encode one integer-like constant.
    fn integer_constant_bytes(
        &self,
        constant: &mir::Constant,
        width: u16,
        is_signed: bool,
        byte_len: usize,
    ) -> CompilerResult<Vec<u8>> {
        self.validate_integer_byte_len(width, byte_len)?;

        if is_signed {
            let value = self.signed_constant_value(constant, width)?;

            return Ok(self.signed_bytes(value, byte_len));
        }

        let value = self.unsigned_constant_value(constant, width)?;

        Ok(self.unsigned_bytes(value, byte_len))
    }

    /// Return one signed integer constant after declared-type range checks.
    fn signed_constant_value(&self, constant: &mir::Constant, width: u16) -> CompilerResult<i128> {
        let value = match constant {
            mir::Constant::Int { value, .. } => *value,
            mir::Constant::UInt { value, .. } => i128::try_from(*value).map_err(|_| {
                type_mismatch(
                    format!("signed {width} bit initializer"),
                    format!("{value}"),
                )
            })?,
            _ => {
                return Err(type_mismatch(
                    format!("signed {width} bit initializer"),
                    format!("{constant:?}"),
                ));
            }
        };

        if !Self::signed_value_fits(value, width) {
            return Err(type_mismatch(
                format!("signed {width} bit initializer"),
                format!("{value}"),
            ));
        }

        Ok(value)
    }

    /// Return one unsigned integer constant after declared-type range checks.
    fn unsigned_constant_value(
        &self,
        constant: &mir::Constant,
        width: u16,
    ) -> CompilerResult<u128> {
        let value = match constant {
            mir::Constant::UInt { value, .. } => *value,
            mir::Constant::Int { value, .. } if *value >= 0 => *value as u128,
            _ => {
                return Err(type_mismatch(
                    format!("unsigned {width} bit initializer"),
                    format!("{constant:?}"),
                ));
            }
        };

        if !Self::unsigned_value_fits(value, width) {
            return Err(type_mismatch(
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
    ) -> CompilerResult<Vec<u8>> {
        let mir::Constant::Float {
            bits,
            format: source,
        } = constant
        else {
            return Err(type_mismatch(
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

    /// Encode one pointer constant.
    fn pointer_constant_bytes(
        &self,
        constant: &mir::Constant,
        byte_len: usize,
    ) -> CompilerResult<Vec<u8>> {
        let bits = match constant {
            mir::Constant::Null => 0usize,
            _ => {
                return Err(type_mismatch(
                    "reference initializer",
                    format!("{constant:?}"),
                ));
            }
        };
        let bytes = bits.to_le_bytes();
        if byte_len != bytes.len() {
            return Err(type_mismatch(
                format!("{} byte reference initializer", bytes.len()),
                format!("{byte_len} byte initializer"),
            ));
        }

        Ok(bytes.to_vec())
    }

    /// Validate integer initializer storage width.
    fn validate_integer_byte_len(&self, width: u16, byte_len: usize) -> CompilerResult<()> {
        let expected_byte_len = self.integer_byte_len(width)?;
        if byte_len != expected_byte_len {
            return Err(type_mismatch(
                format!("{expected_byte_len} byte integer initializer"),
                format!("{byte_len} byte initializer"),
            ));
        }

        Ok(())
    }

    /// Validate pointer-sized initializer storage width.
    fn validate_pointer_byte_len(&self, byte_len: usize) -> CompilerResult<()> {
        let expected_byte_len = usize::from(self.target_layout.pointer_bytes());
        if byte_len != expected_byte_len {
            return Err(type_mismatch(
                format!("{expected_byte_len} byte pointer initializer"),
                format!("{byte_len} byte initializer"),
            ));
        }

        Ok(())
    }

    /// Validate floating-point initializer storage width.
    fn validate_float_byte_len(
        &self,
        format: mir::FloatType,
        byte_len: usize,
    ) -> CompilerResult<()> {
        let expected_byte_len = self.integer_byte_len(format.width())?;
        if byte_len != expected_byte_len {
            return Err(type_mismatch(
                format!("{expected_byte_len} byte {} initializer", format.label()),
                format!("{byte_len} byte initializer"),
            ));
        }

        Ok(())
    }

    /// Return the byte width for one scalar bit width.
    fn integer_byte_len(&self, width: u16) -> CompilerResult<usize> {
        if width == 0 {
            return Err(type_mismatch("nonzero scalar width", "0 bits"));
        }

        let width = usize::from(width);

        Ok(width.div_ceil(8))
    }

    /// Return whether one signed value fits the requested bit width.
    fn signed_value_fits(value: i128, width: u16) -> bool {
        // admit only zero at zero width
        if width == 0 {
            return value == 0;
        }

        // admit everything the widest constant can hold
        if width >= 128 {
            return true;
        }

        let min = -(1i128 << (width - 1));
        let max = (1i128 << (width - 1)) - 1;

        value >= min && value <= max
    }

    /// Return whether one unsigned value fits the requested bit width.
    fn unsigned_value_fits(value: u128, width: u16) -> bool {
        // admit only zero at zero width
        if width == 0 {
            return value == 0;
        }

        // admit everything the widest constant can hold
        if width >= 128 {
            return true;
        }

        value <= ((1u128 << width) - 1)
    }

    /// Encode one signed integer in target byte order.
    fn signed_bytes(&self, value: i128, byte_len: usize) -> Vec<u8> {
        // sign extend the destination and encode the value in target byte order
        let mut bytes = vec![if value < 0 { 0xff } else { 0 }; byte_len];
        let source = if self.target_layout.endian.is_little() {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        };
        let copied = source.len().min(byte_len);

        // keep the low bytes on little endian and the high bytes on big endian
        if self.target_layout.endian.is_little() {
            bytes[..copied].copy_from_slice(&source[..copied]);
        } else {
            let source_start = source.len() - copied;
            let target_start = byte_len - copied;
            bytes[target_start..].copy_from_slice(&source[source_start..]);
        }

        bytes
    }

    /// Encode one unsigned integer in target byte order.
    fn unsigned_bytes(&self, value: u128, byte_len: usize) -> Vec<u8> {
        // zero fill the destination and encode the value in target byte order
        let mut bytes = vec![0; byte_len];
        let source = if self.target_layout.endian.is_little() {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        };
        let copied = source.len().min(byte_len);

        // keep the low bytes on little endian and the high bytes on big endian
        if self.target_layout.endian.is_little() {
            bytes[..copied].copy_from_slice(&source[..copied]);
        } else {
            let source_start = source.len() - copied;
            let target_start = byte_len - copied;
            bytes[target_start..].copy_from_slice(&source[source_start..]);
        }

        bytes
    }

    /// Validate one zero initializer against the declared type.
    fn validate_zero_initializer(&self, ty: mir::TypeId) -> CompilerResult<()> {
        match self.layout(ty)?.shape {
            // leave a zeroed variant for the module initializer
            mir::LayoutShape::Variant(_) => Ok(()),

            // check every aggregate range in turn
            mir::LayoutShape::Struct(_)
            | mir::LayoutShape::Tuple(_)
            | mir::LayoutShape::Class(_)
            | mir::LayoutShape::Array(_)
            | mir::LayoutShape::Vector(_)
            | mir::LayoutShape::Newtype(_) => {
                for range in self.initializer_ranges(ty)? {
                    self.validate_zero_initializer(range.ty)?;
                }

                Ok(())
            }

            // check zeroed scalars and handles
            mir::LayoutShape::None
            | mir::LayoutShape::Scalar
            | mir::LayoutShape::Slice
            | mir::LayoutShape::Dynamic
            | mir::LayoutShape::Function => self.validate_zero_scalar_type(ty),
        }
    }

    /// Validate whether one scalar type accepts a zero initializer.
    fn validate_zero_scalar_type(&self, ty: mir::TypeId) -> CompilerResult<()> {
        let ty_node = self.storage_type(ty)?;

        match ty_node {
            mir::Type::Void
            | mir::Type::Int { .. }
            | mir::Type::Isize
            | mir::Type::Usize
            | mir::Type::Float(_)
            | mir::Type::Boolean
            | mir::Type::Character
            | mir::Type::TypeId => Ok(()),
            // leave a zeroed handle for the module initializer
            mir::Type::Pointer { .. }
            | mir::Type::Reference { .. }
            | mir::Type::Dynamic { .. }
            | mir::Type::Function { .. }
            | mir::Type::Slice { .. } => Ok(()),
            _ => Err(unsupported_zero_initializer(format!("{ty_node:?}"))),
        }
    }

    /// Encode one payload initializer into bytes.
    fn payload_initializer_bytes(
        &self,
        elements: &[mir::GlobalInitializer],
        ty: mir::TypeId,
        offset: usize,
        mut render: GlobalRender<'_, '_>,
    ) -> CompilerResult<Vec<u8>> {
        let layout = self.layout(ty)?;
        let ranges = self.initializer_ranges(ty)?;
        if elements.len() != ranges.len() {
            return Err(type_mismatch(
                format!("{} initializer elements", ranges.len()),
                format!("{} initializer elements", elements.len()),
            ));
        }

        let mut bytes = vec![0u8; layout.byte_len()];
        for (element, range) in elements.iter().zip(ranges) {
            let value_bytes = self.initializer_bytes(
                element,
                range.ty,
                offset + range.offset,
                render.reborrow(),
            )?;
            if value_bytes.len() != range.byte_len {
                return Err(type_mismatch(
                    format!("{} initializer bytes", range.byte_len),
                    format!("{} initializer bytes", value_bytes.len()),
                ));
            }

            let end = range
                .offset
                .checked_add(range.byte_len)
                .ok_or_else(|| layout_overflow("initializer byte range"))?;
            let target = bytes
                .get_mut(range.offset..end)
                .ok_or_else(|| layout_overflow("initializer byte range"))?;
            target.copy_from_slice(&value_bytes);
        }

        Ok(bytes)
    }

    /// Encode one variant initializer.
    fn variant_initializer_bytes(
        &self,
        case: u32,
        payload: Option<&mir::GlobalInitializer>,
        ty: mir::TypeId,
        offset: usize,
        render: GlobalRender<'_, '_>,
    ) -> CompilerResult<Vec<u8>> {
        let layout = self.layout(ty)?;
        let mir::LayoutShape::Variant(variant) = &layout.shape else {
            return Err(type_mismatch(
                "variant initializer layout",
                format!("{ty:?}"),
            ));
        };
        let Some(case_layout) = variant.cases.get(case as usize) else {
            return Err(type_mismatch(
                format!("{} variant cases", variant.cases.len()),
                format!("variant case {case}"),
            ));
        };

        // write the payload at its case offset
        let mut bytes = vec![0u8; layout.byte_len()];
        if let Some(payload) = payload {
            let start = case_layout.payload_offset as usize;
            let payload_bytes =
                self.initializer_bytes(payload, case_layout.ty, offset + start, render)?;
            let target = bytes
                .get_mut(start..start + payload_bytes.len())
                .ok_or_else(|| layout_overflow("variant payload byte range"))?;
            target.copy_from_slice(&payload_bytes);
        }

        // encode the discriminant into its physical field
        let field = variant.encoding.field();
        let start = field.offset as usize;
        let len = field.byte_len as usize;
        let range = bytes
            .get_mut(start..start + len)
            .ok_or_else(|| layout_overflow("variant discriminant byte range"))?;
        let scalar = self.unsigned_value(range);
        let case_count = variant.cases.len() as u32;
        let scalar = variant
            .encoding
            .encode_case(scalar, case, case_layout.discriminant, case_count)
            .ok_or_else(|| type_mismatch("an encodable case", format!("variant case {case}")))?;
        let encoded = self.signed_bytes(scalar as i128, len);
        range.copy_from_slice(&encoded);

        Ok(bytes)
    }

    /// Decode one unsigned scalar from bytes in target byte order.
    fn unsigned_value(&self, bytes: &[u8]) -> u128 {
        let mut buffer = [0u8; 16];
        match self.target_layout.endian.is_little() {
            true => buffer[..bytes.len()].copy_from_slice(bytes),
            false => buffer[16 - bytes.len()..].copy_from_slice(bytes),
        }

        match self.target_layout.endian.is_little() {
            true => u128::from_le_bytes(buffer),
            false => u128::from_be_bytes(buffer),
        }
    }

    /// Return initializer byte ranges for one payload type.
    fn initializer_ranges(&self, ty: mir::TypeId) -> CompilerResult<Vec<InitializerRange>> {
        let layout = self.layout(ty)?;

        match &layout.shape {
            mir::LayoutShape::Struct(layout) => Ok(self.field_ranges(&layout.fields)),
            mir::LayoutShape::Tuple(layout) => Ok(self.field_ranges(&layout.elements)),
            mir::LayoutShape::Class(layout) => Ok(self.field_ranges(&layout.fields)),
            mir::LayoutShape::Array(element) | mir::LayoutShape::Vector(element) => {
                self.element_ranges(element)
            }
            // cover a newtype's backing with its one element
            mir::LayoutShape::Newtype(newtype) => Ok(vec![InitializerRange {
                ty: newtype.backing_type,
                offset: 0,
                byte_len: layout.byte_len(),
            }]),
            _ => Err(type_mismatch(
                "aggregate initializer layout",
                format!("{ty:?}"),
            )),
        }
    }

    /// Return initializer ranges for source-ordered fields.
    fn field_ranges(&self, fields: &[mir::LayoutField]) -> Vec<InitializerRange> {
        let mut fields = fields.iter().collect::<Vec<_>>();
        fields.sort_by_key(|field| field.source_index);

        fields
            .into_iter()
            .map(|field| InitializerRange {
                ty: field.ty,
                offset: field.offset as usize,
                byte_len: field.size as usize,
            })
            .collect()
    }

    /// Return initializer ranges for one fixed repeated layout.
    fn element_ranges(
        &self,
        element: &mir::ElementLayout,
    ) -> CompilerResult<Vec<InitializerRange>> {
        let element_layout = self.layout(element.element)?;
        let count = element.count as usize;
        let stride = element.stride as usize;
        let mut ranges = Vec::with_capacity(count);

        // project one byte range per fixed element
        for index in 0..count {
            let offset = stride
                .checked_mul(index)
                .ok_or_else(|| layout_overflow("initializer element offset"))?;
            ranges.push(InitializerRange {
                ty: element.element,
                offset,
                byte_len: element_layout.byte_len(),
            });
        }

        Ok(ranges)
    }

    /// Return the complete target layout for one MIR type.
    fn layout(&self, ty: mir::TypeId) -> CompilerResult<&mir::Layout> {
        self.layouts
            .type_layout(ty)
            .ok_or_else(|| type_mismatch("program layout", format!("{ty:?}")))
    }

    /// Return whether one MIR value uses scalar static storage.
    fn is_scalar(&self, ty: mir::TypeId) -> bool {
        let Some(ty) = self.object.storage_type(ty) else {
            return false;
        };
        let Some(ty) = self.object.ty(ty).map(|ty| &ty.definition) else {
            return false;
        };

        matches!(
            ty,
            mir::Type::Void
                | mir::Type::Boolean
                | mir::Type::Character
                | mir::Type::Int { .. }
                | mir::Type::Isize
                | mir::Type::Usize
                | mir::Type::Float(_)
                | mir::Type::TypeId
                | mir::Type::Reference { .. }
                | mir::Type::Pointer { .. }
                | mir::Type::FunctionPointer { .. }
        )
    }

    /// Return one transparent object-local storage type.
    fn storage_type(&self, ty: mir::TypeId) -> CompilerResult<&mir::Type> {
        let ty = self
            .object
            .storage_type(ty)
            .ok_or_else(|| invalid_program_input(format!("missing type {ty:?}")))?;
        let ty = self
            .object
            .ty(ty)
            .ok_or_else(|| invalid_program_input(format!("missing type {ty:?}")))?;

        Ok(&ty.definition)
    }
}

impl StaticLinker<'_> {
    /// Build dense global ids from definitions and imported symbols.
    pub(crate) fn index(
        objects: &[(ModuleId, Arc<Object>)],
        type_ids: &HashMap<(ModuleId, mir::TypeId), TypeId>,
    ) -> CompilerResult<(
        HashMap<(ModuleId, mir::GlobalId), GlobalId>,
        Vec<(ModuleId, mir::GlobalId)>,
    )> {
        let mut ids = HashMap::new();
        let mut globals = Vec::new();
        let mut symbols = HashMap::new();

        // assign every definition once
        for (module, object) in objects {
            for global in object.globals() {
                let global_id = global.id;
                if global.linkage.is_import() {
                    continue;
                }
                let placed = symbols.get(&global.symbol).map(|(id, _, _)| *id);
                let id = GlobalId::from(globals.len() as u32);

                // fold a shared copy into its placed definition
                if global.linkage == mir::Linkage::Shared
                    && let Some(placed) = placed
                {
                    ids.insert((*module, global_id), placed);

                    continue;
                }
                // refuse a second definition of an exported symbol
                else if global.linkage.is_exported() && placed.is_some() {
                    return Err(invalid_program_input(format!(
                        "global symbol {:?} has multiple definitions",
                        global.symbol
                    )));
                }
                // record shared and exported definitions
                else if global.linkage == mir::Linkage::Shared || global.linkage.is_exported() {
                    symbols.insert(global.symbol, (id, *module, global));
                }

                ids.insert((*module, global_id), id);
                globals.push((*module, global_id));
            }
        }

        // resolve imports against exported definitions
        for (module, object) in objects {
            for global in object.globals() {
                let global_id = global.id;
                if !global.linkage.is_import() {
                    continue;
                }
                let Some((id, definition_module, definition)) =
                    symbols.get(&global.symbol).copied()
                else {
                    return Err(invalid_program_input(format!(
                        "global symbol {:?} is undefined",
                        global.symbol
                    )));
                };

                // reject imports that disagree with their definition
                if !TypeLinker::same(
                    *module,
                    global.ty,
                    definition_module,
                    definition.ty,
                    type_ids,
                ) || global.mutability != definition.mutability
                    || global.space != definition.space
                {
                    return Err(invalid_program_input(format!(
                        "global symbol {:?} has conflicting declarations",
                        global.symbol
                    )));
                }

                ids.insert((*module, global_id), id);
            }
        }

        Ok((ids, globals))
    }
}

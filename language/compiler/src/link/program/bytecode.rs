use std::collections::HashSet;

use destack_artifact::Object;
use destack_bytecode as bytecode;
use destack_core::{EntryRange, Optional};
use destack_mir as mir;
use destack_program::{CounterId, SamplerId, TypeId, Word};
use destack_source::ModuleId;

use crate::LinkResult;

use super::{FrameLinker, ProgramLinker};

/// Link relocatable bytecode objects into one executable code image.
#[derive(Debug)]
pub(crate) struct BytecodeLinker<'a, 'b> {
    /// Program linker owning final runtime identities.
    program: &'a ProgramLinker<'b>,
    /// Canonical frame state projection.
    frames: &'a FrameLinker<'b>,
}

#[allow(clippy::too_many_arguments)]
impl<'a, 'b> BytecodeLinker<'a, 'b> {
    /// Create one bytecode linker.
    pub(crate) fn new(program: &'a ProgramLinker<'b>, frames: &'a FrameLinker<'b>) -> Self {
        Self { program, frames }
    }

    /// Link every bytecode object into final program order.
    pub(crate) fn link(&self) -> LinkResult<bytecode::CodeBuilder> {
        let mut object_code = Vec::with_capacity(self.program.objects().len());

        // resolve every object-local identity before concatenating functions
        for (module, object) in self.program.objects() {
            object_code.push(self.link_code(*module, object)?);
        }

        let mut functions = Vec::with_capacity(self.program.functions_by_id().len());
        let mut frames = Vec::new();
        let mut registers = Vec::new();
        let mut operations = Vec::new();
        let mut code = Vec::new();

        // place physical functions in dense Program function order
        for &(module, function) in self.program.functions_by_id() {
            let object_index = self.program.object_index(module);
            let object = self.program.object(module);
            let source = self.function(object, function)?;
            let row = self.link_function(
                module,
                function,
                object,
                source,
                &object_code[object_index],
                &mut frames,
                &mut registers,
                &mut operations,
                &mut code,
            )?;
            functions.push(row);
        }

        Ok(bytecode::CodeBuilder::new()
            .functions(functions)
            .frames(frames)
            .registers(registers)
            .operations(operations)
            .code(code))
    }

    /// Resolve one object's instruction identities into final Program ids.
    fn link_code(&self, module: ModuleId, object: &Object) -> LinkResult<Vec<u8>> {
        let bytecode = object.bytecode();
        let mut code = bytecode.code().to_vec();
        let counters = object
            .counters()
            .iter()
            .map(|site| (site.point.function, site.counter))
            .collect::<HashSet<_>>();
        let samplers = object
            .samples()
            .iter()
            .map(|site| (site.point.function, site.sampler))
            .collect::<HashSet<_>>();

        // patch every object-local identity at its exact encoded operand
        for relocation in bytecode.relocations() {
            let start = relocation.byte_offset as usize;
            let end = start + size_of::<u32>();
            let bytes = code.get(start..end).ok_or_else(|| {
                self.program
                    .invalid_input("bytecode relocation is out of range")
            })?;
            let encoded = u32::from_le_bytes(bytes.try_into().map_err(|_| {
                self.program
                    .invalid_input("bytecode relocation has invalid width")
            })?);
            let linked = self.relocation(
                module,
                object,
                relocation.byte_offset,
                relocation.tag,
                encoded,
                &counters,
                &samplers,
            )?;
            let bytes = code.get_mut(start..end).ok_or_else(|| {
                self.program
                    .invalid_input("bytecode relocation is out of range")
            })?;
            bytes.copy_from_slice(&linked.to_le_bytes());
        }

        Ok(code)
    }

    /// Link one physical function and append its owned sections.
    fn link_function(
        &self,
        module: ModuleId,
        function: mir::FunctionId,
        object: &Object,
        source: &bytecode::Function,
        object_code: &[u8],
        frames: &mut Vec<bytecode::FrameMap>,
        registers: &mut Vec<bytecode::RegisterSpan>,
        operations: &mut Vec<bytecode::CodeOffset>,
        code: &mut Vec<u8>,
    ) -> LinkResult<bytecode::Function> {
        let bytecode = object.bytecode();

        // append operation coordinates without changing function-relative offsets
        let operation_start = operations.len() as u32;
        let source_operations = source.operations(bytecode.operations());
        operations.extend_from_slice(source_operations);
        let operation_count = operations.len() as u32 - operation_start;

        // append frame maps in canonical Program frame state order
        let frame_start = frames.len() as u32;
        for index in source.frames.start..source.frames.start + source.frames.len {
            let frame = bytecode
                .frames()
                .get(index as usize)
                .ok_or_else(|| self.program.invalid_input("bytecode frame map is absent"))?;
            let state = object
                .frames()
                .get(index as usize)
                .ok_or_else(|| self.program.invalid_input("logical frame state is absent"))?;
            let state = self
                .frames
                .state(module, state.point)
                .ok_or_else(|| self.program.invalid_input("linked frame state is absent"))?;
            if state.0 != frames.len() as u32 {
                return Err(self
                    .program
                    .invalid_input("bytecode frame maps are not canonical"));
            }

            let register_start = registers.len() as u32;
            let source_registers = frame.registers(bytecode.registers());
            registers.extend_from_slice(source_registers);
            frames.push(bytecode::FrameMap::new(
                frame.code_offset,
                EntryRange::new(register_start, source_registers.len() as u32),
            ));
        }
        let frame_count = frames.len() as u32 - frame_start;

        // append the linked function body when this object defines it
        let code_range = source.code().map(|range| {
            let byte_offset = code.len() as u32;
            let bytes = range.slice(object_code);
            code.extend_from_slice(bytes);

            bytecode::CodeRange {
                byte_offset,
                byte_len: bytes.len() as u32,
            }
        });

        let register_count = self.register_count(object, function, source.register_count)?;

        Ok(bytecode::Function::new(
            Optional::from(code_range),
            EntryRange::new(frame_start, frame_count),
            EntryRange::new(operation_start, operation_count),
            register_count,
        ))
    }

    /// Return the register width required by emitted code and the callable ABI.
    fn register_count(
        &self,
        object: &Object,
        function: mir::FunctionId,
        emitted_register_count: u16,
    ) -> LinkResult<u16> {
        let function = object
            .function(function)
            .ok_or_else(|| self.program.invalid_input("object function is absent"))?;
        let types = function
            .environment
            .iter()
            .chain(function.parameters.iter().map(|parameter| &parameter.ty));
        let mut entry_register_count = 0;

        // size the contiguous entry window from canonical object layouts
        for ty in types {
            let layout = object.layouts().type_layout(*ty).ok_or_else(|| {
                self.program
                    .invalid_input("function parameter layout is absent")
            })?;
            entry_register_count += layout.byte_len().div_ceil(Word::BYTE_LEN);
        }

        let entry_register_count = u16::try_from(entry_register_count).map_err(|_| {
            self.program
                .invalid_input("function register count is out of range")
        })?;

        Ok(emitted_register_count.max(entry_register_count))
    }

    /// Return one physical declaration by common object function identity.
    fn function<'c>(
        &self,
        object: &'c Object,
        function: mir::FunctionId,
    ) -> LinkResult<&'c bytecode::Function> {
        let index = object
            .functions()
            .binary_search_by_key(&function, |declaration| declaration.id)
            .map_err(|_| self.program.invalid_input("object function is absent"))?;

        object
            .bytecode()
            .functions()
            .get(index)
            .ok_or_else(|| self.program.invalid_input("bytecode function is absent"))
    }

    /// Resolve one encoded object-local identity into its final Program id.
    fn relocation(
        &self,
        module: ModuleId,
        object: &Object,
        byte_offset: u32,
        tag: bytecode::RelocationTag,
        index: u32,
        counters: &HashSet<(mir::FunctionId, mir::CounterId)>,
        samplers: &HashSet<(mir::FunctionId, mir::SamplerId)>,
    ) -> LinkResult<u32> {
        if tag == bytecode::RelocationTag::TYPE {
            self.type_id(module, object, index).map(|id| id.0)
        } else if tag == bytecode::RelocationTag::LAYOUT {
            let ty = self.object_type(object, index)?;

            Ok(self.program.layout_id(module, ty).raw())
        } else if tag == bytecode::RelocationTag::FUNCTION {
            let function = object.functions().get(index as usize).ok_or_else(|| {
                self.program
                    .invalid_input("bytecode function operand is absent")
            })?;

            Ok(self.program.function_id(module, function.id).0)
        } else if tag == bytecode::RelocationTag::GLOBAL {
            let global = object.globals().get(index as usize).ok_or_else(|| {
                self.program
                    .invalid_input("bytecode global operand is absent")
            })?;

            Ok(self.program.global_id(module, global.id).0)
        } else if tag == bytecode::RelocationTag::DYNAMIC {
            let table = object
                .dispatch()
                .iter_dynamic_tables()
                .nth(index as usize)
                .ok_or_else(|| {
                    self.program
                        .invalid_input("bytecode dynamic table is absent")
                })?;
            let id = self
                .program
                .dynamic_table_id(module, table.concrete, table.constraint)
                .ok_or_else(|| self.program.invalid_input("linked dynamic table is absent"))?;

            Ok(id.0)
        } else if tag == bytecode::RelocationTag::ALLOCATION {
            object.allocations().get(index as usize).ok_or_else(|| {
                self.program
                    .invalid_input("bytecode allocation site is absent")
            })?;

            Ok(self.program.allocation_id(module, index).0)
        } else if tag == bytecode::RelocationTag::COUNTER {
            let function = self.relocation_function(object, byte_offset)?;

            self.counter_id(module, counters, function, mir::CounterId(index))
                .map(|counter| counter.0)
        } else if tag == bytecode::RelocationTag::SAMPLER {
            let function = self.relocation_function(object, byte_offset)?;

            self.sampler_id(module, samplers, function, mir::SamplerId(index))
                .map(|sampler| sampler.0)
        } else {
            Err(self
                .program
                .invalid_input("unsupported bytecode relocation"))
        }
    }

    /// Return the MIR function that owns one object code offset.
    fn relocation_function(
        &self,
        object: &Object,
        byte_offset: u32,
    ) -> LinkResult<mir::FunctionId> {
        for (declaration, function) in object.functions().iter().zip(object.bytecode().functions())
        {
            let Some(code) = function.code() else {
                continue;
            };
            let end = code.byte_offset + code.byte_len;
            if byte_offset >= code.byte_offset && byte_offset < end {
                return Ok(declaration.id);
            }
        }

        Err(self
            .program
            .invalid_input("bytecode relocation has no owning function"))
    }

    /// Resolve one function-local counter declared by an artifact site.
    fn counter_id(
        &self,
        module: ModuleId,
        counters: &HashSet<(mir::FunctionId, mir::CounterId)>,
        function: mir::FunctionId,
        counter: mir::CounterId,
    ) -> LinkResult<CounterId> {
        if !counters.contains(&(function, counter)) {
            return Err(self
                .program
                .invalid_input("bytecode counter site is absent"));
        }

        Ok(self.program.counter_id(module, function, counter))
    }

    /// Resolve one function-local sampler declared by an artifact site.
    fn sampler_id(
        &self,
        module: ModuleId,
        samplers: &HashSet<(mir::FunctionId, mir::SamplerId)>,
        function: mir::FunctionId,
        sampler: mir::SamplerId,
    ) -> LinkResult<SamplerId> {
        if !samplers.contains(&(function, sampler)) {
            return Err(self
                .program
                .invalid_input("bytecode sampler site is absent"));
        }

        Ok(self.program.sampler_id(module, function, sampler))
    }

    /// Return one object-local MIR type identity.
    fn object_type(&self, object: &Object, index: u32) -> LinkResult<mir::TypeId> {
        object
            .types()
            .get(index as usize)
            .map(|ty| ty.id)
            .ok_or_else(|| {
                self.program
                    .invalid_input("bytecode type operand is absent")
            })
    }

    /// Return one final Program type identity.
    fn type_id(&self, module: ModuleId, object: &Object, index: u32) -> LinkResult<TypeId> {
        let ty = self.object_type(object, index)?;

        Ok(self.program.type_id(module, ty))
    }
}

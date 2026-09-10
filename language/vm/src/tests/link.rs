use std::collections::HashMap;
use std::sync::Arc;

use bytecode::{CodeBuilder, Parser, RelocationTag};
use destack_bytecode as bytecode;
use destack_core::{EntryRange, Optional, StringPool};
use destack_heap::DropId;
use destack_mir::{Space, Storage, TraceTable};
use destack_program as program;
use destack_program::{
    BindingAffinity, BindingBuilder, BindingEffect, BindingId, BindingProvider, BindingReplay,
    DispatchTableBuilder, DropEntry, FrameLayoutBuilder, FrameLayoutId, FramePoint, FrameSlot,
    FrameState, FrameTableBuilder, FunctionBuilder, FunctionId, FunctionTableBuilder,
    GlobalAllocator, GlobalTableBuilder, LayoutBuilder, LayoutId, Program, ProgramBuilder,
    ProgramPoint, SignatureId, StaticBytes, Symbol, TypeDescriptorBuilder, TypeFingerprint, TypeId,
    TypeTableBuilder, Word,
};
use destack_source::FileId;

use super::program::{TEST_GLOBAL_BYTES, TestLayout, TestProgram};

#[allow(clippy::type_complexity)]
impl TestProgram {
    /// Build one linked Program from bytecode source and test metadata.
    pub(crate) fn build(self, source: &str) -> Arc<Program> {
        let object = Parser::new(FileId::new(0), source)
            .parse()
            .expect("test bytecode should parse");
        let (globals, constants, shared_statics, local_statics) = self.globals();

        // resolve object relocations into their dense linked identities
        let mut code = object.code().to_vec();
        for relocation in object.relocations() {
            let start = relocation.byte_offset as usize;
            let end = start + size_of::<u32>();
            let encoded = u32::from_le_bytes(
                code[start..end]
                    .try_into()
                    .expect("relocation operand should be complete"),
            );
            let value = match relocation.tag {
                RelocationTag::TYPE => encoded,
                RelocationTag::LAYOUT => encoded + 1,
                RelocationTag::FUNCTION => encoded,
                RelocationTag::GLOBAL => encoded,
                RelocationTag::DYNAMIC => encoded,
                RelocationTag::ALLOCATION => encoded,
                RelocationTag::COUNTER => encoded,
                RelocationTag::SAMPLER => encoded,
                _ => panic!("unknown test bytecode relocation"),
            };
            code[start..end].copy_from_slice(&value.to_le_bytes());
        }

        // build physical bytecode and canonical Program frame tables
        let (functions, frames, registers, frame_table, _frame_states) = self.frames(&object);
        let code = CodeBuilder::new()
            .functions(functions)
            .frames(frames)
            .registers(registers)
            .code(code)
            .operations(object.operations().iter().copied());

        // build the Program metadata and static storage
        let (types, layouts, traces, drops) = self.types(&object);
        let (strings, names, functions, bindings) = self.functions(&object);
        let types = TypeTableBuilder::new().types(
            (0..types.len())
                .map(|index| TypeFingerprint::from_raw(index as u128))
                .zip(types),
        );
        let globals = GlobalTableBuilder::new().globals(
            (0..globals.len())
                .map(|index| Symbol::from_raw(index as u64))
                .zip(globals),
        );
        let sites = self.sites;
        let program = ProgramBuilder::new(Default::default())
            .bytecode(code)
            .strings(&strings, names)
            .functions(functions)
            .bindings(bindings)
            .types(types)
            .drops(drops)
            .layouts(layouts)
            .frames(frame_table)
            .traces(traces)
            .sites(sites)
            .dispatch(
                DispatchTableBuilder::new()
                    .virtual_tables(self.virtual_tables)
                    .dynamic_tables(self.dynamic_tables),
            )
            .globals(globals)
            .constants(constants)
            .shared_statics(shared_statics)
            .local_statics(local_statics)
            .build()
            .expect("test program should build");

        Arc::new(program)
    }

    /// Return the dense type domain referenced by one test object and its Program tables.
    fn type_count(&self, object: &bytecode::Object) -> usize {
        let mut count = 1;

        // include types named by relocatable instruction operands
        for relocation in object.relocations() {
            if relocation.tag == RelocationTag::TYPE || relocation.tag == RelocationTag::LAYOUT {
                let start = relocation.byte_offset as usize;
                let end = start + size_of::<u32>();
                let index = u32::from_le_bytes(
                    object.code()[start..end]
                        .try_into()
                        .expect("type relocation should be complete"),
                );
                count = count.max(index as usize + 1);
            }
        }

        // include types supplied by test-only Program metadata
        for layout in &self.layouts {
            count = count.max(layout.ty.index() + 1);
        }
        for (concrete, constraint) in self.dynamic_table_ids.keys() {
            count = count.max(concrete.index() + 1);
            count = count.max(constraint.index() + 1);
        }
        for (ty, _, _) in &self.drops {
            count = count.max(ty.index() + 1);
        }
        for signature in self.signatures.iter().flatten() {
            count = count.max(signature.result.index() + 1);
            for parameter in &signature.parameters {
                count = count.max(parameter.index() + 1);
            }
        }
        count
    }

    /// Build concrete Program types for the complete object-local type domain.
    fn types(
        &self,
        object: &bytecode::Object,
    ) -> (
        Vec<TypeDescriptorBuilder>,
        Vec<LayoutBuilder>,
        TraceTable,
        Vec<DropEntry>,
    ) {
        let mut traces = TraceTable::new();
        let type_count = self.type_count(object);
        let mut types = Vec::with_capacity(type_count);
        let mut layouts = Vec::with_capacity(type_count);
        let mut drops = Vec::new();

        // preserve the object-local type domain so ids are already linked ids
        for index in 0..type_count {
            let layout = LayoutId::new(index as u32 + 1);
            let mut descriptor = TypeDescriptorBuilder::new(layout);

            // attach the explicitly configured destructors when present
            let mut entry = DropEntry {
                frame: Optional::none(),
                local: Optional::none(),
                shared: Optional::none(),
            };
            let mut has_drop = false;
            for (_, storage, function) in self.drops.iter().filter(|(ty, _, _)| ty.index() == index)
            {
                match storage {
                    Storage::Frame => entry.frame = Optional::some(*function),
                    Storage::Heap(Space::Local) => entry.local = Optional::some(*function),
                    Storage::Heap(Space::Shared) => entry.shared = Optional::some(*function),
                    Storage::Heap(
                        Space::Constant | Space::Parameter(_) | Space::Slot(_) | Space::Join(_),
                    )
                    | Storage::Static(_) => {
                        panic!("test globals cannot carry destructors")
                    }
                }
                has_drop = true;
            }
            if has_drop {
                let drop = DropId::from_index(drops.len() as u32);
                descriptor = descriptor.drop(drop);
                drops.push(entry);
            }

            types.push(descriptor);
            let ty = TypeId(index as u32);
            let layout = self.layouts.iter().find(|layout| layout.ty == ty);
            let word = self
                .default_signature
                .as_ref()
                .map(|_| TestLayout::word(ty));
            let layout = layout
                .or(word.as_ref())
                .unwrap_or_else(|| panic!("test type t{index} requires a layout"));
            let trace = traces.insert(layout.trace.clone());
            let layout = LayoutBuilder::new(
                layout.shape.clone(),
                layout.byte_len,
                layout.alignment,
                trace,
            );
            layouts.push(layout);
        }

        (types, layouts, traces, drops)
    }

    /// Build dense Program global storage from configured test metadata.
    fn globals(&self) -> (Vec<program::Global>, StaticBytes, StaticBytes, StaticBytes) {
        let mut globals = Vec::with_capacity(self.globals.len());
        let mut constant_space = GlobalAllocator::new();
        let mut shared_static_space = GlobalAllocator::new();
        let mut local_static_space = GlobalAllocator::new();
        let bytes = [0; TEST_GLOBAL_BYTES];

        // preserve configured order so global ids are already linked ids
        for &location in &self.globals {
            let allocator = match location {
                program::GlobalLocation::Constant => &mut constant_space,
                program::GlobalLocation::SharedStatic => &mut shared_static_space,
                program::GlobalLocation::LocalStatic => &mut local_static_space,
            };
            let (offset, byte_len) = allocator.allocate(Word::BYTE_LEN, &bytes);
            globals.push(program::Global::new(
                location,
                offset,
                byte_len,
                TypeId(0),
                location != program::GlobalLocation::Constant,
            ));
        }

        (
            globals,
            constant_space.build(),
            shared_static_space.build(),
            local_static_space.build(),
        )
    }

    /// Build Program function entries from one self-contained bytecode object.
    fn functions(
        &self,
        object: &bytecode::Object,
    ) -> (
        StringPool,
        Vec<destack_core::StringId>,
        FunctionTableBuilder,
        Vec<BindingBuilder>,
    ) {
        let strings = StringPool::new();
        let mut names = Vec::with_capacity(object.functions().len());
        let mut signatures = Vec::with_capacity(object.functions().len());
        let mut functions = Vec::with_capacity(object.functions().len());
        let mut bindings = Vec::with_capacity(self.bindings.len());

        // preserve bytecode function order as dense Program identity
        for (index, _) in object.functions().iter().enumerate() {
            let name = strings.intern(&format!("f{index}"));
            let signature = SignatureId(index as u32);
            let entry = FunctionBuilder::new(name, signature);
            if let Some(binding) = self.bindings.get(&(index as u32)) {
                let binding_name = strings.intern(binding);
                names.push(binding_name);
                bindings.push(BindingBuilder::new(
                    BindingId::from_name(binding),
                    binding_name,
                    FunctionId(index as u32),
                    BindingEffect::Pure,
                    BindingProvider::Runtime,
                    BindingReplay::Recordable,
                    BindingAffinity::None,
                ));
            }
            names.push(name);
            let signature = self
                .signatures
                .get(index)
                .and_then(Option::as_ref)
                .or(self.default_signature.as_ref())
                .unwrap_or_else(|| panic!("test function f{index} requires a signature"))
                .clone();
            signatures.push(signature);
            functions.push(entry);
        }

        let functions = (0..functions.len())
            .map(|index| Symbol::from_raw(index as u64))
            .zip(functions);
        let table = FunctionTableBuilder::new()
            .signatures(signatures)
            .functions(functions);

        (strings, names, table, bindings)
    }

    /// Build physical bytecode maps and canonical Program frame layouts.
    fn frames(
        &self,
        object: &bytecode::Object,
    ) -> (
        Vec<bytecode::Function>,
        Vec<bytecode::FrameMap>,
        Vec<bytecode::RegisterSpan>,
        FrameTableBuilder,
        HashMap<FramePoint, program::FrameStateId>,
    ) {
        assert!(
            object.frames().is_empty() && object.registers().is_empty(),
            "parsed bytecode must not infer frame maps"
        );
        let mut configured = self.frames.clone();
        configured.sort_unstable_by_key(|frame| (frame.function, frame.operation));
        let mut functions = object.functions().to_vec();
        let mut maps = Vec::new();
        let mut registers = Vec::new();
        let mut states = Vec::new();
        let mut layouts = Vec::new();

        // append each function's frame maps in logical operation order
        for (function_index, function) in functions.iter_mut().enumerate() {
            let entry_types = self
                .signatures
                .get(function_index)
                .and_then(Option::as_ref)
                .into_iter()
                .flat_map(|signature| &signature.parameters)
                .copied()
                .collect::<Vec<_>>();
            let entry_register_count = entry_types
                .iter()
                .map(|ty| {
                    self.layouts
                        .iter()
                        .find(|layout| layout.ty == *ty)
                        .map_or(1, |layout| layout.byte_len.div_ceil(Word::BYTE_LEN as u32))
                })
                .sum::<u32>();
            let entry_register_count = u16::try_from(entry_register_count)
                .expect("test function entry should fit one register window");
            function.register_count = function.register_count.max(entry_register_count);

            // append operation maps after the entry state
            for frame in configured
                .iter()
                .filter(|frame| frame.function as usize == function_index)
            {
                let point = ProgramPoint::new(FunctionId(function_index as u32), frame.operation);
                Self::append_frame(
                    FramePoint::operation(point),
                    &frame.values,
                    &mut maps,
                    &mut registers,
                    &mut states,
                    &mut layouts,
                    &self.layouts,
                );
            }
        }

        let frame_states = states
            .iter()
            .enumerate()
            .map(|(index, state)| (state.point, program::FrameStateId(index as u32)))
            .collect();
        let table = FrameTableBuilder::new().states(states).layouts(layouts);

        (functions, maps, registers, table, frame_states)
    }

    /// Append one aligned logical and physical test frame.
    fn append_frame(
        point: FramePoint,
        values: &[(bytecode::RegisterSpan, TypeId)],
        maps: &mut Vec<bytecode::FrameMap>,
        registers: &mut Vec<bytecode::RegisterSpan>,
        states: &mut Vec<FrameState>,
        layouts: &mut Vec<FrameLayoutBuilder>,
        types: &[TestLayout],
    ) {
        let register_start = registers.len() as u32;
        registers.extend(values.iter().map(|(registers, _)| *registers));
        maps.push(bytecode::FrameMap::new(EntryRange::new(
            register_start,
            values.len() as u32,
        )));

        // pack retained values under their concrete Program layouts
        let mut byte_len = 0u32;
        let mut alignment = 1u32;
        let slots = values
            .iter()
            .map(|(_, ty)| {
                let layout = types.iter().find(|layout| layout.ty == *ty);
                let value_byte_len = layout.map_or(Word::BYTE_LEN as u32, |layout| layout.byte_len);
                let value_alignment =
                    layout.map_or(Word::BYTE_LEN as u32, |layout| layout.alignment);
                byte_len = byte_len.next_multiple_of(value_alignment);
                alignment = alignment.max(value_alignment);
                let slot = FrameSlot::new(byte_len, value_byte_len, *ty);
                byte_len += value_byte_len;

                slot
            })
            .collect::<Vec<_>>();
        byte_len = byte_len.next_multiple_of(alignment);
        let layout = FrameLayoutId(layouts.len() as u32);
        layouts.push(FrameLayoutBuilder::new(byte_len, alignment).slots(slots));
        states.push(FrameState::new(point, layout));
    }
}

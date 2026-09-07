use destack_bytecode as bytecode;
use destack_core::{EntryRange, Optional};
use destack_mir as mir;
use destack_program::{Object, Word};
use destack_source::{ModuleId, ProvenanceId};

use crate::LinkResult;

use super::BytecodeLinker;

impl<'a, 'b> BytecodeLinker<'a, 'b> {
    /// Link one bytecode function and append its operation and code sections.
    pub(super) fn link_function(
        &self,
        module: ModuleId,
        function: mir::FunctionId,
        object: &Object,
        source: &bytecode::Function,
        object_code: &[u8],
        operations: &mut Vec<(bytecode::CodeOffset, ProvenanceId)>,
        mappings: &mut Vec<bytecode::Mapping>,
        code: &mut Vec<u8>,
    ) -> LinkResult<bytecode::Function> {
        let bytecode = self.object(object)?;

        // append function relative logical operation offsets
        let operation_start = operations.len() as u32;
        let source_operations = source.operations(bytecode.operations());
        let source_provenance = source.operation_provenances(bytecode.operation_provenances());
        if source_operations.len() != source_provenance.len() {
            return Err(self.program.invalid_input(
                "bytecode operation provenance count differs from operation count",
            ));
        }
        for (&offset, &provenance) in source_operations.iter().zip(source_provenance) {
            let provenance = self.program.provenance(module, provenance)?;
            operations.push((offset, provenance));
        }
        let operation_count = operations.len() as u32 - operation_start;

        // append provenance mappings with Program identities
        let mapping_start = mappings.len() as u32;
        let source_mappings = source.mappings(bytecode.mappings());
        for mapping in source_mappings {
            let provenance = self.program.provenance(module, mapping.provenance)?;
            mappings.push(bytecode::Mapping::new(
                mapping.offset,
                mapping.operation,
                provenance,
            ));
        }
        let mapping_count = mappings.len() as u32 - mapping_start;

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
            EntryRange::new(operation_start, operation_count),
            EntryRange::new(mapping_start, mapping_count),
            register_count,
        ))
    }

    /// Return the register width required by emitted code and the callable ABI.
    pub(super) fn register_count(
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
    pub(super) fn function<'c>(
        &self,
        object: &'c Object,
        function: mir::FunctionId,
    ) -> LinkResult<&'c bytecode::Function> {
        let index = object
            .functions()
            .binary_search_by_key(&function, |declaration| declaration.id)
            .map_err(|_| self.program.invalid_input("object function is absent"))?;

        self.object(object)?
            .functions()
            .get(index)
            .ok_or_else(|| self.program.invalid_input("bytecode function is absent"))
    }
}

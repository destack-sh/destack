use tspp_core::{EntryRange, SectionBuilder};

use crate::tree::object::Header;
use crate::{CodeOffset, CodeRange, FrameMap, Function, Object, RegisterSpan, Relocation};

/// Bytecode object under construction.
#[derive(Debug, Default)]
pub struct ObjectBuilder {
    /// Physical functions in object-local function order.
    functions: Vec<Function>,
    /// Physical frame maps in object-local frame state order.
    frames: Vec<FrameMap>,
    /// Flattened register spans referenced by frame maps.
    registers: Vec<RegisterSpan>,
    /// Function-relative byte offsets of logical operations.
    operations: Vec<CodeOffset>,
    /// Relocatable identity operands in function code.
    relocations: Vec<Relocation>,
    /// Contiguous instruction bytes for every defined function.
    code: Vec<u8>,
}

impl ObjectBuilder {
    /// Create an empty bytecode object builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set physical functions in object-local function order.
    pub fn functions(mut self, functions: impl IntoIterator<Item = Function>) -> Self {
        self.functions = functions.into_iter().collect();

        self
    }

    /// Set physical frame maps in object-local frame state order.
    pub fn frames(mut self, frames: impl IntoIterator<Item = FrameMap>) -> Self {
        self.frames = frames.into_iter().collect();

        self
    }

    /// Set flattened register spans referenced by frame maps.
    pub fn registers(mut self, registers: impl IntoIterator<Item = RegisterSpan>) -> Self {
        self.registers = registers.into_iter().collect();

        self
    }

    /// Set function-relative byte offsets of logical operations.
    pub fn operations(mut self, operations: impl IntoIterator<Item = CodeOffset>) -> Self {
        self.operations = operations.into_iter().collect();

        self
    }

    /// Set relocatable identity operands in function code.
    pub fn relocations(mut self, relocations: impl IntoIterator<Item = Relocation>) -> Self {
        self.relocations = relocations.into_iter().collect();

        self
    }

    /// Set contiguous instruction bytes for every defined function.
    pub fn code(mut self, code: impl Into<Vec<u8>>) -> Self {
        self.code = code.into();

        self
    }

    /// Append logical operation offsets and return their object-local range.
    pub(crate) fn push_operations(
        &mut self,
        operations: impl IntoIterator<Item = CodeOffset>,
    ) -> EntryRange<CodeOffset> {
        let start = self.operations.len();
        self.operations.extend(operations);

        EntryRange::new(start as u32, (self.operations.len() - start) as u32)
    }

    /// Append one encoded function body and return its code range.
    pub(crate) fn push_code(
        &mut self,
        bytes: &[u8],
        relocations: impl IntoIterator<Item = Relocation>,
    ) -> CodeRange {
        let byte_offset = self.code.len() as u32;
        let byte_len = bytes.len() as u32;
        self.code.extend_from_slice(bytes);
        self.relocations.extend(
            relocations
                .into_iter()
                .map(|relocation| relocation.rebase(byte_offset)),
        );

        CodeRange {
            byte_offset,
            byte_len,
        }
    }

    /// Build one immutable bytecode object.
    pub fn build(self) -> Object {
        let mut sections = SectionBuilder::new();
        let mut header = Header::new();
        let header_section = sections.insert([header]);

        // pack physical execution tables
        header.functions = sections.insert(self.functions);
        header.frames = sections.insert(self.frames);
        header.registers = sections.insert(self.registers);
        header.operations = sections.insert(self.operations);

        // pack relocations and executable bytes
        header.relocations = sections.insert(self.relocations);
        header.code = sections.insert(self.code);

        // finalize the fixed header after all section offsets are known
        header.byte_len = sections.view().byte_len() as u64;
        sections.replace(header_section, [header]);

        Object::from_storage(sections.build())
    }
}

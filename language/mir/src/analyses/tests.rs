use destack_core::StringPool;
use destack_source::FileId;

use crate as mir;
use crate::analyses::{FunctionAnalyses, ModuleAnalyses};
use crate::parse::{ParseOptions, Parser};
use crate::{Function, LocalNodeId, Tree};

/// Parse one MIR test module and return its single function.
pub(crate) fn parse_test_function(source: &str) -> (Tree, LocalNodeId<Function>) {
    let (tree, _) = Parser::parse(FileId::new(0), source, ParseOptions::default())
        .finish()
        .expect("parse failed");

    let function_id = tree
        .iter_nodes::<Function>()
        .next()
        .map(|(function_id, _)| function_id)
        .expect("missing function");

    (tree, function_id)
}

/// Test program for analysis tests.
///
/// Parses MIR from text and exposes lookup helpers over the parsed tree.
pub(crate) struct TestProgram {
    /// The MIR tree.
    pub(crate) tree: Tree,
    /// String pool for identifiers (immutable, from parser).
    strings: StringPool,
}

#[allow(dead_code)]
impl TestProgram {
    /// Create a new test program from MIR source text.
    pub(crate) fn new(source: &str) -> Self {
        let (tree, strings) = Parser::parse(FileId::new(0), source, ParseOptions::default())
            .finish()
            .expect("failed to parse MIR");

        Self { tree, strings }
    }

    /// Create a function analysis cache for this program.
    pub(crate) fn function_analyses(&self) -> FunctionAnalyses {
        FunctionAnalyses::new()
    }

    /// Create a module analysis cache for this program.
    pub(crate) fn module_analyses(&self) -> ModuleAnalyses {
        ModuleAnalyses::new()
    }

    /// Return the entry function id, preferring a function named `test`.
    pub(crate) fn entry_function_id(&self) -> LocalNodeId<Function> {
        // require the canonical test entry
        self.tree
            .iter_nodes::<Function>()
            .find(|(_, function)| {
                function.entry().is_some() && self.strings.get(function.name) == "test"
            })
            .map(|(function_id, _)| function_id)
            .expect("missing test function")
    }

    /// Return the stack allocation destinations in a function entry block.
    pub(crate) fn frame_alloc_destinations_in_entry(
        &self,
        function_id: LocalNodeId<Function>,
    ) -> Vec<mir::Value> {
        // read the entry block
        let block_id = self.entry_block_id(function_id);
        let block = self.tree.get(block_id);

        // collect stack allocation destinations in order
        block
            .instructions
            .iter()
            .filter_map(|instruction_id| {
                if let mir::Instruction::FrameAllocZeroed { destination, .. } =
                    self.tree.get(*instruction_id)
                {
                    Some(*destination)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Return the first function id in the program.
    pub(crate) fn first_function_id(&self) -> LocalNodeId<Function> {
        self.tree
            .iter_nodes::<Function>()
            .next()
            .expect("missing function")
            .0
    }

    /// Return the function id for a named function.
    pub(crate) fn function_id_by_name(&self, name: &str) -> LocalNodeId<Function> {
        self.tree
            .iter_nodes::<Function>()
            .find(|(_, function)| self.strings.get(function.name) == name)
            .expect("missing function")
            .0
    }

    /// Return the entry block id for a function.
    pub(crate) fn entry_block_id(
        &self,
        function_id: LocalNodeId<Function>,
    ) -> LocalNodeId<mir::Block> {
        let function = self.tree.get(function_id);

        function.entry().expect("missing entry block")
    }

    /// Return the instruction ids in a block.
    pub(crate) fn instructions_in_block(
        &self,
        block_id: LocalNodeId<mir::Block>,
    ) -> Vec<LocalNodeId<mir::Instruction>> {
        self.tree.get(block_id).instructions.clone()
    }

    /// Return the instruction ids in the entry block.
    pub(crate) fn entry_instructions(
        &self,
        function_id: LocalNodeId<Function>,
    ) -> Vec<LocalNodeId<mir::Instruction>> {
        self.instructions_in_block(self.entry_block_id(function_id))
    }

    /// Return the first intrinsic instruction in the entry block.
    pub(crate) fn first_intrinsic_in_entry(
        &self,
        function_id: LocalNodeId<Function>,
        intrinsic: mir::Intrinsic,
    ) -> LocalNodeId<mir::Instruction> {
        // read the entry block
        let block_id = self.entry_block_id(function_id);
        let block = self.tree.get(block_id);

        // scan instructions in order
        for instruction_id in &block.instructions {
            if matches!(
                self.tree.get(*instruction_id),
                mir::Instruction::Intrinsic { intrinsic: inst, .. } if *inst == intrinsic
            ) {
                return *instruction_id;
            }
        }

        panic!("missing intrinsic instruction");
    }

    /// Return the first call instruction and callee in a function entry block.
    pub(crate) fn first_call_in_entry(
        &self,
        function_id: LocalNodeId<Function>,
    ) -> (LocalNodeId<mir::Instruction>, LocalNodeId<Function>) {
        // read the entry block for the function
        let function = self.tree.get(function_id);
        let block = self.tree.get(function.block(0));

        // locate the first call instruction
        let call_inst = block
            .instructions
            .iter()
            .copied()
            .find(|id| matches!(self.tree.get(*id), mir::Instruction::Call { .. }))
            .expect("missing call instruction");

        // read the callee from the call instruction
        let mir::Instruction::Call {
            function: callee, ..
        } = self.tree.get(call_inst)
        else {
            panic!("expected call instruction");
        };

        (call_inst, *callee)
    }

    /// Attach memory access metadata to an instruction.
    pub(crate) fn insert_memory_accesses(
        &mut self,
        instruction: LocalNodeId<mir::Instruction>,
        accesses: Vec<mir::MemoryAccessMetadata>,
    ) {
        // insert the metadata entries
        self.tree
            .metadata
            .memory
            .insert_memory_accesses(instruction, accesses);
    }

    /// Attach reference-location metadata to an instruction.
    pub(crate) fn insert_reference_location(
        &mut self,
        instruction: LocalNodeId<mir::Instruction>,
        kind: mir::MemoryAccessKind,
        pointer: mir::Value,
        size: Option<u64>,
    ) {
        self.insert_reference_location_with_options(instruction, kind, pointer, size, false, None);
    }

    /// Attach reference-location metadata to an instruction with flags.
    pub(crate) fn insert_reference_location_with_options(
        &mut self,
        instruction: LocalNodeId<mir::Instruction>,
        kind: mir::MemoryAccessKind,
        pointer: mir::Value,
        size: Option<u64>,
        is_volatile: bool,
        ordering: Option<mir::MemoryOrdering>,
    ) {
        // build the access metadata
        let access = mir::MemoryAccessMetadata {
            kind,
            target: mir::MemoryAccessTarget::Reference(pointer),
            size,
            alignment: None,
            is_volatile,
            is_load_invariant: false,
            ordering,
            scope: None,
            memory_scope: None,
            flags: None,
            space: None,
        };

        // insert the metadata entry
        self.tree
            .metadata
            .memory
            .insert_memory_accesses(instruction, vec![access]);
    }
}

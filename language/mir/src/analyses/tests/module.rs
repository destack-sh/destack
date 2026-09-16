use std::sync::Arc;

use destack_core::StringPool;
use destack_source::{DiagnosticSeverity, ModuleId};

use crate as mir;
use crate::analyses::{FunctionCache, ModuleCache};
use crate::parse::{ParseOptions, Parser, test_file};
use crate::{DispatchTable, EffectTable, Function, LayoutBuilder, LayoutTable, LocalNodeId, Tree};

/// One parsed MIR module used by analysis tests.
pub(crate) struct TestModule {
    /// The MIR tree.
    pub(crate) tree: Tree,
    /// Canonical layouts for the represented fixture types.
    pub(crate) layouts: Arc<LayoutTable>,
    /// Canonical MIR dispatch table.
    pub(crate) dispatch: DispatchTable,

    /// Function and call effect table.
    pub(crate) effects: EffectTable,
    /// String pool for identifiers (immutable, from parser).
    strings: StringPool,
}

impl TestModule {
    /// Parse MIR source in the default test module.
    pub(crate) fn new(source: &str) -> Self {
        Self::parse(source, mir::TEST_MODULE)
    }

    /// Parse MIR source with a distinct module identity.
    pub(crate) fn parse(source: &str, module: ModuleId) -> Self {
        // parse the module with its declaring identity
        let file = test_file(source);
        let options = ParseOptions {
            module,
            ..ParseOptions::default()
        };
        let parsed = Parser::parse(&file, options).expect("MIR parser requires text content");
        let (
            tree,
            target_layout,
            mut layouts,
            dispatch,
            _drops,
            effects,
            _profile,
            strings,
            diagnostics,
        ) = parsed.into_parts();

        // reject malformed fixtures
        if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
            panic!("failed to parse MIR: {diagnostics:?}");
        }

        // construct the layouts supplied by lowering in compiler consumers
        LayoutBuilder::new(&tree, &mut layouts, target_layout)
            .layout_reachable_types()
            .expect("fixture types require valid layouts");

        Self {
            tree,
            layouts: Arc::new(layouts),
            dispatch,
            effects,
            strings,
        }
    }

    /// Parse one MIR test module and return its first function.
    pub(crate) fn parse_function(source: &str) -> (Tree, LocalNodeId<Function>) {
        let test = Self::new(source);
        let function = test
            .tree
            .iter_nodes::<Function>()
            .next()
            .map(|(function, _)| function)
            .expect("missing function");

        (test.tree, function)
    }

    /// Create function analyses for this module.
    pub(crate) fn function_analyses(&self) -> FunctionCache {
        FunctionCache::new()
    }

    /// Create analyses for this module.
    pub(crate) fn module_analyses(&self) -> ModuleCache {
        ModuleCache::new()
    }

    /// Return the defined function named `test`.
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

    /// Return the local address destinations in a function entry block.
    pub(crate) fn local_address_destinations_in_entry(
        &self,
        function_id: LocalNodeId<Function>,
    ) -> Vec<mir::Value> {
        // read the entry block
        let block_id = self.entry_block_id(function_id);
        let block = self.tree.get(block_id);

        // collect local address destinations in order
        block
            .instructions
            .iter()
            .filter_map(|instruction_id| {
                if let mir::Instruction::Address {
                    destination, place, ..
                } = self.tree.get(*instruction_id)
                    && matches!(place.origin, mir::PlaceOrigin::Local(_))
                    && place.path.is_root()
                {
                    Some(*destination)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Return the first function id in the module.
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
            call:
                mir::Call {
                    callee:
                        mir::Callee::Direct {
                            function: callee, ..
                        },
                    ..
                },
            ..
        } = self.tree.get(call_inst)
        else {
            panic!("expected call instruction");
        };

        (call_inst, *callee)
    }
}

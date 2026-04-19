use std::sync::Arc;

use destack_core::{ImmutableStringPool, StringPool};
use destack_mir as mir;
use destack_source::{DiffOptions, FileId, ModuleId, PackageId, TargetId, print_diff};
use mir::parse::ParseOptions;

use crate::optimize::{FunctionPass, ModulePass, PipelineContext, PipelineOptions};
use crate::{OptimizeError, OptimizeWarning};

/// Placeholder module id for tests.
fn test_module_id() -> ModuleId {
    ModuleId::new(PackageId::new(0), 0)
}

/// Placeholder target id for tests.
fn test_target_id() -> TargetId {
    test_target_id_for_package(PackageId::new(0), "test")
}

/// Placeholder target id for one package and test name.
fn test_target_id_for_package(package_id: PackageId, name: &str) -> TargetId {
    TargetId::new(package_id, name)
}

/// Test program for optimization passes.
///
/// Parses MIR from text, applies passes, and formats the result back to text.
pub(crate) struct TestProgram {
    /// The MIR node tree.
    pub(crate) tree: mir::NodeTree,
    /// String pool for identifiers (immutable, from parser).
    strings: ImmutableStringPool,
    /// Thread safe string pool for optimization context.
    strings_pool: StringPool,
    /// Errors collected from the last pass run.
    errors: Vec<OptimizeError>,
    /// Warnings collected from the last pass run.
    warnings: Vec<OptimizeWarning>,
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
impl TestProgram {
    /// Create a new test program from MIR source text.
    pub(crate) fn new(source: &str) -> Self {
        let (tree, strings) =
            mir::parse::Parser::parse(FileId::new(0), source, ParseOptions::default())
                .validate()
                .expect("failed to parse MIR");
        let strings_pool = StringPool::new();

        // copy all strings from parser pool to context pool
        // (needed for passes that look up/intern strings via context)
        strings_pool.copy_from_immutable(&strings);

        Self {
            tree,
            strings,
            strings_pool,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Apply a function pass to all functions in the program.
    pub(crate) fn run_pass<P: FunctionPass + ?Sized>(&mut self, pass: &P) {
        self.run_pass_with_options_impl(pass, PipelineOptions::default(), None);
    }

    /// Apply a function pass with custom options.
    pub(crate) fn run_pass_with_options<P: FunctionPass + ?Sized>(
        &mut self,
        pass: &P,
        options: PipelineOptions,
    ) {
        self.run_pass_with_options_impl(pass, options, None);
    }

    /// Apply a function pass with profile data.
    pub(crate) fn run_pass_with_profile<P: FunctionPass + ?Sized>(
        &mut self,
        pass: &P,
        profile: mir::ProfileTable,
    ) {
        self.run_pass_with_options_impl(pass, PipelineOptions::default(), Some(Arc::new(profile)));
    }

    /// Return the entry function id for this program.
    pub(crate) fn entry_function_id(&self) -> mir::LocalNodeId<mir::Function> {
        // pick the first entry as a fallback
        let mut fallback = None;

        // scan for entry functions and prefer @test
        for (function_id, function) in self.tree.iter_nodes::<mir::Function>() {
            // skip non entry functions
            if function.entry.is_none() {
                continue;
            }

            // record the first entry for fallback
            if fallback.is_none() {
                fallback = Some(function_id);
            }

            // prefer the test entry when present
            if self.strings.get(function.name) == "test" {
                return function_id;
            }
        }

        fallback.expect("missing function")
    }

    /// Return the function id for a named function.
    pub(crate) fn function_id_by_name(&self, name: &str) -> mir::LocalNodeId<mir::Function> {
        self.tree
            .iter_nodes::<mir::Function>()
            .find(|(_, function)| self.strings.get(function.name) == name)
            .expect("missing function")
            .0
    }

    /// Set the return borrow region for a named function.
    pub(crate) fn set_function_lifetime(&mut self, name: &str, region: mir::BorrowRegion) {
        // update the target function
        let function_id = self.function_id_by_name(name);
        let function = self.tree.get_mut(function_id);
        function.return_region = region;
    }

    /// Return the entry block id for a function.
    pub(crate) fn entry_block_id(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> mir::LocalNodeId<mir::Block> {
        // read the function
        let function = self.tree.get(function_id);

        // use the explicit entry when present
        if let Some(entry) = function.entry {
            return entry;
        }

        // fall back to the first block when no entry exists
        *function.blocks.first().expect("missing block")
    }

    /// Return the first intrinsic instruction in a function.
    pub(crate) fn first_intrinsic_in_function(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
        intrinsic: mir::Intrinsic,
    ) -> mir::LocalNodeId<mir::Instruction> {
        // read the function blocks
        let function = self.tree.get(function_id);

        // scan blocks in order
        for block_id in &function.blocks {
            let block = self.tree.get(*block_id);
            for instruction_id in &block.instructions {
                if matches!(
                    self.tree.get(*instruction_id),
                    mir::Instruction::Intrinsic { intrinsic: inst, .. } if *inst == intrinsic
                ) {
                    return *instruction_id;
                }
            }
        }

        panic!("missing intrinsic instruction");
    }

    /// Return the first intrinsic instruction in the entry block.
    pub(crate) fn first_intrinsic_in_entry(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
        intrinsic: mir::Intrinsic,
    ) -> mir::LocalNodeId<mir::Instruction> {
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

    /// Return the instruction ids in a block.
    pub(crate) fn instructions_in_block(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
    ) -> Vec<mir::LocalNodeId<mir::Instruction>> {
        // clone the instruction ids for this block
        self.tree.get(block_id).instructions.clone()
    }

    /// Return the instruction ids in the entry block.
    pub(crate) fn entry_instructions(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Vec<mir::LocalNodeId<mir::Instruction>> {
        // read the entry block and clone the instruction ids
        self.instructions_in_block(self.entry_block_id(function_id))
    }

    /// Return call instruction ids in a function.
    pub(crate) fn call_instructions_in_function(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Vec<mir::LocalNodeId<mir::Instruction>> {
        // scan instructions in order
        let function = self.tree.get(function_id);
        let mut call_ids = Vec::new();

        for block_id in &function.blocks {
            let block = self.tree.get(*block_id);
            for instruction_id in &block.instructions {
                if matches!(
                    self.tree.get(*instruction_id),
                    mir::Instruction::Call { .. }
                ) {
                    call_ids.push(*instruction_id);
                }
            }
        }

        call_ids
    }

    /// Create a new alias scope.
    pub(crate) fn create_alias_scope(&mut self) -> mir::MemoryAliasScopeId {
        // create a new domain for the scope
        let domain = self.tree.metadata.memory.alias_scopes.create_domain(None);

        // create the scope within the domain
        self.tree
            .metadata
            .memory
            .alias_scopes
            .create_scope(domain, None)
    }

    /// Create a new type-alias node.
    pub(crate) fn create_type_alias_node(
        &mut self,
        parent: Option<mir::TypeAliasNodeId>,
        is_constant: bool,
    ) -> mir::TypeAliasNodeId {
        // insert a new node into the table
        self.tree
            .metadata
            .memory
            .type_alias
            .create_node(None, parent, is_constant)
    }

    /// Create a new type-alias tag.
    pub(crate) fn create_type_alias_tag(
        &mut self,
        base: mir::TypeAliasNodeId,
        access: mir::TypeAliasNodeId,
        offset: u64,
        size: u64,
        is_immutable: bool,
    ) -> mir::TypeAliasTagId {
        // insert a new tag into the table
        self.tree
            .metadata
            .memory
            .type_alias
            .create_tag(base, access, offset, size, is_immutable)
    }

    /// Return the destination of a stack allocation instruction.
    pub(crate) fn stack_alloc_destination(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
    ) -> mir::Value {
        // extract the destination value from the instruction
        let mir::Instruction::StackAlloc { destination, .. } = self.tree.get(instruction_id) else {
            panic!("expected stack allocation");
        };

        destination
            .value()
            .expect("stack allocation should produce a concrete value")
    }

    /// Return stack allocation destinations from the entry block.
    pub(crate) fn stack_alloc_destinations_in_entry(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Vec<mir::Value> {
        // read the entry block
        let block_id = self.entry_block_id(function_id);
        let block = self.tree.get(block_id);

        // collect stack allocation destinations in order
        block
            .instructions
            .iter()
            .filter_map(|instruction_id| {
                if let mir::Instruction::StackAlloc { destination, .. } =
                    self.tree.get(*instruction_id)
                {
                    Some(
                        destination
                            .value()
                            .expect("stack allocation should produce a concrete value"),
                    )
                } else {
                    None
                }
            })
            .collect()
    }

    /// Return store instruction ids from the entry block.
    pub(crate) fn store_instructions_in_entry(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Vec<mir::LocalNodeId<mir::Instruction>> {
        // read the entry block
        let block_id = self.entry_block_id(function_id);
        let block = self.tree.get(block_id);

        // collect store instructions in order
        block
            .instructions
            .iter()
            .copied()
            .filter(|instruction_id| {
                matches!(
                    self.tree.get(*instruction_id),
                    mir::Instruction::Store { .. }
                )
            })
            .collect()
    }

    /// Attach pointer access metadata to an instruction.
    pub(crate) fn insert_pointer_access(
        &mut self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        kind: mir::MemoryAccessKind,
        pointer: mir::Value,
        size: Option<u64>,
        alias_scopes: Vec<mir::MemoryAliasScopeId>,
        noalias_scopes: Vec<mir::MemoryAliasScopeId>,
        type_alias_tag: Option<mir::TypeAliasTagId>,
    ) {
        self.insert_pointer_access_with_options(
            instruction,
            kind,
            pointer,
            size,
            alias_scopes,
            noalias_scopes,
            type_alias_tag,
            false,
            None,
        );
    }

    /// Attach memory access metadata to an instruction.
    pub(crate) fn insert_memory_accesses(
        &mut self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        accesses: Vec<mir::MemoryAccessMetadata>,
    ) {
        // insert the metadata entries
        self.tree
            .metadata
            .memory
            .insert_memory_accesses(instruction, accesses);
    }

    /// Attach pointer access metadata to an instruction with flags.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn insert_pointer_access_with_options(
        &mut self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        kind: mir::MemoryAccessKind,
        pointer: mir::Value,
        size: Option<u64>,
        alias_scopes: Vec<mir::MemoryAliasScopeId>,
        noalias_scopes: Vec<mir::MemoryAliasScopeId>,
        type_alias_tag: Option<mir::TypeAliasTagId>,
        is_volatile: bool,
        ordering: Option<mir::MemoryOrdering>,
    ) {
        // build the access metadata
        let access = mir::MemoryAccessMetadata {
            kind,
            target: mir::MemoryAccessTarget::Pointer(pointer),
            size,
            alignment: None,
            is_volatile,
            is_load_invariant: false,
            ordering,
            scope: None,
            memory_scope: None,
            semantics: None,
            address_space: None,
            alias_scopes,
            noalias_scopes,
            type_alias_tag,
        };

        // insert the metadata entry
        self.tree
            .metadata
            .memory
            .insert_memory_accesses(instruction, vec![access]);
    }

    /// Return the first call instruction and callee in the entry function.
    pub(crate) fn first_call_in_entry(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> (
        mir::LocalNodeId<mir::Instruction>,
        mir::LocalNodeId<mir::Function>,
    ) {
        // read the entry block for the function
        let function = self.tree.get(function_id);
        let block = self.tree.get(function.blocks[0]);

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

        (
            call_inst,
            callee
                .function()
                .expect("call instruction should reference a concrete function"),
        )
    }

    /// Insert a function pointer type for a callee signature.
    pub(crate) fn call_signature_for_callee(
        &mut self,
        callee: mir::LocalNodeId<mir::Function>,
    ) -> mir::LocalNodeId<mir::Type> {
        // read the callee signature
        let callee_function = self.tree.get(callee);
        let param_tys = callee_function
            .parameters
            .iter()
            .map(|param| param.ty)
            .collect::<Vec<_>>();
        let return_ty = callee_function.return_type;

        // insert the function pointer type
        self.tree.insert_type(mir::Type::FunctionPointer {
            parameters: param_tys,
            result: return_ty,
        })
    }

    /// Internal implementation that handles the borrow correctly.
    fn run_pass_with_options_impl<P: FunctionPass + ?Sized>(
        &mut self,
        pass: &P,
        options: PipelineOptions,
        profile: Option<Arc<mir::ProfileTable>>,
    ) {
        let context = PipelineContext::new(
            &self.strings_pool,
            options,
            test_module_id(),
            test_target_id(),
            profile,
        );

        // collect function ids
        let function_ids: Vec<_> = self
            .tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect();

        // run pass on each function
        for function_id in function_ids {
            let mut function = self.tree.get(function_id).clone();

            // skip imported functions (no body)
            if function.entry.is_none() {
                continue;
            }

            // enforce pass requirements
            if !context.enforce_function_requirements(
                pass.metadata(),
                function_id,
                &function,
                &self.tree,
            ) {
                continue;
            }

            // recompute next_value_id so passes can allocate fresh values
            function.recompute_next_value_id(&self.tree);
            pass.run(&mut function, &mut self.tree, &context);
            *self.tree.get_mut(function_id) = function;
        }

        // collect diagnostics after pass completes
        self.errors = context.take_errors();
        self.warnings = context.take_warnings();
    }

    /// Return the first function id in the program.
    pub(crate) fn first_function_id(&self) -> mir::LocalNodeId<mir::Function> {
        self.tree
            .iter_nodes::<mir::Function>()
            .next()
            .expect("missing function")
            .0
    }

    /// Return entry branch targets for a function.
    pub(crate) fn entry_branch_targets(
        &self,
        function: &mir::Function,
    ) -> (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>) {
        // read the entry block
        let entry = function.entry.expect("missing entry block");
        let entry_block = self.tree.get(entry);
        let terminator = self.tree.get(entry_block.terminator);

        // extract the branch targets
        let mir::Terminator::Branch {
            then_target,
            else_target,
            ..
        } = terminator
        else {
            panic!("expected entry branch");
        };

        // return the targets
        (
            then_target
                .block
                .block()
                .expect("branch should reference a concrete then block"),
            else_target
                .block
                .block()
                .expect("branch should reference a concrete else block"),
        )
    }

    /// Return the jump target for a block.
    pub(crate) fn jump_target(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
    ) -> mir::LocalNodeId<mir::Block> {
        // read the block terminator
        let block = self.tree.get(block_id);
        let terminator = self.tree.get(block.terminator);
        let mir::Terminator::Jump { target, .. } = terminator else {
            panic!("expected jump terminator");
        };

        // return the target
        target
            .block
            .block()
            .expect("jump should reference a concrete block")
    }

    /// Record a jump edge profile count.
    pub(crate) fn record_jump_edge_profile(
        &self,
        profile: &mut mir::ProfileTable,
        source: mir::LocalNodeId<mir::Block>,
        target: mir::LocalNodeId<mir::Block>,
        count: u64,
    ) {
        // record the edge profile count
        profile.edges.insert(
            mir::EdgeKey::new(source, mir::EdgeKind::Jump, target),
            mir::EdgeProfile {
                count: mir::ProfileCount::new(count, mir::ProfileConfidence::Precise),
            },
        );
    }

    /// Record a function entry profile count.
    pub(crate) fn record_function_profile(
        &self,
        profile: &mut mir::ProfileTable,
        function: mir::LocalNodeId<mir::Function>,
        count: u64,
    ) {
        // record the function entry count
        profile.functions.insert(
            function,
            mir::FunctionProfile {
                entry_count: mir::ProfileCount::new(count, mir::ProfileConfidence::Precise),
            },
        );
    }

    /// Record a block execution profile count.
    pub(crate) fn record_block_profile(
        &self,
        profile: &mut mir::ProfileTable,
        block: mir::LocalNodeId<mir::Block>,
        count: u64,
    ) {
        // record the block execution count
        profile.blocks.insert(
            block,
            mir::BlockProfile {
                execution_count: mir::ProfileCount::new(count, mir::ProfileConfidence::Precise),
            },
        );
    }

    /// Record a control flow edge profile count.
    pub(crate) fn record_edge_profile(
        &self,
        profile: &mut mir::ProfileTable,
        source: mir::LocalNodeId<mir::Block>,
        kind: mir::EdgeKind,
        target: mir::LocalNodeId<mir::Block>,
        count: u64,
    ) {
        // record the edge execution count
        let edge = mir::EdgeKey::new(source, kind, target);
        profile.edges.insert(
            edge,
            mir::EdgeProfile {
                count: mir::ProfileCount::new(count, mir::ProfileConfidence::Precise),
            },
        );
    }

    /// Record a callsite profile count.
    pub(crate) fn record_callsite_profile(
        &self,
        profile: &mut mir::ProfileTable,
        callsite: mir::LocalNodeId<mir::Instruction>,
        count: u64,
    ) {
        // record the callsite profile count
        profile.callsites.insert(
            callsite,
            mir::CallSiteProfile {
                total_count: mir::ProfileCount::new(count, mir::ProfileConfidence::Precise),
                targets: Vec::new(),
                unknown_count: mir::ProfileCount::new(0, mir::ProfileConfidence::Precise),
            },
        );
    }

    /// Apply a module pass to the program.
    pub(crate) fn run_module_pass<P: ModulePass + ?Sized>(&mut self, pass: &P) {
        let context = PipelineContext::new(
            &self.strings_pool,
            PipelineOptions::default(),
            test_module_id(),
            test_target_id(),
            None,
        );

        // enforce pass requirements
        if context.enforce_module_requirements(pass.metadata(), &self.tree) {
            pass.run(&mut self.tree, &context);
        }

        // collect diagnostics after pass completes
        self.errors = context.take_errors();
        self.warnings = context.take_warnings();
    }

    /// Apply a module pass with custom options.
    pub(crate) fn run_module_pass_with_options<P: ModulePass + ?Sized>(
        &mut self,
        pass: &P,
        options: PipelineOptions,
    ) {
        let context = PipelineContext::new(
            &self.strings_pool,
            options,
            test_module_id(),
            test_target_id(),
            None,
        );

        // enforce pass requirements
        if context.enforce_module_requirements(pass.metadata(), &self.tree) {
            pass.run(&mut self.tree, &context);
        }

        // collect diagnostics after pass completes
        self.errors = context.take_errors();
        self.warnings = context.take_warnings();
    }

    /// Apply a module pass with profile data.
    pub(crate) fn run_module_pass_with_profile<P: ModulePass + ?Sized>(
        &mut self,
        pass: &P,
        profile: mir::ProfileTable,
    ) {
        let context = PipelineContext::new(
            &self.strings_pool,
            PipelineOptions::default(),
            test_module_id(),
            test_target_id(),
            Some(Arc::new(profile)),
        );

        // enforce pass requirements
        if context.enforce_module_requirements(pass.metadata(), &self.tree) {
            pass.run(&mut self.tree, &context);
        }

        // collect diagnostics after pass completes
        self.errors = context.take_errors();
        self.warnings = context.take_warnings();
    }

    /// Format the MIR back to text.
    pub(crate) fn format(&self) -> String {
        let strings = self.strings_pool.clone().into_immutable();
        mir::format_mir(&self.tree, &strings, mir::MirFormatOptions::default())
    }

    /// Get string by id from the string pool.
    pub(crate) fn get_string(&self, id: destack_core::StringId) -> &str {
        self.strings.get(id)
    }

    /// Get errors from the last pass run.
    pub(crate) fn errors(&self) -> &[OptimizeError] {
        &self.errors
    }

    /// Get warnings from the last pass run.
    pub(crate) fn warnings(&self) -> &[OptimizeWarning] {
        &self.warnings
    }

    /// Assert that the current MIR matches the expected output.
    #[track_caller]
    pub(crate) fn assert_output(&self, expected: &str) {
        let actual = self.format();
        let expected = expected.trim();
        let actual = actual.trim();

        if actual != expected {
            print_diff(expected, actual, &DiffOptions::new());
            panic!("optimization output mismatch");
        }
    }

    /// Assert that the MIR is unchanged from the original source.
    #[track_caller]
    pub(crate) fn assert_unchanged(&self, original: &str) {
        self.assert_output(original);
    }

    /// Assert that no errors were emitted.
    #[track_caller]
    pub(crate) fn assert_no_errors(&self) {
        if !self.errors.is_empty() {
            panic!("expected no errors, got: {:?}", self.errors);
        }
    }

    /// Assert that no warnings were emitted.
    #[track_caller]
    pub(crate) fn assert_no_warnings(&self) {
        if !self.warnings.is_empty() {
            panic!("expected no warnings, got: {:?}", self.warnings);
        }
    }

    /// Assert that at least one error matches the predicate.
    #[track_caller]
    pub(crate) fn assert_error<F>(&self, predicate: F)
    where
        F: Fn(&OptimizeError) -> bool,
    {
        let has_match = self.errors.iter().any(predicate);
        if !has_match {
            panic!(
                "expected an error matching predicate, got: {:?}",
                self.errors
            );
        }
    }

    /// Assert that at least one warning matches the predicate.
    #[track_caller]
    pub(crate) fn assert_warning<F>(&self, predicate: F)
    where
        F: Fn(&OptimizeWarning) -> bool,
    {
        let has_match = self.warnings.iter().any(predicate);
        if !has_match {
            panic!(
                "expected a warning matching predicate, got: {:?}",
                self.warnings
            );
        }
    }

    // ================================================================================
    // legacy compatibility
    // ================================================================================

    /// Apply a function pass and return any errors emitted.
    ///
    /// Prefer using `run_pass` followed by `assert_*` methods instead.
    #[allow(dead_code)]
    pub(crate) fn run_pass_collecting_errors<P: FunctionPass + ?Sized>(
        &mut self,
        pass: &P,
    ) -> Vec<OptimizeError> {
        self.run_pass(pass);
        self.errors.clone()
    }

    /// Create a FunctionAnalyses storage for testing the new infrastructure.
    pub(crate) fn function_analyses<'a>(
        &'a self,
        function: &'a mir::Function,
    ) -> super::FunctionAnalyses<'a> {
        super::FunctionAnalyses::new(function, &self.tree)
    }

    /// Create a ModuleAnalyses storage for testing the new infrastructure.
    pub(crate) fn module_analyses(&self) -> super::ModuleAnalyses<'_> {
        super::ModuleAnalyses::new(&self.tree)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use destack_compiler_macros::declare_pass;
    use destack_core::StringPool;
    use destack_mir as mir;

    use super::TestProgram;
    use crate::OptimizeError;
    use crate::optimize::common::instruction_is_speculatable;
    use crate::optimize::passes::{InterproceduralSccp, LoadPre};
    use crate::optimize::{
        Analysis, AnalysisId, AnalysisPreservation, FunctionAnalyses, FunctionAnalysis, ModulePass,
        PipelineContext, PipelineOptions,
    };

    /// Simple test analysis with no dependencies.
    struct TestAnalysisA {
        computed: bool,
    }

    impl Analysis for TestAnalysisA {
        const ID: AnalysisId = AnalysisId("test-a");
        const DEPENDENCIES: &'static [AnalysisId] = &[];
    }

    impl FunctionAnalysis for TestAnalysisA {
        fn compute(
            _function: &mir::Function,
            _tree: &mir::NodeTree,
            _analyses: &FunctionAnalyses<'_>,
        ) -> Self {
            Self { computed: true }
        }
    }

    /// Test analysis that depends on TestAnalysisA.
    struct TestAnalysisB {
        a_computed: bool,
    }

    impl Analysis for TestAnalysisB {
        const ID: AnalysisId = AnalysisId("test-b");
        const DEPENDENCIES: &'static [AnalysisId] = &[TestAnalysisA::ID];
    }

    impl FunctionAnalysis for TestAnalysisB {
        fn compute(
            _function: &mir::Function,
            _tree: &mir::NodeTree,
            analyses: &FunctionAnalyses<'_>,
        ) -> Self {
            let a = analyses.get::<TestAnalysisA>();
            Self {
                a_computed: a.computed,
            }
        }
    }

    /// Test analysis that depends on TestAnalysisB.
    struct TestAnalysisC {
        b_a_computed: bool,
    }

    impl Analysis for TestAnalysisC {
        const ID: AnalysisId = AnalysisId("test-c");
        const DEPENDENCIES: &'static [AnalysisId] = &[TestAnalysisB::ID];
    }

    impl FunctionAnalysis for TestAnalysisC {
        fn compute(
            _function: &mir::Function,
            _tree: &mir::NodeTree,
            analyses: &FunctionAnalyses<'_>,
        ) -> Self {
            let b = analyses.get::<TestAnalysisB>();
            Self {
                b_a_computed: b.a_computed,
            }
        }
    }

    #[test]
    fn test_compute_analysis_on_demand() {
        let program = TestProgram::new(
            r#"
function test(): void {
b0:
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);

        // initially not cached
        assert!(!analyses.is_cached::<TestAnalysisA>());

        // get computes and caches
        let a = analyses.get::<TestAnalysisA>();
        assert!(a.computed);
        assert!(analyses.is_cached::<TestAnalysisA>());

        // second get returns cached
        let a2 = analyses.get::<TestAnalysisA>();
        assert!(Arc::ptr_eq(&a, &a2));
    }

    /// Pipeline context exposes profile data when provided.
    #[test]
    fn test_pipeline_context_profile_access() {
        // create context inputs
        let strings = StringPool::new();
        let profile = Arc::new(mir::ProfileTable::new(mir::ProfileSource::Instrumentation));

        // build pipeline context
        let context = PipelineContext::new(
            &strings,
            PipelineOptions::default(),
            super::test_module_id(),
            super::test_target_id(),
            Some(profile),
        );

        // verify profile accessors
        assert!(context.has_profile());
        assert!(context.profile().is_some());
    }

    #[test]
    fn test_analysis_compute_dependencies() {
        let program = TestProgram::new(
            r#"
function test(): void {
b0:
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);

        // get B, which depends on A
        let b = analyses.get::<TestAnalysisB>();
        assert!(b.a_computed);

        // A should now be cached (computed as dependency)
        assert!(analyses.is_cached::<TestAnalysisA>());
    }

    #[test]
    fn test_analysis_transitive_dependencies() {
        let program = TestProgram::new(
            r#"
function test(): void {
b0:
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);

        // get C, which depends on B, which depends on A
        let c = analyses.get::<TestAnalysisC>();
        assert!(c.b_a_computed);

        // all should be cached
        assert!(analyses.is_cached::<TestAnalysisA>());
        assert!(analyses.is_cached::<TestAnalysisB>());
        assert!(analyses.is_cached::<TestAnalysisC>());
    }

    #[test]
    fn test_analysis_invalidate_single() {
        let program = TestProgram::new(
            r#"
function test(): void {
b0:
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);

        // compute A
        let _ = analyses.get::<TestAnalysisA>();
        assert!(analyses.is_cached::<TestAnalysisA>());

        // invalidate A
        analyses.invalidate(TestAnalysisA::ID);
        assert!(!analyses.is_cached::<TestAnalysisA>());
    }

    #[test]
    fn test_analysis_invalidate_all() {
        let program = TestProgram::new(
            r#"
function test(): void {
b0:
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);

        // compute all
        let _ = analyses.get::<TestAnalysisC>();
        assert!(analyses.is_cached::<TestAnalysisA>());
        assert!(analyses.is_cached::<TestAnalysisB>());
        assert!(analyses.is_cached::<TestAnalysisC>());

        // invalidate all
        analyses.invalidate_all();
        assert!(!analyses.is_cached::<TestAnalysisA>());
        assert!(!analyses.is_cached::<TestAnalysisB>());
        assert!(!analyses.is_cached::<TestAnalysisC>());
    }

    #[test]
    fn test_analysis_preservation_all() {
        let program = TestProgram::new(
            r#"
function test(): void {
b0:
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);

        // compute all
        let _ = analyses.get::<TestAnalysisC>();

        // preserve all
        analyses.apply_preservation(&AnalysisPreservation::all());

        // all still cached
        assert!(analyses.is_cached::<TestAnalysisA>());
        assert!(analyses.is_cached::<TestAnalysisB>());
        assert!(analyses.is_cached::<TestAnalysisC>());
    }

    #[test]
    fn test_analysis_preservation_none() {
        let program = TestProgram::new(
            r#"
function test(): void {
b0:
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);

        // compute all
        let _ = analyses.get::<TestAnalysisC>();

        // preserve none
        analyses.apply_preservation(&AnalysisPreservation::none());

        // all invalidated
        assert!(!analyses.is_cached::<TestAnalysisA>());
        assert!(!analyses.is_cached::<TestAnalysisB>());
        assert!(!analyses.is_cached::<TestAnalysisC>());
    }

    #[test]
    fn test_analysis_preservation_some() {
        let program = TestProgram::new(
            r#"
function test(): void {
b0:
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses(function);

        // compute all
        let _ = analyses.get::<TestAnalysisC>();

        // preserve only A
        analyses.apply_preservation(&AnalysisPreservation::preserving(&[TestAnalysisA::ID]));

        // A still cached, B and C invalidated
        assert!(analyses.is_cached::<TestAnalysisA>());
        assert!(!analyses.is_cached::<TestAnalysisB>());
        assert!(!analyses.is_cached::<TestAnalysisC>());
    }

    #[test]
    fn test_analysis_id_display() {
        assert_eq!(format!("{}", AnalysisId("cfg")), "cfg");
        assert_eq!(format!("{}", TestAnalysisA::ID), "test-a");
    }

    #[test]
    fn test_analysis_preservation_is_preserved() {
        let all = AnalysisPreservation::all();
        assert!(all.is_preserved(AnalysisId("anything")));

        let none = AnalysisPreservation::none();
        assert!(!none.is_preserved(AnalysisId("anything")));

        let some = AnalysisPreservation::preserving(&[AnalysisId("cfg")]);
        assert!(some.is_preserved(AnalysisId("cfg")));
        assert!(!some.is_preserved(AnalysisId("domtree")));
    }

    declare_pass! {
        /// Require profile data for validation in tests.
        #[pass(id = "test-profile", requires(profile_data))]
        pub TestProfilePass,
        "Test profile requirement enforcement"
    }

    declare_pass! {
        /// Require type layout metadata for validation in tests.
        #[pass(id = "test-layout", requires(type_layouts))]
        pub TestLayoutPass,
        "Test layout requirement enforcement"
    }

    impl ModulePass for TestProfilePass {
        fn run(
            &self,
            _tree: &mut mir::NodeTree,
            _ctx: &PipelineContext<'_>,
        ) -> AnalysisPreservation {
            AnalysisPreservation::all()
        }

        fn name(&self) -> &'static str {
            "TestProfilePass"
        }
    }

    impl ModulePass for TestLayoutPass {
        fn run(
            &self,
            _tree: &mut mir::NodeTree,
            _ctx: &PipelineContext<'_>,
        ) -> AnalysisPreservation {
            AnalysisPreservation::all()
        }

        fn name(&self) -> &'static str {
            "TestLayoutPass"
        }
    }

    /// Emits an error when call effects metadata is missing.
    #[test]
    fn test_requirements_call_effects() {
        let input = r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function root(): int32 {
b0:
    v0: int32 = 1int32
    v1: int32 = call callee(v0): (int32) -> int32
    return v1
}"#;

        let mut test = TestProgram::new(input);
        let options = PipelineOptions {
            require_optimized_metadata: true,
            ..Default::default()
        };

        test.run_module_pass_with_options(&InterproceduralSccp, options);
        test.assert_error(|e| matches!(e, OptimizeError::MissingRequiredMetadata { .. }));
    }

    /// Emits an error when memory access metadata is missing.
    #[test]
    fn test_requirements_memory_access_metadata() {
        let input = r#"
function test(v0: ref<int32, raw>): int32 {
b0(v0: ref<int32, raw>):
    v1: int32 = load v0
    return v1
}"#;

        let mut test = TestProgram::new(input);
        let options = PipelineOptions {
            require_optimized_metadata: true,
            ..Default::default()
        };

        test.run_pass_with_options(&LoadPre, options);
        test.assert_error(|e| matches!(e, OptimizeError::MissingRequiredMetadata { .. }));
    }

    /// Emits an error when profile data is required but missing.
    #[test]
    fn test_requirements_profile_data() {
        let input = r#"
function test(): void {
b0:
    return
}"#;

        let mut test = TestProgram::new(input);
        let options = PipelineOptions {
            require_optimized_metadata: true,
            ..Default::default()
        };

        test.run_module_pass_with_options(&TestProfilePass, options);
        test.assert_error(|e| matches!(e, OptimizeError::MissingRequiredMetadata { .. }));
    }

    /// Emits an error when type layout metadata is missing.
    #[test]
    fn test_requirements_type_layouts() {
        let input = r#"
type Point {
    int32;
    int32;
}
function makePoint(v0: int32, v1: int32): Point {
b0(v0: int32, v1: int32):
    v2: Point = struct Point (v0, v1)
    return v2
}"#;

        let mut test = TestProgram::new(input);

        // remove layout metadata for the struct type
        let struct_type_id = test
            .tree
            .iter_nodes::<mir::Type>()
            .find_map(|(type_id, ty)| match ty {
                mir::Type::Struct { .. } => Some(type_id),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing struct type"));
        let _ = test
            .tree
            .metadata
            .layout
            .layout_by_type
            .remove(&struct_type_id);

        let options = PipelineOptions {
            require_optimized_metadata: true,
            ..Default::default()
        };

        test.run_module_pass_with_options(&TestLayoutPass, options);
        test.assert_error(|e| matches!(e, OptimizeError::MissingRequiredMetadata { .. }));
    }

    /// Borrow address instructions are not speculatable.
    #[test]
    fn test_instruction_is_speculatable_rejects_borrow_addresses() {
        let mut tree = mir::NodeTree::new();

        let pointee = tree.insert_type(mir::Type::Int {
            width: 32,
            is_signed: true,
        });
        let borrowed_ref = tree.insert_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            address_space: mir::AddressSpace::Stack,
            mutability: mir::Mutability::Mutable,
            pointee: pointee.into(),
            is_nullable: false,
        });

        let destination = mir::Value::new(0);
        let local = mir::LocalNodeId::new(0);
        let aggregate = mir::Value::new(1);
        let array = mir::Value::new(2);
        let index = mir::Value::new(3);

        let local_addr = mir::Instruction::LocalAddr {
            destination: destination.into(),
            local: local.into(),
            result_type: borrowed_ref.into(),
        };
        assert!(!instruction_is_speculatable(&local_addr, &tree));

        let field_addr = mir::Instruction::FieldAddr {
            destination: destination.into(),
            aggregate: aggregate.into(),
            index: 0,
            result_type: borrowed_ref.into(),
        };
        assert!(!instruction_is_speculatable(&field_addr, &tree));

        let element_addr = mir::Instruction::ElementAddr {
            destination: destination.into(),
            array: array.into(),
            index: index.into(),
            result_type: borrowed_ref.into(),
        };
        assert!(!instruction_is_speculatable(&element_addr, &tree));
    }

    /// Raw address instructions are speculatable with typed checks.
    #[test]
    fn test_instruction_is_speculatable_allows_raw_addresses() {
        let mut tree = mir::NodeTree::new();

        let pointee = tree.insert_type(mir::Type::Int {
            width: 32,
            is_signed: true,
        });
        let raw_ref = tree.insert_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Raw,
            address_space: mir::AddressSpace::Stack,
            mutability: mir::Mutability::Mutable,
            pointee: pointee.into(),
            is_nullable: false,
        });
        let borrowed_ref = tree.insert_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            address_space: mir::AddressSpace::Stack,
            mutability: mir::Mutability::Mutable,
            pointee: pointee.into(),
            is_nullable: false,
        });

        let destination = mir::Value::new(0);
        let local = mir::LocalNodeId::new(0);

        let raw_addr = mir::Instruction::LocalAddr {
            destination: destination.into(),
            local: local.into(),
            result_type: raw_ref.into(),
        };
        let borrowed_addr = mir::Instruction::LocalAddr {
            destination: destination.into(),
            local: local.into(),
            result_type: borrowed_ref.into(),
        };

        assert!(instruction_is_speculatable(&raw_addr, &tree));
        assert!(!instruction_is_speculatable(&borrowed_addr, &tree));
    }
}

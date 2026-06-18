use std::collections::HashMap;
use std::sync::Arc;

use destack_core::StringPool;
use destack_mir as mir;
use destack_source::{DiffOptions, FileId, ModuleId, PackageId, ProfileId, TargetId, print_diff};

use crate::optimize::{FunctionPass, ModulePass, PipelineContext, PipelineOptions};
use crate::{OptimizeError, OptimizeWarning};
use destack_mir::{FunctionAnalyses, ModuleAnalyses};

/// Placeholder module id for tests.
fn test_module_id() -> ModuleId {
    ModuleId::new(PackageId::new(0), 0)
}

/// Placeholder profile id for tests.
fn test_profile_id() -> ProfileId {
    ProfileId::new(0)
}

/// Placeholder target id for tests.
fn test_target_id() -> TargetId {
    test_target_id_for_package(PackageId::new(0), "test")
}

/// Placeholder target id for one package and test name.
fn test_target_id_for_package(package_id: PackageId, name: &str) -> TargetId {
    TargetId::new(package_id, name)
}

/// Format one MIR fixture into canonical text.
fn canonical_mir_text(source: &str) -> String {
    let (tree, strings) = match mir::parse::Parser::parse(
        FileId::new(0),
        source,
        mir::parse::ParseOptions::default(),
    )
    .finish()
    {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("===EXPECTED_BEGIN===\n{source}\n===EXPECTED_END===");
            panic!("expected MIR fixture is unparseable: {error:?}");
        }
    };
    let formatted = match mir::format_mir(&tree, &strings, mir::MirFormatOptions::default()) {
        Ok(formatted) => formatted,
        Err(error) => {
            eprintln!("===EXPECTED_BEGIN===\n{source}\n===EXPECTED_END===");
            panic!("expected MIR fixture failed to format: {error:?}");
        }
    };

    formatted.trim().to_string()
}

/// Require one MIR fixture to parse.
fn assert_parseable_mir_text(source: &str) {
    let result =
        mir::parse::Parser::parse(FileId::new(0), source, mir::parse::ParseOptions::default())
            .finish();

    if let Err(error) = result {
        eprintln!("===UNPARSEABLE_MIR_BEGIN===\n{source}\n===UNPARSEABLE_MIR_END===");
        panic!("optimizer produced unparseable MIR: {error:?}");
    }
}

/// Test program for optimization passes.
///
/// Parses MIR from text, applies passes, and formats the result back to text.
pub(crate) struct TestProgram {
    /// The MIR tree.
    pub(crate) tree: mir::Tree,
    /// String pool for identifiers (immutable, from parser).
    strings: StringPool,
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
            mir::parse::Parser::parse(FileId::new(0), source, mir::parse::ParseOptions::default())
                .finish()
                .expect("failed to parse MIR");
        let strings_pool = StringPool::new();

        // copy parser strings for passes that intern through context
        strings_pool.ensure_all_from(&strings);

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
        profile: mir::Profile,
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

    /// Return the destination of a stack allocation instruction.
    pub(crate) fn frame_alloc_destination(
        &self,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
    ) -> mir::Value {
        // extract the destination value from the instruction
        let mir::Instruction::FrameAllocZeroed { destination, .. } = self.tree.get(instruction_id)
        else {
            panic!("expected stack allocation");
        };

        *destination
    }

    /// Return stack allocation destinations from the entry block.
    pub(crate) fn frame_alloc_destinations_in_entry(
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
    ) {
        self.insert_pointer_access_with_options(instruction, kind, pointer, size, false, None);
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
            flags: None,
            space: None,
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

        (call_inst, *callee)
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
            .map(mir::FunctionParameter::signature_parameter)
            .collect::<Vec<_>>();
        let return_ty = callee_function.return_type;

        // insert the function pointer type
        self.tree.insert_type(mir::Type::FunctionSignature {
            lifetimes: Vec::new(),
            parameters: param_tys,
            result: return_ty,
        })
    }

    /// Internal implementation that handles the borrow correctly.
    fn run_pass_with_options_impl<P: FunctionPass + ?Sized>(
        &mut self,
        pass: &P,
        options: PipelineOptions,
        profile: Option<Arc<mir::Profile>>,
    ) {
        let context = PipelineContext::new(
            &self.strings_pool,
            options,
            test_module_id(),
            test_profile_id(),
            test_target_id(),
            profile,
            Arc::new(destack_artifact::ProgramAnalysis::new()),
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
            let analyses = context.new_function_analyses();
            pass.run(&mut function, &mut self.tree, &context, &analyses);
            *self.tree.get_mut(function_id) = function;
        }

        // collect diagnostics after pass completes
        self.errors = context
            .take_errors()
            .into_iter()
            .map(|diagnostic| diagnostic.into_inner())
            .collect();
        self.warnings = context
            .take_warnings()
            .into_iter()
            .map(|diagnostic| diagnostic.into_inner())
            .collect();
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
        (then_target.block, else_target.block)
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
        target.block
    }

    /// Record a function's profiled entry execution count, keyed by its symbol.
    pub(crate) fn record_function_entry(
        &self,
        profile: &mut mir::Profile,
        function: mir::LocalNodeId<mir::Function>,
        count: u64,
    ) {
        // store the entry count that scales the function's block frequencies
        let symbol = self.tree.get(function).symbol;
        profile.functions.insert(
            symbol,
            mir::FunctionProfile {
                hash: mir::FunctionHash(0),
                entry: mir::Count::new(count),
                edges: HashMap::new(),
                counts: Vec::new(),
                values: HashMap::new(),
            },
        );
    }

    /// Set relative weights on a block terminator's successors, in field order.
    ///
    /// Ordering matches the terminator's successor fields: branch is then then else,
    /// switch is default then each case, and check is success then failure.
    pub(crate) fn record_successor_weights(
        &self,
        profile: &mut mir::Profile,
        block: mir::LocalNodeId<mir::Block>,
        weights: &[u32],
    ) {
        // find the function that owns this block
        let (_, function) = self
            .tree
            .iter_nodes::<mir::Function>()
            .find(|(_, function)| function.blocks.contains(&block))
            .expect("missing function for profiled block");

        // fetch the profile entry created by record_function_entry
        let function_profile = profile
            .functions
            .get_mut(&function.symbol)
            .expect("missing function profile for successor weights");

        // record each structural successor edge
        let terminator = self.tree.get(self.tree.get(block).terminator);
        let targets = mir::terminator_targets(&self.tree, block, terminator);
        assert_eq!(
            targets.len(),
            weights.len(),
            "successor weight count does not match terminator successor count"
        );
        for ((edge, _), &weight) in targets.into_iter().zip(weights) {
            function_profile
                .edges
                .insert(edge, mir::Count::new(weight as u64));
        }
    }

    /// Build a module-scoped program analysis from the current tree.
    ///
    /// A test module is a standalone program, so its exported symbols are the roots.
    fn module_program_analysis(&self) -> Arc<destack_artifact::ProgramAnalysis> {
        let links = ModuleAnalyses::new().get::<mir::LinkGraph>(&self.tree);
        let roots: Vec<_> = links
            .nodes()
            .filter(|(_, node)| node.linkage().is_exported())
            .map(|(symbol, _)| symbol)
            .collect();
        let supergraph = mir::LinkSupergraph::build([&*links]);

        Arc::new(destack_artifact::ProgramAnalysis::analyze(
            &supergraph,
            &roots,
        ))
    }

    /// Apply a module pass to the program.
    pub(crate) fn run_module_pass<P: ModulePass + ?Sized>(&mut self, pass: &P) {
        let program_analysis = self.module_program_analysis();
        let context = PipelineContext::new(
            &self.strings_pool,
            PipelineOptions::default(),
            test_module_id(),
            test_profile_id(),
            test_target_id(),
            None,
            program_analysis,
        );

        // enforce pass requirements
        if context.enforce_module_requirements(pass.metadata(), &self.tree) {
            let analyses = ModuleAnalyses::new();
            pass.run(&mut self.tree, &context, &analyses);
        }

        // collect diagnostics after pass completes
        self.errors = context
            .take_errors()
            .into_iter()
            .map(|diagnostic| diagnostic.into_inner())
            .collect();
        self.warnings = context
            .take_warnings()
            .into_iter()
            .map(|diagnostic| diagnostic.into_inner())
            .collect();
    }

    /// Apply a module pass with custom options.
    pub(crate) fn run_module_pass_with_options<P: ModulePass + ?Sized>(
        &mut self,
        pass: &P,
        options: PipelineOptions,
    ) {
        let program_analysis = self.module_program_analysis();
        let context = PipelineContext::new(
            &self.strings_pool,
            options,
            test_module_id(),
            test_profile_id(),
            test_target_id(),
            None,
            program_analysis,
        );

        // enforce pass requirements
        if context.enforce_module_requirements(pass.metadata(), &self.tree) {
            let analyses = ModuleAnalyses::new();
            pass.run(&mut self.tree, &context, &analyses);
        }

        // collect diagnostics after pass completes
        self.errors = context
            .take_errors()
            .into_iter()
            .map(|diagnostic| diagnostic.into_inner())
            .collect();
        self.warnings = context
            .take_warnings()
            .into_iter()
            .map(|diagnostic| diagnostic.into_inner())
            .collect();
    }

    /// Apply a module pass with profile data.
    pub(crate) fn run_module_pass_with_profile<P: ModulePass + ?Sized>(
        &mut self,
        pass: &P,
        profile: mir::Profile,
    ) {
        let program_analysis = self.module_program_analysis();
        let context = PipelineContext::new(
            &self.strings_pool,
            PipelineOptions::default(),
            test_module_id(),
            test_profile_id(),
            test_target_id(),
            Some(Arc::new(profile)),
            program_analysis,
        );

        // enforce pass requirements
        if context.enforce_module_requirements(pass.metadata(), &self.tree) {
            let analyses = ModuleAnalyses::new();
            pass.run(&mut self.tree, &context, &analyses);
        }

        // collect diagnostics after pass completes
        self.errors = context
            .take_errors()
            .into_iter()
            .map(|diagnostic| diagnostic.into_inner())
            .collect();
        self.warnings = context
            .take_warnings()
            .into_iter()
            .map(|diagnostic| diagnostic.into_inner())
            .collect();
    }

    /// Format the MIR back to text.
    pub(crate) fn format(&self) -> String {
        let strings = self.strings_pool.clone();
        mir::format_mir(&self.tree, &strings, mir::MirFormatOptions::default()).expect("format MIR")
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
        let expected = canonical_mir_text(expected);
        let actual = actual.trim();

        assert_parseable_mir_text(actual);

        if actual != expected.as_str() {
            eprintln!("===ACTUAL_BEGIN===\n{actual}\n===ACTUAL_END===");
            print_diff(&expected, actual, &DiffOptions::new());
            panic!("optimization output mismatch");
        }
    }

    /// Assert that the MIR is unchanged from the original source.
    #[track_caller]
    pub(crate) fn assert_unchanged(&self, original: &str) {
        let (tree, strings) = mir::parse::Parser::parse(
            FileId::new(0),
            original,
            mir::parse::ParseOptions::default(),
        )
        .finish()
        .expect("failed to parse expected MIR");
        let expected =
            mir::format_mir(&tree, &strings, mir::MirFormatOptions::default()).expect("format MIR");

        self.assert_output(&expected);
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

    /// Create a function analysis cache for this test program.
    pub(crate) fn function_analyses(&self) -> FunctionAnalyses {
        FunctionAnalyses::new()
    }

    /// Create module analyses for this test program.
    pub(crate) fn module_analyses(&self) -> ModuleAnalyses {
        ModuleAnalyses::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::optimize::declare_pass;
    use destack_core::StringPool;
    use destack_mir as mir;
    use destack_mir::{
        Analysis, AnalysisId, FunctionAnalysis, Mutation, instruction_is_speculatable,
    };

    use super::*;
    use crate::OptimizeError;
    use crate::optimize::passes::{InterproceduralSccp, LoadPre};

    /// Simple test analysis with no dependencies.
    struct TestAnalysisA {
        computed: bool,
    }

    impl Analysis for TestAnalysisA {
        const ID: AnalysisId = AnalysisId("test-a");
        // survives value-only changes so partial preservation is observable
        const INVALIDATED_BY: Mutation = Mutation::CONTROL_FLOW;
    }

    impl FunctionAnalysis for TestAnalysisA {
        fn compute(
            _function: &mir::Function,
            _tree: &mir::Tree,
            _analyses: &FunctionAnalyses,
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
    }

    impl FunctionAnalysis for TestAnalysisB {
        fn compute(
            function: &mir::Function,
            tree: &mir::Tree,
            analyses: &FunctionAnalyses,
        ) -> Self {
            let a = analyses.get::<TestAnalysisA>(function, tree);
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
    }

    impl FunctionAnalysis for TestAnalysisC {
        fn compute(
            function: &mir::Function,
            tree: &mir::Tree,
            analyses: &FunctionAnalyses,
        ) -> Self {
            let b = analyses.get::<TestAnalysisB>(function, tree);
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
entry:
    return
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses();

        // initially not cached
        assert!(!analyses.is_cached::<TestAnalysisA>());

        // get computes and caches
        let a = analyses.get::<TestAnalysisA>(function, &program.tree);
        assert!(a.computed);
        assert!(analyses.is_cached::<TestAnalysisA>());

        // second get returns cached
        let a2 = analyses.get::<TestAnalysisA>(function, &program.tree);
        assert!(Arc::ptr_eq(&a, &a2));
    }

    /// Pipeline context exposes profile data when provided.
    #[test]
    fn test_pipeline_context_profile_access() {
        // create context inputs
        let strings = StringPool::new();
        let profile = Arc::new(mir::Profile::new());

        // build pipeline context
        let context = PipelineContext::new(
            &strings,
            PipelineOptions::default(),
            super::test_module_id(),
            super::test_profile_id(),
            super::test_target_id(),
            Some(profile),
            Arc::new(destack_artifact::ProgramAnalysis::new()),
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
entry:
    return
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses();

        // get B, which depends on A
        let b = analyses.get::<TestAnalysisB>(function, &program.tree);
        assert!(b.a_computed);

        // A should now be cached (computed as dependency)
        assert!(analyses.is_cached::<TestAnalysisA>());
    }

    #[test]
    fn test_analysis_transitive_dependencies() {
        let program = TestProgram::new(
            r#"
function test(): void {
entry:
    return
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses();

        // get C, which depends on B, which depends on A
        let c = analyses.get::<TestAnalysisC>(function, &program.tree);
        assert!(c.b_a_computed);

        // all should be cached
        assert!(analyses.is_cached::<TestAnalysisA>());
        assert!(analyses.is_cached::<TestAnalysisB>());
        assert!(analyses.is_cached::<TestAnalysisC>());
    }

    #[test]
    fn test_apply_no_change_preserves_all() {
        let program = TestProgram::new(
            r#"
function test(): void {
entry:
    return
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses();

        // compute all
        let _ = analyses.get::<TestAnalysisC>(function, &program.tree);

        // a pass that changed nothing keeps every analysis
        analyses.apply(Mutation::NONE);

        // all still cached
        assert!(analyses.is_cached::<TestAnalysisA>());
        assert!(analyses.is_cached::<TestAnalysisB>());
        assert!(analyses.is_cached::<TestAnalysisC>());
    }

    #[test]
    fn test_apply_full_change_invalidates_all() {
        let program = TestProgram::new(
            r#"
function test(): void {
entry:
    return
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses();

        // compute all
        let _ = analyses.get::<TestAnalysisC>(function, &program.tree);

        // a pass that changed everything clears every analysis
        analyses.apply(Mutation::ALL);

        // all invalidated
        assert!(!analyses.is_cached::<TestAnalysisA>());
        assert!(!analyses.is_cached::<TestAnalysisB>());
        assert!(!analyses.is_cached::<TestAnalysisC>());
    }

    #[test]
    fn test_apply_partial_change_preserves_by_mask() {
        let program = TestProgram::new(
            r#"
function test(): void {
entry:
    return
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = program.function_analyses();

        // compute a structural analysis (invalidated by control flow only) and a
        // data-flow analysis (invalidated by any change)
        let _ = analyses.get::<mir::ControlFlowGraph>(function, &program.tree);
        let _ = analyses.get::<mir::ConstantPropagation>(function, &program.tree);

        // a value-only change preserves the control-flow graph and invalidates the
        // value-dependent analysis
        analyses.apply(Mutation::VALUES);

        assert!(analyses.is_cached::<mir::ControlFlowGraph>());
        assert!(!analyses.is_cached::<mir::ConstantPropagation>());
    }

    #[test]
    fn test_analysis_id_display() {
        assert_eq!(format!("{}", AnalysisId("cfg")), "cfg");
        assert_eq!(format!("{}", TestAnalysisA::ID), "test-a");
    }

    #[test]
    fn test_mutation_set_operations() {
        // the empty change touches nothing
        assert!(Mutation::NONE.is_none());
        assert!(!Mutation::NONE.intersects(Mutation::ALL));

        // a union carries both kinds
        let both = Mutation::CONTROL_FLOW | Mutation::VALUES;
        assert_eq!(both, Mutation::ALL);
        assert!(both.intersects(Mutation::CONTROL_FLOW));
        assert!(both.intersects(Mutation::VALUES));

        // distinct kinds do not intersect
        assert!(!Mutation::CONTROL_FLOW.intersects(Mutation::VALUES));
    }

    declare_pass! {
        /// Require profile data for validation in tests.
        #[pass(id = "test-profile", requires(profile_data))]
        pub(super) TestProfilePass,
        "Test profile requirement enforcement"
    }

    declare_pass! {
        /// Require type layout metadata for validation in tests.
        #[pass(id = "test-layout", requires(type_layouts))]
        pub(super) TestLayoutPass,
        "Test layout requirement enforcement"
    }

    impl ModulePass for TestProfilePass {
        fn run(
            &self,
            _tree: &mut mir::Tree,
            _ctx: &PipelineContext<'_>,
            _analyses: &ModuleAnalyses,
        ) -> Mutation {
            Mutation::NONE
        }

        fn name(&self) -> &'static str {
            "TestProfilePass"
        }
    }

    impl ModulePass for TestLayoutPass {
        fn run(
            &self,
            _tree: &mut mir::Tree,
            _ctx: &PipelineContext<'_>,
            _analyses: &ModuleAnalyses,
        ) -> Mutation {
            Mutation::NONE
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
entry(v0: int32):
    return v0
}

function root(): int32 {
entry:
    v0: int32 = 1
    v1: int32 = call callee(v0)
    return v1
}
"#;

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
entry(v0: ref<int32, raw>):
    v1: int32 = load v0
    return v1
}
"#;

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
entry:
    return
}
"#;

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
entry(v0: int32, v1: int32):
    v2: Point = struct Point (v0, v1)
    return v2
}
"#;

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
        let mut tree = mir::Tree::new();

        let pointee = tree.insert_type(mir::Type::Int {
            width: 32,
            is_signed: true,
        });
        let borrowed_ref = tree.insert_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::empty(),
            space: mir::Space::Frame,
            access: mir::Access::Mutable,
            pointee,
            nullability: mir::Nullability::None,
        });

        let destination = mir::Value::new(0);
        let local = mir::LocalNodeId::new(0);
        let aggregate = mir::Value::new(1);
        let array = mir::Value::new(2);
        let index = mir::Value::new(3);

        let local_addr = mir::Instruction::LocalAddr {
            destination,
            local,
            result_type: borrowed_ref,
        };
        assert!(!instruction_is_speculatable(&local_addr, &tree));

        let field_addr = mir::Instruction::FieldAddr {
            destination,
            aggregate,
            index: 0,
            result_type: borrowed_ref,
        };
        assert!(!instruction_is_speculatable(&field_addr, &tree));

        let element_addr = mir::Instruction::ElementAddr {
            destination,
            array,
            index,
            result_type: borrowed_ref,
        };
        assert!(!instruction_is_speculatable(&element_addr, &tree));
    }

    /// Raw address instructions are speculatable with typed checks.
    #[test]
    fn test_instruction_is_speculatable_allows_raw_addresses() {
        let mut tree = mir::Tree::new();

        let pointee = tree.insert_type(mir::Type::Int {
            width: 32,
            is_signed: true,
        });
        let raw_ref = tree.insert_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Raw,
            lifetime: mir::Lifetime::empty(),
            space: mir::Space::Frame,
            access: mir::Access::Mutable,
            pointee,
            nullability: mir::Nullability::None,
        });
        let borrowed_ref = tree.insert_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::empty(),
            space: mir::Space::Frame,
            access: mir::Access::Mutable,
            pointee,
            nullability: mir::Nullability::None,
        });

        let destination = mir::Value::new(0);
        let local = mir::LocalNodeId::new(0);

        let raw_addr = mir::Instruction::LocalAddr {
            destination,
            local,
            result_type: raw_ref,
        };
        let borrowed_addr = mir::Instruction::LocalAddr {
            destination,
            local,
            result_type: borrowed_ref,
        };

        assert!(instruction_is_speculatable(&raw_addr, &tree));
        assert!(!instruction_is_speculatable(&borrowed_addr, &tree));
    }
}

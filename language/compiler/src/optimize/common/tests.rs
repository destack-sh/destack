use std::sync::Arc;

use destack_base::{ImmutableStringPool, StringPool};
use destack_mir as mir;
use destack_source::{DiffOptions, ModuleId, PackageId, print_diff};
use destack_workspace::TargetId;

use crate::optimize::{FunctionPass, ModulePass, PipelineContext, PipelineOptions};
use crate::{OptimizeError, OptimizeWarning};

/// Placeholder module id for tests.
fn test_module_id() -> ModuleId {
    ModuleId::new(PackageId::new(0), 0)
}

/// Placeholder target id for tests.
fn test_target_id() -> TargetId {
    TargetId::new(PackageId::new(0), "test")
}

/// Test program for optimization passes.
///
/// Parses MIR from text, applies passes, and formats the result back to text.
pub(crate) struct TestProgram {
    /// The MIR node tree.
    pub(crate) tree: mir::NodeTree,
    /// String pool for identifiers (immutable, from parser).
    strings: ImmutableStringPool,
    /// Thread-safe string pool for optimization context.
    strings_pool: StringPool,
    /// Errors collected from the last pass run.
    errors: Vec<OptimizeError>,
    /// Warnings collected from the last pass run.
    warnings: Vec<OptimizeWarning>,
}

#[allow(dead_code)]
impl TestProgram {
    /// Create a new test program from MIR source text.
    pub(crate) fn new(source: &str) -> Self {
        let (tree, strings) = mir::parse::Parser::parse(source).expect("failed to parse MIR");
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
        // scan for a function that has an entry block
        self.tree
            .iter_nodes::<mir::Function>()
            .find(|(_, function)| function.entry.is_some())
            .expect("missing function")
            .0
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
            .map(|param| param.ty)
            .collect::<Vec<_>>();
        let return_ty = callee_function.return_type;

        // insert the function pointer type
        self.tree.insert(mir::Type::FunctionPointer {
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
        // select the first function id
        let function_id = self
            .tree
            .iter_nodes::<mir::Function>()
            .next()
            .expect("missing function")
            .0;

        // return the id

        function_id
    }

    /// Return entry branch targets for a function.
    pub(crate) fn entry_branch_targets(
        &self,
        function: &mir::Function,
    ) -> (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>) {
        // read the entry block
        let entry = function.entry.expect("missing entry block");
        let entry_block = self.tree.get(entry);

        // extract the branch targets
        let mir::Terminator::Branch {
            then_target,
            else_target,
            ..
        } = &entry_block.terminator
        else {
            panic!("expected entry branch");
        };

        // return the targets

        (*then_target, *else_target)
    }

    /// Return the jump target for a block.
    pub(crate) fn jump_target(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
    ) -> mir::LocalNodeId<mir::Block> {
        // read the block terminator
        let block = self.tree.get(block_id);
        let mir::Terminator::Jump { target, .. } = &block.terminator else {
            panic!("expected jump terminator");
        };

        // return the target

        *target
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

    /// Apply a module pass to the program.
    pub(crate) fn run_module_pass<P: ModulePass + ?Sized>(&mut self, pass: &P) {
        let context = PipelineContext::new(
            &self.strings_pool,
            PipelineOptions::default(),
            test_module_id(),
            test_target_id(),
            None,
        );

        pass.run(&mut self.tree, &context);

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
    pub(crate) fn get_string(&self, id: destack_base::StringId) -> &str {
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
        let has_match = self.errors.iter().any(|e| predicate(e));

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
        let has_match = self.warnings.iter().any(|w| predicate(w));

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

    use destack_base::StringPool;
    use destack_mir as mir;

    use super::TestProgram;
    use crate::optimize::{
        Analysis, AnalysisId, AnalysisPreservation, FunctionAnalyses, FunctionAnalysis,
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
            r#"function @test() -> void {
block0:
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
            r#"function @test() -> void {
block0:
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
            r#"function @test() -> void {
block0:
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
            r#"function @test() -> void {
block0:
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
            r#"function @test() -> void {
block0:
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
            r#"function @test() -> void {
block0:
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
            r#"function @test() -> void {
block0:
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
            r#"function @test() -> void {
block0:
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
}

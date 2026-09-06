use std::sync::Arc;

use destack_core::{FxIndexMap, StringPool};
use destack_mir as mir;
use destack_mir::{AnalysisCache, FunctionCache};
use destack_source::{File, FileId, FileType, ModuleId, PackageId, ProfileId, TargetId, Uri};

use crate::optimize::{FunctionPass, MirOptimized, ModulePass, PipelineContext, PipelineOptions};
use crate::tests::assert_formatted_snapshot;
use crate::{OptimizeError, OptimizeWarning};

/// Integer widths available to optimizer test inputs after lowering.
const SUPPORTED_INTEGER_WIDTHS: [u16; 6] = [8, 16, 32, 64, 128, 256];

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

/// Build one optimizer MIR fixture file.
fn test_mir_file(source: &str) -> File {
    File::from_text(
        FileId::new(0),
        "test.mir".to_string(),
        Uri::from_string("test.mir"),
        None,
        FileType::Text,
        source.to_string(),
    )
    .expect("test MIR source should load")
}

/// Parse and format one expected MIR fixture.
fn expected_mir_text(source: &str) -> String {
    let file = test_mir_file(source);
    let parsed = mir::parse::Parser::parse(&file, mir::parse::ParseOptions::default())
        .expect("test MIR should be text");
    let (tree, strings) = match parsed.finish() {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("===EXPECTED_BEGIN===\n{source}\n===EXPECTED_END===");
            panic!("expected MIR fixture is unparseable: {error:?}");
        }
    };
    let formatted = mir::Formatter::new(
        &tree,
        mir::TargetLayout::default(),
        &strings,
        mir::FormatOptions::default(),
    )
    .format()
    .expect("format expected MIR");

    formatted.trim().to_string()
}

/// Require one MIR fixture to parse.
fn assert_parseable_mir_text(source: &str) {
    let file = test_mir_file(source);
    let result = mir::parse::Parser::parse(&file, mir::parse::ParseOptions::default())
        .expect("test MIR should be text")
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
    /// The optimized MIR artifact under test.
    pub(crate) optimized: MirOptimized,
    /// Target ABI layout.
    target: mir::TargetLayout,
    /// String pool for identifiers (immutable, from parser).
    strings: StringPool,
    /// Thread safe string pool for optimization context.
    strings_pool: StringPool,
    /// Errors collected from the last pass run.
    errors: Vec<OptimizeError>,
    /// Warnings collected from the last pass run.
    warnings: Vec<OptimizeWarning>,
}

impl TestProgram {
    /// Create a new test program from MIR source text.
    pub(crate) fn new(source: &str) -> Self {
        let file = test_mir_file(source);
        let parsed = mir::parse::Parser::parse(&file, mir::parse::ParseOptions::default())
            .expect("test MIR should be text");
        if parsed
            .diagnostics
            .has_diagnostics_of_severity(destack_source::DiagnosticSeverity::Error)
        {
            panic!("failed to parse MIR: {:?}", parsed.diagnostics);
        }
        let (mut tree, target, layouts, dispatch, drops, accesses, effects, profile, strings, _) =
            parsed.into_parts();

        // mirror the primitive universe guaranteed by MIR lowering
        let primitive_types = std::iter::once(mir::Type::TypeId).chain(
            SUPPORTED_INTEGER_WIDTHS.into_iter().flat_map(|width| {
                [
                    mir::Type::Int {
                        width,
                        is_signed: false,
                    },
                    mir::Type::Int {
                        width,
                        is_signed: true,
                    },
                ]
            }),
        );
        for ty in primitive_types {
            if tree.find_type(&ty).is_none() {
                tree.intern_type(ty);
            }
        }
        let strings_pool = StringPool::new();

        // copy parser strings for passes that intern through context
        strings_pool.ensure_all_from(&strings);

        Self {
            optimized: MirOptimized {
                tree,
                layouts,
                dispatch,
                drops,
                accesses,
                effects,
                profile,
            },
            target,
            strings,
            strings_pool,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Apply a function pass to all functions in the program.
    pub(crate) fn run_pass<P: FunctionPass + ?Sized>(&mut self, pass: &P) {
        self.run_function_pass(pass, None);
    }

    /// Apply a function pass with profile data.
    pub(crate) fn run_pass_with_profile<P: FunctionPass + ?Sized>(
        &mut self,
        pass: &P,
        profile: mir::Profile,
    ) {
        self.run_function_pass(pass, Some(Arc::new(profile)));
    }

    /// Return the entry function id for this program.
    pub(crate) fn entry_function_id(&self) -> mir::LocalNodeId<mir::Function> {
        // require the canonical test entry
        self.optimized
            .tree
            .iter_nodes::<mir::Function>()
            .find(|(_, function)| {
                function.entry().is_some() && self.strings.get(function.name) == "test"
            })
            .map(|(function_id, _)| function_id)
            .expect("missing test function")
    }

    /// Return the function id for a named function.
    pub(crate) fn function_id_by_name(&self, name: &str) -> mir::LocalNodeId<mir::Function> {
        self.optimized
            .tree
            .iter_nodes::<mir::Function>()
            .find(|(_, function)| self.strings.get(function.name) == name)
            .expect("missing function")
            .0
    }

    /// Return the type id for a named type.
    pub(crate) fn type_id_by_name(&self, name: &str) -> mir::TypeId {
        self.optimized
            .tree
            .iter_nodes::<mir::TypeDeclaration>()
            .find(|(_, declaration)| self.strings.get(declaration.name) == name)
            .map(|(_, declaration)| declaration.ty)
            .unwrap_or_else(|| panic!("missing type {name}"))
    }

    /// Return the entry block id for a function.
    pub(crate) fn entry_block_id(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> mir::LocalNodeId<mir::Block> {
        // require the defined function entry
        let function = self.optimized.tree.get(function_id);
        function.entry().expect("missing function entry")
    }

    /// Return the instruction ids in a block.
    pub(crate) fn instructions_in_block(
        &self,
        block_id: mir::LocalNodeId<mir::Block>,
    ) -> Vec<mir::LocalNodeId<mir::Instruction>> {
        // clone the instruction ids for this block
        self.optimized.tree.get(block_id).instructions.clone()
    }

    /// Return the instruction ids in the entry block.
    pub(crate) fn entry_instructions(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Vec<mir::LocalNodeId<mir::Instruction>> {
        // read the entry block and clone the instruction ids
        self.instructions_in_block(self.entry_block_id(function_id))
    }

    /// Return local address destinations from the entry block.
    pub(crate) fn local_address_destinations_in_entry(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Vec<mir::Value> {
        // read the entry block
        let block_id = self.entry_block_id(function_id);
        let block = self.optimized.tree.get(block_id);

        // collect local address destinations in order
        block
            .instructions
            .iter()
            .filter_map(|instruction_id| {
                if let mir::Instruction::LocalAddr { destination, .. } =
                    self.optimized.tree.get(*instruction_id)
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
        let block = self.optimized.tree.get(block_id);

        // collect store instructions in order
        block
            .instructions
            .iter()
            .copied()
            .filter(|instruction_id| {
                matches!(
                    self.optimized.tree.get(*instruction_id),
                    mir::Instruction::Store { .. }
                )
            })
            .collect()
    }

    /// Attach pointer access entries to an instruction.
    pub(crate) fn insert_pointer_access(
        &mut self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        kind: mir::MemoryOperation,
        pointer: mir::Value,
        size: Option<u64>,
    ) {
        self.insert_pointer_access_with_options(instruction, kind, pointer, size, false, None);
    }

    /// Attach pointer memory accesses to an instruction with ordering.
    pub(crate) fn insert_pointer_access_with_options(
        &mut self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        kind: mir::MemoryOperation,
        pointer: mir::Value,
        size: Option<u64>,
        is_volatile: bool,
        ordering: Option<mir::MemoryOrdering>,
    ) {
        // choose the access order
        let order = if is_volatile {
            mir::MemoryAccessOrder::Volatile
        } else if let Some(ordering) = ordering {
            mir::MemoryAccessOrder::Atomic(mir::AtomicAccess::ordered(ordering))
        } else {
            mir::MemoryAccessOrder::Plain
        };

        // build the access
        let access = mir::MemoryAccess {
            operation: kind,
            target: mir::MemoryTarget::Address(pointer),
            byte_len: size,
            alignment_bytes: None,
            order,
        };

        // insert the memory access
        self.optimized.accesses.insert(instruction, vec![access]);
    }

    /// Attach one virtual method table with a method at slot 0.
    pub(crate) fn add_virtual_method_table(&mut self, class: mir::TypeId, callee: mir::FunctionId) {
        // build the canonical virtual table fixture
        let table = mir::VirtualTable {
            concrete: class,
            methods: vec![callee],
        };

        // index the table through dispatch tables
        self.optimized.dispatch.insert_virtual_table(table);
    }

    /// Attach one dynamic method table with a method at slot 0.
    pub(crate) fn add_dynamic_method_table(
        &mut self,
        concrete: mir::TypeId,
        constraint: mir::TypeId,
        callee: mir::FunctionId,
    ) {
        // build the canonical dynamic table fixture
        let table = mir::DynamicTable {
            concrete,
            constraint,
            entries: vec![mir::DynamicEntry::Function { function: callee }],
            names: Vec::new(),
        };

        // index the table through dispatch tables
        self.optimized.dispatch.insert_dynamic_table(table);
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
        let function = self.optimized.tree.get(function_id);
        let block = self.optimized.tree.get(function.block(0));

        // locate the first call instruction
        let call_inst = block
            .instructions
            .iter()
            .copied()
            .find(|id| matches!(self.optimized.tree.get(*id), mir::Instruction::Call { .. }))
            .expect("missing call instruction");

        // read the callee from the call instruction
        let mir::Instruction::Call { call, .. } = self.optimized.tree.get(call_inst) else {
            panic!("expected call instruction");
        };
        let callee = call.callee.function().expect("expected direct call");

        (call_inst, callee)
    }

    /// Apply a function pass with optional profile data.
    fn run_function_pass<P: FunctionPass + ?Sized>(
        &mut self,
        pass: &P,
        profile: Option<Arc<mir::Profile>>,
    ) {
        let context = PipelineContext::new(
            &self.strings_pool,
            PipelineOptions::default(),
            test_module_id(),
            test_profile_id(),
            test_target_id(),
            profile,
            Arc::new(destack_artifact::ProgramAnalysis::new()),
        );

        // collect function ids
        let function_ids: Vec<_> = self
            .optimized
            .tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect();

        // run pass on each function
        for function_id in function_ids {
            let mut function = self.optimized.tree.get(function_id).clone();

            // skip imported functions (no body)
            if function.entry().is_none() {
                continue;
            }

            // recompute next_value_id so passes can allocate fresh values
            function.recompute_next_value_id(&self.optimized.tree);
            let mut analyses = self.function_cache();
            pass.run(&mut function, &mut self.optimized, &context, &mut analyses);
            *self.optimized.tree.get_mut(function_id) = function;
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
        self.optimized
            .tree
            .iter_nodes::<mir::Function>()
            .next()
            .expect("missing function")
            .0
    }

    /// Record a function's profiled entry execution count, keyed by its symbol.
    pub(crate) fn record_function_entry(
        &self,
        profile: &mut mir::Profile,
        function: mir::LocalNodeId<mir::Function>,
        count: u64,
    ) {
        // store the entry count that scales the function's block frequencies
        let symbol = self.optimized.tree.get(function).symbol;
        profile.functions.insert(
            symbol,
            mir::FunctionProfile {
                hash: mir::FunctionHash(0),
                entry: mir::Count::new(count),
                edges: FxIndexMap::default(),
                counts: Vec::new(),
                values: FxIndexMap::default(),
            },
        );
    }

    /// Build profile data for one hot dispatch callsite.
    pub(crate) fn profile_dispatch_call(
        &mut self,
        caller: mir::FunctionId,
        callsite: mir::Point,
        callee: mir::FunctionId,
        receiver_type: mir::TypeId,
        hot_count: u64,
        unknown_count: u64,
    ) -> mir::Profile {
        let mut profile = mir::Profile::new();

        // record the function entry and receiver type samples
        self.record_function_entry(&mut profile, caller, hot_count + unknown_count);

        // record the two profiles consumed by dispatch specialization
        self.record_call_target_profile(
            &mut profile,
            caller,
            callsite,
            callee,
            hot_count,
            unknown_count,
        );
        self.record_receiver_type_profile(
            &mut profile,
            caller,
            callsite,
            receiver_type,
            hot_count,
            unknown_count,
        );

        profile
    }

    /// Record one observed call target distribution.
    pub(crate) fn record_call_target_profile(
        &mut self,
        profile: &mut mir::Profile,
        function: mir::FunctionId,
        callsite: mir::Point,
        callee: mir::FunctionId,
        hot_count: u64,
        unknown_count: u64,
    ) {
        // build the observed callee histogram
        let callee_symbol = self.optimized.tree.get(callee).symbol;
        let value = mir::ValueProfile::Calls(mir::Histogram {
            buckets: vec![(callee_symbol, mir::Count::new(hot_count))],
            unknown: mir::Count::new(unknown_count),
        });

        // attach it to the static profile point
        self.record_value_profile(
            profile,
            function,
            mir::SampleSite::CallTarget(callsite),
            value,
        );
    }

    /// Record one observed receiver type distribution.
    pub(crate) fn record_receiver_type_profile(
        &mut self,
        profile: &mut mir::Profile,
        function: mir::FunctionId,
        callsite: mir::Point,
        receiver_type: mir::TypeId,
        hot_count: u64,
        unknown_count: u64,
    ) {
        // build the observed receiver histogram
        let value = mir::ValueProfile::Types(mir::Histogram {
            buckets: vec![(receiver_type, mir::Count::new(hot_count))],
            unknown: mir::Count::new(unknown_count),
        });

        // attach it to the static profile point
        self.record_value_profile(
            profile,
            function,
            mir::SampleSite::ReceiverType(callsite),
            value,
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
            .optimized
            .tree
            .iter_nodes::<mir::Function>()
            .find(|(_, function)| function.blocks().contains(&block))
            .expect("missing function for profiled block");

        // fetch the profile entry created by record_function_entry
        let function_profile = profile
            .functions
            .get_mut(&function.symbol)
            .expect("missing function profile for successor weights");

        // record each structural successor edge
        let terminator = self
            .optimized
            .tree
            .get(self.optimized.tree.get(block).terminator);
        let targets = terminator.targets(&self.optimized.tree, block);
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

    /// Record one value profile at a semantic sample site.
    fn record_value_profile(
        &mut self,
        profile: &mut mir::Profile,
        function: mir::FunctionId,
        site: mir::SampleSite,
        value: mir::ValueProfile,
    ) {
        // map the semantic sample site to a stable sampler id
        let sampler = self.profile_sampler(function, site);

        // write the observed profile data under the function's persistent symbol
        let symbol = self.optimized.tree.get(function).symbol;
        let function_profile = profile
            .functions
            .get_mut(&symbol)
            .expect("missing function profile");
        function_profile.values.insert(sampler, value);
    }

    /// Return the sampler id for one semantic sample site.
    fn profile_sampler(
        &mut self,
        function: mir::FunctionId,
        site: mir::SampleSite,
    ) -> mir::SamplerId {
        // create the function profile table for the first sampled site
        let profile_map = self
            .optimized
            .profile
            .functions
            .entry(function)
            .or_insert_with(|| mir::FunctionProfileTable::new(mir::FunctionHash(0)));

        // reuse an existing sampler when this site was already registered
        if let Some(sampler) = profile_map.sampler(&site) {
            sampler
        }
        // otherwise append a new semantic site
        else {
            profile_map.insert_sampler(site)
        }
    }

    /// Build a module-scoped program analysis from the current tree.
    ///
    /// A test module is a standalone program, so its exported symbols are the roots.
    fn module_program_analysis(&self) -> Arc<destack_artifact::ProgramAnalysis> {
        let mut analyses = self.analysis_cache();
        let links = analyses.link(
            &self.optimized.tree,
            &self.optimized.effects,
            &self.optimized.dispatch,
            &self.optimized.drops,
        );
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

        // run the pass against the whole optimized artifact
        let mut analyses = self.analysis_cache();
        pass.run(&mut self.optimized, &context, &mut analyses);

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

        // run the pass against the whole optimized artifact
        let mut analyses = self.analysis_cache();
        pass.run(&mut self.optimized, &context, &mut analyses);

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
        mir::Formatter::new(
            &self.optimized.tree,
            self.target,
            &strings,
            mir::FormatOptions::default(),
        )
        .format()
        .expect("format MIR")
    }

    /// Assert that the current MIR matches the expected output.
    #[track_caller]
    pub(crate) fn assert_output(&self, expected: &str) {
        let actual = self.format();
        let formatted = expected_mir_text(expected);
        let actual = actual.trim();

        assert_parseable_mir_text(actual);
        assert_formatted_snapshot(actual, expected, &formatted);
    }

    /// Assert that the MIR is unchanged from the original source.
    #[track_caller]
    pub(crate) fn assert_unchanged(&self, original: &str) {
        let file = test_mir_file(original);
        let (tree, strings) = mir::parse::Parser::parse(&file, mir::parse::ParseOptions::default())
            .expect("test MIR should be text")
            .finish()
            .expect("failed to parse expected MIR");
        let expected = mir::Formatter::new(
            &tree,
            mir::TargetLayout::default(),
            &strings,
            mir::FormatOptions::default(),
        )
        .format()
        .expect("format MIR");

        self.assert_output(&expected);
    }

    /// Create a function analysis cache.
    pub(crate) fn function_cache(&self) -> FunctionCache {
        FunctionCache::new()
    }

    /// Create a module analysis cache.
    pub(crate) fn analysis_cache(&self) -> AnalysisCache {
        AnalysisCache::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use destack_core::{StringId, StringPool};
    use destack_mir as mir;
    use destack_mir::{Mutation, instruction_is_speculatable};

    use super::*;

    /// Build one function with the requested SSA value types.
    fn test_function(
        tree: &mut mir::Tree,
        value_types: &[mir::LocalNodeId<mir::Type>],
    ) -> mir::Function {
        let terminator = tree.insert(mir::Terminator::Return { value: None });
        let entry = tree.insert(mir::Block {
            parameters: Vec::new(),
            instructions: Vec::new(),
            terminator,
        });
        let body = mir::FunctionBody::new(
            entry,
            vec![entry],
            Vec::new(),
            value_types.iter().copied().map(Some).collect(),
            value_types.len() as u32,
            tree,
        );
        let name = StringId::for_text("test");

        mir::Function {
            name,
            generics: Vec::new(),
            arguments: Vec::new(),
            symbol: mir::Symbol::named(name),
            linkage: mir::Linkage::Local,
            allocation: mir::AllocationMode::Any,
            parameters: Vec::new(),
            lifetimes: Vec::new(),
            return_type: mir::TypeId::from(value_types[0]),
            environment: None,
            binding: None,
            body: Some(body),
        }
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
    fn test_mutation_set_operations() {
        // the empty change touches nothing
        assert!(Mutation::NONE.is_none());
        assert!(!Mutation::NONE.intersects(Mutation::ALL));

        // a union carries both kinds
        let both = Mutation::CONTROL | Mutation::VALUE;
        assert!(both.intersects(Mutation::CONTROL));
        assert!(both.intersects(Mutation::VALUE));
        assert!(!both.intersects(Mutation::MEMORY));

        // distinct kinds do not intersect
        assert!(!Mutation::CONTROL.intersects(Mutation::VALUE));
    }

    /// Pointer and sealed borrow address computations are speculatable.
    #[test]
    fn test_instruction_is_speculatable_allows_address_computations() {
        let mut tree = mir::Tree::new();

        let pointee = tree.intern_type(mir::Type::Int {
            width: 32,
            is_signed: true,
        });
        let pointer = tree.intern_type(mir::Type::Pointer {
            access: mir::Access::Mutable,
            pointee,
        });
        let borrowed_ref = tree.intern_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::empty(),
            storage: mir::Storage::Frame,
            access: mir::Access::Mutable,
            pointee,
        });
        let function = test_function(&mut tree, &[pointee]);

        let destination = mir::Value::new(0);
        let local = mir::LocalNodeId::new(0);
        let aggregate = mir::Value::new(1);
        let array = mir::Value::new(2);
        let index = mir::Value::new(3);

        let pointer_addr = mir::Instruction::LocalAddr {
            destination,
            local,
            result_type: pointer,
            kind: mir::AddressKind::Projection,
        };
        assert!(instruction_is_speculatable(&pointer_addr, &function, &tree));

        let local_addr = mir::Instruction::LocalAddr {
            destination,
            local,
            result_type: borrowed_ref,
            kind: mir::AddressKind::Projection,
        };
        assert!(instruction_is_speculatable(&local_addr, &function, &tree));

        let field_addr = mir::Instruction::FieldAddr {
            destination,
            aggregate,
            field: 0,
            result_type: borrowed_ref,
            kind: mir::AddressKind::Projection,
        };
        assert!(instruction_is_speculatable(&field_addr, &function, &tree));

        let element_addr = mir::Instruction::ElementAddr {
            destination,
            base: array,
            index,
            result_type: borrowed_ref,
            kind: mir::AddressKind::Projection,
        };
        assert!(instruction_is_speculatable(&element_addr, &function, &tree));
    }

    /// Managed reference results pin address computations in place.
    #[test]
    fn test_instruction_is_speculatable_rejects_managed_addresses() {
        let mut tree = mir::Tree::new();

        let pointee = tree.intern_type(mir::Type::Int {
            width: 32,
            is_signed: true,
        });
        let managed_ref = tree.intern_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Managed,
            lifetime: mir::Lifetime::empty(),
            storage: mir::Storage::Heap(mir::Space::Local),
            access: mir::Access::Mutable,
            pointee,
        });
        let function = test_function(&mut tree, &[pointee]);

        let field_addr = mir::Instruction::FieldAddr {
            destination: mir::Value::new(0),
            aggregate: mir::Value::new(1),
            field: 0,
            result_type: managed_ref,
            kind: mir::AddressKind::Projection,
        };
        assert!(!instruction_is_speculatable(&field_addr, &function, &tree));
    }

    /// Floating division and remainder can move across control flow without trapping.
    #[test]
    fn test_instruction_is_speculatable_distinguishes_numeric_types() {
        let mut tree = mir::Tree::new();
        let int = tree.intern_type(mir::Type::Int {
            width: 32,
            is_signed: true,
        });
        let float = tree.intern_type(mir::Type::FLOAT64);
        let function = test_function(&mut tree, &[int, int, float, float]);

        let integer_divide = mir::Instruction::Binary {
            destination: mir::Value::new(1),
            operator: mir::BinaryOperator::Divide,
            left: mir::Value::new(0),
            right: mir::Value::new(0),
        };
        let float_remainder = mir::Instruction::Binary {
            destination: mir::Value::new(3),
            operator: mir::BinaryOperator::Remainder,
            left: mir::Value::new(2),
            right: mir::Value::new(2),
        };

        assert!(!instruction_is_speculatable(
            &integer_divide,
            &function,
            &tree
        ));
        assert!(instruction_is_speculatable(
            &float_remainder,
            &function,
            &tree
        ));
    }
}

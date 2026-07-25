use std::collections::HashMap;
use std::sync::Arc;

use destack_core::StringPool;
use destack_mir as mir;
use destack_source::{
    DiffOptions, File, FileId, FileType, ModuleId, PackageId, ProfileId, TargetId, Uri, print_diff,
};

use crate::optimize::{FunctionPass, MirOptimized, ModulePass, PipelineContext, PipelineOptions};
use crate::{OptimizeError, OptimizeWarning};
use destack_mir::{FunctionAnalysisCache, TreeAnalysisCache};

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
    let formatted = mir::format_mir(
        &tree,
        mir::TargetLayout::default(),
        &strings,
        mir::MirFormatOptions::default(),
    )
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
        let (
            mut tree,
            target,
            mut types,
            layouts,
            dispatch,
            drops,
            memory,
            effects,
            profile,
            strings,
            _,
        ) = parsed.into_parts();

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
        types.rebuild_primitive_types(&tree);
        let strings_pool = StringPool::new();

        // copy parser strings for passes that intern through context
        strings_pool.ensure_all_from(&strings);

        Self {
            optimized: MirOptimized {
                tree,
                target,
                types,
                layouts,
                dispatch,
                drops,
                memory,
                effects,
                profile,
            },
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
        // read the function
        let function = self.optimized.tree.get(function_id);

        // use the explicit entry when present
        if let Some(entry) = function.entry() {
            return entry;
        }

        // fall back to the first block when no entry exists
        *function.blocks().first().expect("missing block")
    }

    /// Return the first intrinsic instruction in a function.
    pub(crate) fn first_intrinsic_in_function(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
        intrinsic: mir::Intrinsic,
    ) -> mir::LocalNodeId<mir::Instruction> {
        // read the function blocks
        let function = self.optimized.tree.get(function_id);

        // scan blocks in order
        for block_id in function.blocks() {
            let block = self.optimized.tree.get(*block_id);
            for instruction_id in &block.instructions {
                if matches!(
                    self.optimized.tree.get(*instruction_id),
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
        let block = self.optimized.tree.get(block_id);

        // scan instructions in order
        for instruction_id in &block.instructions {
            if matches!(
                self.optimized.tree.get(*instruction_id),
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

    /// Return call instruction ids in a function.
    pub(crate) fn call_instructions_in_function(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
    ) -> Vec<mir::LocalNodeId<mir::Instruction>> {
        // scan instructions in order
        let function = self.optimized.tree.get(function_id);
        let mut call_ids = Vec::new();

        for block_id in function.blocks() {
            let block = self.optimized.tree.get(*block_id);
            for instruction_id in &block.instructions {
                if matches!(
                    self.optimized.tree.get(*instruction_id),
                    mir::Instruction::Call { .. }
                ) {
                    call_ids.push(*instruction_id);
                }
            }
        }

        call_ids
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

    /// Attach memory access entries to an instruction.
    pub(crate) fn insert_memory_accesses(
        &mut self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        accesses: Vec<mir::MemoryAccess>,
    ) {
        // insert the memory table entries
        self.optimized
            .memory
            .insert_memory_accesses(instruction, accesses);
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
            target: mir::MemoryTarget::Reference(pointer),
            byte_len: size,
            alignment_bytes: None,
            order,
        };

        // insert the memory access
        self.optimized
            .memory
            .insert_memory_accesses(instruction, vec![access]);
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

    /// Insert a function reference type for a callee signature.
    pub(crate) fn call_signature_for_callee(
        &mut self,
        callee: mir::LocalNodeId<mir::Function>,
    ) -> mir::LocalNodeId<mir::Type> {
        // read the callee signature
        let callee_function = self.optimized.tree.get(callee);
        let param_tys = callee_function
            .parameters
            .iter()
            .map(mir::FunctionParameter::signature_parameter)
            .collect::<Vec<_>>();
        let return_ty = callee_function.return_type;

        // insert the function reference type
        self.optimized
            .tree
            .intern_type(mir::Type::FunctionSignature {
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
            let analyses = self.function_analysis_cache();
            pass.run(&mut function, &mut self.optimized, &context, &analyses);
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

    /// Return entry branch targets for a function.
    pub(crate) fn entry_branch_targets(
        &self,
        function: &mir::Function,
    ) -> (mir::LocalNodeId<mir::Block>, mir::LocalNodeId<mir::Block>) {
        // read the entry block
        let entry = function.entry().expect("missing entry block");
        let entry_block = self.optimized.tree.get(entry);
        let terminator = self.optimized.tree.get(entry_block.terminator);

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
        let block = self.optimized.tree.get(block_id);
        let terminator = self.optimized.tree.get(block.terminator);
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
        let symbol = self.optimized.tree.get(function).symbol;
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

    /// Build profile data for one hot dispatch callsite.
    pub(crate) fn profile_dispatch_call(
        &mut self,
        caller: mir::FunctionId,
        callsite: mir::CallSite,
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
        callsite: mir::CallSite,
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
        callsite: mir::CallSite,
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
        let analyses = self.tree_analysis_cache();
        let links = analyses.get::<mir::LinkGraph>(&self.optimized.tree);
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
        let analyses = self.tree_analysis_cache();
        pass.run(&mut self.optimized, &context, &analyses);

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

        // run the pass against the whole optimized artifact
        let analyses = self.tree_analysis_cache();
        pass.run(&mut self.optimized, &context, &analyses);

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
        let analyses = self.tree_analysis_cache();
        pass.run(&mut self.optimized, &context, &analyses);

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
        mir::format_mir(
            &self.optimized.tree,
            self.optimized.target,
            &strings,
            mir::MirFormatOptions::default(),
        )
        .expect("format MIR")
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
        let expected = expected_mir_text(expected);
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
        let file = test_mir_file(original);
        let (tree, strings) = mir::parse::Parser::parse(&file, mir::parse::ParseOptions::default())
            .expect("test MIR should be text")
            .finish()
            .expect("failed to parse expected MIR");
        let expected = mir::format_mir(
            &tree,
            mir::TargetLayout::default(),
            &strings,
            mir::MirFormatOptions::default(),
        )
        .expect("format MIR");

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
    // old pass helpers
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
    pub(crate) fn function_analysis_cache(&self) -> FunctionAnalysisCache {
        FunctionAnalysisCache::new(&self.optimized.memory, &self.optimized.effects)
    }

    /// Create a tree analysis cache for this test program.
    pub(crate) fn tree_analysis_cache(&self) -> TreeAnalysisCache {
        TreeAnalysisCache::new(
            &self.optimized.dispatch,
            &self.optimized.memory,
            &self.optimized.effects,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use destack_core::StringPool;
    use destack_mir as mir;
    use destack_mir::{
        Analysis, AnalysisId, FunctionAnalysis, Mutation, instruction_is_speculatable,
    };

    use super::*;

    /// Simple test analysis with no dependencies.
    struct TestAnalysisA {
        computed: bool,
    }

    impl Analysis for TestAnalysisA {
        const ID: AnalysisId = AnalysisId("test-a");
        // survives value-only changes so partial preservation is observable
        const INVALIDATED_BY: Mutation = Mutation::CONTROL;
    }

    impl FunctionAnalysis for TestAnalysisA {
        fn compute(
            _function: &mir::Function,
            _tree: &mir::Tree,
            _analyses: &FunctionAnalysisCache,
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
            analyses: &FunctionAnalysisCache,
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
            analyses: &FunctionAnalysisCache,
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

        let function_id = program
            .optimized
            .tree
            .iter_nodes::<mir::Function>()
            .next()
            .unwrap()
            .0;
        let function = program.optimized.tree.get(function_id);
        let analyses = program.function_analysis_cache();

        // initially not cached
        assert!(!analyses.is_cached::<TestAnalysisA>());

        // get computes and caches
        let a = analyses.get::<TestAnalysisA>(function, &program.optimized.tree);
        assert!(a.computed);
        assert!(analyses.is_cached::<TestAnalysisA>());

        // second get returns cached
        let a2 = analyses.get::<TestAnalysisA>(function, &program.optimized.tree);
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

        let function_id = program
            .optimized
            .tree
            .iter_nodes::<mir::Function>()
            .next()
            .unwrap()
            .0;
        let function = program.optimized.tree.get(function_id);
        let analyses = program.function_analysis_cache();

        // get B, which depends on A
        let b = analyses.get::<TestAnalysisB>(function, &program.optimized.tree);
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

        let function_id = program
            .optimized
            .tree
            .iter_nodes::<mir::Function>()
            .next()
            .unwrap()
            .0;
        let function = program.optimized.tree.get(function_id);
        let analyses = program.function_analysis_cache();

        // get C, which depends on B, which depends on A
        let c = analyses.get::<TestAnalysisC>(function, &program.optimized.tree);
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

        let function_id = program
            .optimized
            .tree
            .iter_nodes::<mir::Function>()
            .next()
            .unwrap()
            .0;
        let function = program.optimized.tree.get(function_id);
        let analyses = program.function_analysis_cache();

        // compute all
        let _ = analyses.get::<TestAnalysisC>(function, &program.optimized.tree);

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

        let function_id = program
            .optimized
            .tree
            .iter_nodes::<mir::Function>()
            .next()
            .unwrap()
            .0;
        let function = program.optimized.tree.get(function_id);
        let analyses = program.function_analysis_cache();

        // compute all
        let _ = analyses.get::<TestAnalysisC>(function, &program.optimized.tree);

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

        let function_id = program
            .optimized
            .tree
            .iter_nodes::<mir::Function>()
            .next()
            .unwrap()
            .0;
        let function = program.optimized.tree.get(function_id);
        let analyses = program.function_analysis_cache();

        // compute a structural analysis (invalidated by control flow only) and a
        // data-flow analysis (invalidated by any change)
        let _ = analyses.get::<mir::ControlFlowGraph>(function, &program.optimized.tree);
        let _ = analyses.get::<mir::ConstantPropagation>(function, &program.optimized.tree);

        // a value-only change preserves the control-flow graph and invalidates the
        // value-dependent analysis
        analyses.apply(Mutation::VALUE);

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
        let both = Mutation::CONTROL | Mutation::VALUE;
        assert!(both.intersects(Mutation::CONTROL));
        assert!(both.intersects(Mutation::VALUE));
        assert!(!both.intersects(Mutation::MEMORY));

        // distinct kinds do not intersect
        assert!(!Mutation::CONTROL.intersects(Mutation::VALUE));
    }

    /// Borrow address instructions are not speculatable.
    #[test]
    fn test_instruction_is_speculatable_rejects_borrow_addresses() {
        let mut tree = mir::Tree::new();

        let pointee = tree.intern_type(mir::Type::Int {
            width: 32,
            is_signed: true,
        });
        let borrowed_ref = tree.intern_type(mir::Type::Reference {
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
            field: 0,
            result_type: borrowed_ref,
        };
        assert!(!instruction_is_speculatable(&field_addr, &tree));

        let element_addr = mir::Instruction::ElementAddr {
            destination,
            base: array,
            index,
            result_type: borrowed_ref,
        };
        assert!(!instruction_is_speculatable(&element_addr, &tree));
    }

    /// Raw address instructions are speculatable with typed checks.
    #[test]
    fn test_instruction_is_speculatable_allows_raw_addresses() {
        let mut tree = mir::Tree::new();

        let pointee = tree.intern_type(mir::Type::Int {
            width: 32,
            is_signed: true,
        });
        let raw_ref = tree.intern_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Raw,
            lifetime: mir::Lifetime::empty(),
            space: mir::Space::Frame,
            access: mir::Access::Mutable,
            pointee,
            nullability: mir::Nullability::None,
        });
        let borrowed_ref = tree.intern_type(mir::Type::Reference {
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

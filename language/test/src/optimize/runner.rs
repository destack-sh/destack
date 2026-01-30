use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use destack_base::{ImmutableStringPool, StringPool};
use destack_compiler::{
    CompositePipeline, FunctionPass, FunctionPipeline, FunctionToModuleAdaptor, ModulePass,
    ModulePipeline, OptimizationLevel, Pipeline, PipelineContext, PipelineOptions,
    RepeatedPipeline, default_pipeline,
};
use destack_mir as mir;
use destack_source::{FileId, ModuleId, PackageId};
use destack_vm::diagnostic::RuntimeResult;
use destack_vm::memory::Value;
use destack_vm::{ExecutionOutcome, ExecutionOutput, Isolate, IsolateOptions};
use destack_workspace::TargetId;
use mir::parse::ParseOptions;

use crate::harness::{RunContext, Suite, TestCase, TestOptions, TestResult};

use destack_test_mirbench as program;

/// All optimization levels exercised by the optimizer bench suite.
const OPTIMIZATION_LEVELS: [OptimizationLevel; 5] = [
    OptimizationLevel::O0,
    OptimizationLevel::O1,
    OptimizationLevel::O2,
    OptimizationLevel::O3,
    OptimizationLevel::O4,
];

/// Run options for optimizer bench suites.
#[derive(Debug, Clone)]
pub struct OptimizeRunOptions {
    /// Optional optimization level filter.
    pub level_filter: Option<OptimizationLevel>,
    /// Whether to enable diagnostic mismatch isolation.
    pub diagnostic: bool,
    /// Whether to enable matrix pass isolation.
    pub matrix: bool,
    /// Whether to emit trace output during runs.
    pub trace: bool,
    /// Optional instruction limit for execution.
    pub max_instruction_limit: Option<u64>,
}

impl OptimizeRunOptions {
    /// Return true when a level should run under this configuration.
    fn allows_level(&self, level: OptimizationLevel) -> bool {
        // honor the optional level filter
        match self.level_filter {
            Some(filter) => filter == level,
            None => true,
        }
    }
}

/// Perf sample data captured for a single program and level.
#[derive(Debug, Clone)]
struct PerfSample {
    /// Program name.
    name: String,
    /// Optimization level.
    level: OptimizationLevel,
    /// Compile time in milliseconds.
    compile_ms: f64,
    /// Runtime in milliseconds.
    runtime_ms: f64,
    /// Baseline runtime in milliseconds.
    baseline_runtime_ms: f64,
    /// MIR instruction count after optimization.
    mir_instructions: u64,
    /// Baseline MIR instruction count.
    baseline_mir_instructions: u64,
    /// Threaded instructions executed by the VM.
    threaded_instructions: u64,
    /// Baseline threaded instructions executed by the VM.
    baseline_threaded_instructions: u64,
}

/// Allowed diagnostics for a bench fixture.
#[derive(Debug, Default)]
struct BenchAllowList {
    /// Explicitly allowed error codes.
    error_codes: HashSet<String>,
    /// Explicitly allowed warning codes.
    warning_codes: HashSet<String>,
}

impl BenchAllowList {
    /// Parse allow list metadata from a source string.
    fn from_source(source: &str) -> Self {
        // start with an empty allow list
        let mut allow = Self::default();

        // scan for allow directives
        for line in source.lines() {
            let line = line.trim();
            let directive = match line.strip_prefix("//") {
                Some(rest) => rest.trim(),
                None => continue,
            };

            // parse allow list directives
            if let Some(rest) = directive.strip_prefix("expect-errors=") {
                for code in rest.split(',') {
                    let code = code.trim();
                    if !code.is_empty() {
                        allow.error_codes.insert(code.to_string());
                    }
                }
            } else if let Some(rest) = directive.strip_prefix("expect-warnings=") {
                for code in rest.split(',') {
                    let code = code.trim();
                    if !code.is_empty() {
                        allow.warning_codes.insert(code.to_string());
                    }
                }
            }
        }

        allow
    }

    /// Return true when an error is explicitly allowed.
    fn allows_error(&self, error: &destack_compiler::OptimizeError) -> bool {
        self.error_codes.contains(error.code())
    }

    /// Return true when a warning is explicitly allowed.
    fn allows_warning(&self, warning: &destack_compiler::OptimizeWarning) -> bool {
        self.warning_codes.contains(warning.code())
    }

    /// Return true when any diagnostics are expected.
    fn has_expectations(&self) -> bool {
        !self.error_codes.is_empty() || !self.warning_codes.is_empty()
    }
}

/// Optimizer bench suite for validation runs.
#[derive(Debug)]
pub struct OptimizeValidateSuite {
    /// Shared run options.
    options: OptimizeRunOptions,
    /// Root path for the fixtures.
    root: PathBuf,
    /// Discovered test cases.
    cases: Vec<TestCase>,
}

impl OptimizeValidateSuite {
    /// Load the validation suite with the provided options.
    pub fn load(options: OptimizeRunOptions) -> Self {
        // locate fixture root
        let root = program::fixtures_root();

        // collect bench fixtures
        let fixtures = collect_mir_files(&root);
        let cases = fixtures
            .into_iter()
            .map(|path| build_fixture_case(&root, path))
            .collect();

        Self {
            options,
            root,
            cases,
        }
    }
}

impl Suite for OptimizeValidateSuite {
    fn name(&self) -> &'static str {
        "optimize_validate"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        self.cases.clone()
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        // read the source program
        let source = match fs::read_to_string(&case.path) {
            Ok(source) => source,
            Err(error) => {
                return TestResult::Failed {
                    message: format!("failed to read {}: {error}", case.path.display()),
                };
            }
        };

        // parse allow list directives
        let allow_list = BenchAllowList::from_source(&source);
        let package_id = PackageId::from_synthetic_path(&self.root);
        let target_id = TargetId::new(package_id, "native");
        let options = PipelineOptions::default();
        let module_id = module_id_for_fixture(&self.root, &case.path, package_id);

        // run all configured optimization levels
        let mut ran_any = false;
        for level in OPTIMIZATION_LEVELS {
            if !self.options.allows_level(level) {
                continue;
            }

            // emit trace output when requested
            if self.options.trace {
                eprintln!("optimize bench {} {level:?}", case.path.display());
            }

            ran_any = true;
            let pipeline = default_pipeline(level);
            if let Err(message) = optimize_source(
                &source,
                module_id,
                &target_id,
                &pipeline,
                options.clone(),
                &allow_list,
            ) {
                return TestResult::Failed { message };
            }
        }

        // skip if no levels matched the filter
        if !ran_any {
            return TestResult::Skipped {
                reason: "filtered out by optimization level".to_string(),
            };
        }

        TestResult::Passed
    }
}

/// Optimizer bench suite for baseline runs.
#[derive(Debug)]
pub struct OptimizeBaselineSuite {
    /// Shared run options.
    options: OptimizeRunOptions,
    /// Discovered test cases.
    cases: Vec<TestCase>,
    /// Program lookup by name.
    programs: HashMap<String, &'static program::Program>,
}

impl OptimizeBaselineSuite {
    /// Load the baseline suite with the provided options.
    pub fn load(options: OptimizeRunOptions) -> Self {
        // locate fixture root
        let root = program::fixtures_root();

        // prepare program containers
        let mut cases = Vec::new();
        let mut programs = HashMap::new();

        // build program cases
        for entry in program::all_programs() {
            let path = root.join(format!("{}.mir", entry.name));
            let case = TestCase::file(entry.name, path, "destack_test::optimize::baseline");
            cases.push(case);
            programs.insert(entry.name.to_string(), entry);
        }

        Self {
            options,
            cases,
            programs,
        }
    }
}

impl Suite for OptimizeBaselineSuite {
    fn name(&self) -> &'static str {
        "optimize_baseline"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        self.cases.clone()
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        // resolve the program metadata
        let program = match self.programs.get(&case.name) {
            Some(program) => *program,
            None => {
                return TestResult::Failed {
                    message: format!("program '{}' not found", case.name),
                };
            }
        };

        // parse allow list directives
        let allow_list = BenchAllowList::from_source(program.source);
        if allow_list.has_expectations() {
            return TestResult::Skipped {
                reason: "skipped by allow list directives".to_string(),
            };
        }

        // emit trace output when requested
        if self.options.trace {
            eprintln!("baseline bench {}", program.name);
        }

        // run the baseline program
        let output = match baseline_output_for_program_default_args(
            program,
            self.options.max_instruction_limit,
        ) {
            Ok(output) => output,
            Err(message) => return TestResult::Failed { message },
        };

        // compare against expected output
        let expected = (program.expected)();
        if output.value != expected {
            return TestResult::Failed {
                message: format!(
                    "'{}' (baseline): expected {:?}, got {:?}",
                    program.name, expected, output.value
                ),
            };
        }

        TestResult::Passed
    }
}

/// Optimizer bench suite for execute runs.
#[derive(Debug)]
pub struct OptimizeExecuteSuite {
    /// Shared run options.
    options: OptimizeRunOptions,
    /// Root path for the fixtures.
    root: PathBuf,
    /// Discovered test cases.
    cases: Vec<TestCase>,
    /// Program lookup by name.
    programs: HashMap<String, &'static program::Program>,
}

impl OptimizeExecuteSuite {
    /// Load the execute suite with the provided options.
    pub fn load(options: OptimizeRunOptions) -> Self {
        // locate fixture root
        let root = program::fixtures_root();

        // prepare program containers
        let mut cases = Vec::new();
        let mut programs = HashMap::new();

        // build program cases
        for entry in program::all_programs() {
            let path = root.join(format!("{}.mir", entry.name));
            let case = TestCase::file(entry.name, path, "destack_test::optimize::execute");
            cases.push(case);
            programs.insert(entry.name.to_string(), entry);
        }

        Self {
            options,
            root,
            cases,
            programs,
        }
    }
}

impl Suite for OptimizeExecuteSuite {
    fn name(&self) -> &'static str {
        "optimize_execute"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        self.cases.clone()
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        // resolve the program metadata
        let program = match self.programs.get(&case.name) {
            Some(program) => *program,
            None => {
                return TestResult::Failed {
                    message: format!("program '{}' not found", case.name),
                };
            }
        };

        // parse allow list directives
        let allow_list = BenchAllowList::from_source(program.source);
        if allow_list.has_expectations() {
            return TestResult::Skipped {
                reason: "skipped by allow list directives".to_string(),
            };
        }

        // run the baseline program for comparison
        let baseline_output =
            match baseline_output_for_program(program, self.options.max_instruction_limit) {
                Ok(output) => output,
                Err(message) => return TestResult::Failed { message },
            };

        // prepare shared configuration
        let package_id = PackageId::from_synthetic_path(&self.root);
        let target_id = TargetId::new(package_id, "native");
        let options = PipelineOptions::default();

        // optimize and validate each configured level
        let mut ran_any = false;
        for level in OPTIMIZATION_LEVELS {
            if !self.options.allows_level(level) {
                continue;
            }

            // emit trace output when requested
            if self.options.trace {
                eprintln!("execute bench {} {level:?}", program.name);
            }

            ran_any = true;
            let pipeline = default_pipeline(level);
            let module_id = module_id_for_program(package_id, program.name);
            let (tree, strings) = match optimize_source(
                program.source,
                module_id,
                &target_id,
                &pipeline,
                options.clone(),
                &allow_list,
            ) {
                Ok(output) => output,
                Err(message) => return TestResult::Failed { message },
            };

            // execute the optimized program
            let output = run_program_with_tree_result(
                program,
                tree,
                strings,
                self.options.max_instruction_limit,
            );
            let output = match output {
                Ok(output) => output,
                Err(error) => {
                    let message =
                        format!("'{}' ({level:?}): execution failed: {error}", program.name);
                    let message = build_execute_failure(
                        program,
                        level,
                        &target_id,
                        &options,
                        &baseline_output,
                        &self.options,
                        message,
                    );
                    return TestResult::Failed { message };
                }
            };

            // compare outputs against the baseline
            if output.value != baseline_output.value {
                let message = format!(
                    "'{}' ({level:?}): expected {:?}, got {:?}",
                    program.name, baseline_output.value, output.value
                );
                let message = build_execute_failure(
                    program,
                    level,
                    &target_id,
                    &options,
                    &baseline_output,
                    &self.options,
                    message,
                );
                return TestResult::Failed { message };
            }
        }

        // skip if no levels matched the filter
        if !ran_any {
            return TestResult::Skipped {
                reason: "filtered out by optimization level".to_string(),
            };
        }

        TestResult::Passed
    }
}

/// Optimizer bench suite for perf runs.
#[derive(Debug)]
pub struct OptimizePerfSuite {
    /// Shared run options.
    options: OptimizeRunOptions,
    /// Root path for the fixtures.
    root: PathBuf,
    /// Discovered test cases.
    cases: Vec<TestCase>,
    /// Program lookup by name.
    programs: HashMap<String, &'static program::Program>,
    /// Collected perf samples.
    samples: Mutex<Vec<PerfSample>>,
}

impl OptimizePerfSuite {
    /// Load the perf suite with the provided options.
    pub fn load(options: OptimizeRunOptions) -> Self {
        // locate fixture root
        let root = program::fixtures_root();

        // prepare program containers
        let mut cases = Vec::new();
        let mut programs = HashMap::new();

        // build program cases
        for entry in program::all_programs() {
            let path = root.join(format!("{}.mir", entry.name));
            let case = TestCase::file(entry.name, path, "destack_test::optimize::perf");
            cases.push(case);
            programs.insert(entry.name.to_string(), entry);
        }

        Self {
            options,
            root,
            cases,
            programs,
            samples: Mutex::new(Vec::new()),
        }
    }
}

impl Suite for OptimizePerfSuite {
    fn name(&self) -> &'static str {
        "optimize_perf"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        self.cases.clone()
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        // resolve the program metadata
        let program = match self.programs.get(&case.name) {
            Some(program) => *program,
            None => {
                return TestResult::Failed {
                    message: format!("program '{}' not found", case.name),
                };
            }
        };

        // parse allow list directives
        let allow_list = BenchAllowList::from_source(program.source);
        if allow_list.has_expectations() {
            return TestResult::Skipped {
                reason: "skipped by allow list directives".to_string(),
            };
        }

        // parse baseline tree to count MIR instructions
        let (baseline_tree, baseline_strings) = match parse_mir_source(program.source) {
            Ok(output) => output,
            Err(message) => return TestResult::Failed { message },
        };
        let baseline_mir_instructions = count_mir_instructions(&baseline_tree);

        // run baseline program for correctness and baseline metrics
        let baseline_start = Instant::now();
        let baseline_output = match run_program_with_tree_result(
            program,
            baseline_tree,
            baseline_strings,
            self.options.max_instruction_limit,
        ) {
            Ok(output) => output,
            Err(error) => {
                return TestResult::Failed {
                    message: format!("bench '{}' baseline failed: {error}", program.name),
                };
            }
        };
        let baseline_runtime_ms = baseline_start.elapsed().as_secs_f64() * 1000.0;
        let baseline_threaded_instructions =
            baseline_output.statistics.threaded_instructions_executed;

        // prepare shared configuration
        let package_id = PackageId::from_synthetic_path(&self.root);
        let target_id = TargetId::new(package_id, "native");
        let options = PipelineOptions::default();

        // optimize and execute each configured level
        let mut ran_any = false;
        for level in OPTIMIZATION_LEVELS {
            if !self.options.allows_level(level) {
                continue;
            }

            // emit trace output when requested
            if self.options.trace {
                eprintln!("perf bench {} {level:?}", program.name);
            }

            ran_any = true;
            let pipeline = default_pipeline(level);
            let module_id = module_id_for_program(package_id, program.name);

            // optimize with compile timing
            let compile_start = Instant::now();
            let (tree, strings) = match optimize_source(
                program.source,
                module_id,
                &target_id,
                &pipeline,
                options.clone(),
                &allow_list,
            ) {
                Ok(output) => output,
                Err(message) => return TestResult::Failed { message },
            };
            let compile_ms = compile_start.elapsed().as_secs_f64() * 1000.0;

            // compute MIR instruction count after optimization
            let mir_instructions = count_mir_instructions(&tree);

            // execute the optimized program with runtime timing
            let runtime_start = Instant::now();
            let output = run_program_with_tree_result(
                program,
                tree,
                strings,
                self.options.max_instruction_limit,
            );
            let runtime_ms = runtime_start.elapsed().as_secs_f64() * 1000.0;
            let output = match output {
                Ok(output) => output,
                Err(error) => {
                    return TestResult::Failed {
                        message: format!("bench '{}' ({level:?}) failed: {error}", program.name),
                    };
                }
            };

            // compare outputs against the baseline
            if output.value != baseline_output.value {
                return TestResult::Failed {
                    message: format!(
                        "'{}' ({level:?}): expected {:?}, got {:?}",
                        program.name, baseline_output.value, output.value
                    ),
                };
            }

            let sample = PerfSample {
                name: program.name.to_string(),
                level,
                compile_ms,
                runtime_ms,
                baseline_runtime_ms,
                mir_instructions,
                baseline_mir_instructions,
                threaded_instructions: output.statistics.threaded_instructions_executed,
                baseline_threaded_instructions,
            };

            self.samples
                .lock()
                .expect("perf samples lock poisoned")
                .push(sample);
        }

        // skip if no levels matched the filter
        if !ran_any {
            return TestResult::Skipped {
                reason: "filtered out by optimization level".to_string(),
            };
        }

        TestResult::Passed
    }

    fn report(&self, _results: &[(TestCase, TestResult)], _context: &RunContext<'_>) {
        let mut samples = self
            .samples
            .lock()
            .expect("perf samples lock poisoned")
            .clone();

        samples.sort_by(|a, b| match a.name.cmp(&b.name) {
            std::cmp::Ordering::Equal => level_index(a.level).cmp(&level_index(b.level)),
            order => order,
        });

        if samples.is_empty() {
            return;
        }

        println!();
        println!("perf summary (ms, threaded inst, mir inst)");
        for sample in samples {
            let runtime_delta = sample.runtime_ms - sample.baseline_runtime_ms;
            let mir_delta =
                sample.mir_instructions as i64 - sample.baseline_mir_instructions as i64;
            let threaded_delta =
                sample.threaded_instructions as i64 - sample.baseline_threaded_instructions as i64;

            println!(
                "{} {level:?} compile={compile:.2} runtime={runtime:.2} (delta={runtime_delta:+.2}) threaded={threaded} (delta={threaded_delta:+}) mir={mir} (delta={mir_delta:+})",
                sample.name,
                level = sample.level,
                compile = sample.compile_ms,
                runtime = sample.runtime_ms,
                runtime_delta = runtime_delta,
                threaded = sample.threaded_instructions,
                threaded_delta = threaded_delta,
                mir = sample.mir_instructions,
                mir_delta = mir_delta,
            );
        }
    }
}

/// Collect all MIR bench fixtures under the root.
fn collect_mir_files(root: &Path) -> Vec<PathBuf> {
    // collect files with a depth first walk
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();

    // walk all directories under the root
    while let Some(dir) = pending.pop() {
        let entries =
            fs::read_dir(&dir).unwrap_or_else(|error| panic!("failed to read {dir:?}: {error}"));

        // scan directory entries
        for entry in entries {
            let entry =
                entry.unwrap_or_else(|error| panic!("failed to read entry in {dir:?}: {error}"));
            let path = entry.path();

            // descend into directories
            if path.is_dir() {
                pending.push(path);
                continue;
            }

            // record mir files
            if path.extension().and_then(|ext| ext.to_str()) == Some("mir") {
                files.push(path);
            }
        }
    }

    // ensure deterministic order for test output
    files.sort();

    files
}

/// Build a test case for a fixture path.
fn build_fixture_case(root: &Path, path: PathBuf) -> TestCase {
    // compute a stable relative name
    let relative = path.strip_prefix(root).unwrap_or(&path);
    let name = relative.to_string_lossy().replace('\\', "/");

    // build the test case
    TestCase::file(name, path, "destack_test::optimize::validate")
}

/// Return the module id for a bench fixture path.
fn module_id_for_fixture(root: &Path, path: &Path, package_id: PackageId) -> ModuleId {
    // compute a stable module id for the fixture path
    ModuleId::from_path(package_id, path, Some(root))
}

/// Return the module id for a bench program name.
fn module_id_for_program(package_id: PackageId, name: &str) -> ModuleId {
    // build a module id for the program name
    let relative = PathBuf::from(format!("bench/{name}.mir"));
    ModuleId::from_relative_path(package_id, &relative)
}

/// Parse a MIR source string into a tree and string pool.
fn parse_mir_source(source: &str) -> Result<(mir::NodeTree, ImmutableStringPool), String> {
    // parse the source program
    mir::parse::Parser::parse(FileId::new(0), source, ParseOptions::default())
        .map_err(|error| format!("failed to parse mir: {error}"))
}

/// Count MIR instructions across all function bodies.
fn count_mir_instructions(tree: &mir::NodeTree) -> u64 {
    let mut count = 0u64;

    for (_, function) in tree.iter_nodes::<mir::Function>() {
        if function.entry.is_none() {
            continue;
        }

        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            count += block.instructions.len() as u64 + 1;
        }
    }

    count
}

/// Map optimization levels to a stable sort index.
fn level_index(level: OptimizationLevel) -> u8 {
    match level {
        OptimizationLevel::O0 => 0,
        OptimizationLevel::O1 => 1,
        OptimizationLevel::O2 => 2,
        OptimizationLevel::O3 => 3,
        OptimizationLevel::O4 => 4,
    }
}

/// Optimize a source program and return the optimized tree and strings.
fn optimize_source(
    source: &str,
    module_id: ModuleId,
    target_id: &TargetId,
    pipeline: &dyn Pipeline,
    options: PipelineOptions,
    allow_list: &BenchAllowList,
) -> Result<(mir::NodeTree, ImmutableStringPool), String> {
    // parse the source program
    let (mut tree, strings) = parse_mir_source(source)?;

    // build the pipeline context
    let strings_pool = StringPool::new();
    strings_pool.copy_from_immutable(&strings);
    let mut ctx = PipelineContext::new(&strings_pool, options, module_id, target_id.clone(), None);

    // run optimization and check diagnostics
    pipeline.run(&mut tree, &mut ctx);
    let diagnostics = ctx.diagnostics();
    let errors = diagnostics.take_errors();
    let warnings = diagnostics.take_warnings();

    let unexpected_errors: Vec<_> = errors
        .iter()
        .filter(|error| !allow_list.allows_error(error))
        .collect();
    let unexpected_warnings: Vec<_> = warnings
        .iter()
        .filter(|warning| !allow_list.allows_warning(warning))
        .collect();

    if !unexpected_errors.is_empty() || !unexpected_warnings.is_empty() {
        return Err(format!(
            "unexpected diagnostics:\nerrors: {unexpected_errors:#?}\nwarnings: {unexpected_warnings:#?}"
        ));
    }

    // return optimized output
    let optimized_strings = strings_pool.clone().into_immutable();
    Ok((tree, optimized_strings))
}

/// Return the baseline output for a bench program.
fn baseline_output_for_program(
    program: &program::Program,
    max_instruction_limit: Option<u64>,
) -> Result<ExecutionOutput, String> {
    // parse the source program
    let (tree, strings) = parse_mir_source(program.source)?;

    // run the baseline program with quick profile args
    run_program_with_tree_result(program, tree, strings, max_instruction_limit)
        .map_err(|error| format!("bench '{}' baseline failed: {error}", program.name))
}

/// Return the baseline output for a bench program using default args.
fn baseline_output_for_program_default_args(
    program: &program::Program,
    max_instruction_limit: Option<u64>,
) -> Result<ExecutionOutput, String> {
    // parse the source program
    let (tree, strings) = parse_mir_source(program.source)?;

    // run the baseline program with default args
    run_program_with_tree_result_default_args(program, tree, strings, max_instruction_limit)
        .map_err(|error| format!("bench '{}' baseline failed: {error}", program.name))
}

/// Run a program and compare against the baseline output.
fn run_and_compare_output(
    program: &program::Program,
    tree: mir::NodeTree,
    strings: ImmutableStringPool,
    baseline_output: &ExecutionOutput,
    max_instruction_limit: Option<u64>,
    context: &str,
) -> Result<(), String> {
    // execute the program
    let output = run_program_with_tree_result(program, tree, strings, max_instruction_limit);
    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return Err(format!(
                "bench '{}' {context}: execution failed: {error}",
                program.name
            ));
        }
    };

    // compare outputs
    if output.value != baseline_output.value {
        return Err(format!(
            "bench '{}' {context}: expected {:?}, got {:?}",
            program.name, baseline_output.value, output.value
        ));
    }

    Ok(())
}

/// Run a single function pass over the module.
fn run_function_pass(
    pass: &dyn FunctionPass,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
) -> bool {
    // track whether any pass invalidated analyses
    let mut any_changed = false;

    // collect function ids
    let function_ids: Vec<_> = tree
        .iter_nodes::<mir::Function>()
        .map(|(id, _)| id)
        .collect();

    for function_id in function_ids {
        let mut function = tree.get(function_id).clone();

        // skip imported functions
        if function.entry.is_none() {
            continue;
        }

        // recompute next_value_id so passes can allocate fresh values
        function.recompute_next_value_id(tree);
        let preserved = pass.run(&mut function, tree, ctx);
        if !preserved.preserves_all() {
            any_changed = true;
        }

        // write function back
        *tree.get_mut(function_id) = function;
    }

    any_changed
}

/// Run a single module pass over the module.
fn run_module_pass(
    pass: &dyn ModulePass,
    tree: &mut mir::NodeTree,
    ctx: &PipelineContext<'_>,
) -> bool {
    // run the pass and track invalidation
    let preserved = pass.run(tree, ctx);

    !preserved.preserves_all()
}

/// Run a matrix case and return a mismatch reason if any.
fn run_matrix_case(
    program: &program::Program,
    target_id: &TargetId,
    options: &PipelineOptions,
    baseline_output: &ExecutionOutput,
    max_instruction_limit: Option<u64>,
    label: &str,
    apply: impl FnOnce(&mut mir::NodeTree, &StringPool, &PipelineContext<'_>),
) -> Option<String> {
    // parse the source program
    let (mut tree, strings) =
        mir::parse::Parser::parse(FileId::new(0), program.source, ParseOptions::default()).ok()?;

    // build the pipeline context
    let strings_pool = StringPool::new();
    strings_pool.copy_from_immutable(&strings);
    let module_id = ModuleId::from_relative_path(
        target_id.package_id,
        &PathBuf::from(format!("bench/{}.mir", program.name)),
    );
    let ctx = PipelineContext::new(
        &strings_pool,
        options.clone(),
        module_id,
        target_id.clone(),
        None,
    );

    // apply the case and execute
    apply(&mut tree, &strings_pool, &ctx);

    // verify the resulting MIR tree
    if let Err(error) = mir::Verifier::new(&tree).verify_tree() {
        let message = format_verify_error(&tree, error);
        return Some(format!("case '{label}': verification failed: {message}"));
    }

    let context = format!("case '{label}'");
    run_and_compare_output(
        program,
        tree,
        strings_pool.into_immutable(),
        baseline_output,
        max_instruction_limit,
        &context,
    )
    .err()
}

/// Format a verifier error with context when available.
fn format_verify_error(tree: &mir::NodeTree, error: mir::VerifyError) -> String {
    // include instruction context for undefined value errors
    match error {
        mir::VerifyError::UseOfUndefinedValue { value, anchor } => {
            if anchor.node.ty == mir::NodeType::Instruction {
                let instruction_id = mir::LocalNodeId::<mir::Instruction>::new(anchor.node.id);
                let instruction = tree.get(instruction_id);
                if let Some((block_id, block)) = find_instruction_block(tree, instruction_id) {
                    let definition = find_value_definition(tree, value);
                    return format!(
                        "{error:?} at instruction {instruction_id:?} in {block_id:?}: {instruction:?} (params: {:?}, def: {definition})",
                        block.parameters
                    );
                }

                return format!("{error:?} at instruction {instruction_id:?}: {instruction:?}");
            }

            format!("{error:?}")
        }
        _ => format!("{error:?}"),
    }
}

/// Locate the block containing an instruction.
fn find_instruction_block(
    tree: &mir::NodeTree,
    instruction_id: mir::LocalNodeId<mir::Instruction>,
) -> Option<(mir::LocalNodeId<mir::Block>, mir::Block)> {
    // scan blocks to find the instruction
    for (block_id, block) in tree.iter_nodes::<mir::Block>() {
        if block.instructions.contains(&instruction_id) {
            return Some((block_id, block.clone()));
        }
    }

    None
}

/// Describe where a value is defined, if known.
fn find_value_definition(tree: &mir::NodeTree, value: mir::Value) -> String {
    // scan blocks for parameter definitions
    for (block_id, block) in tree.iter_nodes::<mir::Block>() {
        if let Some(param) = block.parameters.iter().find(|param| param.value == value) {
            return format!("{block_id:?} param {param:?}");
        }

        // scan instructions for destination definitions
        for instruction_id in &block.instructions {
            let instruction = tree.get(*instruction_id);
            if instruction.destination() == Some(value) {
                return format!("{block_id:?} {instruction_id:?} {instruction:?}");
            }
        }
    }

    "unknown".to_string()
}

/// Run matrix diagnostics for the given pipeline.
fn run_matrix_pipeline(
    program: &program::Program,
    target_id: &TargetId,
    options: &PipelineOptions,
    baseline_output: &ExecutionOutput,
    max_instruction_limit: Option<u64>,
    pipeline: &dyn Pipeline,
    path: &mut Vec<String>,
) -> Option<String> {
    // handle composite pipelines
    if let Some(composite) = pipeline.as_any().downcast_ref::<CompositePipeline>() {
        for child in composite.pipelines() {
            let reason = run_matrix_pipeline(
                program,
                target_id,
                options,
                baseline_output,
                max_instruction_limit,
                child.as_ref(),
                path,
            );
            if reason.is_some() {
                return reason;
            }
        }

        return None;
    }

    // handle repeated pipelines
    if let Some(repeat) = pipeline.as_any().downcast_ref::<RepeatedPipeline>() {
        let repeat_label = format!("repeat({})", repeat.max_iterations());
        path.push(repeat_label);
        let reason = run_matrix_pipeline(
            program,
            target_id,
            options,
            baseline_output,
            max_instruction_limit,
            repeat.inner(),
            path,
        );
        path.pop();
        return reason;
    }

    // handle function pipelines
    if let Some(function_pipeline) = pipeline.as_any().downcast_ref::<FunctionPipeline>() {
        for pass in function_pipeline.passes() {
            let mut label_parts = path.clone();
            label_parts.push(pass.name().to_string());
            let label = label_parts.join(" / ");

            let reason = run_matrix_case(
                program,
                target_id,
                options,
                baseline_output,
                max_instruction_limit,
                &label,
                |tree, _strings_pool, ctx| {
                    run_function_pass(pass.as_ref(), tree, ctx);
                },
            );
            if reason.is_some() {
                return reason;
            }
        }

        return None;
    }

    // handle function adaptor pipelines
    if let Some(adaptor) = pipeline.as_any().downcast_ref::<FunctionToModuleAdaptor>() {
        return run_matrix_pipeline(
            program,
            target_id,
            options,
            baseline_output,
            max_instruction_limit,
            adaptor.inner(),
            path,
        );
    }

    // handle module pipelines
    if let Some(module_pipeline) = pipeline.as_any().downcast_ref::<ModulePipeline>() {
        for pass in module_pipeline.passes() {
            let mut label_parts = path.clone();
            label_parts.push(pass.name().to_string());
            let label = label_parts.join(" / ");

            let reason = run_matrix_case(
                program,
                target_id,
                options,
                baseline_output,
                max_instruction_limit,
                &label,
                |tree, _strings_pool, ctx| {
                    run_module_pass(pass.as_ref(), tree, ctx);
                },
            );
            if reason.is_some() {
                return reason;
            }
        }

        return None;
    }

    Some(format!(
        "bench '{}' encountered unsupported pipeline '{}'",
        program.name,
        pipeline.name()
    ))
}

/// Diagnose the first pass that changes execution output.
fn diagnose_mismatch(
    program: &program::Program,
    level: OptimizationLevel,
    target_id: &TargetId,
    options: &PipelineOptions,
    baseline_output: &ExecutionOutput,
    max_instruction_limit: Option<u64>,
) -> Option<String> {
    // parse the source program
    let (mut tree, strings) =
        mir::parse::Parser::parse(FileId::new(0), program.source, ParseOptions::default()).ok()?;

    // build the pipeline context
    let strings_pool = StringPool::new();
    strings_pool.copy_from_immutable(&strings);
    let module_id = ModuleId::from_relative_path(
        target_id.package_id,
        &PathBuf::from(format!("bench/{}.mir", program.name)),
    );
    let mut ctx = PipelineContext::new(
        &strings_pool,
        options.clone(),
        module_id,
        target_id.clone(),
        None,
    );

    // run each pass in order using a pass major traversal
    let pipeline = default_pipeline(level);
    diagnose_pipeline(
        program,
        &mut tree,
        &strings_pool,
        &mut ctx,
        &pipeline,
        baseline_output,
        max_instruction_limit,
        0,
        None,
    )
    .err()
}

/// Format a diagnostic mismatch message.
fn format_pass_context(depth: usize, iteration: Option<(usize, usize)>, pass_name: &str) -> String {
    // format context with optional iteration
    if let Some((iteration, max_iterations)) = iteration {
        format!("{depth} iter {iteration}/{max_iterations} pass '{pass_name}'")
    } else {
        format!("{depth} pass '{pass_name}'")
    }
}

/// Walk a pipeline and return whether any changes occurred.
#[allow(clippy::too_many_arguments)]
fn diagnose_pipeline(
    program: &program::Program,
    tree: &mut mir::NodeTree,
    strings_pool: &StringPool,
    ctx: &mut PipelineContext<'_>,
    pipeline: &dyn Pipeline,
    baseline_output: &ExecutionOutput,
    max_instruction_limit: Option<u64>,
    depth: usize,
    iteration: Option<(usize, usize)>,
) -> Result<bool, String> {
    // handle composite pipelines
    if let Some(composite) = pipeline.as_any().downcast_ref::<CompositePipeline>() {
        let mut any_changed = false;

        for child in composite.pipelines() {
            let changed = diagnose_pipeline(
                program,
                tree,
                strings_pool,
                ctx,
                child.as_ref(),
                baseline_output,
                max_instruction_limit,
                depth,
                iteration,
            )?;
            any_changed |= changed;
        }

        return Ok(any_changed);
    }

    // handle repeated pipelines
    if let Some(repeat) = pipeline.as_any().downcast_ref::<RepeatedPipeline>() {
        let mut any_changed = false;
        let max_iterations = repeat.max_iterations();

        for iteration_index in 0..max_iterations {
            let iteration = (iteration_index + 1, max_iterations);
            let changed = diagnose_pipeline(
                program,
                tree,
                strings_pool,
                ctx,
                repeat.inner(),
                baseline_output,
                max_instruction_limit,
                depth + 1,
                Some(iteration),
            )?;
            any_changed |= changed;

            if !changed {
                break;
            }
        }

        return Ok(any_changed);
    }

    // handle function pipelines
    if let Some(function_pipeline) = pipeline.as_any().downcast_ref::<FunctionPipeline>() {
        let mut any_changed = false;

        for pass in function_pipeline.passes() {
            let changed = run_function_pass(pass.as_ref(), tree, ctx);
            any_changed |= changed;

            let context = format_pass_context(depth, iteration, pass.name());
            run_and_compare_output(
                program,
                tree.clone(),
                strings_pool.clone().into_immutable(),
                baseline_output,
                max_instruction_limit,
                &context,
            )?;
        }

        return Ok(any_changed);
    }

    // handle function adaptor pipelines
    if let Some(adaptor) = pipeline.as_any().downcast_ref::<FunctionToModuleAdaptor>() {
        return diagnose_pipeline(
            program,
            tree,
            strings_pool,
            ctx,
            adaptor.inner(),
            baseline_output,
            max_instruction_limit,
            depth,
            iteration,
        );
    }

    // handle module pipelines
    if let Some(module_pipeline) = pipeline.as_any().downcast_ref::<ModulePipeline>() {
        let mut any_changed = false;

        for pass in module_pipeline.passes() {
            let changed = run_module_pass(pass.as_ref(), tree, ctx);
            any_changed |= changed;

            let context = format_pass_context(depth, iteration, pass.name());
            run_and_compare_output(
                program,
                tree.clone(),
                strings_pool.clone().into_immutable(),
                baseline_output,
                max_instruction_limit,
                &context,
            )?;
        }

        return Ok(any_changed);
    }

    Err(format!(
        "bench '{}' encountered unsupported pipeline '{}'",
        program.name,
        pipeline.name()
    ))
}

/// Run a coroutine program with a resume handler.
fn run_coroutine(
    isolate: &mut Isolate,
    entry_id: mir::LocalNodeId<mir::Function>,
    args: &[Value],
    resume_value: fn(args: &[Value], yield_index: usize, yielded: Value) -> Value,
) -> RuntimeResult<ExecutionOutput> {
    // start execution
    let mut outcome = isolate.run_function_yielding(entry_id, args)?;
    let mut yield_index = 0usize;

    // continue until completion
    loop {
        match outcome {
            // return on completion
            ExecutionOutcome::Completed { output } => return Ok(output),
            // resume after suspension
            ExecutionOutcome::Yielded { yielded } => {
                let resume = resume_value(args, yield_index, yielded.value);
                yield_index += 1;
                outcome = isolate.resume(yielded.continuation, resume)?;
            }
        }
    }
}

/// Execute a program using an already optimized tree.
fn run_program_with_tree_result(
    program: &program::Program,
    tree: mir::NodeTree,
    strings: ImmutableStringPool,
    max_instruction_limit: Option<u64>,
) -> RuntimeResult<ExecutionOutput> {
    // configure an isolate like the bench harness
    let mut options = IsolateOptions::unbounded();
    options.limits.max_stack_depth = 4096;
    options.limits.max_heap_cells = 5_000_000;
    options.limits.max_raw_cells = 5_000_000;

    // apply instruction limits when requested
    if let Some(limit) = max_instruction_limit {
        options.limits.max_instructions = Some(limit);
    }

    // build isolate and arguments
    let mut isolate = Isolate::with_options(tree, strings, options)?;
    let args = program.args_for_profile(&isolate, program::BenchProfileKind::Quick);

    // execute using the requested runner
    run_program_with_isolate(program, &mut isolate, &args)
}

/// Execute a program using default arguments.
fn run_program_with_tree_result_default_args(
    program: &program::Program,
    tree: mir::NodeTree,
    strings: ImmutableStringPool,
    max_instruction_limit: Option<u64>,
) -> RuntimeResult<ExecutionOutput> {
    // configure an isolate like the bench harness
    let mut options = IsolateOptions::unbounded();
    options.limits.max_stack_depth = 4096;
    options.limits.max_heap_cells = 5_000_000;
    options.limits.max_raw_cells = 5_000_000;

    // apply instruction limits when requested
    if let Some(limit) = max_instruction_limit {
        options.limits.max_instructions = Some(limit);
    }

    // build isolate and arguments
    let mut isolate = Isolate::with_options(tree, strings, options)?;
    let args = (program.default_args)(&isolate);

    // execute using the requested runner
    run_program_with_isolate(program, &mut isolate, &args)
}

/// Execute a program using an isolate and explicit arguments.
fn run_program_with_isolate(
    program: &program::Program,
    isolate: &mut Isolate,
    args: &[Value],
) -> RuntimeResult<ExecutionOutput> {
    // resolve entry id
    let entry_id = isolate
        .function_id_by_name(program.entry)
        .unwrap_or_else(|_| panic!("function '{}' not found", program.entry));

    // execute based on runner configuration
    match program.runner {
        program::ProgramRunner::Function => isolate.run_function(entry_id, args),
        program::ProgramRunner::Coroutine { resume_value } => {
            run_coroutine(isolate, entry_id, args, resume_value)
        }
    }
}

/// Build a detailed failure message for execute mismatches.
fn build_execute_failure(
    program: &program::Program,
    level: OptimizationLevel,
    target_id: &TargetId,
    options: &PipelineOptions,
    baseline_output: &ExecutionOutput,
    run_options: &OptimizeRunOptions,
    message: String,
) -> String {
    // run individual pass cases in matrix mode
    if run_options.matrix {
        let pipeline = default_pipeline(level);
        let mut path = Vec::new();
        if let Some(reason) = run_matrix_pipeline(
            program,
            target_id,
            options,
            baseline_output,
            run_options.max_instruction_limit,
            &pipeline,
            &mut path,
        ) {
            return reason;
        }
    }

    // run prefix diagnostics when enabled or requested by the matrix
    if (run_options.diagnostic || run_options.matrix)
        && let Some(reason) = diagnose_mismatch(
            program,
            level,
            target_id,
            options,
            baseline_output,
            run_options.max_instruction_limit,
        )
    {
        return reason;
    }

    message
}

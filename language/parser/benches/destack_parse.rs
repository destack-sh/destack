use criterion::profiler::Profiler;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use destack_core::StringPool;
use destack_parser::{Parser, ParserOptions, ParserTriviaMode};
use destack_source::{File, FileId, FileType, LanguageType, Uri, glob};
use pprof::ProfilerGuard;
use pprof::flamegraph::Options as FlamegraphOptions;
use rayon::prelude::*;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{env, fs};

const GENERATED_APP_FILE_COUNT: usize = 64;
const GENERATED_APP_ITEM_COUNT: usize = 192;
const GENERATED_TYPE_FILE_COUNT: usize = 32;
const GENERATED_TYPE_ITEM_COUNT: usize = 384;
const GENERATED_RECOVERY_FILE_COUNT: usize = 16;
const GENERATED_RECOVERY_ITEM_COUNT: usize = 256;

/// One parser benchmark corpus.
#[derive(Debug)]
struct ParserBenchCorpus {
    /// The benchmark corpus name.
    name: String,
    /// The source files in this corpus.
    files: Vec<Arc<File>>,
    /// Total corpus source bytes.
    bytes: u64,
    /// Total corpus source lines.
    lines: u64,
}

/// One parser benchmark stage.
#[derive(Debug, Copy, Clone)]
enum ParserBenchStage {
    /// Lex the file into token buffers without parsing.
    Lex,
    /// Parse the file and retain raw comments without attaching them.
    ParseWithoutAttach,
    /// Parse the file through the full parser pipeline.
    Parse,
}

impl ParserBenchStage {
    /// Return the benchmark name for this stage.
    fn name(self) -> &'static str {
        match self {
            Self::Lex => "lex",
            Self::ParseWithoutAttach => "parse_without_attach",
            Self::Parse => "parse",
        }
    }
}

/// One parser stage summary from a dry run.
#[derive(Debug, Copy, Clone, Default)]
struct ParserBenchStageStats {
    /// The semantic token count.
    tokens: u64,
    /// The retained side token count.
    side_tokens: u64,
    /// The parsed DIR node count.
    nodes: u64,
    /// The retained comment count.
    comments: u64,
    /// The parser diagnostic count.
    errors: u64,
}

impl ParserBenchStageStats {
    /// Accumulate one summary into this summary.
    fn add(&mut self, other: Self) {
        self.tokens = self.tokens.saturating_add(other.tokens);
        self.side_tokens = self.side_tokens.saturating_add(other.side_tokens);
        self.nodes = self.nodes.saturating_add(other.nodes);
        self.comments = self.comments.saturating_add(other.comments);
        self.errors = self.errors.saturating_add(other.errors);
    }
}

/// Return whether a file type is supported by the parser bench.
fn is_parser_source_file_type(file_type: FileType) -> bool {
    matches!(
        file_type,
        FileType::Destack
            | FileType::DestackDeclaration
            | FileType::JavaScript
            | FileType::JavaScriptXml
            | FileType::TypeScript
            | FileType::TypeScriptXml
            | FileType::TypeScriptDeclaration
    )
}

/// Return source extensions covered by corpus collection.
fn parser_source_extensions() -> &'static [&'static str] {
    &["ds", "d.ds", "ts", "tsx", "d.ts"]
}

/// Return the parser bench worker count.
fn parser_bench_worker_count() -> usize {
    let physical_cores = num_cpus::get_physical();
    physical_cores.max(1)
}

/// Create one parser for a source file.
fn lex_parser(file: Arc<File>, trivia_mode: ParserTriviaMode) -> Parser {
    let language_type = LanguageType::try_from(file.ty).expect("file type has no parser language");
    Parser::lex_file_with_options(
        file,
        language_type,
        ParserOptions {
            trivia_mode,
            ..ParserOptions::default()
        },
        Arc::new(StringPool::new()),
    )
}

/// Parse one file through the full parser pipeline.
fn parse_file(file: Arc<File>, trivia_mode: ParserTriviaMode) -> Parser {
    let mut parser = lex_parser(file, trivia_mode);
    if trivia_mode.keeps_comments() {
        parser.parse();
    } else {
        parser.parse_without_attaching_comments();
    }

    parser
}

/// Run one parser benchmark stage for one file.
fn run_parser_stage(file: Arc<File>, stage: ParserBenchStage, trivia_mode: ParserTriviaMode) {
    match stage {
        ParserBenchStage::Lex => {
            let mut parser = lex_parser(file, trivia_mode);
            let tokens = parser.take_tokens();
            black_box(tokens);
        }
        ParserBenchStage::ParseWithoutAttach => {
            let mut parser = lex_parser(file, trivia_mode);
            let roots = parser.parse_without_attaching_comments();
            black_box((parser, roots));
        }
        ParserBenchStage::Parse => {
            let mut parser = lex_parser(file, trivia_mode);
            let roots = if trivia_mode.keeps_comments() {
                parser.parse()
            } else {
                parser.parse_without_attaching_comments()
            };
            black_box((parser, roots));
        }
    }
}

/// Summarize one parser benchmark stage for one file.
fn summarize_parser_stage(
    file: Arc<File>,
    stage: ParserBenchStage,
    trivia_mode: ParserTriviaMode,
) -> ParserBenchStageStats {
    let mut parser = lex_parser(file, trivia_mode);

    match stage {
        ParserBenchStage::Lex => {
            let (tokens, side_tokens) = parser.take_tokens();

            ParserBenchStageStats {
                tokens: tokens.len() as u64,
                side_tokens: side_tokens.len() as u64,
                ..ParserBenchStageStats::default()
            }
        }
        ParserBenchStage::ParseWithoutAttach => {
            parser.parse_without_attaching_comments();
            let (tokens, side_tokens) = parser.take_tokens();

            ParserBenchStageStats {
                tokens: tokens.len() as u64,
                side_tokens: side_tokens.len() as u64,
                nodes: parser.tree.node_count() as u64,
                comments: parser.tree.comments().len() as u64,
                errors: parser.errors.len() as u64,
            }
        }
        ParserBenchStage::Parse => {
            if trivia_mode.keeps_comments() {
                parser.parse();
            } else {
                parser.parse_without_attaching_comments();
            }
            let (tokens, side_tokens) = parser.take_tokens();

            ParserBenchStageStats {
                tokens: tokens.len() as u64,
                side_tokens: side_tokens.len() as u64,
                nodes: parser.tree.node_count() as u64,
                comments: parser.tree.comments().len() as u64,
                errors: parser.errors.len() as u64,
            }
        }
    }
}

/// Summarize one parser benchmark stage for one corpus.
fn summarize_parser_corpus(
    corpus: &ParserBenchCorpus,
    stage: ParserBenchStage,
    trivia_mode: ParserTriviaMode,
) -> ParserBenchStageStats {
    let mut stats = ParserBenchStageStats::default();

    for file in &corpus.files {
        stats.add(summarize_parser_stage(file.clone(), stage, trivia_mode));
    }

    stats
}

/// Return the benchmark name for a parser trivia mode.
fn parser_trivia_mode_name(trivia_mode: ParserTriviaMode) -> &'static str {
    match trivia_mode {
        ParserTriviaMode::Ignore => "ignore",
        ParserTriviaMode::Documentation => "documentation",
        ParserTriviaMode::Full => "full",
    }
}

/// Return the parser trivia mode for benchmarks.
fn parser_trivia_mode() -> ParserTriviaMode {
    let Some(value) = env::var("DESTACK_PARSE_TRIVIA")
        .ok()
        .or_else(|| env::var("DESTACK_PARSE_RETAIN_TRIVIA").ok())
    else {
        return ParserTriviaMode::Documentation;
    };

    match value.as_str() {
        "0" | "false" | "False" | "FALSE" | "ignore" => ParserTriviaMode::Ignore,
        "doc" | "docs" | "documentation" => ParserTriviaMode::Documentation,
        "1" | "true" | "True" | "TRUE" | "full" | "trivia" => ParserTriviaMode::Full,
        _ => panic!("invalid parser trivia mode: {value}"),
    }
}

/// Return the parser benchmark stage from the environment.
fn parser_bench_stage() -> ParserBenchStage {
    let Some(value) = env::var("DESTACK_PARSE_STAGE").ok() else {
        return ParserBenchStage::Parse;
    };

    match value.as_str() {
        "lex" | "lexer" => ParserBenchStage::Lex,
        "parse-no-attach" | "parse_no_attach" | "parse_without_attach" => {
            ParserBenchStage::ParseWithoutAttach
        }
        "parse" | "full" => ParserBenchStage::Parse,
        _ => panic!("invalid parser benchmark stage: {value}"),
    }
}

/// Read an optional comma separated file list from the environment.
fn parser_files_from_env() -> Option<Vec<PathBuf>> {
    let files_env = env::var("DESTACK_PARSE_FILES").ok()?;
    let files: Vec<PathBuf> = files_env
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .collect();
    if files.is_empty() { None } else { Some(files) }
}

/// Return the parser corpus filter from the environment.
fn parser_corpus_filter() -> Option<Vec<String>> {
    let corpus_env = env::var("DESTACK_PARSE_CORPUS").ok()?;
    let filters = corpus_env
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    if filters.is_empty() {
        None
    } else {
        Some(filters)
    }
}

/// Return whether a corpus should run.
fn should_run_corpus(name: &str, filter: Option<&[String]>) -> bool {
    let Some(filter) = filter else {
        return true;
    };

    filter.iter().any(|item| item == name)
}

/// Return whether a path has a component with the given name.
fn path_has_component(path: &Path, component: &str) -> bool {
    path.components().any(|part| part.as_os_str() == component)
}

/// Collect parser source files under one root.
fn collect_parser_sources(
    root: &Path,
    include_stress: bool,
    include_conformance: bool,
) -> Vec<PathBuf> {
    let root = root.to_string_lossy();
    let mut source_files = Vec::new();

    for extension in parser_source_extensions() {
        source_files.extend(glob(&format!("{root}/**/*.{extension}")));
    }

    source_files.retain(|path| {
        if !include_stress && path_has_component(path, "stress") {
            return false;
        }

        if !include_conformance && path_has_component(path, "conformance") {
            return false;
        }

        true
    });
    source_files.sort();
    source_files.dedup();

    source_files
}

/// Load one file backed parser corpus.
fn load_file_corpus(name: impl Into<String>, paths: &[PathBuf]) -> Option<ParserBenchCorpus> {
    let name = name.into();
    let mut lines = 0_u64;
    let mut bytes = 0_u64;
    let mut files = Vec::with_capacity(paths.len());

    for path in paths {
        let file_type = FileType::from_path_or_unknown(path);
        let is_parser_source = is_parser_source_file_type(file_type);
        assert!(is_parser_source, "path is not a parser source: {path:?}");

        let content = fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("file system error while reading {}", path.display()));
        lines = lines.saturating_add(content.lines().count() as u64);
        bytes = bytes.saturating_add(content.len() as u64);

        let file_id = FileId::new(files.len() as u128);
        let (file_name, uri) = Uri::from_path_with_name(path);
        let file = File::from_text(file_id, file_name, uri, None, file_type, content);
        files.push(Arc::new(file));
    }

    if files.is_empty() {
        None
    } else {
        Some(ParserBenchCorpus {
            name,
            files,
            bytes,
            lines,
        })
    }
}

/// Build one generated parser corpus.
fn generated_corpus(
    name: &str,
    file_type: FileType,
    extension: &str,
    file_count: usize,
    generate: impl Fn(usize) -> String,
) -> ParserBenchCorpus {
    let mut lines = 0_u64;
    let mut bytes = 0_u64;
    let mut files = Vec::with_capacity(file_count);

    for index in 0..file_count {
        let content = generate(index);
        lines = lines.saturating_add(content.lines().count() as u64);
        bytes = bytes.saturating_add(content.len() as u64);

        let file_name = format!("{name}_{index}.{extension}");
        let file = File::from_text(
            FileId::new(index as u128),
            file_name.clone(),
            Uri::from_string(&file_name),
            None,
            file_type,
            content,
        );
        files.push(Arc::new(file));
    }

    ParserBenchCorpus {
        name: name.to_string(),
        files,
        bytes,
        lines,
    }
}

/// Generate one app shaped parser source file.
fn generate_app_source(module_index: usize, item_count: usize, include_tree: bool) -> String {
    let mut source = String::with_capacity(item_count * 420);
    source.push_str("import { sharedValue } from \"./shared\";\n");
    source.push_str("export type Maybe<T> = T | null | undefined;\n");

    for index in 0..item_count {
        source.push_str(&format!(
            "/** Model shape for generated app item {module_index}_{index}. */\n"
        ));
        source.push_str(&format!(
            "export type Model{module_index}_{index}<T> = {{ readonly id: string; readonly value: T; readonly next?: Maybe<Model{module_index}_{index}<T>>; readonly items: readonly T[] }};\n"
        ));
        source.push_str(&format!(
            "// ordinary service comment {module_index}_{index}\n"
        ));
        source.push_str(&format!(
            "export interface Service{module_index}_{index}<T> {{ load(value: T): Promise<Model{module_index}_{index}<T>>; save(model: Model{module_index}_{index}<T>): void; }}\n"
        ));
        source.push_str(&format!(
            "export const value{module_index}_{index} = {{ id: \"{module_index}_{index}\", value: sharedValue + {index}, items: [sharedValue, {index}] }};\n"
        ));
        source.push_str(&format!(
            "/// Compute generated app item {module_index}_{index}.\n"
        ));
        source.push_str(&format!(
            "export function compute{module_index}_{index}<T extends {{ readonly score: number }}>(items: readonly T[], fallback: T): Model{module_index}_{index}<T> {{ const selected = items.find((item) => item.score > {index}) ?? fallback; return {{ id: \"{module_index}_{index}\", value: selected, items: [...items, fallback] }}; }}\n"
        ));

        if include_tree && index % 4 == 0 {
            source.push_str(&format!(
                "/* ordinary tree comment {module_index}_{index} */\n"
            ));
            source.push_str(&format!(
                "export const View{module_index}_{index} = <Panel key={{value{module_index}_{index}.id}}><Item value={{value{module_index}_{index}.value}} /></Panel>;\n"
            ));
        }
    }

    source
}

/// Generate one type heavy parser source file.
fn generate_type_source(module_index: usize, item_count: usize) -> String {
    let mut source = String::with_capacity(item_count * 260);

    for index in 0..item_count {
        source.push_str(&format!(
            "/** Conditional type case {module_index}_{index}. */\n"
        ));
        source.push_str(&format!(
            "export type TypeCase{module_index}_{index}<T> = T extends {{ readonly tag: \"case{index}\" }} ? {{ readonly value: T; readonly next: TypeCase{module_index}_{index}<readonly T[]> }} : {{ readonly fallback: keyof T }};\n"
        ));
        source.push_str(&format!(
            "// ordinary mapped type comment {module_index}_{index}\n"
        ));
        source.push_str(&format!(
            "export type MappedCase{module_index}_{index}<T> = {{ readonly [Key in keyof T as `case${{Key & string}}`]: T[Key] }};\n"
        ));
    }

    source
}

/// Generate one damaged parser recovery source file.
fn generate_recovery_source(module_index: usize, item_count: usize) -> String {
    let mut source = String::with_capacity(item_count * 180);

    for index in 0..item_count {
        source.push_str(&format!(
            "/** Broken call recovery case {module_index}_{index}. */\n"
        ));
        source.push_str(&format!(
            "export const brokenCall{module_index}_{index} = invoke{index}(alpha{index}, , beta{index});\n"
        ));
        source.push_str(&format!(
            "// ordinary recovered call comment {module_index}_{index}\n"
        ));
        source.push_str(&format!(
            "export const recoveredCall{module_index}_{index} = invoke{index}(alpha{index}, beta{index});\n"
        ));
        source.push_str(&format!(
            "export type BrokenType{module_index}_{index}<T> = T extends ;\n"
        ));
        source.push_str(&format!(
            "export type RecoveredType{module_index}_{index}<T> = T | null;\n"
        ));
    }

    source
}

/// Build the default parser benchmark corpora.
fn collect_default_corpora(workspace_root: &Path) -> Vec<ParserBenchCorpus> {
    let filter = parser_corpus_filter();
    let filter = filter.as_deref();
    let mut corpora = Vec::new();

    if should_run_corpus("library", filter) {
        let paths = collect_parser_sources(&workspace_root.join("language/library"), false, false);
        if let Some(corpus) = load_file_corpus("library", &paths) {
            corpora.push(corpus);
        }
    }

    if should_run_corpus("fixtures", filter) {
        let paths =
            collect_parser_sources(&workspace_root.join("language/test/fixtures"), false, false);
        if let Some(corpus) = load_file_corpus("fixtures", &paths) {
            corpora.push(corpus);
        }
    }

    if should_run_corpus("generated_app_ds", filter) {
        corpora.push(generated_corpus(
            "generated_app_ds",
            FileType::Destack,
            "ds",
            GENERATED_APP_FILE_COUNT,
            |index| generate_app_source(index, GENERATED_APP_ITEM_COUNT, false),
        ));
    }

    if should_run_corpus("generated_app_tsx", filter) {
        corpora.push(generated_corpus(
            "generated_app_tsx",
            FileType::TypeScriptXml,
            "tsx",
            GENERATED_APP_FILE_COUNT,
            |index| generate_app_source(index, GENERATED_APP_ITEM_COUNT, true),
        ));
    }

    if should_run_corpus("generated_types_dts", filter) {
        corpora.push(generated_corpus(
            "generated_types_dts",
            FileType::TypeScriptDeclaration,
            "d.ts",
            GENERATED_TYPE_FILE_COUNT,
            |index| generate_type_source(index, GENERATED_TYPE_ITEM_COUNT),
        ));
    }

    if should_run_corpus("generated_recovery_ts", filter) {
        corpora.push(generated_corpus(
            "generated_recovery_ts",
            FileType::TypeScript,
            "ts",
            GENERATED_RECOVERY_FILE_COUNT,
            |index| generate_recovery_source(index, GENERATED_RECOVERY_ITEM_COUNT),
        ));
    }

    corpora
}

/// Pprof profiler for Criterion benches.
struct PprofProfiler {
    /// The sampling frequency in hertz.
    frequency: i32,
    /// The active profiler guard.
    active_profiler: Option<ProfilerGuard<'static>>,
}

impl PprofProfiler {
    /// Create a new profiler with the given frequency.
    fn new(frequency: i32) -> Self {
        Self {
            frequency,
            active_profiler: None,
        }
    }
}

impl Profiler for PprofProfiler {
    /// Start a profiling session.
    fn start_profiling(&mut self, _benchmark_id: &str, _benchmark_dir: &Path) {
        self.active_profiler = Some(ProfilerGuard::new(self.frequency).unwrap());
    }

    /// Stop profiling and write the flamegraph.
    fn stop_profiling(&mut self, _benchmark_id: &str, benchmark_dir: &Path) {
        // ensure the output directory exists
        fs::create_dir_all(benchmark_dir).unwrap();

        // open the flamegraph output file
        let output_path = benchmark_dir.join("flamegraph.svg");
        let output_file = fs::File::create(&output_path).unwrap_or_else(|_| {
            panic!("file system error while creating {}", output_path.display())
        });

        // build and write the flamegraph
        if let Some(profiler) = self.active_profiler.take() {
            let mut options = FlamegraphOptions::default();
            profiler
                .report()
                .build()
                .unwrap()
                .flamegraph_with_options(output_file, &mut options)
                .expect("error while writing flamegraph");
        }
    }
}

/// Benchmark parsing for workspace sources.
fn bench_parse(criterion: &mut Criterion) {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root_path = manifest_dir
        .ancestors()
        .find(|p| p.join("VERSION.txt").exists())
        .unwrap_or(&manifest_dir)
        .to_path_buf();
    let corpora = match parser_files_from_env() {
        Some(paths) => load_file_corpus("custom", &paths).into_iter().collect(),
        None => collect_default_corpora(&workspace_root_path),
    };
    assert!(
        !corpora.is_empty(),
        "no parser benchmark corpora selected, check DESTACK_PARSE_CORPUS"
    );

    let mut group = criterion.benchmark_group("destack_parser");
    let trivia_mode = parser_trivia_mode();
    let trivia_mode_name = parser_trivia_mode_name(trivia_mode);
    let stage = parser_bench_stage();
    let stage_name = stage.name();

    for corpus in &corpora {
        let stats = summarize_parser_corpus(corpus, stage, trivia_mode);
        eprintln!(
            "parser corpus {}: {} stage, {} trivia, {} files, {} bytes, {} lines, {} tokens, {} side tokens, {} nodes, {} comments, {} errors",
            corpus.name,
            stage_name,
            trivia_mode_name,
            corpus.files.len(),
            corpus.bytes,
            corpus.lines,
            stats.tokens,
            stats.side_tokens,
            stats.nodes,
            stats.comments,
            stats.errors
        );
        group.throughput(Throughput::Bytes(corpus.bytes));
        group.bench_with_input(
            BenchmarkId::new(
                format!("{stage_name}_{trivia_mode_name}"),
                corpus.name.as_str(),
            ),
            corpus,
            |bencher, corpus| {
                bencher.iter(|| {
                    for file in corpus.files.iter() {
                        run_parser_stage(file.clone(), stage, trivia_mode);
                    }
                });
            },
        );
    }

    group.finish();
}

/// Resolve a single-file path for targeted benchmarks.
fn resolve_single_file_path(workspace_root: &Path) -> PathBuf {
    // use env override if provided
    if let Ok(path) = env::var("DESTACK_PARSE_FILE") {
        return PathBuf::from(path);
    }

    // fall back to a representative library file
    workspace_root.join("language/library/src/fs/binding/file.ds")
}

/// Load a single source file for benchmarking.
fn load_single_file(path: &Path) -> (Arc<File>, u64) {
    // file type
    let file_type = FileType::from_path_or_unknown(path);
    let is_parser_source = is_parser_source_file_type(file_type);
    assert!(is_parser_source, "path is not a parser source: {path:?}");

    // file content
    let content = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("file system error while reading {}", path.display()));
    let line_count = content.lines().count() as u64;

    // register file
    let file_id = FileId::new(0);
    let (file_name, uri) = Uri::from_path_with_name(path);
    let file = File::from_text(file_id, file_name, uri, None, file_type, content);

    (Arc::new(file), line_count)
}

/// Benchmark parsing for a single source file.
fn bench_parse_single(criterion: &mut Criterion) {
    // workspace root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root_path = manifest_dir
        .ancestors()
        .find(|p| p.join("VERSION.txt").exists())
        .unwrap_or(&manifest_dir)
        .to_path_buf();

    // load file
    let source_path = resolve_single_file_path(&workspace_root_path);
    let (file, total_lines) = load_single_file(&source_path);

    // benchmark
    let mut group = criterion.benchmark_group("destack_parser_single");
    group.throughput(Throughput::Elements(total_lines));
    let worker_count = parser_bench_worker_count();
    let trivia_mode = parser_trivia_mode();
    let trivia_mode_name = parser_trivia_mode_name(trivia_mode);

    // oxc style: single thread parse
    group.bench_with_input(
        BenchmarkId::new(format!("parse_{trivia_mode_name}"), "single-thread"),
        &file,
        |bencher, file| {
            bencher.iter(|| {
                // parse full pipeline
                let parser = parse_file(file.clone(), trivia_mode);
                black_box(parser);
            });
        },
    );

    // oxc style: no drop parse
    group.bench_with_input(
        BenchmarkId::new(format!("parse_{trivia_mode_name}"), "no-drop"),
        &file,
        |bencher, file| {
            bencher.iter_with_large_drop(|| parse_file(file.clone(), trivia_mode));
        },
    );

    // oxc style: parallel parse throughput
    group.bench_with_input(
        BenchmarkId::new(format!("parse_{trivia_mode_name}"), "parallel"),
        &file,
        |bencher, file| {
            bencher.iter(|| {
                (0..worker_count).into_par_iter().for_each(|_| {
                    let parser = parse_file(file.clone(), trivia_mode);
                    black_box(parser);
                });
            });
        },
    );

    // keep compatibility alias for existing profile commands
    group.bench_with_input(
        BenchmarkId::new("parse", "single"),
        &file,
        |bencher, file| {
            bencher.iter(|| {
                let parser = parse_file(file.clone(), trivia_mode);
                black_box(parser);
            });
        },
    );

    // parse main alias: kept for historical benchmark compatibility
    group.bench_with_input(
        BenchmarkId::new("main", "single-thread"),
        &file,
        |bencher, file| {
            bencher.iter(|| {
                let parser = parse_file(file.clone(), trivia_mode);
                black_box(parser);
            });
        },
    );

    // parse main alias: no drop
    group.bench_with_input(
        BenchmarkId::new("main", "no-drop"),
        &file,
        |bencher, file| {
            bencher.iter_with_large_drop(|| parse_file(file.clone(), trivia_mode));
        },
    );

    // parse main alias: parallel throughput
    group.bench_with_input(
        BenchmarkId::new("main", "parallel"),
        &file,
        |bencher, file| {
            bencher.iter(|| {
                (0..worker_count).into_par_iter().for_each(|_| {
                    let parser = parse_file(file.clone(), trivia_mode);
                    black_box(parser);
                });
            });
        },
    );

    group.finish();
}

/// Configure Criterion with pprof.
fn profiler() -> Criterion {
    // allow a higher sample rate for deeper flamegraphs
    let sample_rate = env::var("DESTACK_PPROF_HZ")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(100);

    Criterion::default().with_profiler(PprofProfiler::new(sample_rate))
}

criterion_group! {
    name = benches;
    config = profiler();
    targets = bench_parse, bench_parse_single
}
criterion_main!(benches);

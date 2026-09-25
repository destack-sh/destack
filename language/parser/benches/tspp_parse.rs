use criterion::profiler::Profiler;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use pprof::ProfilerGuard;
use pprof::flamegraph::Options as FlamegraphOptions;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{env, fs};
use tspp_dir::{Expression, Tree, TypeExpression};
use tspp_parser::{CommentRetention, Parse, ParseOptions, Parser};
use tspp_source::{File, FileId, FileType, LanguageType, ModuleId, PackageId, Uri, glob};

const GENERATED_APP_FILE_COUNT: usize = 64;
const GENERATED_APP_ITEM_COUNT: usize = 192;
const GENERATED_TYPE_FILE_COUNT: usize = 32;
const GENERATED_TYPE_ITEM_COUNT: usize = 384;
const GENERATED_RECOVERY_FILE_COUNT: usize = 16;
const GENERATED_RECOVERY_ITEM_COUNT: usize = 256;
const SOURCE_EXTENSIONS: &[&str] = &["tspp", "d.tspp"];

/// One parser benchmark corpus.
#[derive(Debug)]
struct Corpus {
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
enum Stage {
    /// Lex the file into token buffers without parsing.
    Lex,
    /// Parse root expressions without final parser completion.
    ParseRoots,
    /// Parse the file through the full parser pipeline.
    Parse,
}

impl Stage {
    /// Return the benchmark name for this stage.
    fn name(self) -> &'static str {
        match self {
            Self::Lex => "lex",
            Self::ParseRoots => "parse_roots",
            Self::Parse => "parse",
        }
    }
}

/// One parser stage summary from a dry run.
#[derive(Debug, Clone, Default)]
struct Stats {
    /// The semantic token count.
    tokens: u64,
    /// The parsed DIR node count.
    nodes: u64,
    /// The parsed expression count.
    expressions: u64,
    /// The parsed type expression count.
    type_expressions: u64,
    /// The retained comment count.
    comments: u64,
    /// The parser diagnostic count.
    errors: u64,
    /// The source-local unique string count.
    strings: u64,
    /// The retained bytes of source-local string storage.
    string_bytes: u64,
}

impl Stats {
    /// Summarize one parser after a benchmark stage.
    fn from_parser(parser: &Parser, token_count: usize) -> Self {
        Self {
            tokens: token_count as u64,
            nodes: parser.tree.node_count() as u64,
            expressions: parser.tree.iter_nodes::<Expression>().count() as u64,
            type_expressions: parser.tree.iter_nodes::<TypeExpression>().count() as u64,
            comments: parser.comments().len() as u64,
            errors: parser.errors.len() as u64,
            strings: parser.strings.len() as u64,
            string_bytes: parser.strings.owned_bytes() as u64,
        }
    }

    /// Summarize one completed parse.
    fn from_parse(parse: &Parse) -> Self {
        Self {
            tokens: parse.tokens.len() as u64,
            nodes: parse.tree.node_count() as u64,
            expressions: parse.tree.iter_nodes::<Expression>().count() as u64,
            type_expressions: parse.tree.iter_nodes::<TypeExpression>().count() as u64,
            comments: parse.comments.len() as u64,
            errors: parse.errors.len() as u64,
            strings: parse.strings.len() as u64,
            string_bytes: parse.strings.owned_bytes() as u64,
        }
    }

    /// Accumulate one summary into this summary.
    fn add(&mut self, other: Self) {
        self.tokens += other.tokens;
        self.nodes += other.nodes;
        self.expressions += other.expressions;
        self.type_expressions += other.type_expressions;
        self.comments += other.comments;
        self.errors += other.errors;
        self.strings += other.strings;
        self.string_bytes += other.string_bytes;
    }
}

/// Return whether a file type is supported by the parser bench.
fn is_source_file_type(file_type: FileType) -> bool {
    matches!(file_type, FileType::Tspp | FileType::TsppDeclaration)
}

/// Return the parser bench worker count.
fn worker_count() -> usize {
    let physical_cores = num_cpus::get_physical();
    physical_cores.max(1)
}

/// Create one parser for a source file.
fn prepare_parser(file: Arc<File>, comment_retention: CommentRetention) -> Parser {
    let language_type = LanguageType::try_from(file.ty).expect("file type has no parser language");
    let module_id = ModuleId::new(PackageId::new(0), file.id.0);

    Parser::new(
        file,
        language_type,
        Tree::new(module_id),
        ParseOptions {
            comment_retention,
            ..ParseOptions::default()
        },
    )
}

/// Parse one file through the full parser pipeline.
fn parse_file(file: Arc<File>, comment_retention: CommentRetention) -> Parse {
    let parser = prepare_parser(file, comment_retention);

    parser.parse()
}

/// Run one parser benchmark stage for one file.
fn run_stage(file: Arc<File>, stage: Stage, comment_retention: CommentRetention) {
    match stage {
        Stage::Lex => {
            let mut parser = prepare_parser(file, comment_retention);
            let tokens = parser.take_tokens();
            black_box(tokens);
        }
        Stage::ParseRoots => {
            let mut parser = prepare_parser(file, comment_retention);
            let roots = parser.parse_roots();
            black_box((parser, roots));
        }
        Stage::Parse => {
            let parser = prepare_parser(file, comment_retention);
            black_box(parser.parse());
        }
    }
}

/// Summarize one parser benchmark stage for one file.
fn summarize_stage(file: Arc<File>, stage: Stage, comment_retention: CommentRetention) -> Stats {
    let mut parser = prepare_parser(file, comment_retention);

    // run the selected stage
    match stage {
        Stage::Lex => {
            let tokens = parser.take_tokens();

            Stats::from_parser(&parser, tokens.len())
        }
        Stage::ParseRoots => {
            parser.parse_roots();

            let tokens = parser.take_tokens();
            Stats::from_parser(&parser, tokens.len())
        }
        Stage::Parse => {
            let parse = parser.parse();

            Stats::from_parse(&parse)
        }
    }
}

/// Summarize one parser benchmark stage for one corpus.
fn summarize_corpus(corpus: &Corpus, stage: Stage, comment_retention: CommentRetention) -> Stats {
    let mut stats = Stats::default();

    for file in &corpus.files {
        stats.add(summarize_stage(file.clone(), stage, comment_retention));
    }

    stats
}

/// Return the benchmark name for one comment retention mode.
fn comment_retention_name(comment_retention: CommentRetention) -> &'static str {
    match comment_retention {
        CommentRetention::Ignore => "ignore",
        CommentRetention::Documentation => "documentation",
        CommentRetention::All => "all",
    }
}

/// Return the comment retention mode for benchmarks.
fn comment_retention() -> CommentRetention {
    let Some(value) = env::var("TSPP_PARSE_COMMENTS").ok() else {
        return CommentRetention::Documentation;
    };

    match value.as_str() {
        "0" | "false" | "False" | "FALSE" | "ignore" => CommentRetention::Ignore,
        "doc" | "docs" | "documentation" => CommentRetention::Documentation,
        "1" | "true" | "True" | "TRUE" | "all" => CommentRetention::All,
        _ => panic!("invalid parser comment retention: {value}"),
    }
}

/// Return the parser benchmark stage from the environment.
fn benchmark_stage() -> Stage {
    let Some(value) = env::var("TSPP_PARSE_STAGE").ok() else {
        return Stage::Parse;
    };

    match value.as_str() {
        "lex" | "lexer" => Stage::Lex,
        "roots" | "parse_roots" => Stage::ParseRoots,
        "parse" | "full" => Stage::Parse,
        _ => panic!("invalid parser benchmark stage: {value}"),
    }
}

/// Read an optional comma separated file list from the environment.
fn configured_files() -> Option<Vec<PathBuf>> {
    let files_env = env::var("TSPP_PARSE_FILES").ok()?;
    let files: Vec<PathBuf> = files_env
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .collect();
    if files.is_empty() { None } else { Some(files) }
}

/// Return the parser corpus filter from the environment.
fn corpus_filter() -> Option<Vec<String>> {
    let corpus_env = env::var("TSPP_PARSE_CORPUS").ok()?;
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

/// Return whether the single-file benchmark group should run.
fn should_run_single_file_bench() -> bool {
    if env::var("TSPP_PARSE_FILE").is_ok() {
        return true;
    }

    if env::var("TSPP_PARSE_CORPUS").is_ok() || env::var("TSPP_PARSE_FILES").is_ok() {
        return false;
    }

    true
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
fn collect_parser_sources(root: &Path, include_stress: bool) -> Vec<PathBuf> {
    let root = root.to_string_lossy();
    let mut source_files = Vec::new();

    for extension in SOURCE_EXTENSIONS {
        source_files.extend(
            glob(&format!("{root}/**/*.{extension}")).expect("collect parser benchmark sources"),
        );
    }

    source_files.retain(|path| {
        if !include_stress && path_has_component(path, "stress") {
            return false;
        }

        true
    });
    source_files.sort();
    source_files.dedup();

    source_files
}

/// Load one file backed parser corpus.
fn load_file_corpus(name: impl Into<String>, paths: &[PathBuf]) -> Option<Corpus> {
    let name = name.into();
    let mut lines = 0_u64;
    let mut bytes = 0_u64;
    let mut files = Vec::with_capacity(paths.len());

    for path in paths {
        let file_type = FileType::from_path(path).expect("bench path should have a file type");
        let is_parser_source = is_source_file_type(file_type);
        assert!(is_parser_source, "path is not a parser source: {path:?}");

        let content = fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("file system error while reading {}", path.display()));
        lines = lines.saturating_add(content.lines().count() as u64);
        bytes = bytes.saturating_add(content.len() as u64);

        let file_id = FileId::new(files.len() as u64);
        let (file_name, uri) = Uri::from_path_with_name(path);
        let file = File::from_text(file_id, file_name, uri, None, file_type, content)
            .expect("benchmark source should load");
        files.push(Arc::new(file));
    }

    if files.is_empty() {
        None
    } else {
        Some(Corpus {
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
) -> Corpus {
    let mut lines = 0_u64;
    let mut bytes = 0_u64;
    let mut files = Vec::with_capacity(file_count);

    for index in 0..file_count {
        let content = generate(index);
        lines = lines.saturating_add(content.lines().count() as u64);
        bytes = bytes.saturating_add(content.len() as u64);

        let file_name = format!("{name}_{index}.{extension}");
        let file = File::from_text(
            FileId::new(index as u64),
            file_name.clone(),
            Uri::from_string(&file_name),
            None,
            file_type,
            content,
        )
        .expect("benchmark source should load");
        files.push(Arc::new(file));
    }

    Corpus {
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
            "export function compute{module_index}_{index}<T: {{ readonly score: number }}>(items: readonly T[], fallback: T): Model{module_index}_{index}<T> {{ const selected = items.find((item) => item.score > {index}) ?? fallback; return {{ id: \"{module_index}_{index}\", value: selected, items: [...items, fallback] }}; }}\n"
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
fn collect_default_corpora(workspace_root: &Path) -> Vec<Corpus> {
    let filter = corpus_filter();
    let filter = filter.as_deref();
    let mut corpora = Vec::new();

    if should_run_corpus("library", filter) {
        let paths = collect_parser_sources(&workspace_root.join("language/library"), false);
        if let Some(corpus) = load_file_corpus("library", &paths) {
            corpora.push(corpus);
        }
    }

    if should_run_corpus("generated_app_ds", filter) {
        corpora.push(generated_corpus(
            "generated_app_ds",
            FileType::Tspp,
            "tspp",
            GENERATED_APP_FILE_COUNT,
            |index| generate_app_source(index, GENERATED_APP_ITEM_COUNT, false),
        ));
    }

    if should_run_corpus("generated_app_tree", filter) {
        corpora.push(generated_corpus(
            "generated_app_tree",
            FileType::Tspp,
            "tspp",
            GENERATED_APP_FILE_COUNT,
            |index| generate_app_source(index, GENERATED_APP_ITEM_COUNT, true),
        ));
    }

    if should_run_corpus("generated_types_declaration", filter) {
        corpora.push(generated_corpus(
            "generated_types_declaration",
            FileType::TsppDeclaration,
            "d.tspp",
            GENERATED_TYPE_FILE_COUNT,
            |index| generate_type_source(index, GENERATED_TYPE_ITEM_COUNT),
        ));
    }

    if should_run_corpus("generated_recovery", filter) {
        corpora.push(generated_corpus(
            "generated_recovery",
            FileType::Tspp,
            "tspp",
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
        .find(|p| p.join("package.json").exists())
        .unwrap_or(&manifest_dir)
        .to_path_buf();
    let corpora = match configured_files() {
        Some(paths) => load_file_corpus("custom", &paths).into_iter().collect(),
        None => collect_default_corpora(&workspace_root_path),
    };
    assert!(
        !corpora.is_empty(),
        "no parser benchmark corpora selected, check TSPP_PARSE_CORPUS"
    );

    let mut group = criterion.benchmark_group("tspp_parser");
    let comment_retention = comment_retention();
    let comment_retention_name = comment_retention_name(comment_retention);
    let stage = benchmark_stage();
    let stage_name = stage.name();

    for corpus in &corpora {
        let stats = summarize_corpus(corpus, stage, comment_retention);
        eprintln!(
            "parser corpus {}: {} stage, {} comments, {} files, {} bytes, {} lines, {} tokens, {} nodes, {} expressions, {} type expressions, {} retained comments, {} strings, {} string bytes, {} errors",
            corpus.name,
            stage_name,
            comment_retention_name,
            corpus.files.len(),
            corpus.bytes,
            corpus.lines,
            stats.tokens,
            stats.nodes,
            stats.expressions,
            stats.type_expressions,
            stats.comments,
            stats.strings,
            stats.string_bytes,
            stats.errors
        );
        group.throughput(Throughput::Bytes(corpus.bytes));
        group.bench_with_input(
            BenchmarkId::new(
                format!("{stage_name}_{comment_retention_name}"),
                corpus.name.as_str(),
            ),
            corpus,
            |bencher, corpus| {
                bencher.iter(|| {
                    for file in &corpus.files {
                        run_stage(file.clone(), stage, comment_retention);
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
    if let Ok(path) = env::var("TSPP_PARSE_FILE") {
        return PathBuf::from(path);
    }

    // fall back to a representative library file
    workspace_root.join("language/library/src/fs/binding/file.tspp")
}

/// Load a single source file for benchmarking.
fn load_single_file(path: &Path) -> (Arc<File>, u64) {
    // file type
    let file_type = FileType::from_path(path).expect("bench path should have a file type");
    let is_parser_source = is_source_file_type(file_type);
    assert!(is_parser_source, "path is not a parser source: {path:?}");

    // file content
    let content = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("file system error while reading {}", path.display()));
    let line_count = content.lines().count() as u64;

    // register file
    let file_id = FileId::new(0);
    let (file_name, uri) = Uri::from_path_with_name(path);
    let file = File::from_text(file_id, file_name, uri, None, file_type, content)
        .expect("benchmark source should load");

    (Arc::new(file), line_count)
}

/// Benchmark parsing for a single source file.
fn bench_parse_single(criterion: &mut Criterion) {
    if !should_run_single_file_bench() {
        return;
    }

    // workspace root
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root_path = manifest_dir
        .ancestors()
        .find(|p| p.join("package.json").exists())
        .unwrap_or(&manifest_dir)
        .to_path_buf();

    // load file
    let source_path = resolve_single_file_path(&workspace_root_path);
    let (file, total_lines) = load_single_file(&source_path);

    // benchmark
    let mut group = criterion.benchmark_group("tspp_parser_single");
    group.throughput(Throughput::Elements(total_lines));
    let worker_count = worker_count();
    let comment_retention = comment_retention();
    let comment_retention_name = comment_retention_name(comment_retention);

    // oxc style: single thread parse
    group.bench_with_input(
        BenchmarkId::new(format!("parse_{comment_retention_name}"), "single-thread"),
        &file,
        |bencher, file| {
            bencher.iter(|| {
                // parse full pipeline
                let parse = parse_file(file.clone(), comment_retention);
                black_box(parse);
            });
        },
    );

    // oxc style: no drop parse
    group.bench_with_input(
        BenchmarkId::new(format!("parse_{comment_retention_name}"), "no-drop"),
        &file,
        |bencher, file| {
            bencher.iter_with_large_drop(|| parse_file(file.clone(), comment_retention));
        },
    );

    // oxc style: parallel parse throughput
    group.bench_with_input(
        BenchmarkId::new(format!("parse_{comment_retention_name}"), "parallel"),
        &file,
        |bencher, file| {
            bencher.iter(|| {
                (0..worker_count).into_par_iter().for_each(|_| {
                    let parse = parse_file(file.clone(), comment_retention);
                    black_box(parse);
                });
            });
        },
    );

    group.finish();
}

/// Configure Criterion and enable pprof when requested.
fn profiler() -> Criterion {
    let criterion = Criterion::default();
    if env::var_os("TSPP_PPROF").is_none() {
        return criterion;
    }

    // allow a higher sample rate for deeper flamegraphs
    let sample_rate = env::var("TSPP_PPROF_HZ")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(100);

    criterion.with_profiler(PprofProfiler::new(sample_rate))
}

criterion_group! {
    name = benches;
    config = profiler();
    targets = bench_parse, bench_parse_single
}
criterion_main!(benches);

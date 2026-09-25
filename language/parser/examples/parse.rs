use std::env;
use std::error::Error;
use std::fs::{self, File as FsFile};
use std::hint::black_box;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use pprof::ProfilerGuardBuilder;
use pprof::flamegraph::Options as FlamegraphOptions;
use tspp_dir::Tree;
use tspp_parser::{CommentRetention, Parse, ParseOptions, Parser};
use tspp_source::{File, FileId, FileType, LanguageType, ModuleId, PackageId, Uri};

const DEFAULT_SECONDS: u64 = 10;
const DEFAULT_SAMPLE_HZ: i32 = 997;
const DEFAULT_OUTPUT_PATH: &str = "language/parser/target/flamegraphs/parse.svg";

/// One parser pipeline stage measured by the profiler.
#[derive(Debug, Copy, Clone)]
enum ParserStage {
    /// Tokenize the source file into a parser cursor.
    Lex,
    /// Parse the complete source file.
    Parse,
}

impl ParserStage {
    /// Return one parser stage from an environment variable.
    fn from_env(name: &str, default: Self) -> Result<Self, Box<dyn Error>> {
        let Some(value) = env::var(name).ok() else {
            return Ok(default);
        };

        match value.as_str() {
            "lex" => Ok(Self::Lex),
            "parse" => Ok(Self::Parse),
            _ => Err(format!("invalid parser stage for {name}: {value}").into()),
        }
    }
}

/// Options for one parser profile run.
#[derive(Debug)]
struct ProfileOptions {
    /// The source file to parse repeatedly.
    source_path: PathBuf,
    /// The flamegraph output path.
    output_path: PathBuf,
    /// The sampling duration.
    duration: Duration,
    /// The sampling frequency in hertz.
    sample_hz: i32,
    /// The parser comment retention mode.
    comment_retention: CommentRetention,
    /// The parser pipeline stage to profile.
    stage: ParserStage,
}

impl ProfileOptions {
    /// Build run options from arguments and environment variables.
    fn from_env() -> Result<Self, Box<dyn Error>> {
        let source_path = source_path()?;
        let output_path = output_path();
        let seconds = number_from_env("TSPP_PARSE_SECONDS", DEFAULT_SECONDS)?;
        let sample_hz = number_from_env("TSPP_PARSE_HZ", DEFAULT_SAMPLE_HZ)?;
        let comment_retention =
            comment_retention_from_env("TSPP_PARSE_COMMENTS", CommentRetention::Documentation)?;
        let stage = ParserStage::from_env("TSPP_PARSE_STAGE", ParserStage::Parse)?;

        Ok(Self {
            source_path,
            output_path,
            duration: Duration::from_secs(seconds),
            sample_hz,
            comment_retention,
            stage,
        })
    }
}

/// Parse one file repeatedly under the pprof sampler.
fn main() -> Result<(), Box<dyn Error>> {
    let options = ProfileOptions::from_env()?;
    let file = load_file(&options.source_path)?;
    let language = file_language(file.ty)?;
    let comment_retention = options.comment_retention;
    let stage = options.stage;

    // prepare the sampler before measuring parser work
    let guard = ProfilerGuardBuilder::default()
        .frequency(options.sample_hz)
        .blocklist(&["libsystem", "libc", "libpthread", "libdyld"])
        .build()?;

    // sample only the selected parser stage
    let start = Instant::now();
    let mut runs = 0u64;
    while start.elapsed() < options.duration {
        match stage {
            ParserStage::Lex => {
                let module_id = ModuleId::new(PackageId::new(0), file.id.0);
                black_box(Parser::new(
                    file.clone(),
                    language,
                    Tree::new(module_id),
                    ParseOptions {
                        comment_retention,
                        ..ParseOptions::default()
                    },
                ));
            }
            ParserStage::Parse => {
                black_box(parse_file(file.clone(), language, comment_retention));
            }
        }
        runs += 1;
    }
    let run_elapsed = start.elapsed();

    // write artifacts while samples are still live
    let report = guard.report().build()?;
    write_flamegraph(&options.output_path, &report)?;
    write_folded_stacks(&folded_path(&options.output_path), &report)?;

    print_result(&file, stage, &options.output_path, run_elapsed, runs);

    Ok(())
}

/// Return the parser language for one file type.
fn file_language(file_type: FileType) -> Result<LanguageType, Box<dyn Error>> {
    LanguageType::try_from(file_type)
        .map_err(|file_type| format!("unsupported parser file type: {file_type:?}").into())
}

/// Load one source file.
fn load_file(path: &Path) -> Result<Arc<File>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let file_type = FileType::from_path(path)
        .ok_or_else(|| format!("unsupported parser file: {}", path.display()))?;
    let file_id = FileId::new(0);
    let (file_name, uri) = Uri::from_path_with_name(path);
    let file = File::from_text(file_id, file_name, uri, None, file_type, content)?;

    Ok(Arc::new(file))
}

/// Parse one file through the selected parser pipeline.
fn parse_file(
    file: Arc<File>,
    language: LanguageType,
    comment_retention: CommentRetention,
) -> Parse {
    let module_id = ModuleId::new(PackageId::new(0), file.id.0);
    let parser = Parser::new(
        file,
        language,
        Tree::new(module_id),
        ParseOptions {
            comment_retention,
            ..ParseOptions::default()
        },
    );

    parser.parse()
}

/// Write a pprof report as an SVG flamegraph.
fn write_flamegraph(path: &Path, report: &pprof::Report) -> Result<(), Box<dyn Error>> {
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }

    let output = FsFile::create(path)?;
    let mut options = FlamegraphOptions::default();
    report.flamegraph_with_options(output, &mut options)?;

    Ok(())
}

/// Write folded stack lines next to the flamegraph.
fn write_folded_stacks(path: &Path, report: &pprof::Report) -> Result<(), Box<dyn Error>> {
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
    }

    let mut output = FsFile::create(path)?;
    for (frames, count) in report.data.iter() {
        let mut line = String::new();
        line.push_str(&frames.thread_name_or_id());

        for frame in frames.frames.iter().rev() {
            for symbol in frame.iter().rev() {
                line.push(';');
                line.push_str(&symbol.name());
            }
        }

        writeln!(output, "{line} {count}")?;
    }

    Ok(())
}

/// Print parser pipeline throughput.
fn print_result(file: &File, stage: ParserStage, output_path: &Path, elapsed: Duration, runs: u64) {
    let seconds = elapsed.as_secs_f64();
    let bytes = file.text().len() as u64 * runs;
    let megabytes = bytes as f64 / 1_000_000.0;
    let lines = file.text().lines().count() as u64 * runs;

    let verb = match stage {
        ParserStage::Lex => "lexed",
        ParserStage::Parse => "parsed",
    };
    eprintln!("{verb} {runs} files, {megabytes:.2} MB, {lines} lines, {seconds:.3}s");
    eprintln!(
        "throughput: {:.2} MB/s, {:.0} lines/s",
        megabytes / seconds,
        lines as f64 / seconds
    );
    eprintln!("flamegraph: {}", output_path.display());
    eprintln!("folded: {}", folded_path(output_path).display());
}

/// Return the folded stack output path next to a flamegraph path.
fn folded_path(path: &Path) -> PathBuf {
    let mut path = path.to_path_buf();
    path.set_extension("folded");

    path
}

/// Return the source path from the first argument or environment.
fn source_path() -> Result<PathBuf, Box<dyn Error>> {
    let path = env::args()
        .nth(1)
        .or_else(|| env::var("TSPP_PARSE_FILE").ok())
        .ok_or("usage: parse <source-file>")?;

    Ok(PathBuf::from(path))
}

/// Return the flamegraph output path.
fn output_path() -> PathBuf {
    env::var("TSPP_PARSE_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_OUTPUT_PATH))
}

/// Return one parsed numeric environment value.
fn number_from_env<T>(name: &str, default: T) -> Result<T, Box<dyn Error>>
where
    T: std::str::FromStr,
    T::Err: Error + 'static,
{
    let Some(value) = env::var(name).ok() else {
        return Ok(default);
    };

    Ok(value.parse()?)
}

/// Return one parsed comment retention environment value.
fn comment_retention_from_env(
    name: &str,
    default: CommentRetention,
) -> Result<CommentRetention, Box<dyn Error>> {
    let Some(value) = env::var(name).ok() else {
        return Ok(default);
    };

    match value.as_str() {
        "0" | "false" | "False" | "FALSE" | "ignore" => Ok(CommentRetention::Ignore),
        "doc" | "docs" | "documentation" => Ok(CommentRetention::Documentation),
        "1" | "true" | "True" | "TRUE" | "all" => Ok(CommentRetention::All),
        _ => Err(format!("invalid comment retention for {name}: {value}").into()),
    }
}

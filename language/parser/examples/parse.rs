use std::env;
use std::error::Error;
use std::fs::{self, File as FsFile};
use std::hint::black_box;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use destack_core::StringPool;
use destack_parser::{Parser, ParserOptions};
use destack_source::{File, FileId, FileType, LanguageType, Uri};
use pprof::ProfilerGuardBuilder;
use pprof::flamegraph::Options as FlamegraphOptions;

const DEFAULT_SECONDS: u64 = 10;
const DEFAULT_SAMPLE_HZ: i32 = 997;
const DEFAULT_OUTPUT_PATH: &str = "language/parser/target/flamegraphs/parse.svg";

/// Options for one parser profile run.
#[derive(Debug)]
struct RunOptions {
    /// The source file to parse repeatedly.
    source_path: PathBuf,
    /// The flamegraph output path.
    output_path: PathBuf,
    /// The sampling duration.
    duration: Duration,
    /// The sampling frequency in hertz.
    sample_hz: i32,
    /// Whether the parser should retain and attach trivia.
    retain_trivia: bool,
}

/// Parse one file repeatedly under the pprof sampler.
fn main() -> Result<(), Box<dyn Error>> {
    let options = RunOptions::from_env()?;
    let file = load_file(&options.source_path)?;
    let language = file_language(file.ty)?;
    let parser_options = ParserOptions {
        retain_trivia_tokens: options.retain_trivia,
        ..ParserOptions::default()
    };

    // sample only the parse loop
    let start = Instant::now();
    let guard = ProfilerGuardBuilder::default()
        .frequency(options.sample_hz)
        .blocklist(&["libsystem", "libc", "libpthread", "libdyld"])
        .build()?;

    let mut parses = 0u64;
    while start.elapsed() < options.duration {
        black_box(parse_file(file.clone(), language, parser_options));
        parses += 1;
    }

    // write artifacts while samples are still live
    let report = guard.report().build()?;
    write_flamegraph(&options.output_path, &report)?;
    write_folded_stacks(&folded_path(&options.output_path), &report)?;

    print_result(&file, &options.output_path, start.elapsed(), parses);

    Ok(())
}

impl RunOptions {
    /// Build run options from arguments and environment variables.
    fn from_env() -> Result<Self, Box<dyn Error>> {
        let source_path = source_path()?;
        let output_path = output_path();
        let seconds = number_from_env("DESTACK_PARSE_SECONDS", DEFAULT_SECONDS)?;
        let sample_hz = number_from_env("DESTACK_PARSE_HZ", DEFAULT_SAMPLE_HZ)?;
        let retain_trivia = bool_from_env("DESTACK_PARSE_TRIVIA", true)?;

        Ok(Self {
            source_path,
            output_path,
            duration: Duration::from_secs(seconds),
            sample_hz,
            retain_trivia,
        })
    }
}

/// Return the parser language for one file type.
fn file_language(file_type: FileType) -> Result<LanguageType, Box<dyn Error>> {
    LanguageType::try_from(file_type)
        .map_err(|file_type| format!("unsupported parser file type: {file_type:?}").into())
}

/// Load one source file.
fn load_file(path: &Path) -> Result<Arc<File>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let file_type = FileType::from_path_or_unknown(path);
    let file_id = FileId::new(0);
    let (file_name, uri) = Uri::from_path_with_name(path);
    let file = File::from_text(file_id, file_name, uri, None, file_type, content);

    Ok(Arc::new(file))
}

/// Parse one file through the selected parser pipeline.
fn parse_file(file: Arc<File>, language: LanguageType, options: ParserOptions) -> Parser {
    let strings = Arc::new(StringPool::new());
    let mut parser = Parser::lex_file_with_options(file, language, options, strings);

    // attach comments only when requested
    if options.retain_trivia_tokens {
        parser.parse();
    } else {
        parser.parse_without_trivia();
    }

    parser
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

/// Print parser throughput for one run.
fn print_result(file: &File, output_path: &Path, elapsed: Duration, parses: u64) {
    let seconds = elapsed.as_secs_f64();
    let bytes = file.text().len() as u64 * parses;
    let megabytes = bytes as f64 / 1_000_000.0;
    let lines = file.text().lines().count() as u64 * parses;

    eprintln!(
        "parsed {parses} files, {:.2} MB, {lines} lines, {:.3}s",
        megabytes, seconds
    );
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
        .or_else(|| env::var("DESTACK_PARSE_FILE").ok())
        .ok_or("usage: parse <source-file>")?;

    Ok(PathBuf::from(path))
}

/// Return the flamegraph output path.
fn output_path() -> PathBuf {
    env::var("DESTACK_PARSE_OUTPUT")
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

/// Return one parsed boolean environment value.
fn bool_from_env(name: &str, default: bool) -> Result<bool, Box<dyn Error>> {
    let Some(value) = env::var(name).ok() else {
        return Ok(default);
    };

    match value.as_str() {
        "0" | "false" | "False" | "FALSE" => Ok(false),
        "1" | "true" | "True" | "TRUE" => Ok(true),
        _ => Err(format!("invalid boolean for {name}: {value}").into()),
    }
}

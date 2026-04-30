use std::sync::Arc;
use std::{env, fs};

use destack_parser::{Parser, ParserOptions};
use destack_source::{File, FileId, FileType, LanguageType, Uri};

/// Return whether parser-side trivia retention should stay enabled.
fn retain_trivia_tokens_from_env() -> bool {
    env::var("DESTACK_PARSE_RETAIN_TRIVIA")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .map(|value| value > 0)
        .unwrap_or(true)
}

/// Print parser timing data for one source file.
fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| panic!("missing source path argument"));
    let path = std::path::PathBuf::from(path);
    let file_type = FileType::from_path_or_unknown(&path);
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("file system error while reading {}", path.display()));
    let language = LanguageType::from(file_type);
    let (file_name, uri) = Uri::from_path_with_name(&path);
    let file = File::from_text(FileId::new(0), file_name, uri, None, file_type, content);

    let retain_trivia_tokens = retain_trivia_tokens_from_env();
    let mut parser = Parser::lex_file_with_options(
        Arc::new(file),
        language,
        ParserOptions {
            retain_trivia_tokens,
            ..ParserOptions::default()
        },
    );
    let nodes = if retain_trivia_tokens {
        parser.parse()
    } else {
        parser.parse_without_trivia()
    };

    let mut timings = parser.timing_snapshot().unwrap_or_default();
    timings.sort_by(|left, right| {
        right
            .self_duration
            .cmp(&left.self_duration)
            .then(right.duration.cmp(&left.duration))
            .then(left.name.cmp(right.name))
    });

    println!("parsed root expressions: {}", nodes.len());
    println!("string count: {}", parser.strings.len());
    println!("node count: {}", parser.tree.next_id());
    println!();
    println!("self %      self ms   incl ms    count  tag");

    let total_self_seconds: f64 = timings
        .iter()
        .map(|entry| entry.self_duration.as_secs_f64())
        .sum();

    for entry in timings {
        let self_seconds = entry.self_duration.as_secs_f64();
        let inclusive_seconds = entry.duration.as_secs_f64();
        let self_share = if total_self_seconds > 0.0 {
            100.0 * self_seconds / total_self_seconds
        } else {
            0.0
        };

        println!(
            "{:>6.2}%  {:>10.3} {:>9.3}  {:>7}  {}",
            self_share,
            self_seconds * 1000.0,
            inclusive_seconds * 1000.0,
            entry.count,
            entry.name,
        );
    }
}

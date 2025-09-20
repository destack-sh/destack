//! Print subcommand for syntax-highlighted output without formatting.

use crate::cli::source::{read_source, render_semantic_spans, semantic_spans_from_source};
use crate::console::console;
use crate::console::parse::CommandArguments;

pub const HELP: &str = "Print Dyst source with semantic highlighting.\n\t--file <path>      Read input from file\n\t--string <string>  Read input from provided string";

/// Parse input and render the semantic-colored output.
pub fn run(ctx: CommandArguments) -> i32 {
    // read input source
    let source = match read_source(&ctx) {
        Ok(source) => source,
        Err(error) => {
            console::error(&format!("Read input error: {error}"));
            return 1;
        }
    };

    // parse semantic spans
    let colored_output = match semantic_spans_from_source(&source) {
        Ok(spans) => render_semantic_spans(&spans),
        Err(error) => {
            console::warn(&format!("semantic highlighting error: {error}"));
            source.content.clone()
        }
    };

    // print colored output
    console::write_line(&colored_output);
    0
}

use destack_terminal::{CommandArguments, console};

use crate::source::{get_semantic_spans_from_source, read_source, render_semantic_spans};

pub const HELP: &str = r"Print Dyst source with semantic highlighting.
	--file <path>      Read input from file
	--string <string>  Read input from provided string";

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
    let colored_output = match get_semantic_spans_from_source(&source) {
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

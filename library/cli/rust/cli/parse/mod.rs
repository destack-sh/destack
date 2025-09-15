//! Lexes and parse Destack source code.

use std::fs;
use std::path::Path;

use dyst_language_source::{Source, SourceId};

use crate::console::CommandArguments;
use crate::console::parse::CommandApp;

mod ast;
mod token;

pub(super) const DEFAULT_MAX_LEXEME_LEN: usize = 80;

/// Create the parse CLI app.
pub fn app() -> CommandApp {
    CommandApp::new("parse")
        .help("parser tools")
        .default_command("token")
        .command(
            "token",
            token::parse_token,
            Some(
                "Parse source into Tokens (with Spans).
			--file <path>      Read input from file
			--string <string>  Read input from provided string
			--no-color         Disable ANSI colors
			--no-pager         Print directly instead of use less -R
			--max-lexeme <n>   Truncate lexeme preview to n chars"
                    .to_owned(),
            ),
        )
        .command(
            "ast",
            ast::parse_ast,
            Some(
                "Parse source into Tokens (with Spans).
			--file <path>      Read input from file
			--string <string>  Read input from provided string
            )"
                .to_owned(),
            ),
        )
}

/// Resolve the input to lex from the command arguments (file or string).
pub(crate) fn read_source(ctx: &CommandArguments) -> Result<Source, String> {
    if let Some(path) = ctx.option("file") {
        let p = Path::new(path);
        fs::read_to_string(p)
            .map_err(|e| format!("failed to read {path}: {e}"))
            .map(|s| Source::from_string(SourceId::new(0), path.to_string(), s))
    } else if let Some(string) = ctx.option("string") {
        Ok(Source::from_string(
            SourceId::new(0),
            "<string>".to_string(),
            string.to_string(),
        ))
    } else {
        Err("provide --file <path> or --string <string>".to_string())
    }
}

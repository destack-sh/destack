use crate::source::get_file_from_arguments;

use destack_terminal::{CommandArguments, console};

pub const HELP: &str = r"Transpile source files.
	--file <path>      Read input from file
	--string <string>  Read input from provided string
    --silent           Don't print anything to the console (except errors)
    ";

/// Transpile source into its final JavaScript.
pub fn run(ctx: CommandArguments) -> i32 {
    let _silent = ctx.flag("silent");

    // read input source
    let _source = match get_file_from_arguments(&ctx) {
        Ok(source) => source,
        Err(error) => {
            console::error(&format!("Read input error: {error}"));
            return 1;
        }
    };

    // transpile source
    todo!("transpile source")
}

use destack_terminal::{CommandArguments, console};

use crate::source::get_file_from_arguments;

pub const HELP: &str = r"Compile source files.
	--file <path>      Read input from file
	--string <string>  Read input from provided string
    --silent           Don't print anything to the console (except errors)
    ";

/// Compile source into its final DIR.
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

    // compile source
    todo!("compile source")
}

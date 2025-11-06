use destack_terminal::CommandArguments;

pub const HELP: &str = r"Transpile source files.
	--file <path>      Read input from file
	--string <string>  Read input from provided string
    --silent           Don't print anything to the console (except errors)
    ";

/// Transpile source into its final JavaScript.
pub fn run(ctx: CommandArguments) -> i32 {
    let _silent = ctx.flag("silent");

    // transpile source
    todo!("transpile source")
}

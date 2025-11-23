use clap::Parser;
use destack_cli::cli::Cli;
use destack_cli::command::version::VersionCommands;
use destack_cli::{compile, lex, parse, resolve, transpile, version};

fn main() {
    let cli = Cli::parse();
    let exit_code = match cli {
        Cli::Lex(args) => lex::run(&args),
        Cli::Parse(args) => parse::run(&args),
        Cli::Resolve(args) => resolve::run(&args),
        Cli::Compile(args) => compile::run(&args),
        Cli::Transpile(args) => transpile::run(&args),
        Cli::Version { subcommand } => match subcommand {
            VersionCommands::Bump => version::bump(),
        },
    };
    std::process::exit(exit_code);
}

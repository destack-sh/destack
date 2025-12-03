use clap::Parser;
use destack_cli::cli::{Cli, Command};
use destack_cli::command::version::VersionCommands;
use destack_cli::{compile, lex, parse, resolve, transpile, version};

fn main() {
    let cli = Cli::parse();
    cli.tracing.init();

    let exit_code = match cli.command {
        Command::Lex(args) => lex::run(&args),
        Command::Parse(args) => parse::run(&args),
        Command::Resolve(args) => resolve::run(&args),
        Command::Compile(args) => compile::run(&args),
        Command::Transpile(args) => transpile::run(&args),
        Command::Version { subcommand } => match subcommand {
            VersionCommands::Show => version::show(),
            cmd => version::bump(&cmd),
        },
    };
    std::process::exit(exit_code);
}

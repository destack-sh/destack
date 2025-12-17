use clap::Parser;
use destack_cli::cli::{Cli, Command};
use destack_cli::{build, check, clean, fmt, init, lint, lsp, run};

#[cfg(feature = "dev")]
use destack_cli::command::{DevCommand, dev};

fn main() {
    let cli = Cli::parse();
    cli.tracing.init();

    let exit_code = match cli.command {
        Command::Check(args) => check::run(&args),
        Command::Build(args) => build::run(&args),
        Command::Run(args) => run::run(&args),
        Command::Lint(args) => lint::run(&args),
        Command::Format(args) => fmt::run(&args),
        Command::Init(args) => init::run(&args),
        Command::Clean(args) => clean::run(&args),
        Command::Lsp(args) => lsp::run(&args),
        #[cfg(feature = "dev")]
        Command::Dev(subcommand) => match subcommand {
            DevCommand::Parse(args) => dev::parse::run(&args),
            DevCommand::Dir(args) => dev::dir::run(&args),
            DevCommand::Resolve(args) => dev::resolve::run(&args),
            DevCommand::Version(cmd) => match cmd {
                dev::VersionCommands::Show => dev::version::show(),
                cmd => dev::version::bump(&cmd),
            },
        },
    };
    std::process::exit(exit_code);
}

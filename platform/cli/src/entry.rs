use std::ffi::OsString;

use clap::FromArgMatches;

use crate::cli::{Cli, Command, HelpMode, build_command};
use crate::{
    bench, build, cache, check, clean, completions, config, console, daemon, doc, doctor, eval,
    explain, fmt, info, init, lint, lsp, repl, run, targets, task, test, version,
};

#[cfg(feature = "dev")]
use crate::command::{DevCommand, dev};

/// Default subcommand selection for alias binaries.
#[derive(Clone, Copy, Debug)]
pub enum DefaultCommand {
    /// No default command.
    None,
    /// Default to the build command.
    Build,
    /// Default to the run command.
    Run,
}

impl DefaultCommand {
    /// Return the command name, if any.
    fn as_str(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Build => Some("build"),
            Self::Run => Some("run"),
        }
    }
}

/// Run the CLI with optional default command injection.
pub fn run(default_command: DefaultCommand) -> i32 {
    // normalize args with default command handling
    let args = normalize_args(default_command);

    // respect color overrides when building help text
    if let Some(mode) = parse_color_override(&args) {
        console::set_color_mode(mode);
    }

    let help_mode = help_mode_for_args(&args);

    // parse the cli and initialize tracing
    let matches = build_command(help_mode).get_matches_from(args);
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    cli.tracing.apply_console_settings();
    cli.tracing.init();

    // dispatch the resolved command
    match cli.command {
        Command::Check(args) => check::run(&args),
        Command::Build(args) => build::run(&args),
        Command::Run(args) => run::run(&args),
        Command::Eval(args) => eval::run(&args),
        Command::Lint(args) => lint::run(&args),
        Command::Format(args) => fmt::run(&args),
        Command::Init(args) => init::run(&args),
        Command::Clean(args) => clean::run(&args),
        Command::Cache(args) => cache::run(&args),
        Command::Info(args) => info::run(&args),
        Command::Config(args) => config::run(&args),
        Command::Targets(args) => targets::run(&args),
        Command::Version(args) => version::run(&args),
        Command::Completions(args) => completions::run(&args),
        Command::Explain(args) => explain::run(&args),
        Command::Doctor(args) => doctor::run(&args),
        Command::Test(args) => test::run(&args),
        Command::Bench(args) => bench::run(&args),
        Command::Doc(args) => doc::run(&args),
        Command::Task(args) => task::run(&args),
        Command::Lsp(args) => lsp::run(&args),
        Command::Daemon(args) => daemon::run(&args),
        Command::Repl(args) => repl::run(&args),
        #[cfg(feature = "dev")]
        Command::Dev(subcommand) => match subcommand {
            DevCommand::Resolve(args) => dev::resolve::run(&args),
            DevCommand::Stats(args) => dev::stats::run(&args),
            DevCommand::Version(cmd) => match cmd {
                dev::VersionCommands::Show => dev::version::show(),
                cmd => dev::version::bump(&cmd),
            },
        },
    }
}

/// Normalize argv with an optional default subcommand.
fn normalize_args(default_command: DefaultCommand) -> Vec<OsString> {
    // collect argv as os strings
    let mut args: Vec<OsString> = std::env::args_os().collect();

    // skip injection when no default is configured
    let Some(command) = default_command.as_str() else {
        return args;
    };

    // detect whether a subcommand is already provided
    let has_subcommand = args
        .get(1)
        .and_then(|arg| arg.to_str())
        .is_some_and(is_known_subcommand);

    // inject the default command when missing
    if !has_subcommand {
        args.insert(1, OsString::from(command));
    }

    // return normalized argv
    args
}

/// Return whether an argument matches a known subcommand.
fn is_known_subcommand(arg: &str) -> bool {
    // match against known subcommand spellings and flags
    matches!(
        arg,
        "check"
            | "build"
            | "compile"
            | "run"
            | "exec"
            | "eval"
            | "lint"
            | "format"
            | "fmt"
            | "init"
            | "clean"
            | "cache"
            | "info"
            | "config"
            | "completions"
            | "completion"
            | "targets"
            | "explain"
            | "doctor"
            | "env"
            | "version"
            | "test"
            | "bench"
            | "doc"
            | "task"
            | "lsp"
            | "daemon"
            | "repl"
            | "typecheck"
            | "dev"
            | "--help"
            | "-h"
            | "--version"
            | "-V"
    )
}

/// Parse a color mode override from raw args.
fn parse_color_override(args: &[OsString]) -> Option<console::ColorMode> {
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        let Some(arg_str) = arg.to_str() else {
            continue;
        };

        let value = if let Some(rest) = arg_str.strip_prefix("--color=") {
            Some(rest.to_string())
        } else if arg_str == "--color" {
            iter.next()
                .and_then(|value| value.to_str())
                .map(String::from)
        } else {
            None
        };

        let Some(value) = value else {
            continue;
        };

        return match value.as_str() {
            "auto" => Some(console::ColorMode::Auto),
            "always" => Some(console::ColorMode::Always),
            "never" => Some(console::ColorMode::Never),
            _ => None,
        };
    }

    None
}

/// Determine which help layout to use from raw args.
fn help_mode_for_args(args: &[OsString]) -> HelpMode {
    let mut saw_short = false;
    let mut saw_long = false;
    for arg in args {
        let Some(arg_str) = arg.to_str() else {
            continue;
        };
        if arg_str == "--help" {
            saw_long = true;
        } else if arg_str == "-h" {
            saw_short = true;
        } else if arg_str == "help" {
            saw_long = true;
        }
    }

    if saw_long {
        return HelpMode::Full;
    }

    if saw_short {
        return HelpMode::Compact;
    }

    HelpMode::Compact
}

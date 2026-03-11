use std::ffi::OsString;

use clap::FromArgMatches;

use crate::cli::{Cli, Command, HelpMode, build_command};
use crate::console;

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
    cli.command.run()
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
        .is_some_and(Command::is_known_subcommand);

    // inject the default command when missing
    if !has_subcommand {
        args.insert(1, OsString::from(command));
    }

    // return normalized argv
    args
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

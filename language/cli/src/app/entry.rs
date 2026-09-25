use std::ffi::OsString;

use clap::FromArgMatches;
use futures::executor::block_on;

use crate::app::{Cli, Command, HelpMode, build_command};
use crate::console;

/// Default subcommand selection for alias binaries.
#[derive(Clone, Copy, Debug)]
pub enum DefaultCommand {
    /// No default command.
    None,
    /// Default to the build command.
    Build,
}

impl DefaultCommand {
    /// Return the command name, if any.
    fn as_str(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Build => Some("build"),
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

    // parse the cli and apply console settings
    let matches = build_command(help_mode).get_matches_from(args);
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    cli.console.apply();

    // dispatch the resolved command
    block_on(cli.command.run())
}

/// Normalize argv with an optional default subcommand.
fn normalize_args(default_command: DefaultCommand) -> Vec<OsString> {
    // collect argv as os strings
    let args: Vec<OsString> = std::env::args_os().collect();
    normalize_args_with(default_command, args)
}

/// Normalize one argv vector with default command handling.
fn normalize_args_with(default_command: DefaultCommand, mut args: Vec<OsString>) -> Vec<OsString> {
    if args.is_empty() {
        return args;
    }

    // skip injection when no default is configured
    if let Some(command) = default_command.as_str() {
        let has_subcommand = args
            .get(1)
            .and_then(|arg| arg.to_str())
            .is_some_and(Command::is_known_subcommand);

        // inject the default command when missing
        if !has_subcommand {
            args.insert(1, OsString::from(command));
        }
    }

    rewrite_unknown_subcommand_to_task(args)
}

/// Rewrite one unknown bare subcommand into one task invocation.
fn rewrite_unknown_subcommand_to_task(args: Vec<OsString>) -> Vec<OsString> {
    let Some(command) = args.get(1).and_then(|arg| arg.to_str()) else {
        return args;
    };

    // keep known commands, help flags, and option-first invocation unchanged
    if command.starts_with('-') || Command::is_known_subcommand(command) {
        return args;
    }

    let mut rewritten = Vec::with_capacity(args.len() + 3);
    rewritten.push(args[0].clone());
    rewritten.push(OsString::from("task"));
    rewritten.push(OsString::from(command));

    // forward the remaining argv to the task command
    if args.len() > 2 {
        rewritten.push(OsString::from("--"));
        rewritten.extend(args.into_iter().skip(2));
    }

    rewritten
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

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{DefaultCommand, normalize_args_with};

    /// Rewrite one unknown top-level verb into one task command.
    #[test]
    fn test_normalize_args_rewrites_unknown_subcommand_to_task() {
        let args = vec![
            OsString::from("tspp"),
            OsString::from("up"),
            OsString::from("--force"),
        ];

        let args = normalize_args_with(DefaultCommand::None, args);

        assert_eq!(
            args,
            vec![
                OsString::from("tspp"),
                OsString::from("task"),
                OsString::from("up"),
                OsString::from("--"),
                OsString::from("--force"),
            ]
        );
    }

    /// Leave one builtin command unchanged.
    #[test]
    fn test_normalize_args_keeps_known_subcommand() {
        let args = vec![OsString::from("tspp"), OsString::from("build")];

        let args = normalize_args_with(DefaultCommand::None, args);

        assert_eq!(args, vec![OsString::from("tspp"), OsString::from("build")]);
    }
}

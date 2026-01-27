use clap::builder::StyledStr;
use clap::builder::styling::{AnsiColor, Style, Styles};
use clap::{CommandFactory, Parser};

use crate::console;

#[cfg(feature = "dev")]
use crate::command::DevCommand;
use crate::command::{
    BenchArgs, BuildArgs, CacheArgs, CheckArgs, CleanArgs, CompletionsArgs, ConfigArgs, DaemonArgs,
    DocArgs, DoctorArgs, EvalArgs, ExplainArgs, FmtArgs, InfoArgs, InitArgs, LintArgs, LspArgs,
    QueryArgs, ReplArgs, RunArgs, TargetsArgs, TaskArgs, TestArgs, VersionArgs,
};
use crate::common::TracingArgs;

/// Base help template for CLI output.
const HELP_TEMPLATE_BASE: &str = "{before-help}{usage-heading} {usage}\n";

/// Build the CLI style palette.
fn destack_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Green.on_default().bold())
        .usage(AnsiColor::Green.on_default().bold())
        .literal(AnsiColor::Cyan.on_default().bold())
        .placeholder(AnsiColor::Blue.on_default())
        .valid(AnsiColor::Green.on_default())
        .invalid(AnsiColor::Red.on_default().bold())
        .context(AnsiColor::Yellow.on_default())
}

/// Root CLI arguments for destack.
#[derive(Parser, Debug)]
#[command(
    name = "destack",
    version,
    about = "Destack is a universal software engine for building correct, optimal, integrated software systems.",
    long_about = None,
    styles = destack_styles()
)]
pub struct Cli {
    #[command(flatten)]
    pub tracing: TracingArgs,

    #[command(subcommand)]
    pub command: Command,
}

/// Top-level CLI command selection.
#[derive(Parser, Debug)]
pub enum Command {
    /// Check source files for type errors and lint issues.
    #[command(alias = "typecheck")]
    Check(CheckArgs),

    /// Compile source files.
    #[command(alias = "compile")]
    Build(BuildArgs),

    /// Compile and run a source file or script.
    #[command(alias = "exec")]
    Run(RunArgs),

    /// Evaluate inline code.
    Eval(EvalArgs),

    /// Lint source files (alias for check).
    Lint(LintArgs),

    /// Format source files.
    #[command(alias = "fmt")]
    Format(FmtArgs),

    /// Initialize a new project.
    Init(InitArgs),

    /// Remove build outputs and caches.
    Clean(CleanArgs),

    /// Show cache locations and settings.
    Cache(CacheArgs),

    /// Show workspace and target information.
    Info(InfoArgs),

    /// Show the resolved configuration.
    Config(ConfigArgs),

    /// List configured build targets.
    Targets(TargetsArgs),

    /// Show version information.
    Version(VersionArgs),

    /// Generate shell completions.
    Completions(CompletionsArgs),

    /// Explain a diagnostic or lint rule.
    Explain(ExplainArgs),

    /// Show environment and workspace diagnostics.
    #[command(alias = "env")]
    Doctor(DoctorArgs),

    /// Run tests.
    Test(TestArgs),

    /// Run benchmarks.
    Bench(BenchArgs),

    /// Generate documentation.
    Doc(DocArgs),

    /// Run workspace tasks.
    Task(TaskArgs),

    /// Start the language server (for editor integration).
    Lsp(LspArgs),

    /// Start the daemon service (for CLI/LSP integration).
    Daemon(DaemonArgs),

    /// Execute workspace queries.
    Query(QueryArgs),

    /// Start a REPL session.
    Repl(ReplArgs),

    /// Developer commands (compiler inspection, version management).
    #[cfg(feature = "dev")]
    #[command(subcommand)]
    Dev(DevCommand),
}

/// Help verbosity for the CLI output.
#[derive(Clone, Copy, Debug)]
pub enum HelpMode {
    /// Show compact help without options.
    Compact,
    /// Show full help with options.
    Full,
}

/// Build the clap command with custom help output.
pub fn build_command(mode: HelpMode) -> clap::Command {
    let mut command = Cli::command();
    let color_enabled = console::color_enabled(console::Stream::Stdout);
    command = command.before_help(build_before_help(color_enabled));
    command.help_template(build_help_template(mode, color_enabled))
}

/// Build the help header block.
fn build_before_help(color_enabled: bool) -> StyledStr {
    let mut text = StyledStr::new();
    let version = env!("CARGO_PKG_VERSION");

    if color_enabled {
        let title_style = Style::new().bold();
        let tagline_style = Style::new().bold();
        let version_style = Style::new().bold().dimmed();
        text.push_str(&format!("{title_style}destack{title_style:#}\n"));
        text.push_str(&format!(
            "{tagline_style}Destack is a universal software engine for building correct, optimal, integrated software systems{tagline_style:#} "
        ));
        text.push_str(&format!("{version_style}({version}){version_style:#}"));
        return text;
    }

    text.push_str("destack\n");
    text.push_str(&format!(
        "Destack is a universal software engine for building correct, optimal, integrated software systems ({version})",
    ));
    text
}

/// Command metadata for grouped help output.
struct CommandEntry {
    /// Command name to display.
    name: &'static str,
    /// Example argument string for display.
    example: &'static str,
    /// Command description text.
    help: &'static str,
    /// Group ordering bucket.
    group: usize,
}

/// Build grouped command help output.
fn build_commands_help(color_enabled: bool) -> String {
    let entries = [
        CommandEntry {
            name: "run",
            example: "./src/main.ds",
            help: "Compile and run a source file or script",
            group: 0,
        },
        CommandEntry {
            name: "eval",
            example: "1 + 2",
            help: "Evaluate inline code",
            group: 0,
        },
        CommandEntry {
            name: "repl",
            example: "",
            help: "Start a REPL session",
            group: 0,
        },
        CommandEntry {
            name: "build",
            example: "./src/main.ds",
            help: "Compile sources for a target",
            group: 0,
        },
        CommandEntry {
            name: "check",
            example: "src/",
            help: "Check source files for type errors and lint issues",
            group: 0,
        },
        CommandEntry {
            name: "lint",
            example: "src/",
            help: "Lint source files (alias for check)",
            group: 0,
        },
        CommandEntry {
            name: "format",
            example: "src/",
            help: "Format source files",
            group: 0,
        },
        CommandEntry {
            name: "test",
            example: "",
            help: "Run tests",
            group: 1,
        },
        CommandEntry {
            name: "bench",
            example: "",
            help: "Run benchmarks",
            group: 1,
        },
        CommandEntry {
            name: "doc",
            example: "",
            help: "Generate documentation",
            group: 1,
        },
        CommandEntry {
            name: "init",
            example: "",
            help: "Initialize a new project",
            group: 2,
        },
        CommandEntry {
            name: "clean",
            example: "",
            help: "Remove build outputs and caches",
            group: 2,
        },
        CommandEntry {
            name: "cache",
            example: "",
            help: "Show cache locations and settings",
            group: 2,
        },
        CommandEntry {
            name: "info",
            example: "",
            help: "Show workspace and target information",
            group: 2,
        },
        CommandEntry {
            name: "config",
            example: "dsconfig.json",
            help: "Show the resolved configuration",
            group: 2,
        },
        CommandEntry {
            name: "targets",
            example: "",
            help: "List configured build targets",
            group: 2,
        },
        CommandEntry {
            name: "task",
            example: "<task>",
            help: "Run workspace tasks",
            group: 2,
        },
        CommandEntry {
            name: "doctor",
            example: "",
            help: "Show environment and workspace diagnostics",
            group: 3,
        },
        CommandEntry {
            name: "explain",
            example: "ER100",
            help: "Explain a diagnostic or lint rule",
            group: 3,
        },
        CommandEntry {
            name: "completions",
            example: "zsh",
            help: "Generate shell completions",
            group: 3,
        },
        CommandEntry {
            name: "lsp",
            example: "",
            help: "Start the language server (for editor integration)",
            group: 3,
        },
        CommandEntry {
            name: "query",
            example: "",
            help: "Execute workspace queries",
            group: 3,
        },
        CommandEntry {
            name: "daemon",
            example: "",
            help: "Start the daemon service (for CLI/LSP integration)",
            group: 3,
        },
        CommandEntry {
            name: "version",
            example: "",
            help: "Show version information",
            group: 3,
        },
        CommandEntry {
            name: "dev",
            example: "",
            help: "Developer commands (compiler inspection, version management)",
            group: 3,
        },
        CommandEntry {
            name: "<command>",
            example: "--help",
            help: "Print help text for command",
            group: 3,
        },
    ];

    let command_width = entries
        .iter()
        .map(|entry| entry.name.len())
        .max()
        .unwrap_or(0);
    let example_width = entries
        .iter()
        .map(|entry| entry.example.len())
        .max()
        .unwrap_or(0);

    let mut output = String::new();
    if color_enabled {
        let heading_style = AnsiColor::Green.on_default().bold();
        output.push_str(&format!("\n{heading_style}Commands:{heading_style:#}\n"));
    } else {
        output.push_str("\nCommands:\n");
    }

    let mut current_group = entries.first().map(|entry| entry.group).unwrap_or(0);
    let command_style = if color_enabled {
        Some(AnsiColor::Cyan.on_default().bold())
    } else {
        None
    };
    let example_style = if color_enabled {
        Some(Style::new().dimmed())
    } else {
        None
    };

    for entry in entries {
        if entry.group != current_group {
            output.push('\n');
            current_group = entry.group;
        }

        let command = format!("{:width$}", entry.name, width = command_width);
        let example = format!("{:width$}", entry.example, width = example_width);
        let command = if let Some(style) = command_style {
            format!("{style}{command}{style:#}")
        } else {
            command
        };
        let example = if let Some(style) = example_style {
            if entry.example.is_empty() {
                example
            } else {
                format!("{style}{example}{style:#}")
            }
        } else {
            example
        };

        output.push_str(&format!("  {command}  {example}  {}\n", entry.help));
    }

    output
}

/// Build the dynamic help template with optional options output.
fn build_help_template(mode: HelpMode, color_enabled: bool) -> StyledStr {
    let mut text = StyledStr::new();
    text.push_str(HELP_TEMPLATE_BASE);
    text.push_str(&build_usage_aliases(color_enabled));
    text.push_str("\n");
    text.push_str(&build_commands_help(color_enabled));
    if matches!(mode, HelpMode::Full) {
        text.push_str("\n");
        text.push_str(&build_options_heading(color_enabled));
        text.push_str("\n{options}\n");
    }
    text
}

/// Build the usage aliases line for the help template.
fn build_usage_aliases(color_enabled: bool) -> String {
    if color_enabled {
        let shortcut_style = AnsiColor::Cyan.on_default().bold();
        let hint_style = Style::new().dimmed();
        return format!(
            "  {hint_style}or{hint_style:#} {shortcut_style}ds{shortcut_style:#} | {shortcut_style}dsc{shortcut_style:#} | {shortcut_style}dsx{shortcut_style:#}"
        );
    }

    "  or ds | dsc | dsx".to_string()
}

/// Build the options heading for help output.
fn build_options_heading(color_enabled: bool) -> String {
    if color_enabled {
        let heading_style = AnsiColor::Green.on_default().bold();
        return format!("{heading_style}Options:{heading_style:#}");
    }

    "Options:".to_string()
}

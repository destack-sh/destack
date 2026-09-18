use clap::builder::StyledStr;
use clap::builder::styling::{AnsiColor, Style, Styles};
use clap::{Args, CommandFactory, Parser, ValueEnum};

use crate::{
    build, cache, check, clean, completions, console, doc, doctor, explain, fmt, info, init, lint,
    lsp, query, rewrite, run, settings, targets, task, test, update, version,
};

#[cfg(feature = "dev")]
use crate::command::DevCommand;
#[cfg(feature = "dev")]
use crate::command::dev::stats;
use crate::command::{
    BuildArgs, CacheArgs, CheckArgs, CleanArgs, CompletionsArgs, DaemonArgs, DocArgs, DoctorArgs,
    ExplainArgs, FmtArgs, InfoArgs, InitArgs, LintArgs, LspArgs, QueryArgs, RewriteArgs, RunArgs,
    SettingsArgs, TargetsArgs, TaskArgs, TestArgs, UpdateArgs, VersionArgs,
};

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

/// CLI color mode.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ColorModeArg {
    /// Enable colors when the output supports it.
    #[default]
    Auto,
    /// Always emit ANSI colors.
    Always,
    /// Never emit ANSI colors.
    Never,
}

impl From<ColorModeArg> for console::ColorMode {
    /// Convert a CLI color mode into a console color mode.
    fn from(mode: ColorModeArg) -> Self {
        match mode {
            ColorModeArg::Auto => console::ColorMode::Auto,
            ColorModeArg::Always => console::ColorMode::Always,
            ColorModeArg::Never => console::ColorMode::Never,
        }
    }
}

/// Global console arguments.
#[derive(Args, Debug, Clone, Default)]
pub struct ConsoleArgs {
    /// Color mode.
    #[arg(long = "color", value_enum, hide_possible_values = true, global = true)]
    pub color: Option<ColorModeArg>,
}

impl ConsoleArgs {
    /// Apply global console settings.
    pub fn apply(&self) {
        if let Some(mode) = self.color {
            console::set_color_mode(mode.into());
        }
    }
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
    pub console: ConsoleArgs,

    #[command(subcommand)]
    pub command: Command,
}

/// Top-level CLI command selection.
#[derive(Parser, Debug)]
pub enum Command {
    /// Check source files for type errors and lint issues.
    #[command(alias = "typecheck")]
    Check(CheckArgs),

    /// Build a program and run it.
    Run(RunArgs),

    /// Compile source files.
    #[command(alias = "compile")]
    Build(BuildArgs),

    /// Lint source files (alias for check).
    Lint(LintArgs),

    /// Format source files.
    #[command(alias = "fmt")]
    Format(FmtArgs),

    /// Query source files with a structural pattern.
    Query(QueryArgs),

    /// Rewrite source files with a structural pattern.
    Rewrite(RewriteArgs),

    /// Initialize a new project.
    Init(InitArgs),

    /// Remove build outputs and caches.
    Clean(CleanArgs),

    /// Show artifact cache usage.
    Cache(CacheArgs),

    /// Show resolved machine and workspace settings.
    Settings(SettingsArgs),

    /// Show workspace and target information.
    Info(InfoArgs),

    /// List configured build targets.
    Targets(TargetsArgs),

    /// Show version information.
    Version(VersionArgs),

    /// Update the installed Destack CLI binaries.
    Update(UpdateArgs),

    /// Generate shell completions.
    Completions(CompletionsArgs),

    /// Explain a diagnostic or lint rule.
    Explain(ExplainArgs),

    /// Show environment and workspace diagnostics.
    #[command(alias = "env")]
    Doctor(DoctorArgs),

    /// Run tests.
    Test(TestArgs),

    /// Generate documentation.
    Doc(DocArgs),

    /// Run workspace tasks.
    Task(TaskArgs),

    /// Start the language server (for editor integration).
    Lsp(LspArgs),

    /// Manage the Destack daemon.
    Daemon(DaemonArgs),

    /// Developer commands (compiler inspection, version management).
    #[cfg(feature = "dev")]
    #[command(subcommand)]
    Dev(DevCommand),
}

impl Command {
    /// Execute the selected CLI command.
    pub async fn run(self) -> i32 {
        match self {
            Self::Check(args) => check::run(&args).await,
            Self::Build(args) => build::run(&args).await,
            Self::Run(args) => run::run(&args).await,
            Self::Lint(args) => lint::run(&args).await,
            Self::Format(args) => fmt::run(&args).await,
            Self::Query(args) => query::run(&args).await,
            Self::Rewrite(args) => rewrite::run(&args).await,
            Self::Init(args) => init::run(&args),
            Self::Clean(args) => clean::run(&args).await,
            Self::Cache(args) => cache::run(&args).await,
            Self::Settings(args) => settings::run(&args).await,
            Self::Info(args) => info::run(&args).await,
            Self::Targets(args) => targets::run(&args).await,
            Self::Version(args) => version::run(&args),
            Self::Update(args) => update::run(&args),
            Self::Completions(args) => completions::run(&args),
            Self::Explain(args) => explain::run(&args),
            Self::Doctor(args) => doctor::run(&args).await,
            Self::Test(args) => test::run(&args).await,
            Self::Doc(args) => doc::run(&args).await,
            Self::Task(args) => task::run(&args).await,
            Self::Lsp(args) => lsp::run(&args),
            Self::Daemon(args) => crate::command::daemon::run(&args),
            #[cfg(feature = "dev")]
            Self::Dev(subcommand) => match subcommand {
                DevCommand::Stats(args) => stats::run(&args),
                DevCommand::Version(command) => command.run(),
            },
        }
    }

    /// Return whether an argument matches a known subcommand or alias.
    pub fn is_known_subcommand(arg: &str) -> bool {
        if matches!(arg, "--help" | "-h" | "--version" | "-V" | "help") {
            return true;
        }

        Cli::command().find_subcommand(arg).is_some()
    }
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
    help: Option<&'static str>,
    /// Group ordering bucket.
    group: usize,
}

/// Build grouped command help output.
fn build_commands_help(color_enabled: bool) -> String {
    let entries = vec![
        CommandEntry {
            name: "run",
            example: "./src/main.ds",
            help: None,
            group: 0,
        },
        CommandEntry {
            name: "eval",
            example: "1 + 2",
            help: None,
            group: 0,
        },
        CommandEntry {
            name: "build",
            example: "./src/main.ds",
            help: None,
            group: 0,
        },
        CommandEntry {
            name: "check",
            example: ".",
            help: None,
            group: 0,
        },
        CommandEntry {
            name: "lint",
            example: ".",
            help: None,
            group: 0,
        },
        CommandEntry {
            name: "format",
            example: "src/",
            help: None,
            group: 0,
        },
        CommandEntry {
            name: "query",
            example: "'fetch($URL)' .",
            help: None,
            group: 0,
        },
        CommandEntry {
            name: "rewrite",
            example: "'fetch($URL)' 'client.fetch($URL)' .",
            help: None,
            group: 0,
        },
        CommandEntry {
            name: "test",
            example: "",
            help: None,
            group: 1,
        },
        CommandEntry {
            name: "doc",
            example: "",
            help: None,
            group: 1,
        },
        CommandEntry {
            name: "init",
            example: "",
            help: None,
            group: 2,
        },
        CommandEntry {
            name: "clean",
            example: "",
            help: None,
            group: 2,
        },
        CommandEntry {
            name: "cache",
            example: "",
            help: None,
            group: 2,
        },
        CommandEntry {
            name: "settings",
            example: "",
            help: None,
            group: 2,
        },
        CommandEntry {
            name: "info",
            example: "",
            help: None,
            group: 2,
        },
        CommandEntry {
            name: "targets",
            example: "",
            help: None,
            group: 2,
        },
        CommandEntry {
            name: "task",
            example: "<task>",
            help: None,
            group: 2,
        },
        CommandEntry {
            name: "doctor",
            example: "",
            help: None,
            group: 3,
        },
        CommandEntry {
            name: "explain",
            example: "unresolved-reference",
            help: None,
            group: 3,
        },
        CommandEntry {
            name: "completions",
            example: "zsh",
            help: None,
            group: 3,
        },
        CommandEntry {
            name: "lsp",
            example: "",
            help: None,
            group: 3,
        },
        CommandEntry {
            name: "workspace",
            example: "",
            help: None,
            group: 3,
        },
        CommandEntry {
            name: "version",
            example: "",
            help: None,
            group: 3,
        },
        CommandEntry {
            name: "update",
            example: "",
            help: None,
            group: 3,
        },
        CommandEntry {
            name: "<command>",
            example: "--help",
            help: Some("Print help text for command"),
            group: 3,
        },
    ];

    #[cfg(feature = "dev")]
    let entries = {
        let mut entries = entries;
        entries.insert(
            entries.len() - 1,
            CommandEntry {
                name: "dev",
                example: "",
                help: None,
                group: 3,
            },
        );
        entries
    };

    let command_definition = Cli::command();

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

        let command_label = format!("{:width$}", entry.name, width = command_width);
        let example = format!("{:width$}", entry.example, width = example_width);
        let command_label = if let Some(style) = command_style {
            format!("{style}{command_label}{style:#}")
        } else {
            command_label
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
        let help = entry.help.map(str::to_string).unwrap_or_else(|| {
            command_definition
                .find_subcommand(entry.name)
                .and_then(|subcommand| subcommand.get_about())
                .map(ToString::to_string)
                .unwrap_or_default()
        });

        output.push_str(&format!("  {command_label}  {example}  {help}\n"));
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
            "  {hint_style}or{hint_style:#} {shortcut_style}ds{shortcut_style:#} | {shortcut_style}dsc{shortcut_style:#}"
        );
    }

    "  or ds | dsc".to_string()
}

/// Build the options heading for help output.
fn build_options_heading(color_enabled: bool) -> String {
    if color_enabled {
        let heading_style = AnsiColor::Green.on_default().bold();
        return format!("{heading_style}Options:{heading_style:#}");
    }

    "Options:".to_string()
}

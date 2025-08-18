//! Minimal clap-like parser and CLI dispatcher.

use std::collections::HashMap;
use std::env;

/// Parsed context for a command execution.
#[derive(Debug, Clone)]
pub struct CommandArgs {
    /// Flag arguments with optional values (--key[=value] or -k [value])
    pub flags: HashMap<String, Option<String>>,
    /// Positional arguments
    pub positionals: Vec<String>,
}

impl CommandArgs {
    /// Parse command arguments from an iterator of strings.
    ///
    /// Handles:
    /// - Long flags (`--flag`)
    /// - Short flags (`-f`)
    /// - Flag values (`--key=value` or `--key value`)
    /// - Positional arguments
    /// - The special `--` marker stops flag parsing and treats remaining arguments as positionals
    pub fn parse<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let mut flags: HashMap<String, Option<String>> = HashMap::new();
        let mut positionals: Vec<String> = Vec::new();
        let args: Vec<String> = iter.into_iter().collect();

        let mut i = 0usize;
        while i < args.len() {
            let a = &args[i];

            // stop flag parsing after --
            if a == "--" {
                positionals.extend(args[i + 1..].to_vec());
                break;
            }
            // parse long flags --key[=value]
            else if let Some(body) = a.strip_prefix("--") {
                if let Some(eq) = body.find('=') {
                    let k = body[..eq].to_string();
                    let v = body[eq + 1..].to_string();
                    flags.insert(k, Some(v));
                } else if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    flags.insert(body.to_string(), Some(args[i + 1].clone()));
                    i += 1;
                } else {
                    flags.insert(body.to_string(), None);
                }

            // parse short flags -k [value]
            } else if a.starts_with('-') && a.len() == 2 {
                let k = a[1..].to_string();
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    flags.insert(k, Some(args[i + 1].clone()));
                    i += 1;
                } else {
                    flags.insert(k, None);
                }
            }
            // parse positional arguments
            else {
                positionals.push(a.clone());
            }

            i += 1;
        }

        Self { flags, positionals }
    }

    /// Test presence of a flag.
    pub fn flag(&self, name: &str) -> bool {
        self.flags.contains_key(name)
    }

    /// Get value of an option flag.
    pub fn option(&self, name: &str) -> Option<&str> {
        self.flags.get(name).and_then(|v| v.as_deref())
    }
}

/// Command function signature; return exit code.
pub type CommandFn = fn(CommandArgs) -> i32;

/// A CLI application with sub-CLIs and commands.
#[derive(Debug, Clone)]
pub struct App {
    /// Application name
    pub name: String,
    /// Optional help text for the application
    pub help: Option<String>,
    /// Registered commands with their functions and help text
    commands: HashMap<String, (CommandFn, Option<String>)>,
    /// Nested sub-applications
    sub_apps: HashMap<String, App>,
}

impl App {
    /// Create a new app with a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            help: None,
            commands: HashMap::new(),
            sub_apps: HashMap::new(),
        }
    }

    /// Set help text for the app.
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Register a command.
    pub fn command(
        mut self,
        name: impl Into<String>,
        func: CommandFn,
        help: Option<String>,
    ) -> Self {
        self.commands.insert(name.into(), (func, help));
        self
    }

    /// Register a sub-app under a name.
    pub fn sub_app(mut self, name: impl Into<String>, app: App) -> Self {
        self.sub_apps.insert(name.into(), app);
        self
    }

    /// Run from process arguments.
    pub fn run(&self) -> i32 {
        let mut argv: Vec<String> = env::args().skip(1).collect();
        if argv.is_empty() || argv[0] == "-h" || argv[0] == "--help" {
            return self._show_help();
        }
        let first = argv.remove(0);
        if let Some(app) = self.sub_apps.get(&first) {
            return app.run_with_args(argv);
        }
        if let Some((func, _)) = self.commands.get(&first) {
            let ctx = CommandArgs::parse(argv);
            return (func)(ctx);
        }
        super::console::error(&format!("unknown command: `{first}`"));
        self._show_help()
    }

    /// Run with supplied arguments (first token already consumed as sub-app name).
    pub fn run_with_args(&self, args: Vec<String>) -> i32 {
        if args.is_empty() || args[0] == "-h" || args[0] == "--help" {
            return self._show_help();
        }
        let mut rest = args.clone();
        let first = rest.remove(0);
        if let Some(app) = self.sub_apps.get(&first) {
            return app.run_with_args(rest);
        }
        if let Some((func, _)) = self.commands.get(&first) {
            let ctx = CommandArgs::parse(rest);
            return (func)(ctx);
        }
        super::console::error(&format!("unknown command: `{first}`"));
        self._show_help()
    }

    /// Display help information for this app.
    fn _show_help(&self) -> i32 {
        if let Some(help) = &self.help {
            super::console::print(help);
        }
        if !self.sub_apps.is_empty() {
            super::console::info("CLIs:");
            let mut names: Vec<_> = self.sub_apps.keys().cloned().collect();
            names.sort();
            for n in names {
                println!("  {n}");
            }
        }
        if !self.commands.is_empty() {
            super::console::info("Commands:");
            let mut names: Vec<_> = self.commands.keys().cloned().collect();
            names.sort();
            for n in names {
                let help = self
                    .commands
                    .get(&n)
                    .and_then(|(_, h)| h.as_deref())
                    .unwrap_or("");
                println!("  {n}  {help}");
            }
        }
        1
    }
}

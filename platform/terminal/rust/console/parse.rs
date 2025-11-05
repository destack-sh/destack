//! Minimal clap-like parser and CLI dispatcher.

use std::collections::{HashMap, HashSet};
use std::env;

/// Parsed context for a command execution.
#[derive(Debug, Clone)]
pub struct CommandArguments {
    /// Flag arguments with optional values (--key[=value] or -k [value])
    pub flags: HashMap<String, Option<String>>,
    /// Positional arguments
    pub positionals: Vec<String>,
}

impl CommandArguments {
    /// Parse command arguments from an iterator of strings with:
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

            // parse long flags --key[=value]
            } else if let Some(body) = a.strip_prefix("--") {
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

            // parse positional arguments
            } else {
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

    /// Get a positional argument by index if present.
    pub fn positional(&self, index: usize) -> Option<&str> {
        self.positionals.get(index).map(|value| value.as_str())
    }

    /// Get the first positional argument if present.
    pub fn first_positional(&self) -> Option<&str> {
        self.positional(0)
    }
}

/// Command function signature; return exit code.
pub type CommandFn = fn(CommandArguments) -> i32;

/// A CLI application with sub-CLIs and commands.
#[derive(Debug, Clone)]
pub struct CommandApp {
    /// Application name
    pub name: String,
    /// Optional help text for the application
    pub help: Option<String>,
    /// Default command to run if no command is provided
    pub default_command: Option<String>,
    /// Registered commands with their functions and help text
    commands: HashMap<String, (CommandFn, Option<String>)>,
    /// Nested sub-applications
    sub_apps: HashMap<String, CommandApp>,
    /// Alias mapping from shorthand name to command or CLI id
    aliases: HashMap<String, String>,
}

impl CommandApp {
    /// Create a new app with a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            help: None,
            default_command: None,
            commands: HashMap::new(),
            sub_apps: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    /// Set help text for the app.
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Set default command to run if no command is provided
    pub fn default_command(mut self, command: impl Into<String>) -> Self {
        self.default_command = Some(command.into());
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
    pub fn sub_app(mut self, name: impl Into<String>, app: CommandApp) -> Self {
        self.sub_apps.insert(name.into(), app);
        self
    }

    /// Register an alias pointing to a command or CLI name.
    pub fn alias(mut self, alias: impl Into<String>, target: impl Into<String>) -> Self {
        let alias = alias.into();
        let target = target.into();
        if alias != target {
            self.aliases.insert(alias, target);
        }
        self
    }

    /// Run from process arguments.
    pub fn run(&self) -> i32 {
        // collect command line arguments, skipping program name, and delegate
        let argv: Vec<String> = env::args().skip(1).collect();
        self.run_with_args(argv)
    }

    /// Run with supplied arguments (first token already consumed as sub-app name).
    pub fn run_with_args(&self, args: Vec<String>) -> i32 {
        // help flag
        if (!args.is_empty()) && (args[0] == "-h" || args[0] == "--help") {
            return self._show_help();
        }

        // if no args or first arg looks like a flag, try default command
        if args.is_empty() || args[0].starts_with('-') {
            if let Some(default) = &self.default_command {
                let resolved_default = match self.resolve_alias(default) {
                    Ok(value) => value,
                    Err(message) => {
                        super::error(&message);
                        return 1;
                    }
                };
                // if default refers to a sub-app, delegate with current args
                if let Some(app) = self.sub_apps.get(&resolved_default) {
                    return app.run_with_args(args);
                }
                // if default refers to a command, execute it with parsed args
                if let Some((func, _)) = self.commands.get(&resolved_default) {
                    let ctx = CommandArguments::parse(args);
                    return (func)(ctx);
                }
                // NOTE #Robustness: unknown default configured, fall through to help
            }
            return self._show_help();
        }

        // extract first argument as command/sub-app name and resolve aliases
        let mut rest = args.clone();
        let original_first = rest.remove(0);
        let token = match self.resolve_alias(&original_first) {
            Ok(value) => value,
            Err(message) => {
                super::error(&message);
                return 1;
            }
        };

        // try to run as sub-app first
        if let Some(app) = self.sub_apps.get(&token) {
            return app.run_with_args(rest);
        }

        // try to run as command
        if let Some((func, _)) = self.commands.get(&token) {
            let ctx = CommandArguments::parse(rest);
            return (func)(ctx);
        }

        // command not found, show error and help
        super::error(&format!("unknown command: `{original_first}`"));
        self._show_help()
    }

    fn resolve_alias(&self, name: &str) -> Result<String, String> {
        let mut current = name.to_string();
        let mut visited = HashSet::new();
        while let Some(mapped) = self.aliases.get(&current) {
            if !visited.insert(current.clone()) {
                return Err(format!("alias loop detected for `{current}`"));
            }
            current = mapped.clone();
        }
        Ok(current)
    }

    /// Display help information for this app.
    fn _show_help(&self) -> i32 {
        // show main help text if available
        if let Some(help) = &self.help {
            super::print(help);
        }

        // list available sub-apps
        if !self.sub_apps.is_empty() {
            super::info("CLIs:");
            let mut names: Vec<_> = self.sub_apps.keys().cloned().collect();
            names.sort();
            for n in names {
                println!("  {n}");
            }
        }

        // list available commands with help text
        if !self.commands.is_empty() {
            super::info("Commands:");
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

        if !self.aliases.is_empty() {
            super::info("Aliases:");
            let mut names: Vec<_> = self.aliases.keys().cloned().collect();
            names.sort();
            for alias in names {
                if let Some(target) = self.aliases.get(&alias) {
                    match self.resolve_alias(target) {
                        Ok(resolved) => {
                            let kind = if self.sub_apps.contains_key(&resolved) {
                                "cli"
                            } else if self.commands.contains_key(&resolved) {
                                "command"
                            } else {
                                "target"
                            };
                            println!(
                                "  {} -> {} ({kind})",
                                super::bold(&alias),
                                super::bold(&resolved)
                            );
                        }
                        Err(message) => {
                            super::warn(&format!(
                                "{} -> {target} ({message})",
                                super::bold(&alias)
                            ));
                        }
                    }
                }
            }
        }
        1
    }
}

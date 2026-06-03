use std::path::Path;
use zed_extension_api::settings::LspSettings;
use zed_extension_api::{self as zed, LanguageServerId, Result};

/// A resolved language server command and its baseline arguments.
struct ResolvedCommand {
    /// The executable path.
    command: String,
    /// Arguments passed to the executable.
    args: Vec<String>,
}

/// The zed extension entry point for destack language features.
pub(crate) struct DestackExtension;

impl DestackExtension {
    /// Build an error message for missing lsp binaries.
    fn missing_binary_message(worktree: &zed::Worktree) -> String {
        let root = worktree.root_path();
        let workspace_candidates = Self::workspace_binary_candidates(worktree).join(", ");
        format!(
            "could not find Destack binary in worktree `{root}`: checked workspace binaries [{workspace_candidates}] and PATH commands `destack`, `ds`, `dsc`; set lsp.destack-lsp.binary.path to a shared binary path, or build `target/{{debug,release}}/destack` in this worktree"
        )
    }

    /// Return workspace-local destack binary candidates.
    fn workspace_binary_candidates(worktree: &zed::Worktree) -> Vec<String> {
        let (os, _) = zed::current_platform();
        let binary_name = if matches!(os, zed::Os::Windows) {
            "destack.exe"
        } else {
            "destack"
        };

        let root = worktree.root_path();
        vec![
            Path::new(&root)
                .join("target")
                .join("release")
                .join(binary_name)
                .to_string_lossy()
                .to_string(),
            Path::new(&root)
                .join("target")
                .join("debug")
                .join(binary_name)
                .to_string_lossy()
                .to_string(),
        ]
    }

    /// Return true when a command can execute successfully.
    fn command_works(command: &str, args: &[&str]) -> bool {
        let mut probe = zed::process::Command::new(command);
        for arg in args {
            probe = probe.arg(*arg);
        }

        match probe.output() {
            Ok(output) => output.status == Some(0),
            Err(_) => false,
        }
    }

    /// Resolve a workspace-local destack binary when available.
    fn workspace_binary_command(worktree: &zed::Worktree) -> Option<ResolvedCommand> {
        for candidate in Self::workspace_binary_candidates(worktree) {
            if Self::command_works(&candidate, &["--version"]) {
                return Some(ResolvedCommand {
                    command: candidate,
                    args: vec!["lsp".to_string()],
                });
            }
        }

        None
    }

    /// Resolve a default lsp command in the worktree path.
    fn fallback_command(worktree: &zed::Worktree) -> Option<ResolvedCommand> {
        if let Some(command) = Self::workspace_binary_command(worktree) {
            return Some(command);
        }

        if let Some(command) = worktree
            .which("destack")
            .or_else(|| worktree.which("ds"))
            .or_else(|| worktree.which("dsc"))
        {
            return Some(ResolvedCommand {
                command,
                args: vec!["lsp".to_string()],
            });
        }

        None
    }

    /// Return true when the command should receive a leading `lsp` subcommand.
    pub(crate) fn should_inject_lsp_subcommand(command: &str, args: &[String]) -> bool {
        if args.first().is_some_and(|value| value == "lsp") {
            return false;
        }

        let Some(file_name) = Path::new(command)
            .file_name()
            .and_then(|name| name.to_str())
        else {
            return false;
        };
        let file_name = file_name.to_ascii_lowercase();
        let normalized = file_name.strip_suffix(".exe").unwrap_or(file_name.as_str());

        matches!(normalized, "destack" | "ds" | "dsc")
    }

    /// Add the `lsp` subcommand for known destack binaries when missing.
    pub(crate) fn inject_lsp_subcommand(command: &str, mut args: Vec<String>) -> Vec<String> {
        if !Self::should_inject_lsp_subcommand(command, &args) {
            return args;
        }

        args.insert(0, "lsp".to_string());
        args
    }

    /// Resolve final command arguments from configured and fallback values.
    pub(crate) fn resolve_command_args(
        command: &str,
        configured_args: Option<Vec<String>>,
        fallback_args: Vec<String>,
    ) -> Vec<String> {
        match configured_args {
            // respect explicit settings and still inject for known wrapper binaries
            Some(args) => Self::inject_lsp_subcommand(command, args),
            // use fallback args only when settings do not provide arguments
            None => fallback_args,
        }
    }
}

impl zed::Extension for DestackExtension {
    /// Create the extension instance.
    fn new() -> Self {
        Self
    }

    /// Build the lsp command for the requested language server.
    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree).ok();
        let binary_settings = settings
            .as_ref()
            .and_then(|lsp_settings| lsp_settings.binary.as_ref());
        let fallback = Self::fallback_command(worktree);

        let command = binary_settings
            .and_then(|binary| binary.path.clone())
            .or_else(|| fallback.as_ref().map(|resolved| resolved.command.clone()))
            .ok_or_else(|| Self::missing_binary_message(worktree))?;

        let configured_args = binary_settings.and_then(|binary| binary.arguments.clone());
        let fallback_args = fallback
            .as_ref()
            .filter(|resolved| resolved.command == command)
            .map(|resolved| resolved.args.clone())
            .unwrap_or_default();
        let args = Self::resolve_command_args(&command, configured_args, fallback_args);
        let env = binary_settings
            .and_then(|binary| binary.env.clone())
            .map(|env| env.into_iter().collect())
            .unwrap_or_default();

        Ok(zed::Command { command, args, env })
    }

    /// Forward language server initialization options from zed settings.
    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .map(|lsp_settings| lsp_settings.initialization_options.clone())
    }

    /// Forward language server workspace configuration from zed settings.
    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        LspSettings::for_worktree(language_server_id.as_ref(), worktree)
            .map(|lsp_settings| lsp_settings.settings.clone())
    }
}

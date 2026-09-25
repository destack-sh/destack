use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use zed::settings::LspSettings;
use zed::{
    Architecture, DownloadedFileType, LanguageServerId, LanguageServerInstallationStatus, Os,
    Result,
};
use zed_extension_api as zed;

const COMMAND_NAME: &str = "tspp";
const RELEASE_REPOSITORY: &str = "destack-sh/tspp";

/// One supported TS++ release target.
struct ReleaseTarget {
    /// The Rust target triple.
    triple: &'static str,
    /// The release archive type.
    archive_type: DownloadedFileType,
    /// The release archive extension.
    archive_extension: &'static str,
    /// The platform executable name.
    executable_name: &'static str,
}

/// The resolved TS++ language server command.
struct ServerCommand {
    /// The executable path.
    command: String,
    /// Arguments passed to the executable.
    arguments: Vec<String>,
    /// Environment variables passed to the executable.
    environment: HashMap<String, String>,
}

impl ServerCommand {
    /// Create one command and add the TS++ LSP subcommand when required.
    fn new(
        command: String,
        mut arguments: Vec<String>,
        environment: HashMap<String, String>,
    ) -> Result<Self> {
        let Some(executable_name) = Path::new(&command)
            .file_name()
            .and_then(|name| name.to_str())
        else {
            return Err(format!("invalid TS++ language server command: `{command}`"));
        };
        let executable_name = executable_name.to_ascii_lowercase();
        let executable_name = match executable_name.strip_suffix(".exe") {
            Some(executable_name) => executable_name,
            None => &executable_name,
        };
        if executable_name == COMMAND_NAME && arguments.first().is_none_or(|value| value != "lsp") {
            arguments.insert(0, "lsp".to_string());
        }

        Ok(Self {
            command,
            arguments,
            environment,
        })
    }
}

impl From<ServerCommand> for zed::Command {
    /// Build the command accepted by the Zed extension host.
    fn from(command: ServerCommand) -> Self {
        Self {
            command: command.command,
            args: command.arguments,
            env: command.environment.into_iter().collect(),
        }
    }
}

/// The Zed extension entry point for TS++ language support.
pub(crate) struct TsppExtension {
    /// The managed binary resolved during this extension session.
    managed_command: Option<String>,
}

impl TsppExtension {
    /// Resolve the complete language server command.
    fn resolve_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<ServerCommand> {
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)?;
        let binary = settings.binary.as_ref();
        let arguments = binary
            .and_then(|settings| settings.arguments.clone())
            .unwrap_or_default();
        let environment = binary
            .and_then(|settings| settings.env.clone())
            .unwrap_or_default();

        // use the configured command as the authoritative selection
        if let Some(command) = binary.and_then(|settings| settings.path.clone()) {
            return ServerCommand::new(command, arguments, environment);
        }

        // prefer repository builds during toolchain development
        if let Some(command) = Self::resolve_workspace_command(worktree)? {
            return ServerCommand::new(command, arguments, environment);
        }

        // use an installed toolchain when available
        if let Some(command) = worktree.which(COMMAND_NAME) {
            return ServerCommand::new(command, arguments, environment);
        }

        // install the matching release for a clean editor installation
        let command = self.resolve_managed_command(language_server_id)?;

        ServerCommand::new(command, arguments, environment)
    }

    /// Resolve the first executable workspace build.
    fn resolve_workspace_command(worktree: &zed::Worktree) -> Result<Option<String>> {
        let executable_name = if matches!(zed::current_platform().0, Os::Windows) {
            "tspp.exe"
        } else {
            COMMAND_NAME
        };
        let root = PathBuf::from(worktree.root_path());
        let candidates = [
            root.join("target").join("release").join(executable_name),
            root.join("target").join("debug").join(executable_name),
        ];

        // probe each candidate in build-profile order
        for candidate in candidates {
            let command = candidate.into_os_string().into_string().map_err(|path| {
                format!(
                    "TS++ workspace command is not valid UTF-8: {}",
                    PathBuf::from(path).display()
                )
            })?;
            if Self::command_works(&command) {
                return Ok(Some(command));
            }
        }

        Ok(None)
    }

    /// Return whether one command can execute successfully.
    fn command_works(command: &str) -> bool {
        zed::process::Command::new(command)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status == Some(0))
    }

    /// Resolve or install the toolchain release matching this extension.
    fn resolve_managed_command(&mut self, language_server_id: &LanguageServerId) -> Result<String> {
        if let Some(command) = self.managed_command.as_ref()
            && Self::command_works(command)
        {
            return Ok(command.clone());
        }

        let version = env!("CARGO_PKG_VERSION");
        let target = Self::release_target()?;
        let package_name = format!("tspp-{version}-{}", target.triple);
        let release_directory = Path::new("server").join(&package_name);
        let command = release_directory
            .join(&package_name)
            .join(target.executable_name);
        let command = command.into_os_string().into_string().map_err(|path| {
            format!(
                "TS++ managed command is not valid UTF-8: {}",
                PathBuf::from(path).display()
            )
        })?;
        if !Self::command_works(&command) {
            let install = Self::install(language_server_id, version, &target, &release_directory);
            if let Err(error) = install {
                zed::set_language_server_installation_status(
                    language_server_id,
                    &LanguageServerInstallationStatus::Failed(error.clone()),
                );

                return Err(error);
            }
        }
        if !Self::command_works(&command) {
            return Err(format!("installed TS++ release cannot run: {command}"));
        }

        self.managed_command = Some(command.clone());

        Ok(command)
    }

    /// Install one TS++ release archive.
    fn install(
        language_server_id: &LanguageServerId,
        version: &str,
        target: &ReleaseTarget,
        release_directory: &Path,
    ) -> Result<()> {
        zed::set_language_server_installation_status(
            language_server_id,
            &LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let release = zed::github_release_by_tag_name(RELEASE_REPOSITORY, &format!("v{version}"))?;
        let archive_name = format!(
            "tspp-{version}-{}.{}",
            target.triple, target.archive_extension
        );
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == archive_name)
            .ok_or_else(|| format!("TS++ release has no asset named {archive_name}"))?;

        // download and extract the exact extension release
        zed::set_language_server_installation_status(
            language_server_id,
            &LanguageServerInstallationStatus::Downloading,
        );
        if release_directory.exists() {
            fs::remove_dir_all(release_directory).map_err(|error| {
                format!(
                    "failed to remove incomplete TS++ release {}: {error}",
                    release_directory.display()
                )
            })?;
        }
        let release_directory = release_directory.to_str().ok_or_else(|| {
            format!(
                "TS++ release directory is not valid UTF-8: {}",
                release_directory.display()
            )
        })?;
        zed::download_file(&asset.download_url, release_directory, target.archive_type)?;

        // restore executable permissions removed by archive transport
        if !matches!(zed::current_platform().0, Os::Windows) {
            let command = Path::new(release_directory)
                .join(format!("tspp-{version}-{}", target.triple))
                .join(target.executable_name);
            let command = command.to_str().ok_or_else(|| {
                format!(
                    "TS++ managed command is not valid UTF-8: {}",
                    command.display()
                )
            })?;
            zed::make_file_executable(command)?;
        }
        zed::set_language_server_installation_status(
            language_server_id,
            &LanguageServerInstallationStatus::None,
        );

        Ok(())
    }

    /// Resolve the active release target.
    fn release_target() -> Result<ReleaseTarget> {
        let (operating_system, architecture) = zed::current_platform();
        match (operating_system, architecture) {
            (Os::Mac, Architecture::Aarch64) => Ok(ReleaseTarget {
                triple: "aarch64-apple-darwin",
                archive_type: DownloadedFileType::GzipTar,
                archive_extension: "tar.gz",
                executable_name: COMMAND_NAME,
            }),
            (Os::Mac, Architecture::X8664) => Ok(ReleaseTarget {
                triple: "x86_64-apple-darwin",
                archive_type: DownloadedFileType::GzipTar,
                archive_extension: "tar.gz",
                executable_name: COMMAND_NAME,
            }),
            (Os::Linux, Architecture::Aarch64) => Ok(ReleaseTarget {
                triple: "aarch64-unknown-linux-gnu",
                archive_type: DownloadedFileType::GzipTar,
                archive_extension: "tar.gz",
                executable_name: COMMAND_NAME,
            }),
            (Os::Linux, Architecture::X8664) => Ok(ReleaseTarget {
                triple: "x86_64-unknown-linux-gnu",
                archive_type: DownloadedFileType::GzipTar,
                archive_extension: "tar.gz",
                executable_name: COMMAND_NAME,
            }),
            (Os::Windows, Architecture::X8664) => Ok(ReleaseTarget {
                triple: "x86_64-pc-windows-msvc",
                archive_type: DownloadedFileType::Zip,
                archive_extension: "zip",
                executable_name: "tspp.exe",
            }),
            _ => Err(format!(
                "TS++ has no release for {operating_system:?}/{architecture:?}"
            )),
        }
    }
}

impl zed::Extension for TsppExtension {
    /// Create the extension instance.
    fn new() -> Self {
        Self {
            managed_command: None,
        }
    }

    /// Build the LSP command for the requested language server.
    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let command = self.resolve_server_command(language_server_id, worktree)?;

        Ok(command.into())
    }

    /// Forward language server initialization options from Zed settings.
    fn language_server_initialization_options(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)?;

        Ok(settings.initialization_options)
    }

    /// Forward language server workspace configuration from Zed settings.
    fn language_server_workspace_configuration(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<Option<zed::serde_json::Value>> {
        let settings = LspSettings::for_worktree(language_server_id.as_ref(), worktree)?;

        Ok(settings.settings)
    }
}

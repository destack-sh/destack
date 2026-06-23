use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use clap::{Args, ValueEnum};

use crate::command::dev::version;
use crate::console;

/// Version bump kinds for integrated release flows.
#[derive(ValueEnum, Clone, Debug)]
pub enum ReleaseBumpKind {
    /// Bump major version (X.0.0).
    Major,
    /// Bump minor version (x.Y.0).
    Minor,
    /// Bump patch version (x.y.Z).
    Patch,
}

impl ReleaseBumpKind {
    /// Convert to a version command variant.
    fn to_version_command(&self) -> version::VersionCommands {
        match self {
            Self::Major => version::VersionCommands::Major,
            Self::Minor => version::VersionCommands::Minor,
            Self::Patch => version::VersionCommands::Patch,
        }
    }
}

/// Arguments for the integrated release command.
#[derive(Args, Clone, Debug)]
pub struct ReleaseArgs {
    /// Optionally bump the version before running release steps.
    #[arg(long, value_enum)]
    pub bump: Option<ReleaseBumpKind>,

    /// Explicit release version override (defaults to VERSION.txt when unset).
    #[arg(long)]
    pub version: Option<String>,

    /// Artifact directory passed to CLI preflight recipes.
    #[arg(long, default_value = "app/cli/install/artifacts")]
    pub artifacts: String,

    /// Skip preflight checks and staging.
    #[arg(long = "skip-preflight")]
    pub is_skip_preflight: bool,

    /// Publish instead of dry run.
    #[arg(long = "publish")]
    pub is_publish: bool,

    /// Include zed registry publish in the release flow.
    #[arg(long = "publish-zed")]
    pub is_publish_zed: bool,
}

/// Run the integrated release flow.
pub fn run(args: &ReleaseArgs) -> i32 {
    // verify repository root context before running just recipes
    if let Err(error) = validate_repository_root() {
        console::error(&format!("error: {error}"));
        return 1;
    }

    // optionally bump the version first
    if let Some(bump_kind) = &args.bump {
        let version_command = bump_kind.to_version_command();
        let exit_code = version::bump(&version_command);
        if exit_code != 0 {
            return exit_code;
        }
    }

    // resolve the optional version argument for just recipes
    let version_argument = args.version.clone().unwrap_or_default();

    // run preflight checks unless explicitly skipped
    if !args.is_skip_preflight {
        let preflight_arguments = vec![
            OsString::from("app/preflight-cli-publish"),
            OsString::from(&version_argument),
            OsString::from(&args.artifacts),
        ];
        let preflight_exit_code = run_just_command(&preflight_arguments);
        if preflight_exit_code != 0 {
            return preflight_exit_code;
        }
    }

    // run CLI publish in dry mode by default
    let publish_mode_argument = if args.is_publish {
        OsString::from("")
    } else {
        OsString::from("--dry-run")
    };
    let cli_publish_arguments = vec![OsString::from("app/publish"), publish_mode_argument.clone()];
    let cli_publish_exit_code = run_just_command(&cli_publish_arguments);
    if cli_publish_exit_code != 0 {
        return cli_publish_exit_code;
    }

    // run vscode publish to match the integrated release flow
    let vscode_publish_arguments = vec![
        OsString::from("bridge/publish-vscode"),
        publish_mode_argument.clone(),
    ];
    let vscode_publish_exit_code = run_just_command(&vscode_publish_arguments);
    if vscode_publish_exit_code != 0 {
        return vscode_publish_exit_code;
    }

    // optionally include zed registry publish
    if args.is_publish_zed {
        let zed_publish_arguments = vec![
            OsString::from("bridge/publish-zed"),
            publish_mode_argument,
            OsString::from(&version_argument),
        ];
        return run_just_command(&zed_publish_arguments);
    }

    0
}

/// Validate that the current directory looks like the repository root.
fn validate_repository_root() -> Result<(), String> {
    // ensure the version file exists
    if !Path::new("VERSION.txt").exists() {
        return Err("VERSION.txt not found, run this command from the repository root".to_string());
    }

    // ensure app and bridge justfiles exist
    if !Path::new("app/justfile").exists() {
        return Err(
            "app/justfile not found, run this command from the repository root".to_string(),
        );
    }
    if !Path::new("bridge/justfile").exists() {
        return Err(
            "bridge/justfile not found, run this command from the repository root".to_string(),
        );
    }

    Ok(())
}

/// Run a just command and return the corresponding exit code.
fn run_just_command(arguments: &[OsString]) -> i32 {
    // render and print the command for operator visibility
    let rendered_arguments = arguments
        .iter()
        .map(|argument| argument.to_string_lossy().to_string())
        .collect::<Vec<String>>()
        .join(" ");
    console::info(&format!("run: just {rendered_arguments}"));

    // spawn the command with inherited stdio
    let status = Command::new("just")
        .args(arguments)
        .status()
        .map_err(|error| format!("failed to run just: {error}"));

    // report command execution failures
    let status = match status {
        Ok(status) => status,
        Err(error) => {
            console::error(&format!("error: {error}"));
            return 1;
        }
    };

    // return the exit code when present
    if let Some(exit_code) = status.code() {
        return exit_code;
    }

    // return a generic failure when exit code is unavailable
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_release_bump_kind_maps_to_version_command() {
        assert!(matches!(
            ReleaseBumpKind::Major.to_version_command(),
            version::VersionCommands::Major
        ));
        assert!(matches!(
            ReleaseBumpKind::Minor.to_version_command(),
            version::VersionCommands::Minor
        ));
        assert!(matches!(
            ReleaseBumpKind::Patch.to_version_command(),
            version::VersionCommands::Patch
        ));
    }
}

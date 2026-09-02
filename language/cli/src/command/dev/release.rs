use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use clap::Args;

use crate::console;

/// Arguments for the integrated release command.
#[derive(Args, Clone, Debug)]
pub struct ReleaseArgs {
    /// Explicit release version override (defaults to `destack.json` when unset).
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
}

/// Run the integrated release flow.
pub fn run(args: &ReleaseArgs) -> i32 {
    // verify repository root context before running just recipes
    if let Err(error) = validate_repository_root() {
        console::error(&format!("error: {error}"));
        return 1;
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

    // run the complete publish flow in dry mode by default
    let publish_arguments = if args.is_publish {
        vec![OsString::from("publish-release")]
    } else {
        vec![OsString::from("publish"), OsString::from("--dry-run")]
    };

    run_just_command(&publish_arguments)
}

/// Validate that the current directory looks like the repository root.
fn validate_repository_root() -> Result<(), String> {
    // ensure the workspace manifest exists
    if !Path::new("destack.json").exists() {
        return Err(
            "destack.json not found, run this command from the repository root".to_string(),
        );
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

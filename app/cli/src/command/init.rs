use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::{Args, ValueEnum};
use destack_source::{FileSystem, PhysicalFileSystem};
use serde_json::json;

use crate::common::{
    CommandError, CommandReport, FileSystemOverride, ReportArgs, print_json_payload_report,
    print_report, report_error,
};
use crate::console;

/// Template type for initialization.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum Template {
    /// Minimal project with just destack.json.
    #[default]
    Minimal,
    /// Library project with src/index.ds.
    Lib,
    /// Application project with src/main.ds.
    App,
}

/// Arguments for the init command.
#[derive(Args, Debug, Clone)]
pub struct InitArgs {
    /// The directory to initialize (default: current directory).
    #[arg(value_name = "DIR")]
    pub dir: Option<PathBuf>,

    /// Project name (defaults to directory name).
    #[arg(long)]
    pub name: Option<String>,

    /// Project template (minimal|lib|app, default: minimal).
    #[arg(long, short = 't', value_enum, default_value = "minimal")]
    pub template: Template,

    /// Force initialization even if files exist.
    #[arg(long, short = 'f')]
    pub force: bool,

    /// Test only file system override.
    #[arg(skip)]
    pub fs_override: Option<FileSystemOverride>,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

/// JSON payload for init output.
#[derive(serde::Serialize)]
struct InitPayload {
    /// Resolved project name.
    name: String,
    /// Target directory for the project.
    directory: String,
    /// Created filesystem entries.
    created: Vec<String>,
}

/// Initialize a new Destack project.
pub fn run(args: &InitArgs) -> i32 {
    // resolve the target directory
    let dir = match args.dir.clone() {
        Some(dir) => dir,
        None => match std::env::current_dir() {
            Ok(dir) => dir,
            Err(error) => {
                return report_error(
                    "init",
                    &args.report,
                    &format!("failed to read current dir: {error}"),
                );
            }
        },
    };

    // select the file system implementation
    let fs: Arc<dyn FileSystem> = args
        .fs_override
        .as_ref()
        .map(FileSystemOverride::fs)
        .unwrap_or_else(|| Arc::new(PhysicalFileSystem::new()));

    let mut created = Vec::new();

    // ensure directory exists
    let dir_exists = match fs.exists(&dir) {
        Ok(exists) => exists,
        Err(e) => {
            return report_error(
                "init",
                &args.report,
                &format!("failed to check directory: {e}"),
            );
        }
    };
    if !dir_exists {
        if let Err(e) = fs.create_dir_all(&dir) {
            return report_error(
                "init",
                &args.report,
                &format!("failed to create directory: {e}"),
            );
        }
        if !args.report.is_json() {
            console::info(&format!("created directory: {}", dir.display()));
        }
        created.push(dir.display().to_string());
    }

    // determine project name
    let name = args.name.clone().unwrap_or_else(|| {
        dir.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "destack-project".to_string())
    });

    // create destack.json
    let destack_config_path = dir.join("destack.json");
    let destack_config_exists = match fs.exists(&destack_config_path) {
        Ok(exists) => exists,
        Err(e) => {
            return report_error(
                "init",
                &args.report,
                &format!("failed to check destack.json: {e}"),
            );
        }
    };
    if destack_config_exists && !args.force {
        if !args.report.is_json() {
            console::warn("destack.json already exists (use --force to overwrite)");
        }
    } else {
        let config = create_destack_config(&name, args.template);
        if let Err(e) = fs.write(destack_config_path.as_path(), config.as_bytes()) {
            return report_error(
                "init",
                &args.report,
                &format!("failed to write destack.json: {e}"),
            );
        }
        if !args.report.is_json() {
            console::info(&format!("created {}", destack_config_path.display()));
        }
        created.push(destack_config_path.display().to_string());
    }

    // create source files based on template
    match args.template {
        Template::Minimal => {
            // no source files for minimal template
        }
        Template::Lib => {
            if let Err(code) = create_source_file(
                fs.as_ref(),
                &dir,
                "src/index.ds",
                LIB_TEMPLATE,
                args.force,
                args.report.is_json(),
            ) {
                return report_code(args, code);
            }
        }
        Template::App => {
            if let Err(code) = create_source_file(
                fs.as_ref(),
                &dir,
                "src/main.ds",
                APP_TEMPLATE,
                args.force,
                args.report.is_json(),
            ) {
                return report_code(args, code);
            }
        }
    }

    // emit json report or text output
    if args.report.is_json() {
        let payload = InitPayload {
            name,
            directory: dir.display().to_string(),
            created,
        };
        if let Err(code) = print_json_payload_report("init", &args.report, 0, &payload) {
            return code;
        }
    } else {
        console::success(&format!("initialized Destack project: {name}"));
    }
    0
}

/// Report an initialization failure when already returning a code.
fn report_code(args: &InitArgs, code: i32) -> i32 {
    // forward non json output codes
    if code == 0 || !args.report.is_json() {
        return code;
    }

    // emit a failure report for json output
    let message = "failed to create project files";
    let mut report = CommandReport::failure("init", code);
    report.summary = Some(message.to_string());
    report.error = Some(CommandError::new("init_failed", "init", message));
    print_report(&report, args.report.format());
    code
}

/// Create a source file from a template.
fn create_source_file(
    fs: &dyn FileSystem,
    dir: &Path,
    rel_path: &str,
    content: &str,
    force: bool,
    suppress_output: bool,
) -> Result<(), i32> {
    // build the absolute path
    let file_path = dir.join(rel_path);

    // ensure parent directory exists
    if let Some(parent) = file_path.parent() {
        let parent_exists = match fs.exists(parent) {
            Ok(exists) => exists,
            Err(e) => {
                console::error(&format!("failed to check directory: {e}"));
                return Err(1);
            }
        };
        if !parent_exists && let Err(e) = fs.create_dir_all(parent) {
            console::error(&format!("failed to create directory: {e}"));
            return Err(1);
        }
    }

    // handle existing files without force
    let file_exists = match fs.exists(&file_path) {
        Ok(exists) => exists,
        Err(e) => {
            console::error(&format!("failed to check file: {e}"));
            return Err(1);
        }
    };
    if file_exists && !force {
        if !suppress_output {
            console::warn(&format!(
                "{rel_path} already exists (use --force to overwrite)",
            ));
        }
    } else {
        // write the template content
        if let Err(e) = fs.write(&file_path, content.as_bytes()) {
            console::error(&format!("failed to write {rel_path}: {e}"));
            return Err(1);
        }
        if !suppress_output {
            console::info(&format!("created {}", file_path.display()));
        }
    }

    Ok(())
}

/// Build the destack.json content for a template.
fn create_destack_config(_name: &str, template: Template) -> String {
    // select include patterns by template
    let (include, root_dir) = match template {
        Template::Minimal => (vec!["**/*.ds", "**/*.ts"], None),
        Template::Lib | Template::App => (vec!["src/**/*"], Some("src")),
    };

    // define compiler options for the template
    let mut compiler = json!({
        "target": "esnext",
        "module": "esnext",
    });
    // set rootDir for src-based templates
    if let Some(root_dir) = root_dir {
        compiler["rootDir"] = json!(root_dir);
    }

    // build the config payload
    let config = json!({
        "$schema": "https://destack.sh/schemas/destack.schema.json",
        "compiler": compiler,
        "include": include,
        "exclude": ["dist"],
    });

    // serialize the config json
    serde_json::to_string_pretty(&config).unwrap_or_else(|_| config.to_string())
}

const LIB_TEMPLATE: &str = r#"/// Library entry point.

export function greet(name: string): string {
    `Hello, ${name}!`
}
"#;

const APP_TEMPLATE: &str = r#"/// Application entry point.

function main() {
    console.log("Hello, Destack!");
}

main();
"#;

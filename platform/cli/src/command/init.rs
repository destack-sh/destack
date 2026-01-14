use std::fs;
use std::path::{Path, PathBuf};

use clap::{Args, ValueEnum};
use serde_json::json;

use crate::common::{CommandReport, ReportArgs, print_report, report_error};
use crate::console;

/// Template type for initialization.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum Template {
    /// Minimal project with just dsconfig.json.
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

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
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

    let mut created = Vec::new();

    // ensure directory exists
    if !dir.exists() {
        if let Err(e) = fs::create_dir_all(&dir) {
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

    // create dsconfig.json
    let dsconfig_path = dir.join("dsconfig.json");
    if dsconfig_path.exists() && !args.force {
        if !args.report.is_json() {
            console::warn("dsconfig.json already exists (use --force to overwrite)");
        }
    } else {
        let dsconfig = create_dsconfig(&name, args.template);
        if let Err(e) = fs::write(&dsconfig_path, dsconfig) {
            return report_error(
                "init",
                &args.report,
                &format!("failed to write dsconfig.json: {e}"),
            );
        }
        if !args.report.is_json() {
            console::info(&format!("created {}", dsconfig_path.display()));
        }
        created.push(dsconfig_path.display().to_string());
    }

    // create source files based on template
    match args.template {
        Template::Minimal => {
            // no source files for minimal template
        }
        Template::Lib => {
            if let Err(code) = create_source_file(
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
        let mut report = CommandReport::success("init", 0);
        report.data = Some(json!({
            "name": name,
            "directory": dir.display().to_string(),
            "created": created,
        }));
        print_report(&report, args.report.format());
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
    let mut report = CommandReport::failure("init", code);
    report.summary = Some("failed to create project files".to_string());
    print_report(&report, args.report.format());
    code
}

/// Create a source file from a template.
fn create_source_file(
    dir: &Path,
    rel_path: &str,
    content: &str,
    force: bool,
    suppress_output: bool,
) -> Result<(), i32> {
    // build the absolute path
    let file_path = dir.join(rel_path);

    // ensure parent directory exists
    if let Some(parent) = file_path.parent()
        && !parent.exists()
        && let Err(e) = fs::create_dir_all(parent)
    {
        console::error(&format!("failed to create directory: {e}"));
        return Err(1);
    }

    // handle existing files without force
    if file_path.exists() && !force {
        if !suppress_output {
            console::warn(&format!(
                "{rel_path} already exists (use --force to overwrite)",
            ));
        }
    } else {
        // write the template content
        if let Err(e) = fs::write(&file_path, content) {
            console::error(&format!("failed to write {rel_path}: {e}"));
            return Err(1);
        }
        if !suppress_output {
            console::info(&format!("created {}", file_path.display()));
        }
    }

    Ok(())
}

/// Build the dsconfig.json content for a template.
fn create_dsconfig(_name: &str, template: Template) -> String {
    // select include patterns by template
    let include = match template {
        Template::Minimal => r#""include": ["**/*.ds", "**/*.ts"]"#,
        Template::Lib | Template::App => r#""include": ["src/**/*.ds", "src/**/*.ts"]"#,
    };

    format!(
        r#"{{
  "$schema": "https://destack.sh/schemas/dsconfig.schema.json",
  "compilerOptions": {{
    "target": "esnext",
    "module": "esnext",
    "strict": true,
    "lib": ["esnext"]
  }},
  {include},
  "exclude": ["node_modules", "dist"]
}}
"#
    )
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

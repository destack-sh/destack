use std::fs;
use std::path::{Path, PathBuf};

use clap::{Args, ValueEnum};

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
}

/// Initialize a new Destack project.
pub fn run(args: &InitArgs) -> i32 {
    let dir = args
        .dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // ensure directory exists
    if !dir.exists() {
        if let Err(e) = fs::create_dir_all(&dir) {
            console::error(&format!("failed to create directory: {e}"));
            return 1;
        }
        console::info(&format!("created directory: {}", dir.display()));
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
        console::warn("dsconfig.json already exists (use --force to overwrite)");
    } else {
        let dsconfig = create_dsconfig(&name, args.template);
        if let Err(e) = fs::write(&dsconfig_path, dsconfig) {
            console::error(&format!("failed to write dsconfig.json: {e}"));
            return 1;
        }
        console::info(&format!("created {}", dsconfig_path.display()));
    }

    // create source files based on template
    match args.template {
        Template::Minimal => {
            // no source files for minimal template
        }
        Template::Lib => {
            if let Err(code) = create_source_file(&dir, "src/index.ds", LIB_TEMPLATE, args.force) {
                return code;
            }
        }
        Template::App => {
            if let Err(code) = create_source_file(&dir, "src/main.ds", APP_TEMPLATE, args.force) {
                return code;
            }
        }
    }

    console::success(&format!("initialized Destack project: {name}"));
    0
}

fn create_source_file(dir: &Path, rel_path: &str, content: &str, force: bool) -> Result<(), i32> {
    let file_path = dir.join(rel_path);

    // ensure parent directory exists
    if let Some(parent) = file_path.parent()
        && !parent.exists()
        && let Err(e) = fs::create_dir_all(parent)
    {
        console::error(&format!("failed to create directory: {e}"));
        return Err(1);
    }

    if file_path.exists() && !force {
        console::warn(&format!(
            "{rel_path} already exists (use --force to overwrite)",
        ));
    } else {
        if let Err(e) = fs::write(&file_path, content) {
            console::error(&format!("failed to write {rel_path}: {e}"));
            return Err(1);
        }
        console::info(&format!("created {}", file_path.display()));
    }

    Ok(())
}

fn create_dsconfig(_name: &str, template: Template) -> String {
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

use std::path::{Path, PathBuf};

use clap::{Args, ValueEnum};
use destack_source::{FileSystem, PhysicalFileSystem};

use crate::common::{ReportArgs, print_json_payload_report, report_error};
use crate::console;

/// Marker replaced by the resolved project name in template files.
const PROJECT_NAME_MARKER: &str = "\"__DESTACK_PROJECT__\"";

/// Files in the application project template.
const APP_FILES: &[ProjectFile] = &[
    ProjectFile {
        path: "destack.json",
        contents: include_str!("../../template/app/destack.json"),
    },
    ProjectFile {
        path: "src/main.ds",
        contents: include_str!("../../template/app/src/main.ds"),
    },
];

/// One file in a generated project.
struct ProjectFile {
    /// The path relative to the project directory.
    path: &'static str,
    /// The complete file contents.
    contents: &'static str,
}

/// Project template selected during initialization.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum Template {
    /// Application project with src/main.ds.
    #[default]
    App,
}

impl Template {
    /// Return the files in this project template.
    fn files(self) -> &'static [ProjectFile] {
        match self {
            Self::App => APP_FILES,
        }
    }
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

    /// Project template.
    #[arg(long, short = 't', value_enum, default_value_t = Template::default())]
    pub template: Template,

    /// Overwrite files owned by the selected template.
    #[arg(long, short = 'f')]
    pub force: bool,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,
}

impl InitArgs {
    /// Resolve the explicit or directory-derived project name.
    fn project_name(&self, directory: &Path) -> Result<String, String> {
        // select the explicit name or the directory name
        let name = match self.name.as_deref() {
            Some(name) => name,
            None => directory
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| "project name is required for this directory".to_string())?,
        };
        let name = name.trim();

        // reject an empty explicit name
        if name.is_empty() {
            return Err("project name cannot be empty".to_string());
        }

        Ok(name.to_string())
    }
}

/// JSON payload for init output.
#[derive(serde::Serialize)]
struct InitPayload {
    /// Resolved project name.
    name: String,
    /// Target directory for the project.
    directory: String,
    /// Written project files.
    files: Vec<String>,
}

/// Initialize a new Destack project.
pub fn run(args: &InitArgs) -> i32 {
    // resolve the target directory and project name
    let directory = match &args.dir {
        Some(directory) => directory.clone(),
        None => match std::env::current_dir() {
            Ok(directory) => directory,
            Err(error) => {
                return report_error(
                    "init",
                    &args.report,
                    &format!("failed to read current directory: {error}"),
                );
            }
        },
    };
    let name = match args.project_name(&directory) {
        Ok(name) => name,
        Err(error) => return report_error("init", &args.report, &error),
    };

    // write the complete selected template
    let file_system = PhysicalFileSystem::new();
    let files = match write_project(
        &file_system,
        &directory,
        &name,
        args.template.files(),
        args.force,
    ) {
        Ok(files) => files,
        Err(error) => return report_error("init", &args.report, &error),
    };

    // emit the requested report format
    if args.report.is_json() {
        let payload = InitPayload {
            name,
            directory: directory.display().to_string(),
            files: files
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
        };
        if let Err(code) = print_json_payload_report("init", &args.report, 0, &payload) {
            return code;
        }
    } else {
        // list the written files
        for file in files {
            console::info(&format!("wrote {}", file.display()));
        }

        // confirm the initialized project
        console::success(&format!("initialized Destack project: {name}"));
    }

    0
}

/// Write one complete project template after checking every destination.
fn write_project(
    file_system: &dyn FileSystem,
    directory: &Path,
    name: &str,
    template: &[ProjectFile],
    force: bool,
) -> Result<Vec<PathBuf>, String> {
    // render every template file before changing the file system
    let project_name = serde_json::to_string(name)
        .map_err(|error| format!("failed to encode project name: {error}"))?;
    let files = template
        .iter()
        .map(|file| {
            let path = directory.join(file.path);
            let contents = file.contents.replace(PROJECT_NAME_MARKER, &project_name);

            (path, contents)
        })
        .collect::<Vec<_>>();

    // reject conflicts before writing any project file
    if !force {
        for (path, _) in &files {
            let exists = file_system
                .exists(path)
                .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
            if exists {
                return Err(format!(
                    "project file already exists: {} (use --force to overwrite)",
                    path.display()
                ));
            }
        }
    }

    // create the project directory and each required parent
    file_system
        .create_dir_all(directory)
        .map_err(|error| format!("failed to create {}: {error}", directory.display()))?;
    for (path, _) in &files {
        let Some(parent) = path.parent() else {
            return Err(format!("project file has no parent: {}", path.display()));
        };
        file_system
            .create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }

    // write every rendered project file
    for (path, contents) in &files {
        file_system
            .write(path, contents.as_bytes())
            .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    }

    Ok(files.into_iter().map(|(path, _)| path).collect())
}

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use tspp_serde::Reflect;

use serde::{Deserialize, Serialize};
use tspp_core::closest_string;
use tspp_repository::{MANIFEST_FILE_NAME, ManifestFile, Repository, Revision, Root, TraceView};
use tspp_source::DiagnosticCollection;

use super::CommandResult;
use super::common::{
    CommandEnvVar, CommandInput, CommandOptions, CommandRevision, CommandTargetOverrides,
    ManifestOverride, impl_command_input_options,
};
use super::context::CommandContext;
use super::outcome::CommandOutcome;
/// Source label for tasks declared in package.json.
const TASK_SOURCE: &str = "tspp";

/// Task entry for task list output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct TaskEntry {
    /// Project identifier.
    pub project: String,
    /// Task name.
    pub name: String,
    /// Task description.
    pub description: Option<String>,
    /// Task source.
    pub source: Option<String>,
}

/// Payload for task command output.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct TaskPayload {
    /// Task list entries.
    pub tasks: Option<Vec<TaskEntry>>,
    /// Per-project task execution results.
    pub results: Option<Vec<TaskResult>>,
    /// Exit code when executed.
    pub exit_code: Option<i32>,
}

/// One per-project task execution result.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct TaskResult {
    /// Project identifier.
    pub project: String,
    /// Task name.
    pub task: String,
    /// Task command string.
    pub command: String,
    /// Task working directory.
    pub cwd: String,
    /// Whether this was a dry run.
    pub dry_run: bool,
    /// Task source.
    pub source: String,
    /// Exit code when executed.
    pub exit_code: Option<i32>,
}

/// Task selection for the task command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum TaskAction {
    /// List available tasks.
    List,
    /// Run a task by name.
    Run {
        /// Task name.
        name: String,
        /// Task arguments.
        args: Vec<String>,
        /// Whether to only print the command.
        dry_run: bool,
    },
}

/// Options for the task command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TaskOptions {
    /// Task action to run.
    pub action: TaskAction,
    /// Selected projects.
    pub projects: Vec<String>,
    /// Selected workspace groups.
    pub groups: Vec<String>,
}

/// Request to run workspace tasks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TaskInput {
    /// Revision selected for this task request.
    pub revision: CommandRevision,
    /// Input sources for the command.
    pub inputs: Vec<CommandInput>,
    /// Whether package.json should resolve inputs when none are provided.
    pub config_inputs: bool,
    /// Optional working directory for this command.
    pub cwd: Option<PathBuf>,
    /// Optional manifest path override.
    pub manifest: Option<PathBuf>,
    /// Optional target name override.
    pub target: Option<String>,
    /// Optional target overrides.
    pub target_overrides: Option<CommandTargetOverrides>,
    /// Optional profile name override.
    pub profile: Option<String>,
    /// Optional environment overrides.
    pub env: Vec<CommandEnvVar>,
    /// Optional manifest overrides.
    pub overrides: Vec<ManifestOverride>,
    /// Whether the command should watch for changes.
    pub watch: bool,
    /// Whether the command should skip writes.
    pub dry_run: bool,
    /// Trace detail returned for this command.
    pub trace: Option<TraceView>,
    /// Task action to run.
    pub action: TaskAction,
    /// Selected projects.
    pub projects: Vec<String>,
    /// Selected workspace groups.
    pub groups: Vec<String>,
}

impl_command_input_options!(TaskInput {
    action: TaskAction::List,
    projects: Vec::new(),
    groups: Vec::new(),
});

impl CommandContext<'_> {
    /// Execute a task command.
    pub(crate) fn run_task_command(
        &mut self,
        options: &TaskOptions,
    ) -> CommandResult<CommandOutcome<TaskPayload>> {
        // resolve the task scope first
        let projects = self.resolve_task_projects(options)?;

        match &options.action {
            TaskAction::List => {
                let mut entries = Vec::new();
                for project in projects {
                    for task in project.tasks {
                        entries.push(TaskEntry {
                            project: project.project.clone(),
                            name: task.name,
                            description: task.description,
                            source: Some(TASK_SOURCE.to_string()),
                        });
                    }
                }
                let payload = TaskPayload {
                    tasks: Some(entries),
                    results: None,
                    exit_code: None,
                };

                Ok(
                    CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0)
                        .with_data(payload),
                )
            }
            TaskAction::Run {
                name,
                args,
                dry_run,
            } => {
                let missing_projects: Vec<String> = projects
                    .iter()
                    .filter(|project| !project.tasks.iter().any(|task| task.name == *name))
                    .map(|project| project.project.clone())
                    .collect();
                if !missing_projects.is_empty() {
                    return Err(task_missing_from_projects_error(
                        name,
                        &missing_projects,
                        &projects,
                    )
                    .into());
                }

                let mut results = Vec::new();
                let mut aggregate_exit_code = 0;

                // run the selected task in each project
                for project in projects {
                    let task = project
                        .tasks
                        .iter()
                        .find(|task| task.name == *name)
                        .ok_or_else(|| {
                            task_missing_from_projects_error(
                                name,
                                std::slice::from_ref(&project.project),
                                std::slice::from_ref(&project),
                            )
                        })?;
                    let command = task_command(task, args)?;
                    let cwd = task
                        .cwd
                        .clone()
                        .unwrap_or_else(|| project.project_dir.clone());

                    if *dry_run {
                        results.push(TaskResult {
                            project: project.project.clone(),
                            task: task.name.clone(),
                            command,
                            cwd: cwd.display().to_string(),
                            dry_run: true,
                            source: TASK_SOURCE.to_string(),
                            exit_code: None,
                        });
                        continue;
                    }

                    let mut shell = shell_command(&command);
                    shell.current_dir(&cwd);
                    let output_result = shell.output().map_err(|error| {
                        format!("task failed for '{}': {error}", project.project)
                    })?;

                    if !output_result.stdout.is_empty() {
                        self.output.push_stdout(output_result.stdout);
                    }
                    if !output_result.stderr.is_empty() {
                        self.output.push_stderr(output_result.stderr);
                    }

                    let exit_code = output_result.status.code().unwrap_or(1);
                    if aggregate_exit_code == 0 && exit_code != 0 {
                        aggregate_exit_code = exit_code;
                    }

                    results.push(TaskResult {
                        project: project.project.clone(),
                        task: task.name.clone(),
                        command,
                        cwd: cwd.display().to_string(),
                        dry_run: false,
                        source: TASK_SOURCE.to_string(),
                        exit_code: Some(exit_code),
                    });
                }

                let payload = TaskPayload {
                    tasks: None,
                    results: Some(results),
                    exit_code: Some(aggregate_exit_code),
                };

                Ok(CommandOutcome::new(
                    DiagnosticCollection::default(),
                    aggregate_exit_code,
                    0,
                    0,
                    0,
                )
                .with_data(payload))
            }
        }
    }

    /// Resolve the selected task project scope.
    fn resolve_task_projects(&self, options: &TaskOptions) -> CommandResult<Vec<TaskProject>> {
        let revision = self.revision();
        let workspace = self
            .repository
            .root(revision)
            .map_err(|error| format!("failed to derive workspace: {error}"))?;
        let mut projects = if !options.projects.is_empty() || !options.groups.is_empty() {
            load_workspace_task_projects(self.repository.as_ref(), revision, &workspace)?
        } else {
            let project_path = resolve_task_project_path(
                self.repository.as_ref(),
                revision,
                self.repository.path(),
                self.common.manifest.as_deref(),
            )?;
            vec![load_task_project(
                self.repository.as_ref(),
                revision,
                &workspace.root,
                &project_path,
            )?]
        };

        if options.projects.is_empty() && options.groups.is_empty() {
            return Ok(projects);
        }

        let root_options = self
            .repository
            .manifest_for_workspace(revision)
            .map_err(|error| format!("failed to derive workspace options: {error}"))?;
        let selected_names = resolve_task_project_selection_names(
            root_options.as_deref(),
            &projects,
            &options.projects,
            &options.groups,
        )?;
        projects.retain(|project| selected_names.contains(&project.project));
        projects.sort_by(|left, right| left.project.cmp(&right.project));

        Ok(projects)
    }
}

/// One selected task project.
#[derive(Debug, Clone)]
struct TaskProject {
    /// Stable project identifier relative to the root.
    project: String,
    /// Optional package name alias.
    package_name: Option<String>,
    /// Project directory.
    project_dir: PathBuf,
    /// Available tasks.
    tasks: Vec<TaskSpec>,
}

/// Task specification loaded from package.json.
#[derive(Debug, Clone)]
struct TaskSpec {
    /// The task name.
    name: String,
    /// The shell command to execute.
    command: Option<String>,
    /// The task description.
    description: Option<String>,
    /// The working directory for the task.
    cwd: Option<PathBuf>,
}

/// Load one task project from one project path.
fn load_task_project(
    repository: &Repository,
    revision: Revision,
    workspace_root: &Path,
    project_path: &Path,
) -> CommandResult<TaskProject> {
    let project = relative_project_path(project_path, workspace_root);
    let manifest_path = exact_manifest_path(repository, revision, project_path)?;

    // resolve the canonical package alias when this project is a package
    let package_name = repository
        .nearest_package(revision, project_path)
        .map_err(|error| error.to_string())?
        .and_then(|package| package.name.clone());

    // load tasks declared directly by this project
    let tasks = if let Some(manifest_path) = manifest_path.as_deref() {
        load_manifest_tasks(repository, revision, manifest_path)?
    } else {
        Vec::new()
    };

    Ok(TaskProject {
        project,
        package_name,
        project_dir: project_path.to_path_buf(),
        tasks,
    })
}

/// Load all task projects from one workspace.
fn load_workspace_task_projects(
    repository: &Repository,
    revision: Revision,
    workspace: &Root,
) -> CommandResult<Vec<TaskProject>> {
    let mut project_paths = Vec::new();
    let mut seen = HashSet::new();

    // collect unique project paths in workspace order
    for package_path in repository
        .package_roots(revision)
        .map_err(|error| error.to_string())?
    {
        if seen.insert(package_path.clone()) {
            project_paths.push(package_path.clone());
        }
    }

    let mut projects = Vec::new();
    for project_path in project_paths {
        projects.push(load_task_project(
            repository,
            revision,
            &workspace.root,
            &project_path,
        )?);
    }

    Ok(projects)
}

/// Load task specifications from one manifest file.
fn load_manifest_tasks(
    repository: &Repository,
    revision: Revision,
    manifest_path: &Path,
) -> CommandResult<Vec<TaskSpec>> {
    let file_id = repository.file_id(manifest_path);
    let file = repository
        .file(revision, file_id)
        .map_err(|error| format!("failed to load {}: {error}", manifest_path.display()))?
        .ok_or_else(|| format!("failed to load {}", manifest_path.display()))?;
    let config = ManifestFile::parse(&file)
        .map_err(|error| format!("invalid {}: {error}", manifest_path.display()))?
        .ok_or_else(|| format!("{} is not a TS++ manifest", manifest_path.display()))?;

    let mut tasks = Vec::new();
    for (name, task) in &config.tasks {
        tasks.push(TaskSpec {
            name: name.clone(),
            command: task.exec.clone(),
            description: None,
            cwd: None,
        });
    }

    Ok(tasks)
}

/// Build one normalized project path relative to the root.
fn relative_project_path(project_path: &Path, workspace_root: &Path) -> String {
    let Ok(relative_path) = project_path.strip_prefix(workspace_root) else {
        return project_path.display().to_string();
    };
    if relative_path.as_os_str().is_empty() {
        return ".".to_string();
    }

    relative_path.display().to_string()
}

/// Resolve selected project names from explicit projects and groups.
fn resolve_task_project_selection_names(
    root_options: Option<&ManifestFile>,
    projects: &[TaskProject],
    selected_projects: &[String],
    selected_groups: &[String],
) -> CommandResult<HashSet<String>> {
    let mut selected_names = HashSet::new();

    // expand explicit project selectors first
    for selector in selected_projects {
        let project = resolve_project_selector(selector, projects)?;
        selected_names.insert(project);
    }

    let Some(root_options) = root_options else {
        if selected_groups.is_empty() {
            return Ok(selected_names);
        }

        let names = selected_groups.join(", ");
        return Err(format!("workspace groups are not available: {names}").into());
    };

    // expand named workspace groups into project names
    for group_name in selected_groups {
        let members = root_options
            .groups
            .get(group_name)
            .ok_or_else(|| unknown_group_error(group_name, root_options))?;

        for member in members {
            let project = resolve_project_selector(member, projects).map_err(|_| {
                format!("workspace group '{group_name}' references unknown project '{member}'")
            })?;
            selected_names.insert(project);
        }
    }

    Ok(selected_names)
}

/// Resolve one project selector into one stable project id.
fn resolve_project_selector(selector: &str, projects: &[TaskProject]) -> CommandResult<String> {
    if let Some(project) = projects.iter().find(|project| project.project == selector) {
        return Ok(project.project.clone());
    }

    let matching_projects: Vec<&TaskProject> = projects
        .iter()
        .filter(|project| project.package_name.as_deref() == Some(selector))
        .collect();
    if matching_projects.len() == 1 {
        return Ok(matching_projects[0].project.clone());
    }
    if matching_projects.len() > 1 {
        return Err(ambiguous_project_error(selector, &matching_projects).into());
    }

    Err(unknown_project_error(selector, projects).into())
}

/// Build one lookup error for one task missing from selected projects.
fn task_missing_from_projects_error(
    name: &str,
    missing_projects: &[String],
    projects: &[TaskProject],
) -> String {
    let project_names = missing_projects.join(", ");
    let mut available_tasks = Vec::new();

    // collect one stable task name set for suggestions
    for project in projects {
        for task in &project.tasks {
            if !available_tasks.contains(&task.name) {
                available_tasks.push(task.name.clone());
            }
        }
    }

    let suggestion = closest_task_name(name, &available_tasks);

    if let Some(suggestion) = suggestion {
        return format!(
            "task '{name}' not found in projects: {project_names}, did you mean '{suggestion}'?"
        );
    }

    format!("task '{name}' not found in projects: {project_names}")
}

/// Build one unknown project error with suggestions.
fn unknown_project_error(selector: &str, projects: &[TaskProject]) -> String {
    let suggestion = closest_project_name(selector, projects);

    if let Some(suggestion) = suggestion {
        return format!("unknown project '{selector}', did you mean '{suggestion}'?");
    }

    format!("unknown project '{selector}'")
}

/// Build one ambiguous project selector error.
fn ambiguous_project_error(selector: &str, projects: &[&TaskProject]) -> String {
    let matches = projects
        .iter()
        .map(|project| project.project.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    format!("project selector '{selector}' is ambiguous, matches: {matches}")
}

/// Build one unknown workspace group error with suggestions.
fn unknown_group_error(group_name: &str, root_options: &ManifestFile) -> String {
    let groups: Vec<&str> = root_options.groups.keys().map(String::as_str).collect();
    let suggestion = closest_group_name(group_name, &groups);

    if let Some(suggestion) = suggestion {
        return format!("unknown workspace group '{group_name}', did you mean '{suggestion}'?");
    }

    format!("unknown workspace group '{group_name}'")
}

/// Return the closest project name or path selector.
fn closest_project_name<'a>(selector: &str, projects: &'a [TaskProject]) -> Option<&'a str> {
    let candidates = projects.iter().flat_map(|project| {
        let package_name = project.package_name.as_deref().unwrap_or_default();

        [project.project.as_str(), package_name]
            .into_iter()
            .filter(|candidate| !candidate.is_empty())
    });

    closest_string(selector, candidates, closest_task_distance(selector))
}

/// Return the closest workspace group name.
fn closest_group_name<'a>(group_name: &str, groups: &'a [&str]) -> Option<&'a str> {
    closest_string(
        group_name,
        groups.iter().copied(),
        closest_task_distance(group_name),
    )
}

/// Return the closest task name when the input is near one known command.
fn closest_task_name<'a>(name: &str, tasks: &'a [String]) -> Option<&'a str> {
    let candidates = tasks.iter().map(String::as_str);

    closest_string(name, candidates, closest_task_distance(name))
}

/// Return the maximum typo distance allowed for task selectors.
fn closest_task_distance(value: &str) -> usize {
    if value.len() <= 4 { 1 } else { 2 }
}

/// Build one shell command for one task and extra args.
fn task_command(task: &TaskSpec, args: &[String]) -> CommandResult<String> {
    let Some(mut command) = task.command.clone() else {
        return Err(format!("task '{}' is missing exec", task.name).into());
    };
    if !args.is_empty() {
        command.push(' ');
        command.push_str(&args.join(" "));
    }

    Ok(command)
}

/// Resolve the active task project path from cwd or one explicit config path.
fn resolve_task_project_path(
    repository: &Repository,
    revision: Revision,
    cwd: &Path,
    override_path: Option<&Path>,
) -> CommandResult<PathBuf> {
    let base_path = if let Some(override_path) = override_path {
        let resolved = if override_path.is_absolute() {
            override_path.to_path_buf()
        } else {
            cwd.join(override_path)
        };
        let metadata = repository
            .file_metadata(revision, &resolved)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("project path not found: {}", resolved.display()))?;

        if metadata.is_directory {
            resolved
        } else if metadata.is_file {
            resolved
                .parent()
                .map(PathBuf::from)
                .ok_or_else(|| format!("project path not found: {}", resolved.display()))?
        } else {
            return Err(format!("project path not found: {}", resolved.display()).into());
        }
    } else {
        cwd.to_path_buf()
    };

    if let Some(package) = repository
        .nearest_package(revision, &base_path)
        .map_err(|error| error.to_string())?
        && let Some(package_path) = package.path.as_ref()
    {
        return Ok(package_path.clone());
    }

    Ok(base_path)
}

/// Return the TS++ manifest path of one project directory, none when it has none.
fn exact_manifest_path(
    repository: &Repository,
    revision: Revision,
    project_path: &Path,
) -> CommandResult<Option<PathBuf>> {
    let candidate = project_path.join(MANIFEST_FILE_NAME);
    let manifest = repository
        .manifest_for_file(revision, repository.file_id(&candidate))
        .map_err(|error| error.to_string())?;

    Ok(manifest.map(|_| candidate))
}

/// Build a shell command for script execution.
fn shell_command(command: &str) -> Command {
    if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg(command);
        return cmd;
    }

    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd
}

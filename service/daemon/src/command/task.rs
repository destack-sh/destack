use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use destack_source::DiagnosticCollection;
use destack_workspace::{Destack, PackageJson, Workspace};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::context::CommandContext;
use super::dispatch::CommandOutcome;

/// Task entry for task list output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandTaskEntry {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandTaskPayload {
    /// Task list entries.
    pub tasks: Option<Vec<CommandTaskEntry>>,
    /// Per-project task execution results.
    pub results: Option<Vec<CommandTaskResult>>,
    /// Exit code when executed.
    pub exit_code: Option<i32>,
}

/// One per-project task execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandTaskResult {
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CommandTaskAction {
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandTaskOptions {
    /// Task action to run.
    pub action: CommandTaskAction,
    /// Selected projects.
    pub projects: Vec<String>,
    /// Selected workspace groups.
    pub groups: Vec<String>,
}

impl CommandContext<'_> {
    /// Execute a task command.
    pub(super) fn run_task_command(
        &mut self,
        options: &CommandTaskOptions,
    ) -> super::CommandResult<CommandOutcome> {
        // resolve the task scope first
        let projects = self.resolve_task_projects(options)?;

        match &options.action {
            CommandTaskAction::List => {
                let mut entries = Vec::new();
                for project in projects {
                    for task in project.tasks {
                        entries.push(CommandTaskEntry {
                            project: project.project.clone(),
                            name: task.name,
                            description: task.description,
                            source: Some(task.source.as_str().to_string()),
                        });
                    }
                }
                let payload = CommandTaskPayload {
                    tasks: Some(entries),
                    results: None,
                    exit_code: None,
                };
                let data = serde_json::to_value(payload)
                    .map_err(|error| format!("invalid task payload: {error}"))?;
                Ok(
                    CommandOutcome::new(DiagnosticCollection::default(), 0, 0, 0, 0, None)
                        .with_data(data),
                )
            }
            CommandTaskAction::Run {
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
                        .expect("task presence checked above");
                    let command = task_command(task, args);
                    let cwd = task
                        .cwd
                        .clone()
                        .unwrap_or_else(|| project.project_dir.clone());

                    if *dry_run {
                        results.push(CommandTaskResult {
                            project: project.project.clone(),
                            task: task.name.clone(),
                            command,
                            cwd: cwd.display().to_string(),
                            dry_run: true,
                            source: task.source.as_str().to_string(),
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

                    results.push(CommandTaskResult {
                        project: project.project.clone(),
                        task: task.name.clone(),
                        command,
                        cwd: cwd.display().to_string(),
                        dry_run: false,
                        source: task.source.as_str().to_string(),
                        exit_code: Some(exit_code),
                    });
                }

                let payload = CommandTaskPayload {
                    tasks: None,
                    results: Some(results),
                    exit_code: Some(aggregate_exit_code),
                };
                let data = serde_json::to_value(payload)
                    .map_err(|error| format!("invalid task payload: {error}"))?;
                Ok(CommandOutcome::new(
                    DiagnosticCollection::default(),
                    aggregate_exit_code,
                    0,
                    0,
                    0,
                    None,
                )
                .with_data(data))
            }
        }
    }

    /// Resolve the selected task project scope.
    fn resolve_task_projects(
        &self,
        options: &CommandTaskOptions,
    ) -> super::CommandResult<Vec<TaskProject>> {
        let resolver = self.resolver();
        let workspace = self.daemon.session.workspace_snapshot();
        let mut projects = if !options.projects.is_empty() || !options.groups.is_empty() {
            load_workspace_task_projects(&resolver, &workspace)?
        } else {
            let project_path = resolve_task_project_path(
                &resolver,
                &workspace,
                self.program.cwd.as_path(),
                self.common.config_path.as_deref(),
            )?;
            vec![load_task_project(
                &resolver,
                &workspace.root,
                &project_path,
            )?]
        };

        if options.projects.is_empty() && options.groups.is_empty() {
            return Ok(projects);
        }

        let root_config = workspace.config.as_ref().map(Arc::as_ref);
        let selected_names = resolve_task_project_selection_names(
            root_config,
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
    /// Stable project identifier relative to the workspace root.
    project: String,
    /// Optional package name alias.
    package_name: Option<String>,
    /// Project directory.
    project_dir: PathBuf,
    /// Available tasks and scripts.
    tasks: Vec<TaskSpec>,
}

/// Source of one task-like command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskSpecSource {
    /// Task came from destack.json.
    Destack,
    /// Task came from package.json scripts.
    PackageJson,
}

impl TaskSpecSource {
    /// Return the task source label.
    fn as_str(self) -> &'static str {
        match self {
            Self::Destack => "destack",
            Self::PackageJson => "package.json",
        }
    }
}

/// Task specification loaded from destack.json.
#[derive(Debug, Clone)]
struct TaskSpec {
    /// The task name.
    name: String,
    /// The shell command to execute.
    command: String,
    /// The task description.
    description: Option<String>,
    /// The working directory for the task.
    cwd: Option<PathBuf>,
    /// The task source.
    source: TaskSpecSource,
}

/// Load task and script specifications for one project path.
fn load_tasks(
    resolver: &destack_resolver::Resolver,
    project_path: &Path,
    destack_config_path: Option<&Path>,
) -> super::CommandResult<Vec<TaskSpec>> {
    let mut tasks = if let Some(destack_config_path) = destack_config_path {
        load_destack_tasks(resolver, destack_config_path)?
    } else {
        Vec::new()
    };

    // merge package scripts after destack tasks
    let package_scripts = load_package_scripts(resolver, project_path)?;
    for script in package_scripts {
        if tasks.iter().any(|task| task.name == script.name) {
            continue;
        }

        tasks.push(script);
    }

    Ok(tasks)
}

/// Load one task project from one project path.
fn load_task_project(
    resolver: &destack_resolver::Resolver,
    workspace_root: &Path,
    project_path: &Path,
) -> super::CommandResult<TaskProject> {
    let project = relative_project_path(project_path, workspace_root);
    let destack_config_path = exact_destack_config_path(resolver, project_path);
    let package_json_path = exact_package_json_path(resolver, project_path);
    let config = if let Some(destack_config_path) = destack_config_path.as_ref() {
        Some(
            resolver
                .read_destack_config(destack_config_path, destack_resolver::CachePolicy::UseCache)
                .map_err(|error| error.to_string())?,
        )
    } else {
        None
    };
    let package_name = if let Some(config) = config.as_ref() {
        if let Some(name) = config.options.name.clone() {
            Some(name)
        } else if let Some(package_json_path) = package_json_path.as_ref() {
            load_package_name(resolver, package_json_path)?
        } else {
            None
        }
    } else if let Some(package_json_path) = package_json_path.as_ref() {
        load_package_name(resolver, package_json_path)?
    } else {
        None
    };
    let tasks = load_tasks(resolver, project_path, destack_config_path.as_deref())?;

    Ok(TaskProject {
        project,
        package_name,
        project_dir: project_path.to_path_buf(),
        tasks,
    })
}

/// Load all task projects from one workspace.
fn load_workspace_task_projects(
    resolver: &destack_resolver::Resolver,
    workspace: &Workspace,
) -> super::CommandResult<Vec<TaskProject>> {
    let mut project_paths = Vec::new();
    let mut seen = HashSet::new();

    // collect unique project paths in workspace order
    for package_path in &workspace.package_paths {
        if seen.insert(package_path.clone()) {
            project_paths.push(package_path.clone());
        }
    }

    let mut projects = Vec::new();
    for project_path in project_paths {
        projects.push(load_task_project(resolver, &workspace.root, &project_path)?);
    }

    Ok(projects)
}

/// Load task specifications from one destack.json file.
fn load_destack_tasks(
    resolver: &destack_resolver::Resolver,
    destack_config_path: &Path,
) -> super::CommandResult<Vec<TaskSpec>> {
    let content = resolver
        .fs
        .read_to_string(destack_config_path)
        .map_err(|error| format!("failed to read {}: {error}", destack_config_path.display()))?;

    let value: Value =
        serde_json::from_str(&content).map_err(|error| format!("invalid destack.json: {error}"))?;

    let Some(tasks_value) = value.get("tasks") else {
        return Ok(Vec::new());
    };
    let tasks_object = tasks_value
        .as_object()
        .ok_or_else(|| "tasks must be an object".to_string())?;

    let destack_config_dir = destack_config_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| destack_config_path.to_path_buf());

    let mut tasks = Vec::new();
    for (name, value) in tasks_object {
        if let Some(command) = value.as_str() {
            tasks.push(TaskSpec {
                name: name.clone(),
                command: command.to_string(),
                description: None,
                cwd: None,
                source: TaskSpecSource::Destack,
            });
            continue;
        }

        let Some(command) = value.get("command").and_then(|v| v.as_str()) else {
            return Err(format!("task '{name}' is missing a command").into());
        };
        let description = value
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);
        let cwd = value.get("cwd").and_then(|v| v.as_str()).map(|path| {
            let path = PathBuf::from(path);
            if path.is_absolute() {
                path
            } else {
                destack_config_dir.join(path)
            }
        });

        tasks.push(TaskSpec {
            name: name.clone(),
            command: command.to_string(),
            description,
            cwd,
            source: TaskSpecSource::Destack,
        });
    }

    Ok(tasks)
}

/// Load package.json scripts adjacent to one config path.
fn load_package_scripts(
    resolver: &destack_resolver::Resolver,
    project_path: &Path,
) -> super::CommandResult<Vec<TaskSpec>> {
    let Some(package_path) = exact_package_json_path(resolver, project_path) else {
        return Ok(Vec::new());
    };

    let content = resolver
        .fs
        .read_to_string(&package_path)
        .map_err(|error| format!("failed to read {}: {error}", package_path.display()))?;
    let package: PackageJson =
        serde_json::from_str(&content).map_err(|error| format!("invalid package.json: {error}"))?;
    let Some(scripts) = package.scripts else {
        return Ok(Vec::new());
    };
    let package_dir = package_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| project_path.to_path_buf());

    let mut tasks = Vec::new();
    for (name, command) in scripts {
        tasks.push(TaskSpec {
            name,
            command,
            description: None,
            cwd: Some(package_dir.clone()),
            source: TaskSpecSource::PackageJson,
        });
    }

    Ok(tasks)
}

/// Load the package name from one exact package.json path.
fn load_package_name(
    resolver: &destack_resolver::Resolver,
    package_json_path: &Path,
) -> super::CommandResult<Option<String>> {
    let content = resolver
        .fs
        .read_to_string(package_json_path)
        .map_err(|error| format!("failed to read {}: {error}", package_json_path.display()))?;
    let package: PackageJson =
        serde_json::from_str(&content).map_err(|error| format!("invalid package.json: {error}"))?;

    Ok(package.name)
}

/// Build one normalized project path relative to the workspace root.
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
    root_config: Option<&Destack>,
    projects: &[TaskProject],
    selected_projects: &[String],
    selected_groups: &[String],
) -> super::CommandResult<HashSet<String>> {
    let mut selected_names = HashSet::new();

    // expand explicit project selectors first
    for selector in selected_projects {
        let project = resolve_project_selector(selector, projects)?;
        selected_names.insert(project);
    }

    let Some(root_config) = root_config else {
        if selected_groups.is_empty() {
            return Ok(selected_names);
        }

        let names = selected_groups.join(", ");
        return Err(format!("workspace groups are not available: {names}").into());
    };

    // expand named workspace groups into project names
    for group_name in selected_groups {
        let members = root_config
            .options
            .workspace
            .groups
            .get(group_name)
            .ok_or_else(|| unknown_group_error(group_name, root_config))?;

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
fn resolve_project_selector(
    selector: &str,
    projects: &[TaskProject],
) -> super::CommandResult<String> {
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

    let available_specs: Vec<TaskSpec> = available_tasks
        .into_iter()
        .map(|task_name| TaskSpec {
            name: task_name,
            command: String::new(),
            description: None,
            cwd: None,
            source: TaskSpecSource::Destack,
        })
        .collect();
    let suggestion = closest_task_name(name, &available_specs);

    if let Some(suggestion) = suggestion {
        return format!(
            "task or script '{name}' not found in projects: {project_names}, did you mean '{suggestion}'?"
        );
    }

    format!("task or script '{name}' not found in projects: {project_names}")
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
fn unknown_group_error(group_name: &str, root_config: &Destack) -> String {
    let groups: Vec<&str> = root_config
        .options
        .workspace
        .groups
        .keys()
        .map(String::as_str)
        .collect();
    let suggestion = closest_group_name(group_name, &groups);

    if let Some(suggestion) = suggestion {
        return format!("unknown workspace group '{group_name}', did you mean '{suggestion}'?");
    }

    format!("unknown workspace group '{group_name}'")
}

/// Return the closest project name or path selector.
fn closest_project_name<'a>(selector: &str, projects: &'a [TaskProject]) -> Option<&'a str> {
    let mut best_name = None;
    let mut best_distance = usize::MAX;

    for project in projects {
        let package_name = project.package_name.as_deref().unwrap_or_default();
        for candidate in [project.project.as_str(), package_name] {
            if candidate.is_empty() {
                continue;
            }
            let distance = edit_distance(selector, candidate);
            if distance < best_distance {
                best_distance = distance;
                best_name = Some(candidate);
            }
        }
    }

    let threshold = if selector.len() <= 4 { 1 } else { 2 };
    if best_distance <= threshold {
        return best_name;
    }

    None
}

/// Return the closest workspace group name.
fn closest_group_name<'a>(group_name: &str, groups: &'a [&str]) -> Option<&'a str> {
    let mut best_name = None;
    let mut best_distance = usize::MAX;

    for candidate in groups {
        let distance = edit_distance(group_name, candidate);
        if distance < best_distance {
            best_distance = distance;
            best_name = Some(*candidate);
        }
    }

    let threshold = if group_name.len() <= 4 { 1 } else { 2 };
    if best_distance <= threshold {
        return best_name;
    }

    None
}

/// Return the closest task name when the input is near one known command.
fn closest_task_name<'a>(name: &str, tasks: &'a [TaskSpec]) -> Option<&'a str> {
    let mut best_name = None;
    let mut best_distance = usize::MAX;

    // choose one bounded edit distance suggestion
    for task in tasks {
        let distance = edit_distance(name, &task.name);
        if distance < best_distance {
            best_distance = distance;
            best_name = Some(task.name.as_str());
        }
    }

    let threshold = if name.len() <= 4 { 1 } else { 2 };
    if best_distance <= threshold {
        return best_name;
    }

    None
}

/// Build one shell command for one task and extra args.
fn task_command(task: &TaskSpec, args: &[String]) -> String {
    let mut command = task.command.clone();
    if !args.is_empty() {
        command.push(' ');
        command.push_str(&args.join(" "));
    }

    command
}

/// Resolve the active task project path from cwd or one explicit config path.
fn resolve_task_project_path(
    resolver: &destack_resolver::Resolver,
    workspace: &Workspace,
    cwd: &Path,
    override_path: Option<&Path>,
) -> super::CommandResult<PathBuf> {
    let base_path = if let Some(override_path) = override_path {
        let resolved = if override_path.is_absolute() {
            override_path.to_path_buf()
        } else {
            cwd.join(override_path)
        };
        let metadata = resolver
            .fs
            .metadata(&resolved)
            .map_err(|_| format!("project path not found: {}", resolved.display()))?;

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

    if let Some(package_path) = workspace.find_package_for_path(&base_path) {
        return Ok(package_path.clone());
    }

    Ok(base_path)
}

/// Return the exact destack.json path for one project directory.
fn exact_destack_config_path(
    resolver: &destack_resolver::Resolver,
    project_path: &Path,
) -> Option<PathBuf> {
    let candidate = project_path.join("destack.json");
    if resolver
        .fs
        .metadata(&candidate)
        .is_ok_and(|metadata| metadata.is_file)
    {
        return Some(candidate);
    }

    None
}

/// Return the exact package.json path for one project directory.
fn exact_package_json_path(
    resolver: &destack_resolver::Resolver,
    project_path: &Path,
) -> Option<PathBuf> {
    let candidate = project_path.join("package.json");
    if resolver
        .fs
        .metadata(&candidate)
        .is_ok_and(|metadata| metadata.is_file)
    {
        return Some(candidate);
    }

    None
}

/// Compute the edit distance between two short names.
fn edit_distance(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();

    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0; right.len() + 1];

    // dynamic programming rows
    for (left_index, left_char) in left.iter().enumerate() {
        current[0] = left_index + 1;

        for (right_index, right_char) in right.iter().enumerate() {
            let substitution_cost = if left_char == right_char { 0 } else { 1 };
            let deletion = previous[right_index + 1] + 1;
            let insertion = current[right_index] + 1;
            let substitution = previous[right_index] + substitution_cost;

            current[right_index + 1] = deletion.min(insertion).min(substitution);
        }

        std::mem::swap(&mut previous, &mut current);
    }

    previous[right.len()]
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

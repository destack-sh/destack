use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use destack_source::glob;
use destack_workspace::TargetDiscovery;

use crate::common::{InputArgs, InputSource, ProgramArgs};
use crate::error::CliResult;
use crate::pipeline::workspace::{load_destack_config_for_program, workspace_context};

/// Errors returned while resolving input sources.
#[derive(Debug)]
pub enum ResolveSourcesError {
    /// No input sources were available.
    NoInput,
    /// A specific error message.
    Message(String),
}

/// Resolve input sources, optionally falling back to Destack config discovery.
pub fn resolve_sources(
    input: &InputArgs,
    program_args: Option<&ProgramArgs>,
    target_name: Option<&str>,
) -> Result<Vec<InputSource>, ResolveSourcesError> {
    // prefer explicit input args when provided
    if input.has_input() {
        return input
            .to_sources()
            .map_err(|error| ResolveSourcesError::Message(error.to_string()));
    }

    // fall back to destack.json discovery when available
    let Some(program_args) = program_args else {
        return Err(ResolveSourcesError::NoInput);
    };

    let sources = collect_sources_from_destack_config(program_args, target_name)
        .map_err(|error| ResolveSourcesError::Message(error.to_string()))?;

    if sources.is_empty() {
        return Err(ResolveSourcesError::NoInput);
    }

    Ok(sources)
}

/// Collect source files using Destack config discovery rules.
pub fn collect_sources_from_destack_config(
    program_args: &ProgramArgs,
    target_name: Option<&str>,
) -> CliResult<Vec<InputSource>> {
    // resolve workspace context and config
    let context = workspace_context(program_args, None)?;
    let config = load_destack_config_for_program(
        program_args,
        &context.resolver,
        &context.repository,
        context.revision,
        context.repository.workspace_root(),
    )?;

    // select target options from explicit or default target name
    let selected_target = target_name
        .map(str::to_string)
        .or_else(|| config.default_target.clone());
    let target_options = selected_target
        .as_deref()
        .and_then(|name| config.targets.get(name));

    // pick discovery rules from target when available
    let (entries, includes, excludes, discovery) = if let Some(target) = target_options {
        let includes = if target.include.is_empty() {
            config.include.clone()
        } else {
            target.include.clone()
        };
        let mut excludes = config.exclude.clone();
        excludes.extend(target.exclude.iter().cloned());
        (target.entry.clone(), includes, excludes, target.discovery)
    } else {
        (
            Vec::new(),
            config.include.clone(),
            config.exclude.clone(),
            TargetDiscovery::Include,
        )
    };

    // build paths from entry, files, or include globs
    let base_dir = config.directory.clone();
    let mut paths = BTreeSet::new();

    // include explicit entry points for entry discovery
    if discovery == TargetDiscovery::Entry && !entries.is_empty() {
        for entry in entries {
            let path = if entry.is_absolute() {
                entry
            } else {
                base_dir.join(entry)
            };
            paths.insert(path);
        }
    }

    // include explicit files from the config
    if paths.is_empty() && !config.files.is_empty() {
        for file in &config.files {
            let path = base_dir.join(file);
            paths.insert(path);
        }
    }

    // include patterns from the target or config
    if paths.is_empty() {
        let patterns = if includes.is_empty() {
            vec![
                "**/*.ds".to_string(),
                "**/*.ts".to_string(),
                "**/*.tsx".to_string(),
            ]
        } else {
            includes
        };

        let include_paths = expand_patterns(&base_dir, &patterns);
        let exclude_paths = expand_patterns(&base_dir, &excludes);
        for path in include_paths {
            if !exclude_paths.contains(&path) {
                paths.insert(path);
            }
        }
    }

    Ok(paths.into_iter().map(InputSource::File).collect())
}

/// Expand glob patterns relative to a base directory.
fn expand_patterns(base_dir: &Path, patterns: &[String]) -> BTreeSet<PathBuf> {
    // expand each pattern into matching paths
    let mut paths = BTreeSet::new();
    for pattern in patterns {
        let pattern_path = if PathBuf::from(pattern).is_absolute() {
            pattern.to_string()
        } else {
            format!("{}/{}", base_dir.display(), pattern)
        };
        for path in glob(&pattern_path) {
            paths.insert(path);
        }
    }
    paths
}

use crate::generate::core::{lower_camel, upper_camel};
use crate::generate::schema::ModulePath;

use super::name::identifier;

pub(super) const GENERATED_ROOT: &str = "client/typescript/src/_generated";

/// Return the TypeScript import path between two generated semantic modules.
pub(super) fn generated_import_path(source: &ModulePath, target: &ModulePath) -> String {
    let source = generated_target_segments(source);
    let target = generated_target_segments(target);

    module_import_path(&source, &target)
}

/// Return one TypeScript import path from a generated module to a runtime module.
pub(super) fn runtime_import_path(source: &[String], target: &[&str]) -> String {
    let target = target
        .iter()
        .map(|segment| segment.to_string())
        .collect::<Vec<_>>();

    module_import_path(source, &target)
}

/// Return TypeScript generated semantic target segments.
pub(super) fn generated_target_segments(path: &ModulePath) -> Vec<String> {
    let mut segments = Vec::with_capacity(path.segments().len() + 1);
    segments.push("_generated".to_string());
    segments.extend(path.segments().iter().cloned());

    segments
}

/// Return one generated TypeScript output path.
pub(super) fn output_path(path: &ModulePath) -> String {
    format!("{GENERATED_ROOT}/{}.ts", path.slash_path())
}

/// Return one generated module namespace binding.
pub(super) fn module_namespace(path: &ModulePath) -> String {
    let mut segments = path.segments().iter();
    let Some(first) = segments.next() else {
        return "module".to_string();
    };

    let mut name = lower_camel(first);
    for segment in segments {
        name.push_str(&upper_camel(segment));
    }

    identifier(&name)
}

/// Return the TypeScript import path between two module paths.
fn module_import_path(source: &[String], target: &[String]) -> String {
    let source_directory = &source[..source.len() - 1];
    let mut shared = 0;

    // locate the common module prefix
    while shared < source_directory.len()
        && shared < target.len()
        && source_directory[shared] == target[shared]
    {
        shared += 1;
    }

    // walk from the source directory to the target
    let mut segments = Vec::new();
    for _ in shared..source_directory.len() {
        segments.push("..".to_string());
    }
    segments.extend(target[shared..].iter().cloned());

    // preserve an explicit relative module prefix
    let path = segments.join("/");
    if path.starts_with('.') {
        format!("{path}.js")
    } else {
        format!("./{path}.js")
    }
}

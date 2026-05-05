use std::path::Path;

use destack_workspace::{Module, Repository};

/// Return the relative path for one module inside the platform package.
fn platform_relative_path(repository: &Repository, module: &Module) -> Option<String> {
    let libraries = repository.builtins();
    let library_name = libraries.library_name_for_module(module.id)?;

    if library_name != "platform" {
        return None;
    }

    libraries.library_relative_path_for_module(module.id)
}

/// Resolve one platform domain name from one platform module.
pub(crate) fn module_platform_domain(repository: &Repository, module: &Module) -> Option<String> {
    let relative_path = platform_relative_path(repository, module)?;
    let domain = relative_path.split('/').next().unwrap_or_default().trim();

    if domain.is_empty() {
        return None;
    }

    Some(domain.to_string())
}

/// Prefix one implementation name with nested platform path segments when needed.
pub(crate) fn qualify_platform_implementation_name(
    repository: &Repository,
    module: &Module,
    implementation_name: &str,
) -> String {
    let prefix = module
        .path
        .as_deref()
        .and_then(module_platform_implementation_prefix_from_path)
        .or_else(|| {
            platform_relative_path(repository, module)
                .and_then(|relative_path| module_platform_implementation_prefix(&relative_path))
        });
    let Some(prefix) = prefix else {
        return implementation_name.to_string();
    };

    // keep existing qualified names stable
    if implementation_name == prefix || implementation_name.starts_with(&format!("{prefix}.")) {
        return implementation_name.to_string();
    }

    format!("{prefix}.{implementation_name}")
}

/// Return the nested implementation prefix for one platform module path.
fn module_platform_implementation_prefix(relative_path: &str) -> Option<String> {
    let mut parts = relative_path.split('/');

    // skip the top level platform domain
    let _domain = parts.next()?;

    // keep the nested directories below the domain
    let mut prefix_parts = Vec::new();
    for part in parts {
        if part.ends_with(".ds") {
            break;
        }

        let part = part.trim();
        if !part.is_empty() {
            prefix_parts.push(part);
        }
    }

    if prefix_parts.is_empty() {
        return None;
    }

    Some(prefix_parts.join("."))
}

/// Return the nested implementation prefix for one platform module path.
fn module_platform_implementation_prefix_from_path(module_path: &Path) -> Option<String> {
    let mut components = module_path.components().peekable();

    // find the platform directory in the library tree
    while let Some(component) = components.next() {
        let component = component.as_os_str().to_str()?;
        if component == "platform" {
            break;
        }
    }

    // skip the top level platform domain
    let _domain = components.next()?;

    // keep the nested directories below the domain
    let mut prefix_parts = Vec::new();
    for component in components {
        let component = component.as_os_str().to_str()?;
        if component.ends_with(".ds") {
            break;
        }

        let component = component.trim();
        if !component.is_empty() {
            prefix_parts.push(component);
        }
    }

    if prefix_parts.is_empty() {
        return None;
    }

    Some(prefix_parts.join("."))
}

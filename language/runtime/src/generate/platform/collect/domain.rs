use std::path::Path;

/// The builtin URI prefix for platform generator modules.
const PLATFORM_URI_PREFIX: &str = "builtin://platform/";

/// Resolve one platform domain name from one builtin platform module uri.
pub(crate) fn module_platform_domain(module_uri: &str) -> Option<String> {
    let trimmed = module_uri.strip_prefix(PLATFORM_URI_PREFIX)?;
    let domain = trimmed.split('/').next().unwrap_or_default().trim();
    if domain.is_empty() {
        return None;
    }

    Some(domain.to_string())
}

/// Prefix one implementation name with nested platform path segments when needed.
pub(crate) fn qualify_platform_implementation_name(
    module_path: Option<&Path>,
    module_uri: &str,
    implementation_name: &str,
) -> String {
    let prefix = module_path
        .and_then(module_platform_implementation_prefix_from_path)
        .or_else(|| module_platform_implementation_prefix(module_uri));
    let Some(prefix) = prefix else {
        return implementation_name.to_string();
    };

    // keep existing qualified names stable
    if implementation_name == prefix || implementation_name.starts_with(&format!("{prefix}.")) {
        return implementation_name.to_string();
    }

    format!("{prefix}.{implementation_name}")
}

/// Return the nested implementation prefix for one builtin platform module uri.
fn module_platform_implementation_prefix(module_uri: &str) -> Option<String> {
    let trimmed = module_uri.strip_prefix(PLATFORM_URI_PREFIX)?;
    let mut parts = trimmed.split('/');

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

/// Return the nested implementation prefix for one builtin platform module path.
fn module_platform_implementation_prefix_from_path(module_path: &Path) -> Option<String> {
    let mut components = module_path.components().peekable();

    // find the platform directory in the builtin source tree
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

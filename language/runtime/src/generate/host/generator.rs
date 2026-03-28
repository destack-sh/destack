use crate::host::model::{HostArtifact, HostCatalog};
use crate::platform::model::WorkspaceLayout;

use super::emit::{android, apple, rust};

/// Generate the host bridge artifacts for all supported categories.
pub(crate) fn generate_host_artifacts(layout: &WorkspaceLayout) -> Vec<HostArtifact> {
    let generated_catalog = HostCatalog::load();
    let generated_modules = generated_catalog.modules();
    let mut files = Vec::new();

    // Android bridge bindings
    files.extend(android::render_binding_files(layout, &generated_catalog));
    files.extend(rust::render_android_binding_files(
        layout,
        &generated_catalog,
    ));

    // per-module Android bridge surface
    for module in generated_modules {
        files.extend(android::render_module_files(layout, module));
        files.extend(rust::render_android_files(layout, module));
    }

    // Apple bridge bindings
    files.extend(apple::render_binding_files(layout, &generated_catalog));
    files.extend(rust::render_ios_binding_files(layout, &generated_catalog));

    // per-module Apple bridge surface
    for module in generated_modules {
        files.extend(apple::render_module_files(layout, module));
        files.extend(rust::render_ios_files(layout, module));
    }

    files
}

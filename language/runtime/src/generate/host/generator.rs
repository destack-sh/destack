use crate::host::model::{HostArtifact, HostCatalog, HostPlatform};
use crate::platform::model::WorkspaceLayout;

use super::emit::{android, apple, rust};

/// Generate the host bridge artifacts for all supported categories.
pub(crate) fn generate_host_artifacts(layout: &WorkspaceLayout) -> Vec<HostArtifact> {
    let generated_catalog = HostCatalog::load();
    let mut files = Vec::new();

    // shared Rust bridge surface
    for module in generated_catalog.modules() {
        files.extend(rust::render_bridge_files(layout, module));
    }

    // Android bridge bindings
    files.extend(android::render_binding_files(layout, &generated_catalog));
    files.extend(rust::render_android_binding_files(
        layout,
        &generated_catalog,
    ));

    // per-module Android bridge surface
    for module in generated_catalog.modules_for_platform(HostPlatform::Android) {
        files.extend(android::render_module_files(layout, module));
        files.extend(rust::render_android_files(layout, module));
        files.extend(rust::render_android_ingress_files(layout, module));
    }

    // Apple bridge bindings
    files.extend(apple::render_binding_files(layout, &generated_catalog));
    files.extend(rust::render_ios_binding_files(layout, &generated_catalog));

    // per-module Apple bridge surface
    for module in generated_catalog.modules_for_platform(HostPlatform::Ios) {
        files.extend(apple::render_module_files(layout, module));
        files.extend(rust::render_ios_files(layout, module));
        files.extend(rust::render_ios_ingress_files(layout, module));
    }

    files
}

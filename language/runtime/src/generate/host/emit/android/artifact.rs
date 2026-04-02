use super::cpp::{
    render_cpp_ingress_bridge_source, render_cpp_methods_header, render_cpp_methods_source,
    render_cpp_runtime_header, render_cpp_runtime_source,
};
use super::kotlin::render_kotlin_abi;
use super::name::android_package_segment;
use crate::host::model::{HostArtifact, HostLayout, HostModule};
use crate::platform::model::WorkspaceLayout;

/// Render the generated Android bridge artifacts for one module.
pub(crate) fn render_module_files(
    layout: &WorkspaceLayout,
    module: &HostModule,
) -> Vec<HostArtifact> {
    let generated_layout = HostLayout::new(layout);
    let module_segment = module.name();
    let package_segment = android_package_segment(module.abi());
    let kotlin_abi = render_kotlin_abi(module.abi());
    let mut files = vec![
        HostArtifact::new(
            generated_layout.android_cpp_runtime_header(module_segment),
            render_cpp_runtime_header(module.abi()),
        ),
        HostArtifact::new(
            generated_layout.android_cpp_runtime_source(module_segment),
            render_cpp_runtime_source(module.abi()),
        ),
        HostArtifact::new(
            generated_layout.android_cpp_callbacks_header(module_segment),
            render_cpp_methods_header(module.abi()),
        ),
        HostArtifact::new(
            generated_layout.android_cpp_callbacks_source(module_segment),
            render_cpp_methods_source(module.abi()),
        ),
    ];

    if !kotlin_abi.is_empty() {
        files.insert(
            0,
            HostArtifact::new(
                generated_layout.android_kotlin_abi(module_segment, &package_segment),
                kotlin_abi,
            ),
        );
    }

    if module.has_ingress() {
        if let Some(contents) = render_cpp_ingress_bridge_source(module.abi()) {
            files.push(HostArtifact::new(
                generated_layout.android_cpp_ingress_source(module_segment),
                contents,
            ));
        }
    }

    files
}

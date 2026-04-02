use std::path::PathBuf;

use crate::platform::model::WorkspaceLayout;

/// The host generator output layout.
pub(crate) struct HostLayout<'a> {
    /// The workspace layout for the runtime crate.
    workspace_layout: &'a WorkspaceLayout,
}

impl<'a> HostLayout<'a> {
    /// Build the host generator layout from one workspace layout.
    pub(crate) fn new(workspace_layout: &'a WorkspaceLayout) -> Self {
        Self { workspace_layout }
    }

    /// Return the generated Android C++ runtime header path for one module.
    pub(crate) fn android_cpp_runtime_header(&self, module_name: &str) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/android/kotlin/src/main/cpp/bridge/{module_name}/runtime.generated.h"
        ))
    }

    /// Return the generated Android C++ runtime source path for one module.
    pub(crate) fn android_cpp_runtime_source(&self, module_name: &str) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/android/kotlin/src/main/cpp/bridge/{module_name}/runtime.generated.cpp"
        ))
    }

    /// Return the generated Android C++ callbacks header path for one module.
    pub(crate) fn android_cpp_callbacks_header(&self, module_name: &str) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/android/kotlin/src/main/cpp/bridge/{module_name}/callbacks.generated.h"
        ))
    }

    /// Return the generated Android C++ callbacks source path for one module.
    pub(crate) fn android_cpp_callbacks_source(&self, module_name: &str) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/android/kotlin/src/main/cpp/bridge/{module_name}/callbacks.generated.cpp"
        ))
    }

    /// Return the generated Android C++ ingress source path for one module.
    pub(crate) fn android_cpp_ingress_source(&self, module_name: &str) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/android/kotlin/src/main/cpp/bridge/{module_name}/ingress.generated.cpp"
        ))
    }

    /// Return the generated Android Kotlin ABI path for one module.
    pub(crate) fn android_kotlin_abi(&self, module_name: &str, package_segment: &str) -> PathBuf {
        let _ = module_name;
        self.workspace_layout.language_root.join(format!(
            "runtime/android/kotlin/src/main/kotlin/dev/destack/runtime/android/bridge/{package_segment}Abi.generated.kt"
        ))
    }

    /// Return the generated Android host-modules path.
    pub(crate) fn android_kotlin_host_modules(&self) -> PathBuf {
        self.workspace_layout.language_root.join(
            "runtime/android/kotlin/src/main/kotlin/dev/destack/runtime/android/core/RuntimeHostModules.generated.kt",
        )
    }

    /// Return the generated Android host-interface path.
    pub(crate) fn android_kotlin_host_interface(
        &self,
        module_name: &str,
        interface_name: &str,
    ) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/android/kotlin/src/main/kotlin/dev/destack/runtime/android/module/{module_name}/{interface_name}.kt"
        ))
    }

    /// Return the generated Apple Swift ABI path for one module.
    pub(crate) fn apple_swift_abi(&self, module_segment: &str) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/apple/ios/Sources/RuntimeHostIOS/Bridge/{module_segment}Abi.generated.swift"
        ))
    }

    /// Return the generated Apple host-modules path.
    pub(crate) fn apple_swift_host_modules(&self) -> PathBuf {
        self.workspace_layout.language_root.join(
            "runtime/apple/core/Sources/RuntimeHostAppleCore/Core/RuntimeHostModules.generated.swift",
        )
    }

    /// Return the generated Apple host-interface path.
    pub(crate) fn apple_swift_host_interface(
        &self,
        module_name: &str,
        interface_name: &str,
    ) -> PathBuf {
        let module_segment = host_module_segment(module_name);

        self.workspace_layout.language_root.join(format!(
            "runtime/apple/core/Sources/RuntimeHostAppleCore/Module/{module_segment}/{interface_name}.swift"
        ))
    }
}

/// Return one PascalCase host module segment.
fn host_module_segment(module_name: &str) -> String {
    let mut output = String::new();

    for segment in module_name.split('_') {
        let mut characters = segment.chars();
        let Some(first_character) = characters.next() else {
            continue;
        };

        output.push(first_character.to_ascii_uppercase());
        output.extend(characters);
    }

    output
}

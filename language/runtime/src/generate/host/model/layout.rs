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
            "runtime/android/kotlin/src/main/cpp/bridge/generated/{module_name}/runtime.generated.h"
        ))
    }

    /// Return the generated Android C++ runtime source path for one module.
    pub(crate) fn android_cpp_runtime_source(&self, module_name: &str) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/android/kotlin/src/main/cpp/bridge/generated/{module_name}/runtime.generated.cpp"
        ))
    }

    /// Return the generated Android C++ callbacks header path for one module.
    pub(crate) fn android_cpp_callbacks_header(&self, module_name: &str) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/android/kotlin/src/main/cpp/bridge/generated/{module_name}/callbacks.generated.h"
        ))
    }

    /// Return the generated Android C++ callbacks source path for one module.
    pub(crate) fn android_cpp_callbacks_source(&self, module_name: &str) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/android/kotlin/src/main/cpp/bridge/generated/{module_name}/callbacks.generated.cpp"
        ))
    }

    /// Return the generated Android C++ ingress source path for one module.
    pub(crate) fn android_cpp_ingress_source(&self, module_name: &str) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/android/kotlin/src/main/cpp/bridge/generated/{module_name}/ingress.generated.cpp"
        ))
    }

    /// Return the generated Android Kotlin ABI path for one module.
    pub(crate) fn android_kotlin_abi(&self, module_name: &str, package_segment: &str) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/android/kotlin/src/main/kotlin/dev/destack/runtime/android/bridge/generated/{module_name}/{package_segment}Abi.generated.kt"
        ))
    }

    /// Return the generated Apple Swift ABI path for one module.
    pub(crate) fn apple_swift_abi(&self, module_segment: &str) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/apple/ios/Sources/RuntimeHostIOS/Bridge/Generated/{module_segment}/{module_segment}Abi.generated.swift"
        ))
    }

    /// Return the generated Apple bridge source path for one module.
    pub(crate) fn apple_bridge_source(&self, module_segment: &str) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/apple/bridge/BridgeC/Bridge/Generated/{module_segment}/Bridge.generated.c"
        ))
    }

    /// Return the generated Apple runtime header path for one module.
    pub(crate) fn apple_runtime_header(&self, module_segment: &str) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/apple/bridge/BridgeC/include/Bridge/Generated/{module_segment}/Runtime.generated.h"
        ))
    }

    /// Return the generated Apple runtime source path for one module.
    pub(crate) fn apple_runtime_source(&self, module_segment: &str) -> PathBuf {
        self.workspace_layout.language_root.join(format!(
            "runtime/apple/bridge/BridgeC/Bridge/Generated/{module_segment}/Runtime.generated.c"
        ))
    }
}

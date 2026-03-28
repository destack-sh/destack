use crate::host::collect::{
    collect_host_binding_lanes, collect_host_bridge_lanes, collect_host_modules,
    collect_host_runtime_binding_specs,
};

use super::{CallbackLane, HostModule, HostPlatform, RuntimeBindingSpec};

/// The authored host generator catalog.
pub(crate) struct HostCatalog {
    /// The authored host modules.
    modules: Vec<HostModule>,
    /// The iOS callback-backed binding lanes.
    ios_binding_lanes: Vec<CallbackLane>,
    /// The Android callback-backed binding lanes.
    android_binding_lanes: Vec<CallbackLane>,
    /// The iOS runtime-bridge lanes.
    ios_bridge_lanes: Vec<CallbackLane>,
    /// The Android runtime-bridge lanes.
    android_bridge_lanes: Vec<CallbackLane>,
    /// The runtime binding functions.
    runtime_bindings: Vec<RuntimeBindingSpec>,
}

impl HostCatalog {
    /// Load the authored host generator catalog.
    pub(crate) fn load() -> Self {
        let modules = collect_host_modules();
        let ios_binding_lanes = collect_host_binding_lanes(HostPlatform::Ios, &modules);
        let android_binding_lanes = collect_host_binding_lanes(HostPlatform::Android, &modules);
        let ios_bridge_lanes = collect_host_bridge_lanes(HostPlatform::Ios, &modules);
        let android_bridge_lanes = collect_host_bridge_lanes(HostPlatform::Android, &modules);
        let runtime_bindings = collect_host_runtime_binding_specs();

        let catalog = Self {
            modules,
            ios_binding_lanes,
            android_binding_lanes,
            ios_bridge_lanes,
            android_bridge_lanes,
            runtime_bindings,
        };

        catalog.validate();
        catalog
    }

    /// Return the authored host modules.
    pub(crate) fn modules(&self) -> &[HostModule] {
        &self.modules
    }

    /// Return the callback-backed binding lanes for one platform.
    pub(crate) fn binding_lanes(&self, platform: HostPlatform) -> &[CallbackLane] {
        match platform {
            HostPlatform::Ios => &self.ios_binding_lanes,
            HostPlatform::Android => &self.android_binding_lanes,
        }
    }

    /// Return the runtime-bridge lanes for one platform.
    pub(crate) fn bridge_lanes(&self, platform: HostPlatform) -> &[CallbackLane] {
        match platform {
            HostPlatform::Ios => &self.ios_bridge_lanes,
            HostPlatform::Android => &self.android_bridge_lanes,
        }
    }

    /// Return the runtime binding functions.
    pub(crate) fn runtime_bindings(&self) -> &[RuntimeBindingSpec] {
        &self.runtime_bindings
    }

    /// Resolve one generated host module by canonical name.
    pub(crate) fn module(&self, name: &str) -> Option<&HostModule> {
        self.modules.iter().find(|module| module.name() == name)
    }

    /// Validate the resolved host catalog.
    fn validate(&self) {
        use std::collections::BTreeSet;

        // module names
        let mut module_names = BTreeSet::new();
        for module in self.modules() {
            assert!(
                module_names.insert(module.name()),
                "duplicate host module {}",
                module.name()
            );
        }

        // lane uniqueness
        self.validate_lanes(self.binding_lanes(HostPlatform::Ios), "ios binding");
        self.validate_lanes(self.binding_lanes(HostPlatform::Android), "android binding");
        self.validate_lanes(self.bridge_lanes(HostPlatform::Ios), "ios bridge");
        self.validate_lanes(self.bridge_lanes(HostPlatform::Android), "android bridge");

        // runtime bindings
        let mut binding_names = BTreeSet::new();
        let mut binding_types = BTreeSet::new();
        for binding in self.runtime_bindings() {
            assert!(
                binding_names.insert(binding.field_name),
                "duplicate host runtime binding {}",
                binding.field_name
            );
            assert!(
                binding_types.insert(binding.type_name),
                "duplicate host runtime binding type {}",
                binding.type_name
            );
        }
    }

    /// Validate one ordered callback lane list.
    fn validate_lanes(&self, lanes: &[CallbackLane], subject: &str) {
        use std::collections::BTreeSet;

        let mut field_names = BTreeSet::new();
        let mut callback_types = BTreeSet::new();
        for lane in lanes {
            assert!(
                field_names.insert(lane.field_name),
                "duplicate {subject} lane {}",
                lane.field_name
            );
            assert!(
                callback_types.insert(lane.callback_type.as_str()),
                "duplicate {subject} callback type {}",
                lane.callback_type
            );
            assert!(
                !lane.subject.is_empty(),
                "missing {subject} lane subject for {}",
                lane.field_name
            );
        }
    }
}

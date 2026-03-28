use crate::host::model::{CallbackLane, HostModule, HostPlatform};
use destack_runtime::host::abi::core::{
    HostAbiManualLane, HostAbiPlatform, host_abi_binding_lane_names, host_abi_bridge_lane_names,
    host_abi_manual_lanes,
};

/// Collect the callback-backed host binding lanes for one platform.
pub(crate) fn collect_host_binding_lanes(
    platform: HostPlatform,
    modules: &[HostModule],
) -> Vec<CallbackLane> {
    let platform = abi_platform(platform);
    let lane_names = host_abi_binding_lane_names(platform);

    collect_platform_lanes(platform, lane_names, modules)
}

/// Collect the runtime-bridge lanes for one platform.
pub(crate) fn collect_host_bridge_lanes(
    platform: HostPlatform,
    modules: &[HostModule],
) -> Vec<CallbackLane> {
    let platform = abi_platform(platform);
    let lane_names = host_abi_bridge_lane_names(platform);

    collect_platform_lanes(platform, lane_names, modules)
}

/// Lower one generator platform into one authored ABI platform.
fn abi_platform(platform: HostPlatform) -> HostAbiPlatform {
    match platform {
        HostPlatform::Ios => HostAbiPlatform::Ios,
        HostPlatform::Android => HostAbiPlatform::Android,
    }
}

/// Collect the callback lanes for one ordered platform lane set.
fn collect_platform_lanes(
    platform: HostAbiPlatform,
    lane_names: &[&'static str],
    modules: &[HostModule],
) -> Vec<CallbackLane> {
    lane_names
        .iter()
        .map(|name| collect_platform_lane(platform, name, modules))
        .collect()
}

/// Collect one callback lane for one platform and lane name.
fn collect_platform_lane(
    platform: HostAbiPlatform,
    name: &'static str,
    modules: &[HostModule],
) -> CallbackLane {
    // generated lane
    if let Some(module) = modules.iter().find(|module| module.name() == name) {
        return generated_lane(platform, module);
    }

    // handwritten lane
    if let Some(lane) = host_abi_manual_lanes(platform)
        .iter()
        .find(|lane| lane.name == name)
    {
        return manual_lane(lane);
    }

    panic!("missing host callback lane for {:?}: {name}", platform);
}

/// Lower one handwritten callback lane.
fn manual_lane(lane: &HostAbiManualLane) -> CallbackLane {
    CallbackLane {
        field_name: lane.name,
        subject: lane.subject.to_string(),
        callback_type: lane.callback_type.to_string(),
        module_name: lane.module_name,
        submodule_name: lane.submodule_name,
    }
}

/// Build one generated callback lane from one generated host module.
fn generated_lane(platform: HostAbiPlatform, module: &HostModule) -> CallbackLane {
    CallbackLane {
        field_name: module.name(),
        subject: pascal_case(module.name()),
        callback_type: generated_callback_type(platform, module.name()),
        module_name: module.name(),
        submodule_name: "callbacks",
    }
}

/// Return the generated callback type name for one platform and module.
fn generated_callback_type(platform: HostAbiPlatform, module_name: &str) -> String {
    let platform_name = match platform {
        HostAbiPlatform::Ios => "Ios",
        HostAbiPlatform::Android => "Android",
    };
    let module_name = pascal_case(module_name);

    format!("{platform_name}Host{module_name}Callbacks")
}

/// Convert one snake-case lane name into PascalCase.
fn pascal_case(name: &str) -> String {
    let mut output = String::new();

    for segment in name.split('_') {
        let mut characters = segment.chars();
        let Some(first_character) = characters.next() else {
            continue;
        };

        output.push(first_character.to_ascii_uppercase());
        output.extend(characters);
    }

    output
}

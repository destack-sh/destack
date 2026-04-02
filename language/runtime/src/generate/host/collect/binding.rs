use crate::host::model::{RuntimeBindingParameter, RuntimeBindingSpec, RuntimeBindingType};
use destack_runtime::host::abi::core::{
    HostAbiRuntimeBindingParameter, HostAbiRuntimeBindingSpec, HostAbiRuntimeBindingType,
    host_abi_runtime_binding_specs,
};

/// Collect the authored host runtime binding function specs.
pub(crate) fn collect_host_runtime_binding_specs() -> Vec<RuntimeBindingSpec> {
    host_abi_runtime_binding_specs()
        .iter()
        .map(runtime_binding_spec)
        .collect()
}

/// Lower one authored runtime binding function spec.
fn runtime_binding_spec(spec: &HostAbiRuntimeBindingSpec) -> RuntimeBindingSpec {
    // lowered parameters
    let parameters = spec
        .parameters
        .iter()
        .map(runtime_binding_parameter)
        .collect();

    RuntimeBindingSpec {
        field_name: spec.field_name,
        type_name: spec.type_name,
        documentation: spec.documentation,
        result_type: runtime_binding_type(spec.result_type),
        parameters,
    }
}

/// Lower one authored runtime binding parameter.
fn runtime_binding_parameter(
    parameter: &HostAbiRuntimeBindingParameter,
) -> RuntimeBindingParameter {
    RuntimeBindingParameter {
        ty: runtime_binding_type(parameter.ty),
        name: parameter.name,
    }
}

/// Lower one authored runtime binding type.
fn runtime_binding_type(ty: HostAbiRuntimeBindingType) -> RuntimeBindingType {
    match ty {
        HostAbiRuntimeBindingType::RuntimeStatus => RuntimeBindingType::RuntimeStatus,
        HostAbiRuntimeBindingType::DocumentDescriptorSlice => {
            RuntimeBindingType::DocumentDescriptorSlice
        }
        HostAbiRuntimeBindingType::IntentEvent => RuntimeBindingType::IntentEvent,
        HostAbiRuntimeBindingType::NotificationEvent => RuntimeBindingType::NotificationEvent,
        HostAbiRuntimeBindingType::PermissionEvent => RuntimeBindingType::PermissionEvent,
        HostAbiRuntimeBindingType::StringRef => RuntimeBindingType::StringRef,
        HostAbiRuntimeBindingType::StringSlice => RuntimeBindingType::StringSlice,
        HostAbiRuntimeBindingType::LocationSample => RuntimeBindingType::LocationSample,
        HostAbiRuntimeBindingType::TextInputEvent => RuntimeBindingType::TextInputEvent,
        HostAbiRuntimeBindingType::BackgroundEvent => RuntimeBindingType::BackgroundEvent,
        HostAbiRuntimeBindingType::U64 => RuntimeBindingType::U64,
        HostAbiRuntimeBindingType::Bool => RuntimeBindingType::Bool,
    }
}

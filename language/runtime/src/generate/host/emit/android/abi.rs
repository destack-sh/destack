use destack_runtime::host::abi::describe::{
    HostAbiField, HostAbiModule, HostAbiNamedType, HostAbiNamedTypeDefinition, HostAbiType,
};

/// Return one named ABI type from one Android host module.
pub(super) fn android_named_type<'a>(
    module: &'a HostAbiModule,
    name: &str,
) -> &'a HostAbiNamedType {
    module
        .types
        .iter()
        .find(|named_type| named_type.name == name)
        .unwrap_or_else(|| panic!("missing Android named type {name} in {}", module.name))
}

/// Return one named ABI struct field list from one Android host module.
pub(super) fn android_named_struct_fields<'a>(
    module: &'a HostAbiModule,
    name: &str,
) -> &'a [HostAbiField] {
    match &android_named_type(module, name).definition {
        HostAbiNamedTypeDefinition::Struct { fields } => fields,
        HostAbiNamedTypeDefinition::Enum { .. } => {
            panic!(
                "expected Android named struct for {name} in {}",
                module.name
            )
        }
    }
}

/// Return whether one named Android ABI type is one enum.
pub(super) fn android_named_type_is_enum(module: &HostAbiModule, name: &str) -> bool {
    matches!(
        android_named_type(module, name).definition,
        HostAbiNamedTypeDefinition::Enum { .. }
    )
}

/// Return whether one module uses one ABI type anywhere in its surface.
pub(super) fn android_module_uses_type(module: &HostAbiModule, ty: &HostAbiType) -> bool {
    module
        .types
        .iter()
        .any(|named_type| android_named_type_uses_type(module, named_type, ty))
        || module.requests.iter().any(|request| {
            request.result == *ty
                || request
                    .parameters
                    .iter()
                    .any(|parameter| android_type_contains_type(module, &parameter.ty, ty))
        })
        || module.ingress.iter().any(|ingress| {
            ingress.result == *ty
                || ingress
                    .parameters
                    .iter()
                    .any(|parameter| android_type_contains_type(module, &parameter.ty, ty))
        })
}

/// Return whether one named type contains one ABI type.
fn android_named_type_uses_type(
    module: &HostAbiModule,
    named_type: &HostAbiNamedType,
    ty: &HostAbiType,
) -> bool {
    match &named_type.definition {
        HostAbiNamedTypeDefinition::Struct { fields } => fields
            .iter()
            .any(|field| android_type_contains_type(module, &field.ty, ty)),
        HostAbiNamedTypeDefinition::Enum { .. } => false,
    }
}

/// Return whether one ABI type contains one nested ABI type.
fn android_type_contains_type(
    module: &HostAbiModule,
    outer: &HostAbiType,
    inner: &HostAbiType,
) -> bool {
    if outer == inner {
        return true;
    }

    match outer {
        HostAbiType::Named(name) if !android_named_type_is_enum(module, name) => {
            android_named_struct_fields(module, name)
                .iter()
                .any(|field| android_type_contains_type(module, &field.ty, inner))
        }
        HostAbiType::NativeArray(element)
        | HostAbiType::NativeSlice(element)
        | HostAbiType::OutputPointer(element) => android_type_contains_type(module, element, inner),
        _ => false,
    }
}

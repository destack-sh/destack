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
) -> Vec<HostAbiField> {
    match &android_named_type(module, name).definition {
        HostAbiNamedTypeDefinition::Struct { fields } => fields.clone(),
        HostAbiNamedTypeDefinition::TaggedEnum { variants } => variants
            .iter()
            .map(|variant| HostAbiField {
                name: Box::leak(android_tagged_variant_field_name(variant.name).into_boxed_str()),
                documentation: variant.documentation,
                ty: HostAbiType::Named(variant.payload_type),
            })
            .collect(),
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
        HostAbiNamedTypeDefinition::TaggedEnum { .. } => {
            android_named_struct_fields(module, named_type.name)
                .iter()
                .any(|field| android_type_contains_type(module, &field.ty, ty))
        }
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
        HostAbiType::Named(name) => match &android_named_type(module, name).definition {
            HostAbiNamedTypeDefinition::Struct { .. }
            | HostAbiNamedTypeDefinition::TaggedEnum { .. } => {
                android_named_struct_fields(module, name)
                    .iter()
                    .any(|field| android_type_contains_type(module, &field.ty, inner))
            }
            HostAbiNamedTypeDefinition::Enum { .. } => false,
        },
        HostAbiType::NativeArray(element)
        | HostAbiType::NativeSlice(element)
        | HostAbiType::OutputPointer(element) => android_type_contains_type(module, element, inner),
        _ => false,
    }
}

/// Return the lowered field name for one tagged Android payload variant.
fn android_tagged_variant_field_name(name: &str) -> String {
    let mut output = String::new();

    for (index, character) in name.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if index > 0 {
                output.push('_');
            }

            output.push(character.to_ascii_lowercase());
        } else {
            output.push(character);
        }
    }

    output
}

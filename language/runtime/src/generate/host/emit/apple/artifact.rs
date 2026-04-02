use std::collections::BTreeSet;

use super::docs::push_c_doc_comment;
use super::host::{HostRole, render_swift_host_modules, render_swift_host_protocol};
use super::name::apple_pascal_case;
use super::swift::{render_swift_abi, render_swift_runtime_abi, render_swift_runtime_bridge};
use crate::host::model::{HostArtifact, HostCatalog, HostLayout, HostModule, HostPlatform};
use crate::platform::model::WorkspaceLayout;
use destack_runtime::host::abi::describe::{
    HostAbiEnumRepresentation, HostAbiField, HostAbiFunction, HostAbiModule, HostAbiNamedType,
    HostAbiNamedTypeDefinition, HostAbiRuntimeHostWrapperKind, HostAbiType,
};

/// Render the generated Apple bridge binding files.
pub(crate) fn render_binding_files(
    layout: &WorkspaceLayout,
    generated_catalog: &HostCatalog,
) -> Vec<HostArtifact> {
    let generated_layout = HostLayout::new(layout);
    let generated_modules: Vec<_> = generated_catalog
        .modules_for_platform(HostPlatform::Ios)
        .into_iter()
        .map(|module| module.abi().clone())
        .collect();
    let bridge_modules = generated_catalog.modules_for_platform(HostPlatform::Ios);
    let host_modules = generated_modules
        .iter()
        .filter(|module| !module.runtime_host_platforms.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    let generated_host_wrappers = host_modules
        .iter()
        .filter(|module| {
            module.runtime_host_wrapper_kind == HostAbiRuntimeHostWrapperKind::Generated
        })
        .cloned()
        .collect::<Vec<_>>();
    let ingress_modules = generated_catalog
        .modules_for_platform(HostPlatform::Ios)
        .into_iter()
        .filter(|module| module.has_ingress())
        .collect::<Vec<_>>();

    let mut files = vec![
        HostArtifact {
            path: layout
                .language_root
                .join("runtime/apple/bridge/BridgeC/include/Bridge/BaseTypes.generated.h"),
            contents: render_bridge_base_types_header(),
        },
        HostArtifact {
            path: layout
                .language_root
                .join("runtime/apple/bridge/BridgeC/include/Bridge/Types.h"),
            contents: render_bridge_types_header(generated_catalog),
        },
        HostArtifact {
            path: layout
                .language_root
                .join("runtime/apple/bridge/BridgeC/include/Bridge/AbiTypes.generated.h"),
            contents: render_abi_types_header(&generated_modules),
        },
        HostArtifact {
            path: layout
                .language_root
                .join("runtime/apple/bridge/BridgeC/include/Bridge/RuntimeBridge.generated.h"),
            contents: render_runtime_bridge_header(generated_catalog),
        },
        HostArtifact {
            path: layout
                .language_root
                .join("runtime/apple/bridge/BridgeC/include/Bridge/Public.generated.h"),
            contents: render_public_generated_header(),
        },
        HostArtifact {
            path: layout.language_root.join(
                "runtime/apple/ios/Sources/RuntimeHostIOS/Bridge/RuntimeIngress.generated.swift",
            ),
            contents: render_runtime_abi_generated(
                generated_catalog,
                &ingress_modules,
                &generated_modules,
            ),
        },
        HostArtifact {
            path: layout.language_root.join(
                "runtime/apple/ios/Sources/RuntimeHostIOS/Bridge/RuntimeBridge.generated.swift",
            ),
            contents: render_swift_runtime_bridge(&bridge_modules),
        },
        HostArtifact {
            path: generated_layout.apple_swift_host_modules(),
            contents: render_swift_host_modules(&generated_host_wrappers),
        },
    ];

    for module in &host_modules {
        if !module.requests.is_empty() {
            let interface_name = format!("{}Requests", apple_pascal_case(module.name));
            files.push(HostArtifact::new(
                generated_layout.apple_swift_host_interface(module.name, &interface_name),
                render_swift_host_protocol(module, HostRole::Requests),
            ));
        }

        if !module.ingress.is_empty() {
            let interface_name = format!("{}Events", apple_pascal_case(module.name));
            files.push(HostArtifact::new(
                generated_layout.apple_swift_host_interface(module.name, &interface_name),
                render_swift_host_protocol(module, HostRole::Events),
            ));
        }
    }

    files
}

/// Render the generated Apple bridge artifacts for one module.
pub(crate) fn render_module_files(
    layout: &WorkspaceLayout,
    module: &HostModule,
) -> Vec<HostArtifact> {
    let generated_layout = HostLayout::new(layout);

    vec![HostArtifact::new(
        generated_layout.apple_swift_abi(&apple_module_segment(module.abi())),
        render_swift_abi(module),
    )]
}

fn render_bridge_base_types_header() -> String {
    let mut output = String::new();
    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("#ifndef RUNTIME_HOST_APPLE_BRIDGE_BASE_TYPES_GENERATED_H\n");
    output.push_str("#define RUNTIME_HOST_APPLE_BRIDGE_BASE_TYPES_GENERATED_H\n\n");
    output.push_str("#include <stdbool.h>\n");
    output.push_str("#include <stdint.h>\n\n");
    output.push_str("/// One runtime status returned by one ingress call.\n");
    output.push_str("typedef struct DestackRustRuntimeStatus {\n");
    output.push_str("    /// The status code.\n");
    output.push_str("    uint32_t code;\n");
    output.push_str("    /// The optional runtime error identifier.\n");
    output.push_str("    uint64_t error_id;\n");
    output.push_str("} DestackRustRuntimeStatus;\n\n");
    output.push_str("/// One string reference passed through the Apple bridge.\n");
    output.push_str("typedef struct DestackRustStringRef {\n");
    output.push_str("    /// The string data pointer.\n");
    output.push_str("    const uint8_t *data;\n");
    output.push_str("    /// The string length in bytes.\n");
    output.push_str("    uint32_t len;\n");
    output.push_str("} DestackRustStringRef;\n\n");
    output.push_str("/// One string slice passed through the Apple bridge.\n");
    output.push_str("typedef struct DestackRustStringSlice {\n");
    output.push_str("    /// The slice data pointer.\n");
    output.push_str("    const DestackRustStringRef *data;\n");
    output.push_str("    /// The slice length in elements.\n");
    output.push_str("    uint32_t len;\n");
    output.push_str("} DestackRustStringSlice;\n\n");
    output.push_str("/// One `u8` slice passed through the Apple bridge.\n");
    output.push_str("typedef struct DestackRustU8Slice {\n");
    output.push_str("    /// The slice data pointer.\n");
    output.push_str("    const uint8_t *data;\n");
    output.push_str("    /// The slice length in elements.\n");
    output.push_str("    uint32_t len;\n");
    output.push_str("} DestackRustU8Slice;\n\n");
    output.push_str("/// One `i8` slice passed through the Apple bridge.\n");
    output.push_str("typedef struct DestackRustI8Slice {\n");
    output.push_str("    /// The slice data pointer.\n");
    output.push_str("    const int8_t *data;\n");
    output.push_str("    /// The slice length in elements.\n");
    output.push_str("    uint32_t len;\n");
    output.push_str("} DestackRustI8Slice;\n\n");
    output.push_str("/// One `i16` slice passed through the Apple bridge.\n");
    output.push_str("typedef struct DestackRustI16Slice {\n");
    output.push_str("    /// The slice data pointer.\n");
    output.push_str("    const int16_t *data;\n");
    output.push_str("    /// The slice length in elements.\n");
    output.push_str("    uint32_t len;\n");
    output.push_str("} DestackRustI16Slice;\n\n");
    output.push_str("#endif\n");
    output
}

/// Render one generated Apple ABI-types header.
fn render_abi_types_header(generated_modules: &[HostAbiModule]) -> String {
    let mut output = String::new();
    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("#ifndef RUNTIME_HOST_APPLE_BRIDGE_ABI_TYPES_GENERATED_H\n");
    output.push_str("#define RUNTIME_HOST_APPLE_BRIDGE_ABI_TYPES_GENERATED_H\n\n");
    output.push_str("#include \"BaseTypes.generated.h\"\n\n");

    for module in generated_modules {
        for named_type in &module.types {
            render_apple_named_type_forward_declaration(&mut output, named_type);
        }
    }

    if !generated_modules.is_empty() {
        output.push('\n');
    }

    for module in generated_modules {
        for named_type in &module.types {
            if let HostAbiNamedTypeDefinition::Enum { .. } = &named_type.definition {
                render_apple_named_type(&mut output, named_type);
                output.push('\n');
            }
        }
    }

    for generated_type in collect_apple_generated_container_types(generated_modules) {
        render_apple_generated_container_type(&mut output, &generated_type);
        output.push('\n');
    }

    let mut emitted_optional_types = Vec::new();
    for module in generated_modules {
        for named_type in &module.types {
            if let HostAbiNamedTypeDefinition::Struct { .. } = &named_type.definition {
                render_apple_struct_optional_types(
                    &mut output,
                    module,
                    named_type,
                    &mut emitted_optional_types,
                );
                render_apple_named_type(&mut output, named_type);
                output.push('\n');
            }
        }
    }

    for generated_type in collect_apple_generated_optional_types(generated_modules) {
        if emitted_optional_types
            .iter()
            .any(|existing: &String| existing == &generated_type.name)
        {
            continue;
        }

        emitted_optional_types.push(generated_type.name.clone());
        render_apple_generated_optional_type(&mut output, &generated_type);
        output.push('\n');
    }

    output.push_str("#endif\n");
    output
}

/// Render one generated Apple bridge types header.
fn render_bridge_types_header(generated_catalog: &HostCatalog) -> String {
    let mut output = String::new();
    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("#ifndef RUNTIME_HOST_APPLE_BRIDGE_TYPES_H\n");
    output.push_str("#define RUNTIME_HOST_APPLE_BRIDGE_TYPES_H\n\n");
    output.push_str("#include \"BaseTypes.generated.h\"\n");
    output.push_str("#include \"Public.generated.h\"\n");
    output.push_str("#include \"AbiTypes.generated.h\"\n");
    output.push_str("#include \"RuntimeBridge.generated.h\"\n\n");
    output.push_str("#include <stdbool.h>\n");
    output.push_str("#include <stdint.h>\n\n");
    output.push_str("typedef DestackRustStringRef NativeStringRef;\n");
    output.push_str("typedef DestackRustStringSlice NativeStringSlice;\n\n");

    output.push_str("typedef uint32_t (*RegisterRuntimeBridgeBindingsFunction)(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    IosRuntimeBridgeBindings callbacks\n");
    output.push_str(");\n");
    output.push_str("typedef void (*UnregisterRuntimeBridgeBindingsFunction)(\n");
    output.push_str("    uint64_t session_handle\n");
    output.push_str(");\n\n");

    for spec in generated_catalog.runtime_bindings() {
        output.push_str(&format!("/// {}\n", spec.documentation));
        output.push_str(&format!(
            "typedef {} (*{})(",
            spec.result_type.native_name(),
            spec.type_name
        ));

        if spec.parameters.is_empty() {
            output.push_str("void");
        } else {
            output.push('\n');
            for (index, parameter) in spec.parameters.iter().enumerate() {
                let trailing = if index + 1 == spec.parameters.len() {
                    ""
                } else {
                    ","
                };
                output.push_str(&format!(
                    "    {} {}{}\n",
                    parameter.ty.native_name(),
                    parameter.name,
                    trailing
                ));
            }
        }

        output.push_str(");\n\n");
    }

    output.push_str("#endif\n");
    output
}

/// Render one generated Apple public bridge header.
fn render_public_generated_header() -> String {
    let mut output = String::new();
    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("#ifndef RUNTIME_HOST_APPLE_BRIDGE_PUBLIC_GENERATED_H\n");
    output.push_str("#define RUNTIME_HOST_APPLE_BRIDGE_PUBLIC_GENERATED_H\n\n");
    output.push_str("#include \"BaseTypes.generated.h\"\n\n");
    output.push_str("#endif\n");
    output
}

/// Render one generated Apple runtime-bridge public header.
fn render_runtime_bridge_header(generated_catalog: &HostCatalog) -> String {
    let generated_ingress_names = generated_catalog
        .modules_for_platform(HostPlatform::Ios)
        .into_iter()
        .flat_map(|module| module.abi().ingress.iter().map(|ingress| ingress.name))
        .collect::<BTreeSet<_>>();
    let mut output = String::new();
    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("#ifndef RUNTIME_HOST_APPLE_BRIDGE_RUNTIME_GENERATED_H\n");
    output.push_str("#define RUNTIME_HOST_APPLE_BRIDGE_RUNTIME_GENERATED_H\n\n");

    for module in generated_catalog.modules_for_platform(HostPlatform::Ios) {
        render_apple_generated_callback_typedefs(&mut output, module.abi());
        output.push('\n');
    }

    for module in generated_catalog.modules_for_platform(HostPlatform::Ios) {
        render_apple_generated_callback_struct(&mut output, module.abi());
        output.push('\n');
    }

    push_c_doc_comment(
        &mut output,
        "The mobile runtime-bridge callback table for one runtime session.",
        0,
    );
    output.push_str("typedef struct IosRuntimeBridgeBindings {\n");

    for lane in generated_catalog.bridge_lanes(HostPlatform::Ios) {
        output.push_str(&format!(
            "    {} {};\n",
            lane.callback_type, lane.field_name
        ));
    }

    output.push_str("} IosRuntimeBridgeBindings;\n\n");
    push_c_doc_comment(
        &mut output,
        "Register one mobile bridge callback table for one runtime session.",
        0,
    );
    output.push_str("uint32_t destack_host_ios_register_runtime_bridge_bindings(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    IosRuntimeBridgeBindings callbacks\n");
    output.push_str(");\n\n");
    push_c_doc_comment(
        &mut output,
        "Unregister one mobile bridge callback table for one runtime session.",
        0,
    );
    output.push_str("void destack_host_ios_unregister_runtime_bridge_bindings(\n");
    output.push_str("    uint64_t session_handle\n");
    output.push_str(");\n\n");

    for module in generated_catalog.modules_for_platform(HostPlatform::Ios) {
        if !module_has_generated_ingress(module.abi()) {
            continue;
        }

        render_apple_generated_ingress_declaration(&mut output, module.abi());
        output.push('\n');
    }

    for spec in generated_catalog.runtime_bindings() {
        if generated_ingress_names.contains(spec.field_name) {
            continue;
        }

        render_apple_runtime_binding_declaration(&mut output, spec);
        output.push('\n');
    }

    output.push_str("#endif\n");
    output
}

/// Render one generated Apple runtime-assembly helper.
fn render_runtime_abi_generated(
    generated_catalog: &HostCatalog,
    ingress_modules: &[&HostModule],
    _generated_modules: &[HostAbiModule],
) -> String {
    let mut output = String::new();
    output.push_str(&render_swift_runtime_abi(ingress_modules));
    output.push('\n');
    output.push_str(
        "/// Build one runtime bridge callback table with the generated host ABI modules.\n",
    );
    output.push_str("func makeGeneratedRuntimeBridgeBindings() -> IosRuntimeBridgeBindings {\n");
    output.push_str("  IosRuntimeBridgeBindings(\n");

    for (index, lane) in generated_catalog
        .bridge_lanes(HostPlatform::Ios)
        .iter()
        .enumerate()
    {
        let trailing = if index + 1 == generated_catalog.bridge_lanes(HostPlatform::Ios).len() {
            ""
        } else {
            ","
        };
        let module = generated_catalog
            .module_for_platform(HostPlatform::Ios, lane.field_name)
            .unwrap_or_else(|| panic!("missing iOS runtime bridge module {}", lane.field_name));
        let value = apple_generated_runtime_slot_factory(module.abi());

        output.push_str(&format!("    {}: {}{}\n", lane.field_name, value, trailing));
    }

    output.push_str("  )\n");
    output.push_str("}\n");
    output
}

/// One generated Apple container type.
struct AppleGeneratedContainerType {
    /// The canonical container name.
    name: String,
    /// The element type name.
    element_name: String,
    /// The container documentation.
    documentation: String,
    /// Whether the container owns capacity metadata.
    is_owned_array: bool,
}

/// One generated Apple optional wrapper type.
struct AppleGeneratedOptionalType {
    /// The canonical wrapper name.
    name: String,
    /// The wrapped value type.
    value_type: String,
    /// The wrapper documentation.
    documentation: String,
}

/// Render one generated Apple callback struct for one module.
fn render_apple_generated_callback_typedefs(output: &mut String, module: &HostAbiModule) {
    for request in &module.requests {
        let type_name = render_apple_generated_callback_type_name(module, request);
        let callback_subject = render_apple_generated_callback_subject(module, request);
        push_c_doc_comment(
            output,
            &format!("The {callback_subject} callback type registered for one runtime session."),
            0,
        );
        output.push_str(&format!("typedef uint32_t (*{type_name})(\n"));

        for (index, parameter) in request.parameters.iter().enumerate() {
            let trailing = if index + 1 == request.parameters.len() {
                ""
            } else {
                ","
            };
            output.push_str(&format!(
                "    {} {}{}\n",
                render_apple_public_type(&parameter.ty),
                parameter.name,
                trailing
            ));
        }

        output.push_str(");\n\n");
    }
}

/// Render one generated Apple callback struct for one module.
fn render_apple_generated_callback_struct(output: &mut String, module: &HostAbiModule) {
    push_c_doc_comment(
        output,
        &format!(
            "The {} callbacks registered for one runtime session.",
            render_apple_callback_struct_subject(module)
        ),
        0,
    );
    output.push_str(&format!(
        "typedef struct {} {{\n",
        apple_generated_runtime_slot_type(module)
    ));

    for request in &module.requests {
        output.push_str(&format!(
            "    {} {};\n",
            render_apple_generated_callback_type_name(module, request),
            request.name
        ));
    }

    output.push_str(&format!(
        "}} {};\n",
        apple_generated_runtime_slot_type(module)
    ));
}

/// Render one generated Apple ingress declaration for one module.
fn render_apple_generated_ingress_declaration(output: &mut String, module: &HostAbiModule) {
    let ingress = single_ingress(module);
    let ingress_subject = render_apple_generated_ingress_subject(module);
    push_c_doc_comment(
        output,
        &format!("Deliver one {ingress_subject} into one runtime session."),
        0,
    );
    output.push_str(&format!(
        "{} destack_host_ios_{}(\n",
        render_apple_public_type(&ingress.result),
        ingress.name
    ));

    for (index, parameter) in ingress.parameters.iter().enumerate() {
        let trailing = if index + 1 == ingress.parameters.len() {
            ""
        } else {
            ","
        };
        output.push_str(&format!(
            "    {} {}{}\n",
            render_apple_public_type(&parameter.ty),
            parameter.name,
            trailing
        ));
    }

    output.push_str(");\n");
}

/// Render one Apple runtime-binding declaration.
fn render_apple_runtime_binding_declaration(
    output: &mut String,
    spec: &crate::host::model::RuntimeBindingSpec,
) {
    push_c_doc_comment(
        output,
        &format!(
            "{}.",
            spec.documentation
                .trim_end_matches('.')
                .replace("The runtime function that receives one ", "Deliver one ")
                .replace(" into one runtime session", "")
                .replace("event", "event into one runtime session")
        ),
        0,
    );
    output.push_str(&format!(
        "{} destack_host_ios_{}(\n",
        spec.result_type.apple_public_name(),
        spec.field_name
    ));

    for (index, parameter) in spec.parameters.iter().enumerate() {
        let trailing = if index + 1 == spec.parameters.len() {
            ""
        } else {
            ","
        };
        output.push_str(&format!(
            "    {} {}{}\n",
            parameter.ty.apple_public_name(),
            parameter.name,
            trailing
        ));
    }

    output.push_str(");\n");
}

/// Return the Apple callback-struct subject phrase for one module.
fn render_apple_callback_struct_subject(module: &HostAbiModule) -> String {
    module.name.replace('_', "-")
}

/// Return the Apple callback subject phrase for one generated request.
fn render_apple_generated_callback_subject(
    module: &HostAbiModule,
    request: &HostAbiFunction,
) -> String {
    if module.requests.len() == 1 {
        module.name.replace('_', "-")
    } else {
        format!(
            "{}-{}",
            module.name.replace('_', "-"),
            request.name.replace('_', "-")
        )
    }
}

/// Return the Apple callback typedef name used by one generated callback struct field.
fn render_apple_generated_callback_type_name(
    module: &HostAbiModule,
    request: &HostAbiFunction,
) -> String {
    if module.requests.len() == 1 {
        format!("DestackRust{}Callback", apple_module_segment(module))
    } else {
        format!(
            "DestackRust{}{}Callback",
            apple_module_segment(module),
            apple_pascal_case(request.name)
        )
    }
}

/// Return the Apple ingress subject phrase for one generated module.
fn render_apple_generated_ingress_subject(module: &HostAbiModule) -> String {
    let ingress = single_ingress(module);

    ingress
        .documentation
        .trim()
        .trim_end_matches('.')
        .trim_start_matches("Deliver one ")
        .trim_end_matches(" into one runtime session")
        .to_ascii_lowercase()
}

/// Return the generated callback factory for one Apple runtime slot.
fn apple_generated_runtime_slot_factory(module: &HostAbiModule) -> String {
    format!("make{}Callbacks()", apple_runtime_slot_segment(module.name))
}

/// Return the callbacks type name for one generated Apple runtime slot.
fn apple_generated_runtime_slot_type(module: &HostAbiModule) -> String {
    format!(
        "IosHost{}Callbacks",
        apple_runtime_slot_segment(module.name)
    )
}

/// Return the type segment for one Apple runtime slot.
fn apple_runtime_slot_segment(name: &str) -> String {
    let mut output = String::new();

    for segment in name.split('_') {
        let mut chars = segment.chars();
        if let Some(first) = chars.next() {
            output.extend(first.to_uppercase());
            output.extend(chars);
        }
    }

    output
}

/// Return the single ingress function for one module.
fn single_ingress(module: &HostAbiModule) -> &HostAbiFunction {
    module
        .ingress
        .first()
        .unwrap_or_else(|| panic!("missing ingress function for {}", module.name))
}

/// Return whether one module emits generated Apple ingress shims.
fn module_has_generated_ingress(module: &HostAbiModule) -> bool {
    !module.ingress.is_empty()
}

/// Return the Apple module path segment for one module.
fn apple_module_segment(module: &HostAbiModule) -> String {
    apple_pascal_case(module.name)
}

/// Return the exported ABI type name for one authored Rust host ABI type.
fn render_external_abi_name(name: &str) -> &str {
    name
}

/// Render one generated Apple named type.
fn render_apple_named_type(output: &mut String, named_type: &HostAbiNamedType) {
    push_c_doc_comment(output, named_type.documentation, 0);

    match &named_type.definition {
        HostAbiNamedTypeDefinition::Struct { fields } => {
            let external_name = render_external_abi_name(named_type.name);
            output.push_str(&format!("typedef struct DestackRust{external_name} {{\n"));

            for field in fields {
                render_apple_named_field(output, field);
            }

            output.push_str(&format!("}} DestackRust{external_name};\n"));
        }
        HostAbiNamedTypeDefinition::Enum { repr, .. } => {
            let external_name = render_external_abi_name(named_type.name);
            let repr = render_apple_enum_repr(*repr);
            output.push_str(&format!("typedef {repr} DestackRust{external_name};\n"));
        }
        HostAbiNamedTypeDefinition::TaggedEnum { variants } => {
            let external_name = render_external_abi_name(named_type.name);
            let tag_name = format!("DestackRust{external_name}Tag");

            output.push_str(&format!("typedef uint32_t {tag_name};\n\n"));
            output.push_str(&format!("typedef struct DestackRust{external_name} {{\n"));
            output.push_str(&format!("    {tag_name} tag;\n"));

            for variant in variants {
                let field_name = apple_tagged_variant_field_name(variant.name);
                output.push_str(&format!(
                    "    DestackRust{} {};\n",
                    render_external_abi_name(variant.payload_type),
                    field_name
                ));
            }

            output.push_str(&format!("}} DestackRust{external_name};\n"));
        }
    }
}

/// Render one generated Apple named-type forward declaration.
fn render_apple_named_type_forward_declaration(output: &mut String, named_type: &HostAbiNamedType) {
    match &named_type.definition {
        HostAbiNamedTypeDefinition::Struct { .. }
        | HostAbiNamedTypeDefinition::TaggedEnum { .. } => {
            let external_name = render_external_abi_name(named_type.name);
            output.push_str(&format!(
                "typedef struct DestackRust{external_name} DestackRust{external_name};\n"
            ));
        }
        HostAbiNamedTypeDefinition::Enum { .. } => {}
    }
}

/// Render the generated Apple optional wrappers needed by one named struct.
fn render_apple_struct_optional_types(
    output: &mut String,
    module: &HostAbiModule,
    named_type: &HostAbiNamedType,
    emitted_optional_types: &mut Vec<String>,
) {
    let HostAbiNamedTypeDefinition::Struct { fields } = &named_type.definition else {
        return;
    };

    let mut generated_types = Vec::new();
    let mut visited_named_types = Vec::new();

    for field in fields {
        collect_apple_generated_optional_types_from_type(
            module,
            &field.ty,
            &mut generated_types,
            &mut visited_named_types,
        );
    }

    for generated_type in generated_types {
        if emitted_optional_types
            .iter()
            .any(|existing| existing == &generated_type.name)
        {
            continue;
        }

        emitted_optional_types.push(generated_type.name.clone());
        render_apple_generated_optional_type(output, &generated_type);
        output.push('\n');
    }
}

/// Render one generated Apple named field.
fn render_apple_named_field(output: &mut String, field: &HostAbiField) {
    push_c_doc_comment(output, field.documentation, 4);
    output.push_str(&format!(
        "    {} {};\n",
        render_apple_public_type(&field.ty),
        field.name
    ));
}

/// Render one generated Apple container type.
fn render_apple_generated_container_type(
    output: &mut String,
    generated_type: &AppleGeneratedContainerType,
) {
    let element_name = &generated_type.element_name;
    let data_pointer = if generated_type.is_owned_array {
        format!("DestackRust{element_name} *")
    } else {
        format!("const DestackRust{element_name} *")
    };
    push_c_doc_comment(output, &generated_type.documentation, 0);
    output.push_str(&format!("typedef struct {} {{\n", generated_type.name));
    output.push_str(&format!("    {data_pointer}data;\n"));
    output.push_str("    uint32_t len;\n");

    if generated_type.is_owned_array {
        output.push_str("    uint32_t capacity;\n");
    }

    output.push_str(&format!("}} {};\n", generated_type.name));
}

/// Render one generated Apple optional wrapper type.
fn render_apple_generated_optional_type(
    output: &mut String,
    generated_type: &AppleGeneratedOptionalType,
) {
    push_c_doc_comment(output, &generated_type.documentation, 0);
    output.push_str(&format!("typedef struct {} {{\n", generated_type.name));
    output.push_str("    bool has_value;\n");
    output.push_str(&format!("    {} value;\n", generated_type.value_type));
    output.push_str(&format!("}} {};\n", generated_type.name));
}

/// Collect the generated Apple container types required by this module slice.
fn collect_apple_generated_container_types(
    generated_modules: &[HostAbiModule],
) -> Vec<AppleGeneratedContainerType> {
    let mut generated_types = Vec::new();

    for module in generated_modules {
        let mut visited_named_types = Vec::new();

        for function in module.requests.iter().chain(&module.ingress) {
            collect_apple_generated_container_types_from_type(
                module,
                &function.result,
                &mut generated_types,
                &mut visited_named_types,
            );

            for parameter in &function.parameters {
                collect_apple_generated_container_types_from_type(
                    module,
                    &parameter.ty,
                    &mut generated_types,
                    &mut visited_named_types,
                );
            }
        }
    }

    generated_types
}

/// Collect the generated Apple optional wrapper types required by this module slice.
fn collect_apple_generated_optional_types(
    generated_modules: &[HostAbiModule],
) -> Vec<AppleGeneratedOptionalType> {
    let mut generated_types = Vec::new();

    for module in generated_modules {
        let mut visited_named_types = Vec::new();

        for function in module.requests.iter().chain(&module.ingress) {
            collect_apple_generated_optional_types_from_type(
                module,
                &function.result,
                &mut generated_types,
                &mut visited_named_types,
            );

            for parameter in &function.parameters {
                collect_apple_generated_optional_types_from_type(
                    module,
                    &parameter.ty,
                    &mut generated_types,
                    &mut visited_named_types,
                );
            }
        }
    }

    generated_types
}

/// Collect the generated Apple container types reachable from one ABI type.
fn collect_apple_generated_container_types_from_type(
    module: &HostAbiModule,
    ty: &HostAbiType,
    generated_types: &mut Vec<AppleGeneratedContainerType>,
    visited_named_types: &mut Vec<String>,
) {
    match ty {
        HostAbiType::Named(name) => {
            if visited_named_types.iter().any(|existing| existing == name) {
                return;
            }

            visited_named_types.push((*name).to_string());

            if let HostAbiNamedTypeDefinition::Struct { fields } = &module
                .types
                .iter()
                .find(|named_type| named_type.name == *name)
                .unwrap_or_else(|| panic!("missing Apple named type {name} in {}", module.name))
                .definition
            {
                for field in fields {
                    collect_apple_generated_container_types_from_type(
                        module,
                        &field.ty,
                        generated_types,
                        visited_named_types,
                    );
                }
            }
        }
        HostAbiType::NativeArray(inner) => {
            if let HostAbiType::Named(name) = inner.as_ref() {
                let element_name = render_external_abi_name(name).to_string();
                let container_name = format!("DestackRust{element_name}Array");
                push_apple_generated_container_type(
                    generated_types,
                    AppleGeneratedContainerType {
                        name: container_name,
                        element_name,
                        documentation: format!(
                            "One owned {} array passed through the Apple bridge",
                            render_apple_container_subject(name)
                        ),
                        is_owned_array: true,
                    },
                );
            }

            collect_apple_generated_container_types_from_type(
                module,
                inner,
                generated_types,
                visited_named_types,
            );
        }
        HostAbiType::NativeSlice(inner) => {
            if let HostAbiType::Named(name) = inner.as_ref() {
                let element_name = render_external_abi_name(name).to_string();
                let container_name = format!("DestackRust{element_name}Slice");
                push_apple_generated_container_type(
                    generated_types,
                    AppleGeneratedContainerType {
                        name: container_name,
                        element_name,
                        documentation: format!(
                            "One {} slice passed through the Apple bridge",
                            render_apple_container_subject(name)
                        ),
                        is_owned_array: false,
                    },
                );
            }

            collect_apple_generated_container_types_from_type(
                module,
                inner,
                generated_types,
                visited_named_types,
            );
        }
        HostAbiType::OutputPointer(inner) => {
            collect_apple_generated_container_types_from_type(
                module,
                inner,
                generated_types,
                visited_named_types,
            );
        }
        HostAbiType::Optional(inner) => {
            collect_apple_generated_container_types_from_type(
                module,
                inner,
                generated_types,
                visited_named_types,
            );
        }
        _ => {}
    }
}

/// Collect the generated Apple optional wrapper types reachable from one ABI type.
fn collect_apple_generated_optional_types_from_type(
    module: &HostAbiModule,
    ty: &HostAbiType,
    generated_types: &mut Vec<AppleGeneratedOptionalType>,
    visited_named_types: &mut Vec<String>,
) {
    match ty {
        HostAbiType::Named(name) => {
            if visited_named_types.iter().any(|existing| existing == name) {
                return;
            }

            visited_named_types.push((*name).to_string());

            if let HostAbiNamedTypeDefinition::Struct { fields } = &module
                .types
                .iter()
                .find(|named_type| named_type.name == *name)
                .unwrap_or_else(|| panic!("missing Apple named type {name} in {}", module.name))
                .definition
            {
                for field in fields {
                    collect_apple_generated_optional_types_from_type(
                        module,
                        &field.ty,
                        generated_types,
                        visited_named_types,
                    );
                }
            }
        }
        HostAbiType::Optional(inner) => {
            collect_apple_generated_optional_types_from_type(
                module,
                inner,
                generated_types,
                visited_named_types,
            );

            push_apple_generated_optional_type(
                generated_types,
                AppleGeneratedOptionalType {
                    name: render_apple_optional_type_name(inner),
                    value_type: render_apple_public_type(inner),
                    documentation: format!(
                        "One optional {} passed through the Apple bridge",
                        render_apple_type_subject(inner)
                    ),
                },
            );
        }
        HostAbiType::NativeArray(inner)
        | HostAbiType::NativeSlice(inner)
        | HostAbiType::OutputPointer(inner) => {
            collect_apple_generated_optional_types_from_type(
                module,
                inner,
                generated_types,
                visited_named_types,
            );
        }
        _ => {}
    }
}

/// Push one generated Apple container type once.
fn push_apple_generated_container_type(
    generated_types: &mut Vec<AppleGeneratedContainerType>,
    generated_type: AppleGeneratedContainerType,
) {
    if generated_types
        .iter()
        .any(|existing| existing.name == generated_type.name)
    {
        return;
    }

    generated_types.push(generated_type);
}

/// Push one generated Apple optional wrapper type once.
fn push_apple_generated_optional_type(
    generated_types: &mut Vec<AppleGeneratedOptionalType>,
    generated_type: AppleGeneratedOptionalType,
) {
    if generated_types
        .iter()
        .any(|existing| existing.name == generated_type.name)
    {
        return;
    }

    generated_types.push(generated_type);
}

/// Render one Apple enum representation.
fn render_apple_enum_repr(repr: HostAbiEnumRepresentation) -> &'static str {
    match repr {
        HostAbiEnumRepresentation::I32 => "int32_t",
        HostAbiEnumRepresentation::U32 => "uint32_t",
    }
}

/// Render one public Apple ABI field type.
fn render_apple_public_type(ty: &HostAbiType) -> String {
    match ty {
        HostAbiType::U8 => "uint8_t".to_string(),
        HostAbiType::U16 => "uint16_t".to_string(),
        HostAbiType::I8 => "int8_t".to_string(),
        HostAbiType::I16 => "int16_t".to_string(),
        HostAbiType::U32 => "uint32_t".to_string(),
        HostAbiType::I32 => "int32_t".to_string(),
        HostAbiType::U64 => "uint64_t".to_string(),
        HostAbiType::HostRequestId => "uint64_t".to_string(),
        HostAbiType::Bool => "bool".to_string(),
        HostAbiType::F64 => "double".to_string(),
        HostAbiType::StringRef => "DestackRustStringRef".to_string(),
        HostAbiType::OsPath => "DestackRustStringRef".to_string(),
        HostAbiType::StringSlice => "DestackRustStringSlice".to_string(),
        HostAbiType::Named(name) => {
            format!("DestackRust{}", render_external_abi_name(name))
        }
        HostAbiType::NativeSlice(inner) => match inner.as_ref() {
            HostAbiType::Named(name) => {
                format!("DestackRust{}Slice", render_external_abi_name(name))
            }
            HostAbiType::StringRef | HostAbiType::OsPath => "DestackRustStringSlice".to_string(),
            HostAbiType::U8 => "DestackRustU8Slice".to_string(),
            HostAbiType::I8 => "DestackRustI8Slice".to_string(),
            HostAbiType::I16 => "DestackRustI16Slice".to_string(),
            _ => panic!("unsupported Apple public native-slice projection"),
        },
        HostAbiType::NativeArray(inner) => match inner.as_ref() {
            HostAbiType::Named(name) => {
                format!("DestackRust{}Array", render_external_abi_name(name))
            }
            _ => panic!("unsupported Apple public native-array projection"),
        },
        HostAbiType::OutputPointer(inner) => format!("{} *", render_apple_public_type(inner)),
        HostAbiType::HostSessionHandle => "uint64_t".to_string(),
        HostAbiType::HostStatus => "uint32_t".to_string(),
        HostAbiType::RuntimeStatus => "DestackRustRuntimeStatus".to_string(),
        HostAbiType::Optional(inner) => render_apple_optional_type_name(inner),
    }
}

/// Return the generated Apple optional wrapper name for one payload type.
fn render_apple_optional_type_name(ty: &HostAbiType) -> String {
    format!("DestackRustOptional{}", render_apple_type_segment(ty))
}

/// Return the generated Apple type segment for one payload type.
fn render_apple_type_segment(ty: &HostAbiType) -> String {
    match ty {
        HostAbiType::U8 => "U8".to_string(),
        HostAbiType::U16 => "U16".to_string(),
        HostAbiType::I8 => "I8".to_string(),
        HostAbiType::I16 => "I16".to_string(),
        HostAbiType::U32 => "U32".to_string(),
        HostAbiType::I32 => "I32".to_string(),
        HostAbiType::U64 => "U64".to_string(),
        HostAbiType::HostRequestId => "HostRequestId".to_string(),
        HostAbiType::Bool => "Bool".to_string(),
        HostAbiType::F64 => "F64".to_string(),
        HostAbiType::StringRef => "StringRef".to_string(),
        HostAbiType::OsPath => "OsPath".to_string(),
        HostAbiType::StringSlice => "StringSlice".to_string(),
        HostAbiType::Named(name) => render_external_abi_name(name).to_string(),
        HostAbiType::NativeSlice(inner) => {
            format!("{}Slice", render_apple_type_segment(inner))
        }
        HostAbiType::NativeArray(inner) => {
            format!("{}Array", render_apple_type_segment(inner))
        }
        HostAbiType::OutputPointer(inner) => {
            format!("Pointer{}", render_apple_type_segment(inner))
        }
        HostAbiType::HostSessionHandle => "HostSessionHandle".to_string(),
        HostAbiType::HostStatus => "HostStatus".to_string(),
        HostAbiType::RuntimeStatus => "RuntimeStatus".to_string(),
        HostAbiType::Optional(inner) => {
            format!("Optional{}", render_apple_type_segment(inner))
        }
    }
}

/// Return the lowered field name for one tagged Apple payload variant.
fn apple_tagged_variant_field_name(name: &str) -> String {
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

/// Return the generated Apple type subject phrase for one payload type.
fn render_apple_type_subject(ty: &HostAbiType) -> String {
    match ty {
        HostAbiType::StringRef => "string reference".to_string(),
        HostAbiType::StringSlice => "string slice".to_string(),
        HostAbiType::Named(name) => render_apple_container_subject(name),
        _ => "value".to_string(),
    }
}

/// Render one generated Apple container subject phrase.
fn render_apple_container_subject(name: &str) -> String {
    render_external_abi_name(name).chars().enumerate().fold(
        String::new(),
        |mut output, (index, character)| {
            if character.is_ascii_uppercase() && index != 0 {
                output.push(' ');
                output.push(character.to_ascii_lowercase());
            } else {
                output.push(character.to_ascii_lowercase());
            }

            output
        },
    )
}

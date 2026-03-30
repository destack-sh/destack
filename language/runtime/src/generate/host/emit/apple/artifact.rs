use std::collections::BTreeSet;

use super::docs::push_c_doc_comment;
use super::name::apple_pascal_case;
use super::swift::{render_swift_abi, render_swift_runtime_abi, render_swift_runtime_bridge};
use crate::host::model::{HostArtifact, HostCatalog, HostLayout, HostModule, HostPlatform};
use crate::platform::model::WorkspaceLayout;
use destack_runtime::host::abi::describe::{
    HostAbiEnumRepresentation, HostAbiField, HostAbiFunction, HostAbiModule, HostAbiNamedType,
    HostAbiNamedTypeDefinition, HostAbiParameter, HostAbiType,
};

/// Render the generated Apple bridge binding files.
pub(crate) fn render_binding_files(
    layout: &WorkspaceLayout,
    generated_catalog: &HostCatalog,
) -> Vec<HostArtifact> {
    let generated_modules: Vec<_> = generated_catalog
        .modules_for_platform(HostPlatform::Ios)
        .into_iter()
        .map(|module| module.abi().clone())
        .collect();
    vec![
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
            path: layout
                .language_root
                .join("runtime/apple/bridge/BridgeC/Bridge/loader.c"),
            contents: render_loader_source(generated_catalog),
        },
        HostArtifact {
            path: layout
                .language_root
                .join("runtime/apple/ios/Sources/RuntimeHostIOS/Bridge/RuntimeAbi.generated.swift"),
            contents: render_runtime_abi_generated(generated_catalog, &generated_modules),
        },
        HostArtifact {
            path: layout.language_root.join(
                "runtime/apple/ios/Sources/RuntimeHostIOS/Bridge/RuntimeBridge.generated.swift",
            ),
            contents: render_swift_runtime_bridge(&generated_modules),
        },
    ]
}

fn render_loader_source(generated_catalog: &HostCatalog) -> String {
    let mut output = String::new();
    output.push_str("#include \"Bridge/Types.h\"\n");
    output.push_str("#include \"Bridge/Loader.h\"\n\n");
    output.push_str("#include <dlfcn.h>\n");
    output.push_str("#include <stddef.h>\n");
    output.push_str("#include <stdlib.h>\n\n");
    render_apple_direct_runtime_binding_declarations(&mut output, generated_catalog);
    output.push_str(
        "#define RESOLVE_SYMBOL_ONCE(slot, type, handle, name) \\\n+    do { \\\n+        if ((slot) == NULL) { \\\n+            (slot) = (type)dlsym((handle), (name)); \\\n+        } \\\n+    } while (0)\n\n",
    );
    output.push_str("static void *symbol_handle = NULL;\n\n");
    output.push_str("static RuntimeBindings runtime_bindings = {\n");
    output.push_str("    .register_runtime_bridge_bindings = NULL,\n");
    output.push_str("    .unregister_runtime_bridge_bindings = NULL,\n");

    for spec in generated_catalog.runtime_bindings() {
        output.push_str(&format!("    .{} = NULL,\n", spec.field_name));
    }

    output.push_str("};\n\n");
    output.push_str("/// Resolve one directly linked runtime binding table.\n");
    output.push_str("static int resolve_linked_runtime_bindings(RuntimeBindings *out_bindings) {\n");
    output.push_str("    RuntimeBindings bindings = {\n");
    output.push_str(
        "        .register_runtime_bridge_bindings = destack_host_ios_register_runtime_bridge_bindings,\n",
    );
    output.push_str(
        "        .unregister_runtime_bridge_bindings = destack_host_ios_unregister_runtime_bridge_bindings,\n",
    );

    for spec in generated_catalog.runtime_bindings() {
        output.push_str(&format!(
            "        .{} = destack_host_ios_{},\n",
            spec.field_name, spec.field_name
        ));
    }

    output.push_str("    };\n\n");
    output.push_str("    if (\n");
    output.push_str("        bindings.register_runtime_bridge_bindings == NULL ||\n");
    output.push_str("        bindings.unregister_runtime_bridge_bindings == NULL");

    for spec in generated_catalog.runtime_bindings() {
        output.push_str(&format!(
            " ||\n        bindings.{} == NULL",
            spec.field_name
        ));
    }

    output.push_str("\n    ) {\n");
    output.push_str("        return 0;\n");
    output.push_str("    }\n\n");
    output.push_str("    *out_bindings = bindings;\n\n");
    output.push_str("    return 1;\n");
    output.push_str("}\n\n");
    output.push_str("/// Resolve the shared runtime symbol handle from one explicit library path.\n");
    output.push_str("static void *resolve_symbol_handle(void) {\n");
    output.push_str("    if (symbol_handle != NULL) {\n");
    output.push_str("        return symbol_handle;\n");
    output.push_str("    }\n\n");
    output.push_str("    const char *library_path = getenv(\"DESTACK_RUNTIME_HOST_BRIDGE_LIBRARY\");\n");
    output.push_str("    if (library_path != NULL && library_path[0] != '\\0') {\n");
    output.push_str("        symbol_handle = dlopen(library_path, RTLD_NOW | RTLD_GLOBAL);\n");
    output.push_str("        if (symbol_handle != NULL) {\n");
    output.push_str("            return symbol_handle;\n");
    output.push_str("        }\n");
    output.push_str("    }\n\n");
    output.push_str("    return NULL;\n");
    output.push_str("}\n\n");
    output.push_str("/// Resolve the runtime bridge bindings from the current process.\n");
    output.push_str("uint32_t resolve_runtime_bindings(RuntimeBindings *out_bindings) {\n");
    output.push_str("    if (resolve_linked_runtime_bindings(out_bindings)) {\n");
    output.push_str("        return 0;\n");
    output.push_str("    }\n\n");
    output.push_str("    void *handle = resolve_symbol_handle();\n");
    output.push_str("    if (handle == NULL) {\n");
    output.push_str("        return 3;\n");
    output.push_str("    }\n\n");
    output.push_str("    RESOLVE_SYMBOL_ONCE(runtime_bindings.register_runtime_bridge_bindings, RegisterRuntimeBridgeBindingsFunction, handle, \"destack_host_ios_register_runtime_bridge_bindings\");\n");
    output.push_str("    RESOLVE_SYMBOL_ONCE(runtime_bindings.unregister_runtime_bridge_bindings, UnregisterRuntimeBridgeBindingsFunction, handle, \"destack_host_ios_unregister_runtime_bridge_bindings\");\n");

    for spec in generated_catalog.runtime_bindings() {
        output.push_str(&format!(
            "    RESOLVE_SYMBOL_ONCE(runtime_bindings.{}, {}, handle, \"destack_host_ios_{}\");\n",
            spec.field_name, spec.type_name, spec.field_name
        ));
    }

    output.push_str("\n    if (\n");
    output.push_str("        runtime_bindings.register_runtime_bridge_bindings == NULL ||\n");
    output.push_str("        runtime_bindings.unregister_runtime_bridge_bindings == NULL");

    for spec in generated_catalog.runtime_bindings() {
        output.push_str(&format!(
            " ||\n        runtime_bindings.{} == NULL",
            spec.field_name
        ));
    }

    output.push_str("\n    ) {\n");
    output.push_str("        return 3;\n");
    output.push_str("    }\n\n");
    output.push_str("    *out_bindings = runtime_bindings;\n\n");
    output.push_str("    return 0;\n");
    output.push_str("}\n\n");
    output.push_str("/// Register one mobile bridge callback table for one runtime session.\n");
    output.push_str("uint32_t destack_runtime_host_ios_register_runtime_bridge_bindings(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    IosRuntimeBridgeBindings callbacks\n");
    output.push_str(") {\n");
    output.push_str("    RuntimeBindings bindings = {0};\n");
    output.push_str("    uint32_t status = resolve_runtime_bindings(&bindings);\n");
    output.push_str("    if (status != 0) {\n");
    output.push_str("        return status;\n");
    output.push_str("    }\n\n");
    output.push_str("    return bindings.register_runtime_bridge_bindings(session_handle, callbacks);\n");
    output.push_str("}\n\n");
    output.push_str("/// Unregister one mobile bridge callback table for one runtime session.\n");
    output.push_str("void destack_runtime_host_ios_unregister_runtime_bridge_bindings(\n");
    output.push_str("    uint64_t session_handle\n");
    output.push_str(") {\n");
    output.push_str("    RuntimeBindings bindings = {0};\n");
    output.push_str("    if (resolve_runtime_bindings(&bindings) != 0) {\n");
    output.push_str("        return;\n");
    output.push_str("    }\n\n");
    output.push_str("    bindings.unregister_runtime_bridge_bindings(session_handle);\n");
    output.push_str("}\n");
    output
}

/// Render the generated Apple bridge artifacts for one module.
pub(crate) fn render_module_files(
    layout: &WorkspaceLayout,
    module: &HostModule,
) -> Vec<HostArtifact> {
    let generated_layout = HostLayout::new(layout);
    let module_segment = apple_module_segment(module.abi());
    let mut files = vec![HostArtifact::new(
        generated_layout.apple_swift_abi(&module_segment),
        render_swift_abi(module),
    )];

    if module_has_generated_ingress(module.abi()) {
        files.insert(
            0,
            HostArtifact::new(
                generated_layout.apple_bridge_source(&module_segment),
                render_bridge_source(module.abi()),
            ),
        );
        files.insert(
            1,
            HostArtifact::new(
                generated_layout.apple_runtime_header(&module_segment),
                render_runtime_header(module.abi()),
            ),
        );
        files.insert(
            2,
            HostArtifact::new(
                generated_layout.apple_runtime_source(&module_segment),
                render_runtime_source(module.abi()),
            ),
        );
    }

    files
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

/// Render one generated Apple ingress bridge source.
fn render_bridge_source(module: &HostAbiModule) -> String {
    if module.name == "intent" {
        return render_intent_bridge_source();
    }

    let mut output = String::new();
    let ingress = single_ingress(module);
    let module_segment = apple_module_segment(module);

    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("#include \"Bridge/Types.h\"\n");
    output.push_str(&format!(
        "#include \"Bridge/{module_segment}/Runtime.generated.h\"\n\n"
    ));

    render_simple_bridge_helpers(&mut output, ingress);

    push_c_doc_comment(
        &mut output,
        &format!(
            "Deliver one {} into the runtime ingress path.",
            render_apple_generated_ingress_subject(module)
        ),
        0,
    );
    output.push_str(&format!(
        "{} destack_runtime_host_ios_{}(\n",
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

    output.push_str(") {\n");
    output.push_str("    return ");
    output.push_str(&c_wrapper_name(ingress));
    output.push('(');

    for (index, parameter) in ingress.parameters.iter().enumerate() {
        if index != 0 {
            output.push_str(", ");
        }

        output.push_str(&render_simple_bridge_argument(parameter));
    }

    output.push_str(");\n");
    output.push_str("}\n");
    output
}

fn render_simple_bridge_helpers(output: &mut String, ingress: &HostAbiFunction) {
    let uses_string_ref = ingress
        .parameters
        .iter()
        .any(|parameter| matches!(parameter.ty, HostAbiType::StringRef));
    let uses_string_slice = ingress
        .parameters
        .iter()
        .any(|parameter| matches!(parameter.ty, HostAbiType::StringSlice));

    if uses_string_ref {
        output.push_str(
            "/// Convert one public string reference into one internal string reference.\n",
        );
        output.push_str("static NativeStringRef native_string_ref(DestackRustStringRef value) {\n");
        output.push_str("    NativeStringRef native = {\n");
        output.push_str("        .data = value.data,\n");
        output.push_str("        .len = value.len,\n");
        output.push_str("    };\n\n");
        output.push_str("    return native;\n");
        output.push_str("}\n\n");
    }

    if uses_string_slice {
        output.push_str("/// Convert one public string slice into one internal string slice.\n");
        output.push_str(
            "static NativeStringSlice native_string_slice(DestackRustStringSlice values) {\n",
        );
        output.push_str("    NativeStringSlice native = {\n");
        output.push_str("        .data = (const NativeStringRef *)values.data,\n");
        output.push_str("        .len = values.len,\n");
        output.push_str("    };\n\n");
        output.push_str("    return native;\n");
        output.push_str("}\n\n");
    }
}

fn render_simple_bridge_argument(parameter: &HostAbiParameter) -> String {
    match parameter.ty {
        HostAbiType::StringRef => format!("native_string_ref({})", parameter.name),
        HostAbiType::StringSlice => format!("native_string_slice({})", parameter.name),
        _ => parameter.name.to_string(),
    }
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

    for module in generated_modules {
        for named_type in &module.types {
            if let HostAbiNamedTypeDefinition::Struct { .. } = &named_type.definition {
                render_apple_named_type(&mut output, named_type);
                output.push('\n');
            }
        }
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
    output.push_str("#include \"../RuntimeHostAppleBridge.h\"\n\n");
    output.push_str("#include <stdbool.h>\n");
    output.push_str("#include <stdint.h>\n\n");
    output.push_str("typedef struct NativeStringRef {\n");
    output.push_str("    const uint8_t *data;\n");
    output.push_str("    uint32_t len;\n");
    output.push_str("} NativeStringRef;\n\n");
    output.push_str("typedef struct NativeStringSlice {\n");
    output.push_str("    const NativeStringRef *data;\n");
    output.push_str("    uint32_t len;\n");
    output.push_str("} NativeStringSlice;\n\n");

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

    for typedef in APPLE_PUBLIC_OPTIONAL_TYPES {
        render_apple_public_struct_typedef(&mut output, typedef);
        output.push('\n');
    }

    output.push_str("#endif\n");
    output
}

/// One generated Apple public struct field.
struct ApplePublicField {
    /// The field documentation.
    documentation: &'static str,
    /// The field type name.
    ty: &'static str,
    /// The field name.
    name: &'static str,
}

/// One generated Apple public struct typedef.
struct ApplePublicStructTypedef {
    /// The typedef documentation.
    documentation: &'static str,
    /// The typedef name.
    name: &'static str,
    /// The ordered fields.
    fields: &'static [ApplePublicField],
}

const APPLE_PUBLIC_OPTIONAL_TYPES: &[ApplePublicStructTypedef] = &[
    ApplePublicStructTypedef {
        documentation: "One optional string reference passed through the Apple bridge.",
        name: "DestackRustOptionalStringRef",
        fields: &[
            ApplePublicField {
                documentation: "Whether the optional field is present.",
                ty: "bool",
                name: "has_value",
            },
            ApplePublicField {
                documentation: "The wrapped string reference.",
                ty: "DestackRustStringRef",
                name: "value",
            },
        ],
    },
    ApplePublicStructTypedef {
        documentation: "One optional `u32` passed through the Apple bridge.",
        name: "DestackRustOptionalU32",
        fields: &[
            ApplePublicField {
                documentation: "Whether the optional field is present.",
                ty: "bool",
                name: "has_value",
            },
            ApplePublicField {
                documentation: "The wrapped integer value.",
                ty: "uint32_t",
                name: "value",
            },
        ],
    },
    ApplePublicStructTypedef {
        documentation: "One optional `u64` passed through the Apple bridge.",
        name: "DestackRustOptionalU64",
        fields: &[
            ApplePublicField {
                documentation: "Whether the optional field is present.",
                ty: "bool",
                name: "has_value",
            },
            ApplePublicField {
                documentation: "The wrapped integer value.",
                ty: "uint64_t",
                name: "value",
            },
        ],
    },
    ApplePublicStructTypedef {
        documentation: "One optional `i8` passed through the Apple bridge.",
        name: "DestackRustOptionalI8",
        fields: &[
            ApplePublicField {
                documentation: "Whether the optional field is present.",
                ty: "bool",
                name: "has_value",
            },
            ApplePublicField {
                documentation: "The wrapped integer value.",
                ty: "int8_t",
                name: "value",
            },
        ],
    },
];

/// Render one generated Apple public struct typedef.
fn render_apple_public_struct_typedef(output: &mut String, typedef: &ApplePublicStructTypedef) {
    output.push_str(&format!("/// {}\n", typedef.documentation));
    output.push_str(&format!("typedef struct {} {{\n", typedef.name));

    for field in typedef.fields {
        output.push_str(&format!("    /// {}\n", field.documentation));
        output.push_str(&format!("    {} {};\n", field.ty, field.name));
    }

    output.push_str(&format!("}} {};\n", typedef.name));
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
    output.push_str("uint32_t destack_runtime_host_ios_register_runtime_bridge_bindings(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    IosRuntimeBridgeBindings callbacks\n");
    output.push_str(");\n\n");
    push_c_doc_comment(
        &mut output,
        "Unregister one mobile bridge callback table for one runtime session.",
        0,
    );
    output.push_str("void destack_runtime_host_ios_unregister_runtime_bridge_bindings(\n");
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
    _generated_modules: &[HostAbiModule],
) -> String {
    let mut output = String::new();
    output.push_str(&render_swift_runtime_abi(
        generated_catalog.runtime_ingresses(HostPlatform::Ios),
    ));
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

fn render_apple_direct_runtime_binding_declarations(output: &mut String, generated_catalog: &HostCatalog) {
    output.push_str("extern uint32_t destack_host_ios_register_runtime_bridge_bindings(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    IosRuntimeBridgeBindings callbacks\n");
    output.push_str(") __attribute__((weak_import));\n");
    output.push_str("extern void destack_host_ios_unregister_runtime_bridge_bindings(\n");
    output.push_str("    uint64_t session_handle\n");
    output.push_str(") __attribute__((weak_import));\n");

    for spec in generated_catalog.runtime_bindings() {
        output.push('\n');
        output.push_str(&format!(
            "extern {} destack_host_ios_{}(\n",
            spec.result_type.apple_public_name(),
            spec.field_name
        ));

        if spec.parameters.is_empty() {
            output.push_str("    void\n");
        } else {
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
        }

        output.push_str(") __attribute__((weak_import));\n");
    }

    output.push('\n');
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
    if module.name == "intent" {
        render_intent_ingress_declarations(output);
        return;
    }

    let ingress = single_ingress(module);
    let ingress_subject = render_apple_generated_ingress_subject(module);
    push_c_doc_comment(
        output,
        &format!("Deliver one {ingress_subject} into one runtime session."),
        0,
    );
    output.push_str(&format!(
        "{} destack_runtime_host_ios_{}(\n",
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
        "{} destack_runtime_host_ios_{}(\n",
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
    if module.name == "intent" {
        return "intent event".to_string();
    }

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

/// Render one generated Apple runtime header.
fn render_runtime_header(module: &HostAbiModule) -> String {
    if module.name == "intent" {
        return render_intent_runtime_header();
    }

    let ingress = single_ingress(module);
    let module_segment = apple_module_segment(module);
    let header_guard = apple_runtime_header_guard(module);

    let mut output = String::new();
    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str(&format!("#ifndef {header_guard}\n"));
    output.push_str(&format!("#define {header_guard}\n\n"));
    output.push_str("#include \"../Types.h\"\n\n");

    push_c_doc_comment(&mut output, ingress.documentation, 0);
    output.push_str(&format!(
        "{} {}(\n",
        render_c_type(&ingress.result),
        c_wrapper_name(ingress)
    ));

    for (index, parameter) in ingress.parameters.iter().enumerate() {
        let trailing = if index + 1 == ingress.parameters.len() {
            ""
        } else {
            ","
        };

        output.push_str(&format!(
            "    {} {}{}\n",
            render_c_type(&parameter.ty),
            parameter.name,
            trailing
        ));
    }

    output.push_str(");\n\n");
    output.push_str(&format!("#endif // {header_guard}\n"));

    let _ = module_segment;

    output
}

/// Render one generated Apple runtime source file.
fn render_runtime_source(module: &HostAbiModule) -> String {
    if module.name == "intent" {
        return render_intent_runtime_source();
    }

    let ingress = single_ingress(module);
    let module_segment = apple_module_segment(module);
    let mut output = String::new();
    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("#include \"Bridge/Types.h\"\n");
    output.push_str(&format!(
        "#include \"Bridge/{module_segment}/Runtime.generated.h\"\n"
    ));
    output.push_str("#include \"Bridge/Loader.h\"\n\n");
    output.push_str("/// Convert one host status code into one runtime status.\n");
    output.push_str("static DestackRustRuntimeStatus runtime_status_from_code(uint32_t code) {\n");
    output.push_str("    DestackRustRuntimeStatus status = {\n");
    output.push_str("        .code = code,\n");
    output.push_str("        .error_id = 0,\n");
    output.push_str("    };\n\n");
    output.push_str("    return status;\n");
    output.push_str("}\n\n");

    push_c_doc_comment(&mut output, ingress.documentation, 0);
    output.push_str(&format!(
        "{} {}(\n",
        render_c_type(&ingress.result),
        c_wrapper_name(ingress)
    ));

    for (index, parameter) in ingress.parameters.iter().enumerate() {
        let trailing = if index + 1 == ingress.parameters.len() {
            ""
        } else {
            ","
        };

        output.push_str(&format!(
            "    {} {}{}\n",
            render_c_type(&parameter.ty),
            parameter.name,
            trailing
        ));
    }

    output.push_str(") {\n");
    output.push_str("    RuntimeBindings bindings = {0};\n");
    output.push_str("    DestackRustRuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));\n");
    output.push_str("    if (status.code != 0) {\n");
    output.push_str("        return status;\n");
    output.push_str("    }\n\n");
    output.push_str(&format!("    return bindings.{}(", ingress.name));

    for (index, parameter) in ingress.parameters.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }

        output.push_str(parameter.name);
    }

    output.push_str(");\n");
    output.push_str("}\n");
    output
}

/// Render the generated Apple BridgeC intent ingress declarations.
fn render_intent_ingress_declarations(output: &mut String) {
    render_intent_ingress_declaration(
        output,
        "open_url",
        "intent open-url event",
        &[
            ("uint64_t", "session_handle"),
            ("bool", "has_source"),
            ("DestackRustStringRef", "source"),
            ("DestackRustStringRef", "url"),
        ],
    );
    output.push('\n');

    render_intent_ingress_declaration(
        output,
        "open_file",
        "intent open-file event",
        &[
            ("uint64_t", "session_handle"),
            ("bool", "has_source"),
            ("DestackRustStringRef", "source"),
            ("DestackRustStringRef", "path"),
            ("bool", "has_content_type"),
            ("DestackRustStringRef", "content_type"),
        ],
    );
    output.push('\n');

    render_intent_ingress_declaration(
        output,
        "share_text",
        "intent share-text event",
        &[
            ("uint64_t", "session_handle"),
            ("bool", "has_source"),
            ("DestackRustStringRef", "source"),
            ("DestackRustStringRef", "text"),
            ("bool", "has_content_type"),
            ("DestackRustStringRef", "content_type"),
        ],
    );
    output.push('\n');

    render_intent_ingress_declaration(
        output,
        "share_files",
        "intent share-files event",
        &[
            ("uint64_t", "session_handle"),
            ("bool", "has_source"),
            ("DestackRustStringRef", "source"),
            ("DestackRustStringSlice", "paths"),
            ("bool", "has_content_type"),
            ("DestackRustStringRef", "content_type"),
        ],
    );
    output.push('\n');

    render_intent_ingress_declaration(
        output,
        "custom_action",
        "intent custom-action event",
        &[
            ("uint64_t", "session_handle"),
            ("bool", "has_source"),
            ("DestackRustStringRef", "source"),
            ("DestackRustStringRef", "action"),
            ("bool", "has_url"),
            ("DestackRustStringRef", "url"),
            ("DestackRustStringSlice", "paths"),
            ("bool", "has_text"),
            ("DestackRustStringRef", "text"),
            ("bool", "has_content_type"),
            ("DestackRustStringRef", "content_type"),
        ],
    );
}

/// Render one generated Apple BridgeC intent ingress declaration.
fn render_intent_ingress_declaration(
    output: &mut String,
    lane_name: &str,
    subject: &str,
    parameters: &[(&str, &str)],
) {
    push_c_doc_comment(
        output,
        &format!("Deliver one {subject} into one runtime session."),
        0,
    );
    output.push_str(&format!(
        "DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_{lane_name}(\n"
    ));

    for (index, (ty, name)) in parameters.iter().enumerate() {
        let trailing = if index + 1 == parameters.len() {
            ""
        } else {
            ","
        };
        output.push_str(&format!("    {ty} {name}{trailing}\n"));
    }

    output.push_str(");\n");
}

/// Render one generated Apple BridgeC intent ingress bridge source.
fn render_intent_bridge_source() -> String {
    let mut output = String::new();
    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("#include \"Bridge/Types.h\"\n");
    output.push_str("#include \"Bridge/Intent/Runtime.generated.h\"\n\n");
    output
        .push_str("/// Convert one public string reference into one internal string reference.\n");
    output.push_str("static NativeStringRef native_string_ref(DestackRustStringRef value) {\n");
    output.push_str("    NativeStringRef native = {\n");
    output.push_str("        .data = value.data,\n");
    output.push_str("        .len = value.len,\n");
    output.push_str("    };\n\n");
    output.push_str("    return native;\n");
    output.push_str("}\n\n");
    output.push_str("/// Convert one public string slice into one internal string slice.\n");
    output.push_str(
        "static NativeStringSlice native_string_slice(DestackRustStringSlice values) {\n",
    );
    output.push_str("    NativeStringSlice native = {\n");
    output.push_str("        .data = (const NativeStringRef *)values.data,\n");
    output.push_str("        .len = values.len,\n");
    output.push_str("    };\n\n");
    output.push_str("    return native;\n");
    output.push_str("}\n\n");

    output.push_str("/// Deliver one intent open-url event into the runtime ingress path.\n");
    output.push_str("DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_open_url(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    bool has_source,\n");
    output.push_str("    DestackRustStringRef source,\n");
    output.push_str("    DestackRustStringRef url\n");
    output.push_str(") {\n");
    output.push_str("    return send_intent_open_url(\n");
    output.push_str("        session_handle,\n");
    output.push_str("        has_source,\n");
    output.push_str("        native_string_ref(source),\n");
    output.push_str("        native_string_ref(url)\n");
    output.push_str("    );\n");
    output.push_str("}\n\n");

    output.push_str("/// Deliver one intent open-file event into the runtime ingress path.\n");
    output.push_str("DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_open_file(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    bool has_source,\n");
    output.push_str("    DestackRustStringRef source,\n");
    output.push_str("    DestackRustStringRef path,\n");
    output.push_str("    bool has_content_type,\n");
    output.push_str("    DestackRustStringRef content_type\n");
    output.push_str(") {\n");
    output.push_str("    return send_intent_open_file(\n");
    output.push_str("        session_handle,\n");
    output.push_str("        has_source,\n");
    output.push_str("        native_string_ref(source),\n");
    output.push_str("        native_string_ref(path),\n");
    output.push_str("        has_content_type,\n");
    output.push_str("        native_string_ref(content_type)\n");
    output.push_str("    );\n");
    output.push_str("}\n\n");

    output.push_str("/// Deliver one intent share-text event into the runtime ingress path.\n");
    output
        .push_str("DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_share_text(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    bool has_source,\n");
    output.push_str("    DestackRustStringRef source,\n");
    output.push_str("    DestackRustStringRef text,\n");
    output.push_str("    bool has_content_type,\n");
    output.push_str("    DestackRustStringRef content_type\n");
    output.push_str(") {\n");
    output.push_str("    return send_intent_share_text(\n");
    output.push_str("        session_handle,\n");
    output.push_str("        has_source,\n");
    output.push_str("        native_string_ref(source),\n");
    output.push_str("        native_string_ref(text),\n");
    output.push_str("        has_content_type,\n");
    output.push_str("        native_string_ref(content_type)\n");
    output.push_str("    );\n");
    output.push_str("}\n\n");

    output.push_str("/// Deliver one intent share-files event into the runtime ingress path.\n");
    output
        .push_str("DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_share_files(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    bool has_source,\n");
    output.push_str("    DestackRustStringRef source,\n");
    output.push_str("    DestackRustStringSlice paths,\n");
    output.push_str("    bool has_content_type,\n");
    output.push_str("    DestackRustStringRef content_type\n");
    output.push_str(") {\n");
    output.push_str("    return send_intent_share_files(\n");
    output.push_str("        session_handle,\n");
    output.push_str("        has_source,\n");
    output.push_str("        native_string_ref(source),\n");
    output.push_str("        native_string_slice(paths),\n");
    output.push_str("        has_content_type,\n");
    output.push_str("        native_string_ref(content_type)\n");
    output.push_str("    );\n");
    output.push_str("}\n\n");

    output.push_str("/// Deliver one intent custom-action event into the runtime ingress path.\n");
    output.push_str(
        "DestackRustRuntimeStatus destack_runtime_host_ios_notify_intent_custom_action(\n",
    );
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    bool has_source,\n");
    output.push_str("    DestackRustStringRef source,\n");
    output.push_str("    DestackRustStringRef action,\n");
    output.push_str("    bool has_url,\n");
    output.push_str("    DestackRustStringRef url,\n");
    output.push_str("    DestackRustStringSlice paths,\n");
    output.push_str("    bool has_text,\n");
    output.push_str("    DestackRustStringRef text,\n");
    output.push_str("    bool has_content_type,\n");
    output.push_str("    DestackRustStringRef content_type\n");
    output.push_str(") {\n");
    output.push_str("    return send_intent_custom_action(\n");
    output.push_str("        session_handle,\n");
    output.push_str("        has_source,\n");
    output.push_str("        native_string_ref(source),\n");
    output.push_str("        native_string_ref(action),\n");
    output.push_str("        has_url,\n");
    output.push_str("        native_string_ref(url),\n");
    output.push_str("        native_string_slice(paths),\n");
    output.push_str("        has_text,\n");
    output.push_str("        native_string_ref(text),\n");
    output.push_str("        has_content_type,\n");
    output.push_str("        native_string_ref(content_type)\n");
    output.push_str("    );\n");
    output.push_str("}\n");
    output
}

/// Render one generated Apple BridgeC intent runtime header.
fn render_intent_runtime_header() -> String {
    let mut output = String::new();
    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("#ifndef RUNTIME_HOST_APPLE_BRIDGE_INTENT_RUNTIME_H\n");
    output.push_str("#define RUNTIME_HOST_APPLE_BRIDGE_INTENT_RUNTIME_H\n\n");
    output.push_str("#include \"../Types.h\"\n\n");

    output.push_str("/// Send one intent open-url event into the runtime ingress path.\n");
    output.push_str("DestackRustRuntimeStatus send_intent_open_url(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    bool has_source,\n");
    output.push_str("    NativeStringRef source,\n");
    output.push_str("    NativeStringRef url\n");
    output.push_str(");\n");
    output.push_str("/// Send one intent open-file event into the runtime ingress path.\n");
    output.push_str("DestackRustRuntimeStatus send_intent_open_file(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    bool has_source,\n");
    output.push_str("    NativeStringRef source,\n");
    output.push_str("    NativeStringRef path,\n");
    output.push_str("    bool has_mime_type,\n");
    output.push_str("    NativeStringRef mime_type\n");
    output.push_str(");\n");
    output.push_str("/// Send one intent share-text event into the runtime ingress path.\n");
    output.push_str("DestackRustRuntimeStatus send_intent_share_text(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    bool has_source,\n");
    output.push_str("    NativeStringRef source,\n");
    output.push_str("    NativeStringRef text,\n");
    output.push_str("    bool has_mime_type,\n");
    output.push_str("    NativeStringRef mime_type\n");
    output.push_str(");\n");
    output.push_str("/// Send one intent share-files event into the runtime ingress path.\n");
    output.push_str("DestackRustRuntimeStatus send_intent_share_files(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    bool has_source,\n");
    output.push_str("    NativeStringRef source,\n");
    output.push_str("    NativeStringSlice paths,\n");
    output.push_str("    bool has_mime_type,\n");
    output.push_str("    NativeStringRef mime_type\n");
    output.push_str(");\n");
    output.push_str("/// Send one intent custom-action event into the runtime ingress path.\n");
    output.push_str("DestackRustRuntimeStatus send_intent_custom_action(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    bool has_source,\n");
    output.push_str("    NativeStringRef source,\n");
    output.push_str("    NativeStringRef action,\n");
    output.push_str("    bool has_url,\n");
    output.push_str("    NativeStringRef url,\n");
    output.push_str("    NativeStringSlice paths,\n");
    output.push_str("    bool has_text,\n");
    output.push_str("    NativeStringRef text,\n");
    output.push_str("    bool has_mime_type,\n");
    output.push_str("    NativeStringRef mime_type\n");
    output.push_str(");\n\n");
    output.push_str("#endif // RUNTIME_HOST_APPLE_BRIDGE_INTENT_RUNTIME_H\n");
    output
}

/// Render one generated Apple BridgeC intent runtime source.
fn render_intent_runtime_source() -> String {
    let mut output = String::new();
    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("#include \"Bridge/Types.h\"\n");
    output.push_str("#include \"Bridge/Intent/Runtime.generated.h\"\n");
    output.push_str("#include \"Bridge/Loader.h\"\n\n");
    output.push_str("/// Convert one host status code into one runtime status.\n");
    output.push_str("static DestackRustRuntimeStatus runtime_status_from_code(uint32_t code) {\n");
    output.push_str("    DestackRustRuntimeStatus status = {\n");
    output.push_str("        .code = code,\n");
    output.push_str("        .error_id = 0,\n");
    output.push_str("    };\n\n");
    output.push_str("    return status;\n");
    output.push_str("}\n\n");

    output.push_str("/// Send one intent open-url event into the runtime ingress path.\n");
    output.push_str("DestackRustRuntimeStatus send_intent_open_url(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    bool has_source,\n");
    output.push_str("    NativeStringRef source,\n");
    output.push_str("    NativeStringRef url\n");
    output.push_str(") {\n");
    output.push_str("    RuntimeBindings bindings = {0};\n");
    output.push_str("    DestackRustRuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));\n");
    output.push_str("    if (status.code != 0) {\n");
    output.push_str("        return status;\n");
    output.push_str("    }\n\n");
    output.push_str(
        "    return bindings.notify_intent_open_url(session_handle, has_source, source, url);\n",
    );
    output.push_str("}\n\n");

    output.push_str("/// Send one intent open-file event into the runtime ingress path.\n");
    output.push_str("DestackRustRuntimeStatus send_intent_open_file(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    bool has_source,\n");
    output.push_str("    NativeStringRef source,\n");
    output.push_str("    NativeStringRef path,\n");
    output.push_str("    bool has_mime_type,\n");
    output.push_str("    NativeStringRef mime_type\n");
    output.push_str(") {\n");
    output.push_str("    RuntimeBindings bindings = {0};\n");
    output.push_str("    DestackRustRuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));\n");
    output.push_str("    if (status.code != 0) {\n");
    output.push_str("        return status;\n");
    output.push_str("    }\n\n");
    output.push_str("    return bindings.notify_intent_open_file(\n");
    output.push_str("        session_handle,\n");
    output.push_str("        has_source,\n");
    output.push_str("        source,\n");
    output.push_str("        path,\n");
    output.push_str("        has_mime_type,\n");
    output.push_str("        mime_type\n");
    output.push_str("    );\n");
    output.push_str("}\n\n");

    output.push_str("/// Send one intent share-text event into the runtime ingress path.\n");
    output.push_str("DestackRustRuntimeStatus send_intent_share_text(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    bool has_source,\n");
    output.push_str("    NativeStringRef source,\n");
    output.push_str("    NativeStringRef text,\n");
    output.push_str("    bool has_mime_type,\n");
    output.push_str("    NativeStringRef mime_type\n");
    output.push_str(") {\n");
    output.push_str("    RuntimeBindings bindings = {0};\n");
    output.push_str("    DestackRustRuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));\n");
    output.push_str("    if (status.code != 0) {\n");
    output.push_str("        return status;\n");
    output.push_str("    }\n\n");
    output.push_str("    return bindings.notify_intent_share_text(\n");
    output.push_str("        session_handle,\n");
    output.push_str("        has_source,\n");
    output.push_str("        source,\n");
    output.push_str("        text,\n");
    output.push_str("        has_mime_type,\n");
    output.push_str("        mime_type\n");
    output.push_str("    );\n");
    output.push_str("}\n\n");

    output.push_str("/// Send one intent share-files event into the runtime ingress path.\n");
    output.push_str("DestackRustRuntimeStatus send_intent_share_files(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    bool has_source,\n");
    output.push_str("    NativeStringRef source,\n");
    output.push_str("    NativeStringSlice paths,\n");
    output.push_str("    bool has_mime_type,\n");
    output.push_str("    NativeStringRef mime_type\n");
    output.push_str(") {\n");
    output.push_str("    RuntimeBindings bindings = {0};\n");
    output.push_str("    DestackRustRuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));\n");
    output.push_str("    if (status.code != 0) {\n");
    output.push_str("        return status;\n");
    output.push_str("    }\n\n");
    output.push_str("    return bindings.notify_intent_share_files(\n");
    output.push_str("        session_handle,\n");
    output.push_str("        has_source,\n");
    output.push_str("        source,\n");
    output.push_str("        paths,\n");
    output.push_str("        has_mime_type,\n");
    output.push_str("        mime_type\n");
    output.push_str("    );\n");
    output.push_str("}\n\n");

    output.push_str("/// Send one intent custom-action event into the runtime ingress path.\n");
    output.push_str("DestackRustRuntimeStatus send_intent_custom_action(\n");
    output.push_str("    uint64_t session_handle,\n");
    output.push_str("    bool has_source,\n");
    output.push_str("    NativeStringRef source,\n");
    output.push_str("    NativeStringRef action,\n");
    output.push_str("    bool has_url,\n");
    output.push_str("    NativeStringRef url,\n");
    output.push_str("    NativeStringSlice paths,\n");
    output.push_str("    bool has_text,\n");
    output.push_str("    NativeStringRef text,\n");
    output.push_str("    bool has_mime_type,\n");
    output.push_str("    NativeStringRef mime_type\n");
    output.push_str(") {\n");
    output.push_str("    RuntimeBindings bindings = {0};\n");
    output.push_str("    DestackRustRuntimeStatus status = runtime_status_from_code(resolve_runtime_bindings(&bindings));\n");
    output.push_str("    if (status.code != 0) {\n");
    output.push_str("        return status;\n");
    output.push_str("    }\n\n");
    output.push_str("    return bindings.notify_intent_custom_action(\n");
    output.push_str("        session_handle,\n");
    output.push_str("        has_source,\n");
    output.push_str("        source,\n");
    output.push_str("        action,\n");
    output.push_str("        has_url,\n");
    output.push_str("        url,\n");
    output.push_str("        paths,\n");
    output.push_str("        has_text,\n");
    output.push_str("        text,\n");
    output.push_str("        has_mime_type,\n");
    output.push_str("        mime_type\n");
    output.push_str("    );\n");
    output.push_str("}\n");
    output
}

/// Render one generated Apple Swift ABI file.
/// Return the single ingress function for one module.
fn single_ingress(module: &HostAbiModule) -> &HostAbiFunction {
    module
        .ingress
        .first()
        .unwrap_or_else(|| panic!("missing ingress function for {}", module.name))
}

/// Return whether one module emits generated Apple ingress shims.
fn module_has_generated_ingress(module: &HostAbiModule) -> bool {
    !module.ingress.is_empty() || matches!(module.name, "intent")
}

/// Return the Apple module path segment for one module.
fn apple_module_segment(module: &HostAbiModule) -> String {
    apple_pascal_case(module.name)
}

/// Return the C header guard for one Apple runtime wrapper.
fn apple_runtime_header_guard(module: &HostAbiModule) -> String {
    format!(
        "RUNTIME_HOST_APPLE_BRIDGE_{}_RUNTIME_H",
        module.name.to_ascii_uppercase()
    )
}

/// Render one C ABI type.
fn render_c_type(ty: &HostAbiType) -> String {
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
        HostAbiType::StringRef => "NativeStringRef".to_string(),
        HostAbiType::StringSlice => "NativeStringSlice".to_string(),
        HostAbiType::HostSessionHandle => "uint64_t".to_string(),
        HostAbiType::HostStatus => "uint32_t".to_string(),
        HostAbiType::RuntimeStatus => "DestackRustRuntimeStatus".to_string(),
        HostAbiType::NativeArray(element) => {
            format!("{}Array", render_c_type(element))
        }
        HostAbiType::NativeSlice(element) => {
            format!("{}Slice", render_c_type(element))
        }
        HostAbiType::OutputPointer(inner) => format!("{}*", render_c_type(inner)),
        HostAbiType::Named(name) => format!("DestackRust{}", render_external_abi_name(name)),
    }
}

/// Return the exported ABI type name for one authored Rust host ABI type.
fn render_external_abi_name(name: &str) -> &str {
    name.strip_prefix("Host").unwrap_or(name)
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
    }
}

/// Render one generated Apple named-type forward declaration.
fn render_apple_named_type_forward_declaration(output: &mut String, named_type: &HostAbiNamedType) {
    if let HostAbiNamedTypeDefinition::Struct { .. } = &named_type.definition {
        let external_name = render_external_abi_name(named_type.name);
        output.push_str(&format!(
            "typedef struct DestackRust{external_name} DestackRust{external_name};\n"
        ));
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
        HostAbiType::StringSlice => "DestackRustStringSlice".to_string(),
        HostAbiType::Named(name) => {
            format!("DestackRust{}", render_external_abi_name(name))
        }
        HostAbiType::NativeSlice(inner) => match inner.as_ref() {
            HostAbiType::Named(name) => {
                format!("DestackRust{}Slice", render_external_abi_name(name))
            }
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

/// Return one Apple runtime wrapper name for one ingress function.
fn c_wrapper_name(function: &HostAbiFunction) -> String {
    let stem = function
        .name
        .strip_prefix("notify_")
        .unwrap_or(function.name);
    format!("send_{stem}")
}

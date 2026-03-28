use super::abi::{
    android_module_uses_type, android_named_struct_fields, android_named_type,
    android_named_type_is_enum,
};
use super::binding::collect_android_generated_slice_types_from_type;
use super::cpp::{
    android_cpp_methods_request_name, android_cpp_send_name, render_cpp_parameter_declaration,
    render_cpp_type,
};
use super::docs::push_cpp_doc_comment;
use super::kotlin::{
    android_kotlin_method_name, android_kotlin_native_name, android_kotlin_parameter_name,
    android_kotlin_process_name,
};
use super::name::android_pascal_case;
use destack_runtime::host::abi::describe::{
    HostAbiField, HostAbiFunction, HostAbiModule, HostAbiNamedTypeDefinition, HostAbiType,
};

/// One flattened Android C++ bridge leaf.
#[derive(Clone, Debug)]
struct AndroidCppLeaf {
    /// The logical leaf name.
    name: String,
    /// The leaf expression in one native request value.
    expr: String,
    /// The leaf ABI type.
    ty: HostAbiType,
}

/// Collect the flattened input leaves for one Android request parameter.
fn collect_android_cpp_input_leaves(
    module: &HostAbiModule,
    ty: &HostAbiType,
    expr: &str,
    leaf_name: &str,
    leaves: &mut Vec<AndroidCppLeaf>,
) {
    match ty {
        HostAbiType::Named(name) if !android_named_type_is_enum(module, name) => {
            for field in android_named_struct_fields(module, name) {
                collect_android_cpp_input_leaves(
                    module,
                    &field.ty,
                    &format!("{expr}.{}", field.name),
                    field.name,
                    leaves,
                );
            }
        }
        HostAbiType::OutputPointer(_) => {}
        _ => leaves.push(AndroidCppLeaf {
            name: leaf_name.to_string(),
            expr: expr.to_string(),
            ty: ty.clone(),
        }),
    }
}

/// Collect the flattened input leaves for one Android request.
fn collect_android_cpp_request_input_leaves(
    module: &HostAbiModule,
    request: &HostAbiFunction,
) -> Vec<AndroidCppLeaf> {
    let mut leaves = Vec::new();

    for parameter in &request.parameters {
        if matches!(
            parameter.ty,
            HostAbiType::HostSessionHandle | HostAbiType::OutputPointer(_)
        ) {
            continue;
        }

        collect_android_cpp_input_leaves(
            module,
            &parameter.ty,
            parameter.name,
            parameter.name,
            &mut leaves,
        );
    }

    leaves
}

/// Return the output parameters for one Android request.
fn android_cpp_output_parameters(request: &HostAbiFunction) -> Vec<AndroidCppLeaf> {
    request
        .parameters
        .iter()
        .filter_map(|parameter| match &parameter.ty {
            HostAbiType::OutputPointer(inner) => Some(AndroidCppLeaf {
                name: parameter.name.to_string(),
                expr: parameter.name.to_string(),
                ty: inner.as_ref().clone(),
            }),
            _ => None,
        })
        .collect()
}

/// Return whether one Android type tree uses borrowed JNI strings.
pub(super) fn android_cpp_type_uses_jni_strings(module: &HostAbiModule, ty: &HostAbiType) -> bool {
    match ty {
        HostAbiType::StringRef | HostAbiType::StringSlice => true,
        HostAbiType::Named(name) if !android_named_type_is_enum(module, name) => {
            android_named_struct_fields(module, name)
                .iter()
                .any(|field| android_cpp_type_uses_jni_strings(module, &field.ty))
        }
        HostAbiType::NativeArray(inner)
        | HostAbiType::NativeSlice(inner)
        | HostAbiType::OutputPointer(inner) => android_cpp_type_uses_jni_strings(module, inner),
        _ => false,
    }
}

/// Return the JNI signature for one flattened Android bridge leaf.
fn android_cpp_leaf_jni_signature(module: &HostAbiModule, ty: &HostAbiType) -> String {
    match ty {
        HostAbiType::U8 | HostAbiType::I8 | HostAbiType::I16 => "I".to_string(),
        HostAbiType::U32 | HostAbiType::I32 => "I".to_string(),
        HostAbiType::U64 | HostAbiType::HostRequestId => "J".to_string(),
        HostAbiType::Bool => "Z".to_string(),
        HostAbiType::F64 => "D".to_string(),
        HostAbiType::StringRef => "Ljava/lang/String;".to_string(),
        HostAbiType::StringSlice => "[Ljava/lang/String;".to_string(),
        HostAbiType::Named(name) if android_named_type_is_enum(module, name) => "I".to_string(),
        HostAbiType::NativeSlice(inner) => {
            if let HostAbiType::Named(name) = inner.as_ref() {
                format!("[L{};", android_runtime_named_binary_name(module, name))
            } else if android_cpp_primitive_slice_type(inner).is_some() {
                "[I".to_string()
            } else {
                panic!(
                    "unsupported Android JNI request leaf signature for {:?}",
                    ty
                );
            }
        }
        other => panic!(
            "unsupported Android JNI request leaf signature for {:?}",
            other
        ),
    }
}

/// Return the JNI return signature for one Android request.
fn android_cpp_request_return_signature(
    module: &HostAbiModule,
    request: &HostAbiFunction,
) -> String {
    let output_parameters = android_cpp_output_parameters(request);

    if output_parameters.is_empty() {
        return "I".to_string();
    }

    format!(
        "L{};",
        android_runtime_response_binary_name(module, request)
    )
}

/// Return one runtime response binary name for one request response object.
fn android_runtime_response_binary_name(
    module: &HostAbiModule,
    request: &HostAbiFunction,
) -> String {
    format!(
        "dev/destack/runtime/android/module/{}/RuntimeHost{}{}Response",
        module.name,
        android_pascal_case(module.name),
        android_pascal_case(request.name),
    )
}

/// Return one runtime named-type binary name for one authored ABI type.
fn android_runtime_named_binary_name(module: &HostAbiModule, name: &str) -> String {
    format!(
        "dev/destack/runtime/android/module/{}/Runtime{}",
        module.name, name
    )
}

/// Return one JVM getter name for one field.
fn android_cpp_getter_name(field_name: &str, is_bool: bool) -> String {
    if is_bool && field_name.starts_with("is_") {
        return android_kotlin_parameter_name(field_name);
    }

    format!("get{}", android_pascal_case(field_name))
}

/// Return the method-id slot name for one request.
fn android_cpp_method_id_name(request: &HostAbiFunction) -> String {
    format!("{}_method", android_kotlin_method_name(request.name))
}

/// Return the RuntimeBridge method name for one generated request.
fn android_kotlin_bridge_method_name(module: &HostAbiModule, request: &HostAbiFunction) -> String {
    format!(
        "{}{}",
        android_kotlin_parameter_name(module.name),
        android_pascal_case(request.name)
    )
}

/// Return the decode helper function name for one named ABI type.
fn android_cpp_decode_function_name(name: &str) -> String {
    format!("decode_{name}")
}

/// Return the encode helper function name for one named ABI type.
fn android_cpp_encode_function_name(name: &str) -> String {
    format!("encode_{name}")
}

/// Return the decode helper function name for one named slice ABI type.
fn android_cpp_decode_slice_function_name(name: &str) -> String {
    android_cpp_decode_function_name(&format!("{name}Slice"))
}

/// Return the encode helper function name for one named slice ABI type.
fn android_cpp_encode_slice_function_name(name: &str) -> String {
    format!("encode_{name}Slice")
}

/// Return the Android C++ slice type name for one primitive slice element.
pub(crate) fn android_cpp_primitive_slice_type(inner: &HostAbiType) -> Option<&'static str> {
    match inner {
        HostAbiType::U8 => Some("NativeU8Slice"),
        HostAbiType::I8 => Some("NativeI8Slice"),
        HostAbiType::I16 => Some("NativeI16Slice"),
        _ => None,
    }
}

/// Return the Android C++ primitive-slice decode helper name for one element type.
fn android_cpp_primitive_slice_decode_function_name(inner: &HostAbiType) -> Option<&'static str> {
    match inner {
        HostAbiType::U8 => Some("decode_u8_list"),
        HostAbiType::I8 => Some("decode_i8_list"),
        HostAbiType::I16 => Some("decode_i16_list"),
        _ => None,
    }
}

/// Return the Android C++ primitive-slice encode helper name for one element type.
fn android_cpp_primitive_slice_encode_function_name(inner: &HostAbiType) -> Option<&'static str> {
    match inner {
        HostAbiType::U8 => Some("encode_u8_slice"),
        HostAbiType::I8 => Some("encode_i8_slice"),
        HostAbiType::I16 => Some("encode_i16_slice"),
        _ => None,
    }
}

/// Return one output storage slot name.
fn android_cpp_output_storage_name(name: &str) -> String {
    format!("{name}_storage")
}

/// Return the getter signature for one object-returning field.
fn android_cpp_object_signature(module: &HostAbiModule, ty: &HostAbiType) -> String {
    match ty {
        HostAbiType::U8 | HostAbiType::I8 | HostAbiType::I16 => "()Ljava/lang/Integer;".to_string(),
        HostAbiType::U32 | HostAbiType::I32 | HostAbiType::HostStatus => {
            "()Ljava/lang/Integer;".to_string()
        }
        HostAbiType::StringRef => "()Ljava/lang/String;".to_string(),
        HostAbiType::U64 | HostAbiType::HostRequestId => "()Ljava/lang/Long;".to_string(),
        HostAbiType::Bool => "()Ljava/lang/Boolean;".to_string(),
        HostAbiType::F64 => "()Ljava/lang/Double;".to_string(),
        HostAbiType::Named(name) => {
            format!("()L{};", android_runtime_named_binary_name(module, name))
        }
        HostAbiType::NativeArray(_) | HostAbiType::NativeSlice(_) => {
            "()Ljava/util/List;".to_string()
        }
        other => panic!(
            "unsupported Android object getter signature for {:?}",
            other
        ),
    }
}

/// Return the default C++ value for one ABI type.
fn android_cpp_default_value(module: &HostAbiModule, ty: &HostAbiType) -> String {
    match ty {
        HostAbiType::U8 | HostAbiType::I8 | HostAbiType::I16 => "0".to_string(),
        HostAbiType::U32 | HostAbiType::I32 | HostAbiType::U64 | HostAbiType::HostRequestId => {
            "0".to_string()
        }
        HostAbiType::Bool => "false".to_string(),
        HostAbiType::F64 => "0.0".to_string(),
        HostAbiType::StringRef => "NativeStringRef { .data = nullptr, .len = 0 }".to_string(),
        HostAbiType::Named(name) if android_named_type_is_enum(module, name) => {
            format!("static_cast<{name}>(0)")
        }
        HostAbiType::Named(name) => format!("{name} {{}}"),
        HostAbiType::NativeArray(inner) => format!(
            "NativeArray<{}> {{ .data = nullptr, .len = 0, .capacity = 0 }}",
            render_cpp_type(inner)
        ),
        HostAbiType::NativeSlice(inner) => format!(
            "{} {{ .data = nullptr, .len = 0 }}",
            render_cpp_type(&HostAbiType::NativeSlice(inner.clone()))
        ),
        other => panic!("unsupported Android default value for {:?}", other),
    }
}

/// Return the runtime getter call expression for one primitive ABI type.
fn android_cpp_primitive_getter_call(
    module: &HostAbiModule,
    ty: &HostAbiType,
    object_name: &str,
    getter_name: &str,
) -> String {
    match ty {
        HostAbiType::U8 | HostAbiType::I8 | HostAbiType::I16 => format!(
            "static_cast<{}>(call_int_getter(env, {object_name}, \"{getter_name}\"))",
            render_cpp_type(ty)
        ),
        HostAbiType::U32 | HostAbiType::I32 | HostAbiType::HostStatus => format!(
            "static_cast<{}>(call_int_getter(env, {object_name}, \"{getter_name}\"))",
            render_cpp_type(ty)
        ),
        HostAbiType::U64 | HostAbiType::HostRequestId => format!(
            "static_cast<{}>(call_long_getter(env, {object_name}, \"{getter_name}\"))",
            render_cpp_type(ty)
        ),
        HostAbiType::Bool => {
            format!("call_boolean_getter(env, {object_name}, \"{getter_name}\") == JNI_TRUE")
        }
        HostAbiType::F64 => format!(
            "static_cast<{}>(call_double_getter(env, {object_name}, \"{getter_name}\"))",
            render_cpp_type(ty)
        ),
        HostAbiType::Named(name) if android_named_type_is_enum(module, name) => format!(
            "{}(env, call_object_getter(env, {object_name}, \"{getter_name}\", \"{}\"))",
            android_cpp_decode_function_name(name),
            android_cpp_object_signature(module, ty)
        ),
        HostAbiType::StringRef
        | HostAbiType::Named(_)
        | HostAbiType::NativeArray(_)
        | HostAbiType::NativeSlice(_) => {
            panic!("non-primitive getter call requested for {:?}", ty)
        }
        other => panic!("unsupported Android primitive getter call for {:?}", other),
    }
}

/// Render one generic Android C++ methods source file.
pub(super) fn render_generic_cpp_methods_source(module: &HostAbiModule) -> String {
    let mut output = String::new();
    let request_input_leaves: Vec<_> = module
        .requests
        .iter()
        .map(|request| collect_android_cpp_request_input_leaves(module, request))
        .collect();
    let encode_root_types: Vec<_> = request_input_leaves
        .iter()
        .flatten()
        .filter_map(|leaf| match &leaf.ty {
            HostAbiType::NativeSlice(inner) if matches!(inner.as_ref(), HostAbiType::Named(_)) => {
                Some(leaf.ty.clone())
            }
            _ => None,
        })
        .collect();
    let request_output_types: Vec<_> = module
        .requests
        .iter()
        .flat_map(|request| {
            request
                .parameters
                .iter()
                .filter_map(|parameter| match &parameter.ty {
                    HostAbiType::OutputPointer(inner) => Some(inner.as_ref().clone()),
                    _ => None,
                })
        })
        .collect();
    let uses_string_inputs = request_input_leaves
        .iter()
        .flatten()
        .any(|leaf| matches!(leaf.ty, HostAbiType::StringRef));
    let uses_complex_inputs = request_input_leaves.iter().flatten().any(|leaf| {
        matches!(
            leaf.ty,
            HostAbiType::StringSlice | HostAbiType::NativeSlice(_)
        )
    });
    let uses_string_outputs = request_output_types
        .iter()
        .any(|ty| android_cpp_type_uses_jni_strings(module, ty));
    let uses_u8_slice =
        android_module_uses_type(module, &HostAbiType::NativeSlice(Box::new(HostAbiType::U8)));
    let uses_i8_slice =
        android_module_uses_type(module, &HostAbiType::NativeSlice(Box::new(HostAbiType::I8)));
    let uses_i16_slice = android_module_uses_type(
        module,
        &HostAbiType::NativeSlice(Box::new(HostAbiType::I16)),
    );
    let needs_encode_helpers = !encode_root_types.is_empty();
    let needs_decode_helpers = request_output_types.iter().any(|ty| {
        matches!(
            ty,
            HostAbiType::Named(_) | HostAbiType::NativeArray(_) | HostAbiType::NativeSlice(_)
        )
    });
    let needs_vector_output_storage = request_output_types
        .iter()
        .any(|ty| matches!(ty, HostAbiType::NativeArray(_)));
    let decode_root_types: Vec<_> = request_output_types
        .iter()
        .filter(|ty| {
            !matches!(
                ty,
                HostAbiType::Bool
                    | HostAbiType::U32
                    | HostAbiType::I32
                    | HostAbiType::U64
                    | HostAbiType::HostRequestId
            )
        })
        .cloned()
        .collect();

    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("#include \"../../types.h\"\n");
    output.push_str("#include \"../../jni.h\"\n");
    output.push_str("#include \"../../registry.h\"\n");
    output.push_str("#include \"callbacks.generated.h\"\n\n");
    output.push_str("#include <deque>\n");
    output.push_str("#include <string>\n");
    output.push_str("#include <vector>\n\n");
    output.push_str("namespace {\n\n");
    output.push_str("jclass bridge_class = nullptr;\n");

    for request in &module.requests {
        output.push_str(&format!(
            "jmethodID {} = nullptr;\n",
            android_cpp_method_id_name(request)
        ));
    }

    if uses_string_inputs {
        output.push('\n');
        push_cpp_doc_comment(
            &mut output,
            "Build one JNI string borrowed from one native string reference",
            0,
        );
        output.push_str("jstring java_string_or_null(JNIEnv *env, NativeStringRef value) {\n");
        output.push_str("    if (value.data == nullptr) {\n");
        output.push_str("        return nullptr;\n");
        output.push_str("    }\n\n");
        output.push_str("    return new_java_string(env, value);\n");
        output.push_str("}\n");
    }

    if uses_complex_inputs {
        output.push('\n');
        output.push_str("jmethodID resolve_constructor(JNIEnv *env, jclass value_class, const char *signature) {\n");
        output.push_str("    return env->GetMethodID(value_class, \"<init>\", signature);\n");
        output.push_str("}\n");
    }

    if uses_string_outputs {
        output.push('\n');
        push_cpp_doc_comment(
            &mut output,
            "Convert one Java string into one native string reference backed by stable storage",
            0,
        );
        output.push_str(
            "NativeStringRef string_ref_from_java(\n    JNIEnv *env,\n    jstring value,\n    std::deque<std::string> *storage\n) {\n",
        );
        output.push_str("    if (value == nullptr) {\n");
        output.push_str("        return NativeStringRef {\n");
        output.push_str("            .data = nullptr,\n");
        output.push_str("            .len = 0,\n");
        output.push_str("        };\n");
        output.push_str("    }\n\n");
        output.push_str("    const char *chars = env->GetStringUTFChars(value, nullptr);\n");
        output.push_str("    if (chars == nullptr) {\n");
        output.push_str("        return NativeStringRef {\n");
        output.push_str("            .data = nullptr,\n");
        output.push_str("            .len = 0,\n");
        output.push_str("        };\n");
        output.push_str("    }\n\n");
        output.push_str("    jsize length = env->GetStringUTFLength(value);\n");
        output.push_str("    storage->emplace_back(chars, chars + length);\n");
        output.push_str("    env->ReleaseStringUTFChars(value, chars);\n\n");
        output.push_str("    const std::string &owned = storage->back();\n\n");
        output.push_str("    return NativeStringRef {\n");
        output.push_str("        .data = reinterpret_cast<const uint8_t *>(owned.data()),\n");
        output.push_str("        .len = static_cast<uint32_t>(owned.size()),\n");
        output.push_str("    };\n");
        output.push_str("}\n");
    }

    if needs_decode_helpers {
        output.push('\n');
        output.push_str("thread_local std::deque<std::string> result_string_storage;\n");
    }

    if uses_u8_slice {
        output.push_str("thread_local std::deque<std::vector<uint8_t>> result_u8_storage;\n");
    }
    if uses_i8_slice {
        output.push_str("thread_local std::deque<std::vector<int8_t>> result_i8_storage;\n");
    }
    if uses_i16_slice {
        output.push_str("thread_local std::deque<std::vector<int16_t>> result_i16_storage;\n");
    }

    if needs_decode_helpers || uses_u8_slice || uses_i8_slice || uses_i16_slice {
        output.push('\n');
        output.push_str("void clear_result_decode_storage() {\n");
        if needs_decode_helpers {
            output.push_str("    result_string_storage.clear();\n");
        }
        if uses_u8_slice {
            output.push_str("    result_u8_storage.clear();\n");
        }
        if uses_i8_slice {
            output.push_str("    result_i8_storage.clear();\n");
        }
        if uses_i16_slice {
            output.push_str("    result_i16_storage.clear();\n");
        }
        output.push_str("}\n");
    }

    if needs_vector_output_storage {
        for request in &module.requests {
            for output_parameter in android_cpp_output_parameters(request) {
                if let HostAbiType::NativeArray(inner) = output_parameter.ty {
                    if let HostAbiType::Named(name) = inner.as_ref() {
                        output.push_str(&format!(
                            "thread_local std::vector<{}> {}_storage;\n",
                            name, output_parameter.name
                        ));
                    }
                }
            }
        }
    }

    if uses_u8_slice || uses_i8_slice || uses_i16_slice {
        output.push('\n');
        output.push_str("jint call_list_size(JNIEnv *env, jobject value);\n");
        output.push_str("jobject call_list_get(JNIEnv *env, jobject value, jint index);\n");
        output.push_str("jint call_int_getter(JNIEnv *env, jobject value, const char *name);\n");
    }

    if uses_u8_slice {
        output.push('\n');
        output.push_str("jintArray encode_u8_slice(JNIEnv *env, NativeU8Slice values) {\n");
        output
            .push_str("    jintArray array = env->NewIntArray(static_cast<jsize>(values.len));\n");
        output.push_str("    if (array == nullptr) {\n");
        output.push_str("        return nullptr;\n");
        output.push_str("    }\n\n");
        output.push_str("    std::vector<jint> buffer(static_cast<size_t>(values.len));\n");
        output.push_str("    for (uint32_t index = 0; index < values.len; index += 1) {\n");
        output.push_str(
            "        buffer[static_cast<size_t>(index)] = static_cast<jint>(values.data[index]);\n",
        );
        output.push_str("    }\n\n");
        output.push_str(
            "    env->SetIntArrayRegion(array, 0, static_cast<jsize>(values.len), buffer.data());\n",
        );
        output.push_str("    return array;\n");
        output.push_str("}\n\n");
        output.push_str("NativeU8Slice decode_u8_list(JNIEnv *env, jobject list) {\n");
        output.push_str("    if (list == nullptr) {\n");
        output.push_str("        return NativeU8Slice {\n");
        output.push_str("            .data = nullptr,\n");
        output.push_str("            .len = 0,\n");
        output.push_str("        };\n");
        output.push_str("    }\n\n");
        output.push_str("    jint size = call_list_size(env, list);\n");
        output.push_str("    result_u8_storage.emplace_back();\n");
        output.push_str("    auto &storage = result_u8_storage.back();\n");
        output.push_str("    storage.reserve(static_cast<size_t>(size));\n\n");
        output.push_str("    for (jint index = 0; index < size; index += 1) {\n");
        output.push_str("        jobject value = call_list_get(env, list, index);\n");
        output.push_str("        jint resolved = call_int_getter(env, value, \"intValue\");\n");
        output.push_str("        storage.push_back(static_cast<uint8_t>(resolved));\n");
        output.push_str("        env->DeleteLocalRef(value);\n");
        output.push_str("    }\n\n");
        output.push_str("    return NativeU8Slice {\n");
        output.push_str("        .data = storage.data(),\n");
        output.push_str("        .len = static_cast<uint32_t>(storage.size()),\n");
        output.push_str("    };\n");
        output.push_str("}\n");
    }

    if uses_i8_slice {
        output.push('\n');
        output.push_str("jintArray encode_i8_slice(JNIEnv *env, NativeI8Slice values) {\n");
        output
            .push_str("    jintArray array = env->NewIntArray(static_cast<jsize>(values.len));\n");
        output.push_str("    if (array == nullptr) {\n");
        output.push_str("        return nullptr;\n");
        output.push_str("    }\n\n");
        output.push_str("    std::vector<jint> buffer(static_cast<size_t>(values.len));\n");
        output.push_str("    for (uint32_t index = 0; index < values.len; index += 1) {\n");
        output.push_str(
            "        buffer[static_cast<size_t>(index)] = static_cast<jint>(values.data[index]);\n",
        );
        output.push_str("    }\n\n");
        output.push_str(
            "    env->SetIntArrayRegion(array, 0, static_cast<jsize>(values.len), buffer.data());\n",
        );
        output.push_str("    return array;\n");
        output.push_str("}\n\n");
        output.push_str("NativeI8Slice decode_i8_list(JNIEnv *env, jobject list) {\n");
        output.push_str("    if (list == nullptr) {\n");
        output.push_str("        return NativeI8Slice {\n");
        output.push_str("            .data = nullptr,\n");
        output.push_str("            .len = 0,\n");
        output.push_str("        };\n");
        output.push_str("    }\n\n");
        output.push_str("    jint size = call_list_size(env, list);\n");
        output.push_str("    result_i8_storage.emplace_back();\n");
        output.push_str("    auto &storage = result_i8_storage.back();\n");
        output.push_str("    storage.reserve(static_cast<size_t>(size));\n\n");
        output.push_str("    for (jint index = 0; index < size; index += 1) {\n");
        output.push_str("        jobject value = call_list_get(env, list, index);\n");
        output.push_str("        jint resolved = call_int_getter(env, value, \"intValue\");\n");
        output.push_str("        storage.push_back(static_cast<int8_t>(resolved));\n");
        output.push_str("        env->DeleteLocalRef(value);\n");
        output.push_str("    }\n\n");
        output.push_str("    return NativeI8Slice {\n");
        output.push_str("        .data = storage.data(),\n");
        output.push_str("        .len = static_cast<uint32_t>(storage.size()),\n");
        output.push_str("    };\n");
        output.push_str("}\n");
    }

    if uses_i16_slice {
        output.push('\n');
        output.push_str("jintArray encode_i16_slice(JNIEnv *env, NativeI16Slice values) {\n");
        output
            .push_str("    jintArray array = env->NewIntArray(static_cast<jsize>(values.len));\n");
        output.push_str("    if (array == nullptr) {\n");
        output.push_str("        return nullptr;\n");
        output.push_str("    }\n\n");
        output.push_str("    std::vector<jint> buffer(static_cast<size_t>(values.len));\n");
        output.push_str("    for (uint32_t index = 0; index < values.len; index += 1) {\n");
        output.push_str(
            "        buffer[static_cast<size_t>(index)] = static_cast<jint>(values.data[index]);\n",
        );
        output.push_str("    }\n\n");
        output.push_str(
            "    env->SetIntArrayRegion(array, 0, static_cast<jsize>(values.len), buffer.data());\n",
        );
        output.push_str("    return array;\n");
        output.push_str("}\n\n");
        output.push_str("NativeI16Slice decode_i16_list(JNIEnv *env, jobject list) {\n");
        output.push_str("    if (list == nullptr) {\n");
        output.push_str("        return NativeI16Slice {\n");
        output.push_str("            .data = nullptr,\n");
        output.push_str("            .len = 0,\n");
        output.push_str("        };\n");
        output.push_str("    }\n\n");
        output.push_str("    jint size = call_list_size(env, list);\n");
        output.push_str("    result_i16_storage.emplace_back();\n");
        output.push_str("    auto &storage = result_i16_storage.back();\n");
        output.push_str("    storage.reserve(static_cast<size_t>(size));\n\n");
        output.push_str("    for (jint index = 0; index < size; index += 1) {\n");
        output.push_str("        jobject value = call_list_get(env, list, index);\n");
        output.push_str("        jint resolved = call_int_getter(env, value, \"intValue\");\n");
        output.push_str("        storage.push_back(static_cast<int16_t>(resolved));\n");
        output.push_str("        env->DeleteLocalRef(value);\n");
        output.push_str("    }\n\n");
        output.push_str("    return NativeI16Slice {\n");
        output.push_str("        .data = storage.data(),\n");
        output.push_str("        .len = static_cast<uint32_t>(storage.size()),\n");
        output.push_str("    };\n");
        output.push_str("}\n");
    }

    if needs_encode_helpers {
        output.push('\n');
        render_android_cpp_encode_helper_prototypes(&mut output, module, &encode_root_types);
        output.push('\n');
        render_android_cpp_encode_helpers(&mut output, module, &encode_root_types);
    }

    if needs_decode_helpers {
        output.push('\n');
        render_android_cpp_decode_helper_prototypes(&mut output, module, &decode_root_types);
        output.push('\n');
        render_android_cpp_decode_helpers(&mut output, module, &decode_root_types);
    }

    output.push('\n');
    push_cpp_doc_comment(
        &mut output,
        &format!(
            "Resolve the {} bridge methods from one runtime bridge instance",
            module.name
        ),
        0,
    );
    output.push_str("bool resolve_bridge_methods(JNIEnv *env, jobject bridge) {\n");
    output.push_str("    if (bridge_class == nullptr) {\n");
    output.push_str("        jclass local_class = env->GetObjectClass(bridge);\n");
    output.push_str("        if (local_class == nullptr) {\n");
    output.push_str("            return false;\n");
    output.push_str("        }\n\n");
    output.push_str(
        "        bridge_class = reinterpret_cast<jclass>(env->NewGlobalRef(local_class));\n",
    );
    output.push_str("        env->DeleteLocalRef(local_class);\n");
    output.push_str("        if (bridge_class == nullptr) {\n");
    output.push_str("            return false;\n");
    output.push_str("        }\n");
    output.push_str("    }\n\n");

    for (request, leaves) in module.requests.iter().zip(&request_input_leaves) {
        let mut signature = String::from("(");
        for leaf in leaves {
            signature.push_str(&android_cpp_leaf_jni_signature(module, &leaf.ty));
        }
        signature.push(')');
        signature.push_str(&android_cpp_request_return_signature(module, request));

        output.push_str(&format!(
            "    if ({} == nullptr) {{\n",
            android_cpp_method_id_name(request)
        ));
        output.push_str(&format!(
            "        {} = env->GetMethodID(\n",
            android_cpp_method_id_name(request)
        ));
        output.push_str("            bridge_class,\n");
        output.push_str(&format!(
            "            \"{}\",\n",
            android_kotlin_bridge_method_name(module, request)
        ));
        output.push_str(&format!("            \"{}\"\n", signature));
        output.push_str("        );\n");
        output.push_str("    }\n\n");
    }

    output.push_str("    return\n");
    for (index, request) in module.requests.iter().enumerate() {
        let trailing = if index + 1 == module.requests.len() {
            ";"
        } else {
            " &&"
        };
        output.push_str(&format!(
            "        {} != nullptr{}\n",
            android_cpp_method_id_name(request),
            trailing
        ));
    }
    output.push_str("}\n\n");
    output.push_str("}\n\n");
    output.push_str(&format!(
        "bool resolve_{}_methods(JNIEnv *env, jobject bridge) {{\n",
        module.name
    ));
    output.push_str("    return resolve_bridge_methods(env, bridge);\n");
    output.push_str("}\n\n");

    for request in &module.requests {
        render_generic_cpp_request_method(&mut output, module, request);
        output.push('\n');
    }

    output
}

/// Collect the named decode types reachable from one root ABI type.
fn collect_android_cpp_decode_named_types(
    module: &HostAbiModule,
    root_types: &[HostAbiType],
) -> Vec<String> {
    let mut names = Vec::new();

    for ty in root_types {
        collect_android_cpp_decode_named_types_from_type(module, ty, &mut names);
    }

    names
}

/// Collect the named decode types reachable from one ABI type.
pub(super) fn collect_android_cpp_decode_named_types_from_type(
    module: &HostAbiModule,
    ty: &HostAbiType,
    names: &mut Vec<String>,
) {
    match ty {
        HostAbiType::Named(name) => {
            if names.iter().any(|existing| existing == name) {
                return;
            }

            names.push((*name).to_string());

            if let HostAbiNamedTypeDefinition::Struct { fields } =
                &android_named_type(module, name).definition
            {
                for field in fields {
                    collect_android_cpp_decode_named_types_from_type(module, &field.ty, names);
                }
            }
        }
        HostAbiType::NativeArray(inner)
        | HostAbiType::NativeSlice(inner)
        | HostAbiType::OutputPointer(inner) => {
            collect_android_cpp_decode_named_types_from_type(module, inner, names);
        }
        _ => {}
    }
}

/// Render the decode helper prototypes for one Android methods source file.
fn render_android_cpp_decode_helper_prototypes(
    output: &mut String,
    module: &HostAbiModule,
    root_types: &[HostAbiType],
) {
    for name in collect_android_cpp_decode_named_types(module, root_types) {
        if android_named_type_is_enum(module, &name) {
            output.push_str(&format!(
                "{} {}(JNIEnv *env, jobject value);\n",
                name,
                android_cpp_decode_function_name(&name)
            ));
        } else {
            output.push_str(&format!(
                "{} {}(JNIEnv *env, jobject value, std::deque<std::string> *string_storage);\n",
                name,
                android_cpp_decode_function_name(&name)
            ));
        }
    }

    let mut generated_slice_types = Vec::new();
    let mut visited_named_types = Vec::new();

    for ty in root_types {
        collect_android_generated_slice_types_from_type(
            module,
            ty,
            &mut generated_slice_types,
            &mut visited_named_types,
        );
    }

    for generated_type in generated_slice_types {
        output.push_str(&format!(
            "{} {}(JNIEnv *env, jobject value, std::deque<std::string> *string_storage);\n",
            generated_type.name,
            android_cpp_decode_slice_function_name(&generated_type.element_name)
        ));
    }
}

/// Render the encode helper prototypes for one Android methods source file.
fn render_android_cpp_encode_helper_prototypes(
    output: &mut String,
    module: &HostAbiModule,
    root_types: &[HostAbiType],
) {
    for name in collect_android_cpp_decode_named_types(module, root_types) {
        output.push_str(&format!(
            "jobject {}(JNIEnv *env, {} value);\n",
            android_cpp_encode_function_name(&name),
            name,
        ));
    }

    let mut generated_slice_types = Vec::new();
    let mut visited_named_types = Vec::new();

    for ty in root_types {
        collect_android_generated_slice_types_from_type(
            module,
            ty,
            &mut generated_slice_types,
            &mut visited_named_types,
        );
    }

    for generated_type in generated_slice_types {
        output.push_str(&format!(
            "jobjectArray {}(JNIEnv *env, {} value);\n",
            android_cpp_encode_slice_function_name(&generated_type.element_name),
            generated_type.name,
        ));
    }
}

/// Render the decode helper definitions for one Android methods source file.
fn render_android_cpp_decode_helpers(
    output: &mut String,
    module: &HostAbiModule,
    root_types: &[HostAbiType],
) {
    output.push_str("jmethodID resolve_instance_method(\n");
    output.push_str("    JNIEnv *env,\n");
    output.push_str("    jobject value,\n");
    output.push_str("    const char *name,\n");
    output.push_str("    const char *signature\n");
    output.push_str(") {\n");
    output.push_str("    jclass value_class = env->GetObjectClass(value);\n");
    output.push_str("    if (value_class == nullptr) {\n");
    output.push_str("        return nullptr;\n");
    output.push_str("    }\n\n");
    output.push_str("    jmethodID method = env->GetMethodID(value_class, name, signature);\n");
    output.push_str("    env->DeleteLocalRef(value_class);\n\n");
    output.push_str("    return method;\n");
    output.push_str("}\n\n");

    output.push_str("jobject call_object_getter(\n");
    output.push_str("    JNIEnv *env,\n");
    output.push_str("    jobject value,\n");
    output.push_str("    const char *name,\n");
    output.push_str("    const char *signature\n");
    output.push_str(") {\n");
    output
        .push_str("    jmethodID method = resolve_instance_method(env, value, name, signature);\n");
    output.push_str("    if (method == nullptr) {\n");
    output.push_str("        return nullptr;\n");
    output.push_str("    }\n\n");
    output.push_str("    return env->CallObjectMethod(value, method);\n");
    output.push_str("}\n\n");

    output.push_str("jint call_int_getter(JNIEnv *env, jobject value, const char *name) {\n");
    output.push_str("    jmethodID method = resolve_instance_method(env, value, name, \"()I\");\n");
    output.push_str("    return method == nullptr ? 0 : env->CallIntMethod(value, method);\n");
    output.push_str("}\n\n");

    output.push_str("jlong call_long_getter(JNIEnv *env, jobject value, const char *name) {\n");
    output.push_str("    jmethodID method = resolve_instance_method(env, value, name, \"()J\");\n");
    output.push_str("    return method == nullptr ? 0 : env->CallLongMethod(value, method);\n");
    output.push_str("}\n\n");

    output.push_str("jdouble call_double_getter(JNIEnv *env, jobject value, const char *name) {\n");
    output.push_str("    jmethodID method = resolve_instance_method(env, value, name, \"()D\");\n");
    output.push_str("    return method == nullptr ? 0.0 : env->CallDoubleMethod(value, method);\n");
    output.push_str("}\n\n");

    output
        .push_str("jboolean call_boolean_getter(JNIEnv *env, jobject value, const char *name) {\n");
    output.push_str("    jmethodID method = resolve_instance_method(env, value, name, \"()Z\");\n");
    output.push_str(
        "    return method == nullptr ? JNI_FALSE : env->CallBooleanMethod(value, method);\n",
    );
    output.push_str("}\n\n");

    output.push_str("uint64_t decode_boxed_u64(JNIEnv *env, jobject value) {\n");
    output.push_str(
        "    jmethodID method = resolve_instance_method(env, value, \"longValue\", \"()J\");\n",
    );
    output.push_str("    return method == nullptr ? 0 : static_cast<uint64_t>(env->CallLongMethod(value, method));\n");
    output.push_str("}\n\n");

    output.push_str("jint call_list_size(JNIEnv *env, jobject value) {\n");
    output.push_str(
        "    jmethodID method = resolve_instance_method(env, value, \"size\", \"()I\");\n",
    );
    output.push_str("    return method == nullptr ? 0 : env->CallIntMethod(value, method);\n");
    output.push_str("}\n\n");

    output.push_str("jobject call_list_get(JNIEnv *env, jobject value, jint index) {\n");
    output.push_str(
        "    jmethodID method = resolve_instance_method(env, value, \"get\", \"(I)Ljava/lang/Object;\");\n",
    );
    output.push_str(
        "    return method == nullptr ? nullptr : env->CallObjectMethod(value, method, index);\n",
    );
    output.push_str("}\n\n");

    for name in collect_android_cpp_decode_named_types(module, root_types) {
        let named_type = android_named_type(module, &name);

        match &named_type.definition {
            HostAbiNamedTypeDefinition::Enum { variants, .. } => {
                let first_discriminant = variants
                    .first()
                    .map(|variant| variant.discriminant)
                    .unwrap_or(0);
                output.push_str(&format!(
                    "{} {}(JNIEnv *env, jobject value) {{\n",
                    name,
                    android_cpp_decode_function_name(&name)
                ));
                output.push_str("    jint ordinal = call_int_getter(env, value, \"ordinal\");\n\n");
                output.push_str(&format!(
                    "    return static_cast<{}>(ordinal + {});\n",
                    name, first_discriminant
                ));
                output.push_str("}\n\n");
            }
            HostAbiNamedTypeDefinition::Struct { fields } => {
                output.push_str(&format!(
                    "{} {}(JNIEnv *env, jobject value, std::deque<std::string> *string_storage) {{\n",
                    name,
                    android_cpp_decode_function_name(&name)
                ));
                output.push_str(&format!("    {} decoded = {{}};\n\n", name));

                let mut index = 0;
                while index < fields.len() {
                    let field = &fields[index];

                    if let Some(optional_field_name) = field.name.strip_prefix("has_") {
                        if let Some(value_field) = fields.get(index + 1) {
                            if value_field.name == optional_field_name {
                                render_android_cpp_optional_field_decode(
                                    output,
                                    module,
                                    field,
                                    value_field,
                                );
                                index += 2;
                                continue;
                            }
                        }
                    }

                    render_android_cpp_struct_field_decode(output, module, field);
                    index += 1;
                }

                output.push_str("\n    return decoded;\n");
                output.push_str("}\n\n");
            }
        }
    }

    let mut generated_slice_types = Vec::new();
    let mut visited_named_types = Vec::new();

    for ty in root_types {
        collect_android_generated_slice_types_from_type(
            module,
            ty,
            &mut generated_slice_types,
            &mut visited_named_types,
        );
    }

    for generated_type in generated_slice_types {
        let decode_function_name =
            android_cpp_decode_slice_function_name(&generated_type.element_name);
        let element_decode_function_name =
            android_cpp_decode_function_name(&generated_type.element_name);

        output.push_str(&format!(
            "{} {}(JNIEnv *env, jobject value, std::deque<std::string> *string_storage) {{\n",
            generated_type.name, decode_function_name
        ));
        output.push_str(&format!(
            "    thread_local std::vector<{}> storage;\n",
            generated_type.element_name
        ));
        output.push_str("    storage.clear();\n\n");
        output.push_str("    if (value == nullptr) {\n");
        output.push_str(&format!(
            "        return {} {{ .data = nullptr, .len = 0 }};\n",
            generated_type.name
        ));
        output.push_str("    }\n\n");
        output.push_str("    jint count = call_list_size(env, value);\n");
        output.push_str("    storage.reserve(static_cast<size_t>(count));\n");
        output.push_str("    for (jint index = 0; index < count; index += 1) {\n");
        output.push_str("        jobject element = call_list_get(env, value, index);\n");
        output.push_str("        if (element == nullptr) {\n");
        output.push_str("            storage.clear();\n");
        output.push_str(&format!(
            "            return {} {{ .data = nullptr, .len = 0 }};\n",
            generated_type.name
        ));
        output.push_str("        }\n\n");
        output.push_str(&format!(
            "        storage.push_back({}(env, element, string_storage));\n",
            element_decode_function_name
        ));
        output.push_str("        env->DeleteLocalRef(element);\n");
        output.push_str("    }\n\n");
        output.push_str(&format!("    return {} {{\n", generated_type.name));
        output.push_str("        .data = storage.empty() ? nullptr : storage.data(),\n");
        output.push_str("        .len = static_cast<uint32_t>(storage.size()),\n");
        output.push_str("    };\n");
        output.push_str("}\n\n");
    }
}

/// Render the encode helper definitions for one Android methods source file.
fn render_android_cpp_encode_helpers(
    output: &mut String,
    module: &HostAbiModule,
    root_types: &[HostAbiType],
) {
    for name in collect_android_cpp_decode_named_types(module, root_types) {
        let named_type = android_named_type(module, &name);
        match &named_type.definition {
            HostAbiNamedTypeDefinition::Enum { variants, .. } => {
                let binary_name = android_runtime_named_binary_name(module, &name);
                output.push_str(&format!(
                    "jobject {}(JNIEnv *env, {} value) {{\n",
                    android_cpp_encode_function_name(&name),
                    name,
                ));
                output.push_str(&format!(
                    "    jclass value_class = env->FindClass(\"{}\");\n",
                    binary_name,
                ));
                output.push_str("    if (value_class == nullptr) {\n");
                output.push_str("        return nullptr;\n");
                output.push_str("    }\n\n");
                output.push_str("    jobject result = nullptr;\n\n");
                output.push_str("    switch (value) {\n");

                for variant in variants {
                    output.push_str(&format!("        case {}::{}: {{\n", name, variant.name));
                    output.push_str(&format!(
                        "            jfieldID field = env->GetStaticFieldID(value_class, \"{}\", \"L{};\");\n",
                        variant.name, binary_name
                    ));
                    output.push_str("            result = field == nullptr ? nullptr : env->GetStaticObjectField(value_class, field);\n");
                    output.push_str("            break;\n");
                    output.push_str("        }\n");
                }

                output.push_str("        default:\n");
                output.push_str("            break;\n");
                output.push_str("    }\n\n");
                output.push_str("    env->DeleteLocalRef(value_class);\n\n");
                output.push_str("    return result;\n");
                output.push_str("}\n\n");
            }
            HostAbiNamedTypeDefinition::Struct { fields } => {
                output.push_str(&format!(
                    "jobject {}(JNIEnv *env, {} value) {{\n",
                    android_cpp_encode_function_name(&name),
                    name,
                ));
                output.push_str(&format!(
                    "    jclass value_class = env->FindClass(\"{}\");\n",
                    android_runtime_named_binary_name(module, &name),
                ));
                output.push_str("    if (value_class == nullptr) {\n");
                output.push_str("        return nullptr;\n");
                output.push_str("    }\n\n");
                output.push_str(&format!(
                    "    jmethodID constructor = resolve_constructor(env, value_class, \"({})V\");\n",
                    fields
                        .iter()
                        .map(|field| android_cpp_request_object_signature(module, &field.ty))
                        .collect::<Vec<_>>()
                        .join(""),
                ));
                output.push_str("    if (constructor == nullptr) {\n");
                output.push_str("        env->DeleteLocalRef(value_class);\n");
                output.push_str("        return nullptr;\n");
                output.push_str("    }\n\n");

                for field in fields {
                    match &field.ty {
                        HostAbiType::StringRef => {
                            output.push_str(&format!(
                                "    jstring {}_value = java_string_or_null(env, value.{});\n",
                                field.name, field.name
                            ));
                        }
                        HostAbiType::StringSlice => {
                            output.push_str(&format!(
                                "    jobjectArray {}_value = new_java_string_array(env, value.{});\n",
                                field.name, field.name
                            ));
                        }
                        HostAbiType::Named(inner_name) => {
                            output.push_str(&format!(
                                "    jobject {}_value = {}(env, value.{});\n",
                                field.name,
                                android_cpp_encode_function_name(inner_name),
                                field.name
                            ));
                        }
                        HostAbiType::NativeSlice(inner) => {
                            if let HostAbiType::Named(inner_name) = inner.as_ref() {
                                output.push_str(&format!(
                                    "    jobjectArray {}_value = {}(env, value.{});\n",
                                    field.name,
                                    android_cpp_encode_slice_function_name(inner_name),
                                    field.name
                                ));
                            } else if android_cpp_primitive_slice_type(inner).is_some() {
                                output.push_str(&format!(
                                    "    jintArray {}_value = {}(env, value.{});\n",
                                    field.name,
                                    android_cpp_primitive_slice_encode_function_name(inner)
                                        .expect("primitive slice encode helper"),
                                    field.name
                                ));
                            } else {
                                panic!(
                                    "unsupported Android encode helper field type for {:?}",
                                    field.ty
                                );
                            }
                        }
                        _ => {}
                    }
                }

                output.push_str("\n    jobject result = env->NewObject(\n");
                output.push_str("        value_class,\n");
                output.push_str("        constructor");
                for field in fields {
                    let argument = match &field.ty {
                        HostAbiType::StringRef
                        | HostAbiType::StringSlice
                        | HostAbiType::NativeSlice(_) => {
                            format!(",\n        {}_value", field.name)
                        }
                        HostAbiType::Named(_) => {
                            format!(",\n        {}_value", field.name)
                        }
                        HostAbiType::Bool => {
                            format!(",\n        value.{} ? JNI_TRUE : JNI_FALSE", field.name)
                        }
                        HostAbiType::U8
                        | HostAbiType::I8
                        | HostAbiType::I16
                        | HostAbiType::U32
                        | HostAbiType::I32 => {
                            format!(",\n        static_cast<jint>(value.{})", field.name)
                        }
                        HostAbiType::U64 | HostAbiType::HostRequestId => {
                            format!(",\n        static_cast<jlong>(value.{})", field.name)
                        }
                        HostAbiType::F64 => {
                            format!(",\n        static_cast<jdouble>(value.{})", field.name)
                        }
                        other => {
                            panic!("unsupported Android encode helper argument for {:?}", other)
                        }
                    };
                    output.push_str(&argument);
                }
                output.push_str("\n    );\n\n");

                for field in fields {
                    match &field.ty {
                        HostAbiType::StringRef
                        | HostAbiType::StringSlice
                        | HostAbiType::NativeSlice(_) => {
                            output.push_str(&format!(
                                "    if ({}_value != nullptr) {{ env->DeleteLocalRef({}_value); }}\n",
                                field.name, field.name
                            ));
                        }
                        HostAbiType::Named(_) => {
                            output.push_str(&format!(
                                "    if ({}_value != nullptr) {{ env->DeleteLocalRef({}_value); }}\n",
                                field.name, field.name
                            ));
                        }
                        _ => {}
                    }
                }

                output.push_str("    env->DeleteLocalRef(value_class);\n\n");
                output.push_str("    return result;\n");
                output.push_str("}\n\n");
            }
        }
    }

    let mut generated_slice_types = Vec::new();
    let mut visited_named_types = Vec::new();

    for ty in root_types {
        collect_android_generated_slice_types_from_type(
            module,
            ty,
            &mut generated_slice_types,
            &mut visited_named_types,
        );
    }

    for generated_type in generated_slice_types {
        output.push_str(&format!(
            "jobjectArray {}(JNIEnv *env, {} value) {{\n",
            android_cpp_encode_slice_function_name(&generated_type.element_name),
            generated_type.name,
        ));
        output.push_str(&format!(
            "    jclass element_class = env->FindClass(\"{}\");\n",
            android_runtime_named_binary_name(module, &generated_type.element_name),
        ));
        output.push_str("    if (element_class == nullptr) {\n");
        output.push_str("        return nullptr;\n");
        output.push_str("    }\n\n");
        output.push_str(
            "    jobjectArray array = env->NewObjectArray(static_cast<jsize>(value.len), element_class, nullptr);\n",
        );
        output.push_str("    env->DeleteLocalRef(element_class);\n");
        output.push_str("    if (array == nullptr) {\n");
        output.push_str("        return nullptr;\n");
        output.push_str("    }\n\n");
        output.push_str("    for (uint32_t index = 0; index < value.len; index += 1) {\n");
        output.push_str(&format!(
            "        jobject element = {}(env, value.data[index]);\n",
            android_cpp_encode_function_name(&generated_type.element_name),
        ));
        output.push_str("        if (element == nullptr) {\n");
        output.push_str("            env->DeleteLocalRef(array);\n");
        output.push_str("            return nullptr;\n");
        output.push_str("        }\n\n");
        output.push_str(
            "        env->SetObjectArrayElement(array, static_cast<jsize>(index), element);\n",
        );
        output.push_str("        env->DeleteLocalRef(element);\n");
        output.push_str("    }\n\n");
        output.push_str("    return array;\n");
        output.push_str("}\n\n");
    }
}

/// Return the JVM signature for one request-side object field.
fn android_cpp_request_object_signature(module: &HostAbiModule, ty: &HostAbiType) -> String {
    match ty {
        HostAbiType::U8 | HostAbiType::I8 | HostAbiType::I16 => "I".to_string(),
        HostAbiType::U32 | HostAbiType::I32 => "I".to_string(),
        HostAbiType::U64 | HostAbiType::HostRequestId => "J".to_string(),
        HostAbiType::Bool => "Z".to_string(),
        HostAbiType::F64 => "D".to_string(),
        HostAbiType::StringRef => "Ljava/lang/String;".to_string(),
        HostAbiType::StringSlice => "[Ljava/lang/String;".to_string(),
        HostAbiType::Named(name) => {
            format!("L{};", android_runtime_named_binary_name(module, name))
        }
        HostAbiType::NativeSlice(inner) => {
            if let HostAbiType::Named(name) = inner.as_ref() {
                format!("[L{};", android_runtime_named_binary_name(module, name))
            } else if android_cpp_primitive_slice_type(inner).is_some() {
                "[I".to_string()
            } else {
                panic!("unsupported Android request object signature for {:?}", ty);
            }
        }
        other => panic!(
            "unsupported Android request object signature for {:?}",
            other
        ),
    }
}

/// Render one optional struct-field decode block.
fn render_android_cpp_optional_field_decode(
    output: &mut String,
    module: &HostAbiModule,
    has_field: &HostAbiField,
    value_field: &HostAbiField,
) {
    let getter_name = android_cpp_getter_name(value_field.name, false);
    let value_object_name = format!("{}_value", value_field.name);
    let getter_signature = android_cpp_object_signature(module, &value_field.ty);

    output.push_str(&format!(
        "    jobject {} = call_object_getter(env, value, \"{}\", \"{}\");\n",
        value_object_name, getter_name, getter_signature
    ));
    output.push_str(&format!(
        "    decoded.{} = {} != nullptr;\n",
        has_field.name, value_object_name
    ));
    output.push_str(&format!(
        "    decoded.{} = {} == nullptr ? {} : {};\n",
        value_field.name,
        value_object_name,
        android_cpp_default_value(module, &value_field.ty),
        android_cpp_object_decode_expression(module, &value_field.ty, &value_object_name)
    ));
    output.push_str(&format!(
        "    if ({} != nullptr) {{ env->DeleteLocalRef({}); }}\n\n",
        value_object_name, value_object_name
    ));
}

/// Render one regular struct-field decode block.
fn render_android_cpp_struct_field_decode(
    output: &mut String,
    module: &HostAbiModule,
    field: &HostAbiField,
) {
    let getter_name = android_cpp_getter_name(field.name, matches!(field.ty, HostAbiType::Bool));

    match &field.ty {
        HostAbiType::StringRef
        | HostAbiType::Named(_)
        | HostAbiType::NativeArray(_)
        | HostAbiType::NativeSlice(_) => {
            let value_object_name = format!("{}_value", field.name);
            output.push_str(&format!(
                "    jobject {} = call_object_getter(env, value, \"{}\", \"{}\");\n",
                value_object_name,
                getter_name,
                android_cpp_object_signature(module, &field.ty)
            ));
            output.push_str(&format!(
                "    decoded.{} = {};\n",
                field.name,
                android_cpp_object_decode_expression(module, &field.ty, &value_object_name)
            ));
            output.push_str(&format!(
                "    if ({} != nullptr) {{ env->DeleteLocalRef({}); }}\n\n",
                value_object_name, value_object_name
            ));
        }
        _ => {
            output.push_str(&format!(
                "    decoded.{} = {};\n\n",
                field.name,
                android_cpp_primitive_getter_call(module, &field.ty, "value", &getter_name)
            ));
        }
    }
}

/// Return one decode expression for one object-returning value.
fn android_cpp_object_decode_expression(
    module: &HostAbiModule,
    ty: &HostAbiType,
    value_name: &str,
) -> String {
    match ty {
        HostAbiType::StringRef => {
            format!("string_ref_from_java(env, static_cast<jstring>({value_name}), string_storage)")
        }
        HostAbiType::U8 | HostAbiType::I8 | HostAbiType::I16 => format!(
            "static_cast<{}>(call_int_getter(env, {value_name}, \"intValue\"))",
            render_cpp_type(ty)
        ),
        HostAbiType::U32 | HostAbiType::I32 | HostAbiType::HostStatus => format!(
            "static_cast<{}>(call_int_getter(env, {value_name}, \"intValue\"))",
            render_cpp_type(ty)
        ),
        HostAbiType::U64 | HostAbiType::HostRequestId => {
            format!("decode_boxed_u64(env, {value_name})")
        }
        HostAbiType::Bool => {
            format!("call_boolean_getter(env, {value_name}, \"booleanValue\") == JNI_TRUE")
        }
        HostAbiType::F64 => {
            format!("static_cast<double>(call_double_getter(env, {value_name}, \"doubleValue\"))")
        }
        HostAbiType::Named(name) if android_named_type_is_enum(module, name) => format!(
            "{}(env, {value_name})",
            android_cpp_decode_function_name(name)
        ),
        HostAbiType::Named(name) => format!(
            "{}(env, {value_name}, string_storage)",
            android_cpp_decode_function_name(name)
        ),
        HostAbiType::NativeSlice(inner) => {
            if let HostAbiType::Named(name) = inner.as_ref() {
                format!(
                    "{}(env, {}, string_storage)",
                    android_cpp_decode_slice_function_name(name),
                    value_name
                )
            } else if let Some(function_name) =
                android_cpp_primitive_slice_decode_function_name(inner)
            {
                format!("{function_name}(env, {value_name})")
            } else {
                panic!("unsupported Android object decode expression for {:?}", ty);
            }
        }
        other => panic!(
            "unsupported Android object decode expression for {:?}",
            other
        ),
    }
}

/// Render one generic Android request method.
fn render_generic_cpp_request_method(
    output: &mut String,
    module: &HostAbiModule,
    request: &HostAbiFunction,
) {
    let input_leaves = collect_android_cpp_request_input_leaves(module, request);
    let output_parameters = android_cpp_output_parameters(request);
    let uses_direct_response_decode = output_parameters.len() == 1
        && output_parameters[0].name == "response"
        && matches!(output_parameters[0].ty, HostAbiType::Named(_));
    let uses_output_decode_storage = output_parameters.iter().any(|output_parameter| {
        matches!(
            output_parameter.ty,
            HostAbiType::Named(_) | HostAbiType::NativeArray(_) | HostAbiType::NativeSlice(_)
        )
    });

    push_cpp_doc_comment(
        output,
        &format!(
            "Call the {}-{} entrypoint on one registered bridge",
            module.name,
            request.name.replace('_', "-")
        ),
        0,
    );
    output.push_str("uint32_t ");
    output.push_str(&android_cpp_methods_request_name(module, request));
    output.push_str("(\n");
    output.push_str("    JNIEnv *env,\n");
    for (index, parameter) in request.parameters.iter().enumerate() {
        let trailing = if index + 1 == request.parameters.len() {
            ""
        } else {
            ","
        };
        let declaration =
            render_cpp_parameter_declaration(&render_cpp_type(&parameter.ty), parameter.name);
        output.push_str(&format!("    {declaration}{trailing}\n"));
    }
    output.push_str(") {\n");
    output.push_str("    jobject bridge = resolve_bridge(env, session_handle);\n");
    output.push_str("    if (bridge == nullptr) {\n");
    output.push_str("        return HOST_STATUS_NOT_FOUND;\n");
    output.push_str("    }\n\n");

    for (index, leaf) in input_leaves.iter().enumerate() {
        match leaf.ty {
            HostAbiType::StringRef => {
                output.push_str(&format!(
                    "    jstring {}_value = java_string_or_null(env, {});\n",
                    leaf.name, leaf.expr
                ));
            }
            HostAbiType::StringSlice => {
                output.push_str(&format!(
                    "    jobjectArray {}_value = new_java_string_array(env, {});\n",
                    leaf.name, leaf.expr
                ));
                output.push_str(&format!("    if ({}_value == nullptr) {{\n", leaf.name));
                if index != 0 {
                    render_android_cpp_release_input_locals(output, &input_leaves[..index], 8);
                }
                output.push_str("        if (env->ExceptionCheck()) {\n");
                output.push_str("            env->ExceptionClear();\n");
                output.push_str("        }\n");
                output.push_str("        env->DeleteLocalRef(bridge);\n");
                output.push_str("        return HOST_STATUS_FAILED;\n");
                output.push_str("    }\n");
            }
            HostAbiType::NativeSlice(ref inner) => {
                if let HostAbiType::Named(name) = inner.as_ref() {
                    output.push_str(&format!(
                        "    jobjectArray {}_value = {}(env, {});\n",
                        leaf.name,
                        android_cpp_encode_slice_function_name(name),
                        leaf.expr
                    ));
                } else if let Some(function_name) =
                    android_cpp_primitive_slice_encode_function_name(inner)
                {
                    output.push_str(&format!(
                        "    jintArray {}_value = {}(env, {});\n",
                        leaf.name, function_name, leaf.expr
                    ));
                } else {
                    panic!(
                        "unsupported Android request input leaf local for {:?}",
                        leaf.ty
                    );
                }
                output.push_str(&format!("    if ({}_value == nullptr) {{\n", leaf.name));
                if index != 0 {
                    render_android_cpp_release_input_locals(output, &input_leaves[..index], 8);
                }
                output.push_str("        if (env->ExceptionCheck()) {\n");
                output.push_str("            env->ExceptionClear();\n");
                output.push_str("        }\n");
                output.push_str("        env->DeleteLocalRef(bridge);\n");
                output.push_str("        return HOST_STATUS_FAILED;\n");
                output.push_str("    }\n");
            }
            _ => {}
        }
    }

    if output_parameters.is_empty() {
        if input_leaves.is_empty() {
            output.push_str(&format!(
                "\n    jint status = env->CallIntMethod(\n        bridge,\n        {}\n    );\n",
                android_cpp_method_id_name(request)
            ));
        } else {
            output.push_str("\n    jint status = env->CallIntMethod(\n");
            output.push_str("        bridge,\n");
            output.push_str(&format!(
                "        {},\n",
                android_cpp_method_id_name(request)
            ));
            render_android_cpp_request_call_arguments(output, module, &input_leaves);
            output.push_str("    );\n");
        }
    } else {
        if input_leaves.is_empty() {
            output.push_str(&format!(
                "\n    jobject response_object = env->CallObjectMethod(\n        bridge,\n        {}\n    );\n",
                android_cpp_method_id_name(request)
            ));
        } else {
            output.push_str("\n    jobject response_object = env->CallObjectMethod(\n");
            output.push_str("        bridge,\n");
            output.push_str(&format!(
                "        {},\n",
                android_cpp_method_id_name(request)
            ));
            render_android_cpp_request_call_arguments(output, module, &input_leaves);
            output.push_str("    );\n");
        }
        output.push_str("    if (response_object == nullptr) {\n");
        render_android_cpp_release_input_locals(output, &input_leaves, 8);
        output.push_str("        if (env->ExceptionCheck()) {\n");
        output.push_str("            env->ExceptionClear();\n");
        output.push_str("        }\n");
        output.push_str("        env->DeleteLocalRef(bridge);\n");
        output.push_str("        return HOST_STATUS_FAILED;\n");
        output.push_str("    }\n\n");

        if uses_output_decode_storage {
            output.push_str("    clear_result_decode_storage();\n");
            output.push_str(
                "    std::deque<std::string> *string_storage = &result_string_storage;\n\n",
            );
        }

        output
            .push_str("    jint status = call_int_getter(env, response_object, \"getStatus\");\n");

        output.push_str("    if (status == HOST_STATUS_OK) {\n");
        if uses_direct_response_decode {
            let output_parameter = &output_parameters[0];
            output.push_str(&format!(
                "        *{} = {};\n",
                output_parameter.expr,
                android_cpp_object_decode_expression(
                    module,
                    &output_parameter.ty,
                    "response_object"
                )
            ));
        } else {
            for output_parameter in &output_parameters {
                render_android_cpp_output_decode(
                    output,
                    module,
                    output_parameter,
                    "response_object",
                    8,
                );
            }
        }
        output.push_str("    }\n");
    }

    render_android_cpp_release_input_locals(output, &input_leaves, 4);
    if !output_parameters.is_empty() {
        output.push_str("    env->DeleteLocalRef(response_object);\n");
    }
    output.push_str("    env->DeleteLocalRef(bridge);\n\n");
    output.push_str("    if (env->ExceptionCheck()) {\n");
    output.push_str("        env->ExceptionClear();\n");
    output.push_str("        return HOST_STATUS_FAILED;\n");
    output.push_str("    }\n\n");
    output.push_str("    return static_cast<uint32_t>(status);\n");
    output.push_str("}\n");
}

/// Render the call arguments for one request method body.
fn render_android_cpp_request_call_arguments(
    output: &mut String,
    module: &HostAbiModule,
    input_leaves: &[AndroidCppLeaf],
) {
    for (index, leaf) in input_leaves.iter().enumerate() {
        let argument = match &leaf.ty {
            HostAbiType::StringRef | HostAbiType::StringSlice => format!("{}_value", leaf.name),
            HostAbiType::Bool => format!("{} ? JNI_TRUE : JNI_FALSE", leaf.expr),
            HostAbiType::U32 | HostAbiType::I32 => {
                format!("static_cast<jint>({})", leaf.expr)
            }
            HostAbiType::U64 | HostAbiType::HostRequestId => {
                format!("static_cast<jlong>({})", leaf.expr)
            }
            HostAbiType::F64 => format!("static_cast<jdouble>({})", leaf.expr),
            HostAbiType::Named(name) if android_named_type_is_enum(module, name) => {
                format!("static_cast<jint>({})", leaf.expr)
            }
            HostAbiType::U8 | HostAbiType::I8 | HostAbiType::I16 => {
                format!("static_cast<jint>({})", leaf.expr)
            }
            HostAbiType::NativeSlice(inner) => {
                if matches!(inner.as_ref(), HostAbiType::Named(_))
                    || android_cpp_primitive_slice_type(inner).is_some()
                {
                    format!("{}_value", leaf.name)
                } else {
                    panic!(
                        "unsupported Android request call argument for {:?}",
                        leaf.ty
                    );
                }
            }
            other => panic!("unsupported Android request call argument for {:?}", other),
        };

        let trailing = if index + 1 == input_leaves.len() {
            ""
        } else {
            ","
        };
        output.push_str(&format!("        {argument}{trailing}\n"));
    }
}

/// Release the local JNI inputs built for one request call.
fn render_android_cpp_release_input_locals(
    output: &mut String,
    input_leaves: &[AndroidCppLeaf],
    indent: usize,
) {
    let prefix = " ".repeat(indent);

    for leaf in input_leaves {
        match leaf.ty {
            HostAbiType::StringRef | HostAbiType::StringSlice | HostAbiType::NativeSlice(_) => {
                output.push_str(&format!(
                    "{prefix}if ({}_value != nullptr) {{ env->DeleteLocalRef({}_value); }}\n",
                    leaf.name, leaf.name
                ));
            }
            _ => {}
        }
    }
}

/// Render one output decode block.
fn render_android_cpp_output_decode(
    output: &mut String,
    module: &HostAbiModule,
    output_parameter: &AndroidCppLeaf,
    response_object_name: &str,
    indent: usize,
) {
    let prefix = " ".repeat(indent);
    let getter_name = android_cpp_getter_name(
        &output_parameter.name,
        matches!(output_parameter.ty, HostAbiType::Bool),
    );

    match &output_parameter.ty {
        HostAbiType::Bool => {
            output.push_str(&format!(
                "{prefix}*{} = call_boolean_getter(env, {}, \"{}\") == JNI_TRUE;\n",
                output_parameter.expr, response_object_name, getter_name
            ));
        }
        HostAbiType::U32 | HostAbiType::I32 => {
            output.push_str(&format!(
                "{prefix}*{} = static_cast<{}>(call_int_getter(env, {}, \"{}\"));\n",
                output_parameter.expr,
                render_cpp_type(&output_parameter.ty),
                response_object_name,
                getter_name
            ));
        }
        HostAbiType::U64 | HostAbiType::HostRequestId => {
            output.push_str(&format!(
                "{prefix}*{} = static_cast<{}>(call_long_getter(env, {}, \"{}\"));\n",
                output_parameter.expr,
                render_cpp_type(&output_parameter.ty),
                response_object_name,
                getter_name
            ));
        }
        HostAbiType::Named(_) => {
            let value_name = format!("{}_value", output_parameter.name);
            output.push_str(&format!(
                "{prefix}jobject {} = call_object_getter(env, {}, \"{}\", \"{}\");\n",
                value_name,
                response_object_name,
                getter_name,
                android_cpp_object_signature(module, &output_parameter.ty)
            ));
            output.push_str(&format!("{prefix}if ({} == nullptr) {{\n", value_name));
            output.push_str(&format!("{prefix}    status = HOST_STATUS_FAILED;\n"));
            output.push_str(&format!("{prefix}}} else {{\n"));
            output.push_str(&format!(
                "{prefix}    *{} = {};\n",
                output_parameter.expr,
                android_cpp_object_decode_expression(module, &output_parameter.ty, &value_name)
            ));
            output.push_str(&format!(
                "{prefix}    env->DeleteLocalRef({});\n",
                value_name
            ));
            output.push_str(&format!("{prefix}}}\n"));
        }
        HostAbiType::NativeArray(inner) => {
            let HostAbiType::Named(name) = inner.as_ref() else {
                panic!("unsupported Android output array element type");
            };
            let list_name = format!("{}_value", output_parameter.name);
            let storage_name = android_cpp_output_storage_name(&output_parameter.name);

            output.push_str(&format!(
                "{prefix}jobject {} = call_object_getter(env, {}, \"{}\", \"{}\" );\n",
                list_name,
                response_object_name,
                getter_name,
                android_cpp_object_signature(module, &output_parameter.ty)
            ));
            output.push_str(&format!("{prefix}if ({} == nullptr) {{\n", list_name));
            output.push_str(&format!("{prefix}    status = HOST_STATUS_FAILED;\n"));
            output.push_str(&format!("{prefix}}} else {{\n"));
            output.push_str(&format!("{prefix}    clear_result_decode_storage();\n"));
            output.push_str(&format!("{prefix}    {}.clear();\n", storage_name));
            output.push_str(&format!(
                "{prefix}    jint count = call_list_size(env, {});\n",
                list_name
            ));
            output.push_str(&format!(
                "{prefix}    {}.reserve(static_cast<size_t>(count));\n",
                storage_name
            ));
            output.push_str(&format!(
                "{prefix}    for (jint index = 0; index < count; index += 1) {{\n"
            ));
            output.push_str(&format!(
                "{prefix}        jobject element = call_list_get(env, {}, index);\n",
                list_name
            ));
            output.push_str(&format!("{prefix}        if (element == nullptr) {{\n"));
            output.push_str(&format!(
                "{prefix}            status = HOST_STATUS_FAILED;\n"
            ));
            output.push_str(&format!("{prefix}            break;\n"));
            output.push_str(&format!("{prefix}        }}\n\n"));
            output.push_str(&format!(
                "{prefix}        {}.push_back({}(env, element, &result_string_storage));\n",
                storage_name,
                android_cpp_decode_function_name(name)
            ));
            output.push_str(&format!("{prefix}        env->DeleteLocalRef(element);\n"));
            output.push_str(&format!("{prefix}    }}\n\n"));
            output.push_str(&format!("{prefix}    if (status == HOST_STATUS_OK) {{\n"));
            output.push_str(&format!(
                "{prefix}        {}->data = {}.empty() ? nullptr : {}.data();\n",
                output_parameter.expr, storage_name, storage_name
            ));
            output.push_str(&format!(
                "{prefix}        {}->len = static_cast<uint32_t>({}.size());\n",
                output_parameter.expr, storage_name
            ));
            output.push_str(&format!(
                "{prefix}        {}->capacity = static_cast<uint32_t>({}.capacity());\n",
                output_parameter.expr, storage_name
            ));
            output.push_str(&format!("{prefix}    }}\n"));
            output.push_str(&format!(
                "{prefix}    env->DeleteLocalRef({});\n",
                list_name
            ));
            output.push_str(&format!("{prefix}}}\n"));
        }
        other => panic!("unsupported Android output decode for {:?}", other),
    }
}

/// Render one generic Android runtime JNI wrapper source.
pub(super) fn render_generic_cpp_runtime_jni_wrapper_source(
    module: &HostAbiModule,
    ingress: &HostAbiFunction,
) -> String {
    let mut output = String::new();
    let decode_root_types: Vec<_> = ingress
        .parameters
        .iter()
        .filter(|parameter| !matches!(parameter.ty, HostAbiType::HostSessionHandle))
        .map(|parameter| parameter.ty.clone())
        .collect();

    let uses_runtime_strings = decode_root_types
        .iter()
        .any(|ty| android_cpp_type_uses_jni_strings(module, ty));
    let needs_runtime_decode_storage = !decode_root_types.is_empty();

    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("#include \"../../types.h\"\n");
    output.push_str("#include \"../../jni.h\"\n");
    output.push_str("#include \"runtime.generated.h\"\n\n");
    output.push_str("#include <deque>\n");
    output.push_str("#include <string>\n");
    output.push_str("#include <vector>\n\n");
    output.push_str("namespace {\n\n");

    if needs_runtime_decode_storage {
        output.push_str("std::deque<std::string> runtime_string_storage;\n\n");
    }

    if uses_runtime_strings {
        output.push_str(
            "NativeStringRef string_ref_from_java(\n    JNIEnv *env,\n    jstring value,\n    std::deque<std::string> *storage\n) {\n",
        );
        output.push_str("    if (value == nullptr) {\n");
        output.push_str("        return NativeStringRef {\n");
        output.push_str("            .data = nullptr,\n");
        output.push_str("            .len = 0,\n");
        output.push_str("        };\n");
        output.push_str("    }\n\n");
        output.push_str("    const char *chars = env->GetStringUTFChars(value, nullptr);\n");
        output.push_str("    if (chars == nullptr) {\n");
        output.push_str("        return NativeStringRef {\n");
        output.push_str("            .data = nullptr,\n");
        output.push_str("            .len = 0,\n");
        output.push_str("        };\n");
        output.push_str("    }\n\n");
        output.push_str("    jsize length = env->GetStringUTFLength(value);\n");
        output.push_str("    storage->emplace_back(chars, chars + length);\n");
        output.push_str("    env->ReleaseStringUTFChars(value, chars);\n\n");
        output.push_str("    const std::string &owned = storage->back();\n\n");
        output.push_str("    return NativeStringRef {\n");
        output.push_str("        .data = reinterpret_cast<const uint8_t *>(owned.data()),\n");
        output.push_str("        .len = static_cast<uint32_t>(owned.size()),\n");
        output.push_str("    };\n");
        output.push_str("}\n\n");
    }

    render_android_cpp_decode_helper_prototypes(&mut output, module, &decode_root_types);
    output.push('\n');
    render_android_cpp_decode_helpers(&mut output, module, &decode_root_types);
    output.push('\n');
    output.push_str("}\n\n");

    push_cpp_doc_comment(
        &mut output,
        &format!(
            "Deliver one {} into the runtime ingress path",
            ingress.documentation.trim_end_matches('.').to_lowercase()
        ),
        0,
    );
    output.push_str("extern \"C\" JNIEXPORT jlongArray JNICALL\n");
    output.push_str(&format!(
        "Java_dev_destack_runtime_android_bridge_{}_{}_{}(\n",
        module.name,
        android_kotlin_process_name(module),
        android_kotlin_native_name(ingress)
    ));
    output.push_str("    JNIEnv *env,\n");
    output.push_str("    jobject /* abi */");

    for parameter in &ingress.parameters {
        output.push_str(",\n");
        output.push_str("    ");
        output.push_str(&render_cpp_parameter_declaration(
            &render_android_cpp_jni_parameter_type(module, &parameter.ty),
            &android_kotlin_parameter_name(parameter.name),
        ));
    }

    output.push_str("\n) {\n");

    if needs_runtime_decode_storage {
        output.push_str("    runtime_string_storage.clear();\n\n");
    }

    for parameter in &ingress.parameters {
        if matches!(parameter.ty, HostAbiType::HostSessionHandle) {
            continue;
        }

        let decoded_name = format!("decoded_{}", parameter.name);
        let jni_name = android_kotlin_parameter_name(parameter.name);

        output.push_str(&format!(
            "    {} {} = {};\n",
            render_cpp_type(&parameter.ty),
            decoded_name,
            android_cpp_jni_decode_expression(module, &parameter.ty, &jni_name)
        ));
    }

    output.push_str("\n    RuntimeStatus status = ");
    output.push_str(&android_cpp_send_name(ingress));
    output.push_str("(\n");
    for (index, parameter) in ingress.parameters.iter().enumerate() {
        let trailing = if index + 1 == ingress.parameters.len() {
            ""
        } else {
            ","
        };
        output.push_str("        ");
        if matches!(parameter.ty, HostAbiType::HostSessionHandle) {
            output.push_str(&format!(
                "static_cast<uint64_t>({})",
                android_kotlin_parameter_name(parameter.name)
            ));
        } else {
            output.push_str(&format!("decoded_{}", parameter.name));
        }
        output.push_str(trailing);
        output.push('\n');
    }
    output.push_str("    );\n\n");
    output.push_str("    return runtime_status_array(env, status);\n");
    output.push_str("}\n");
    output
}

/// Return the JNI parameter type for one Android ingress parameter.
fn render_android_cpp_jni_parameter_type(_module: &HostAbiModule, ty: &HostAbiType) -> String {
    match ty {
        HostAbiType::HostSessionHandle | HostAbiType::U64 | HostAbiType::HostRequestId => {
            "jlong".to_string()
        }
        HostAbiType::U32 | HostAbiType::I32 => "jint".to_string(),
        HostAbiType::Bool => "jboolean".to_string(),
        HostAbiType::F64 => "jdouble".to_string(),
        HostAbiType::StringRef => "jstring".to_string(),
        HostAbiType::Named(_) | HostAbiType::NativeSlice(_) => "jobject".to_string(),
        other => panic!("unsupported Android JNI parameter type for {:?}", other),
    }
}

/// Return the decode expression for one JNI ingress parameter.
fn android_cpp_jni_decode_expression(
    module: &HostAbiModule,
    ty: &HostAbiType,
    value_name: &str,
) -> String {
    match ty {
        HostAbiType::U64 | HostAbiType::HostRequestId => {
            format!("static_cast<uint64_t>({value_name})")
        }
        HostAbiType::U32 => format!("static_cast<uint32_t>({value_name})"),
        HostAbiType::I32 => format!("static_cast<int32_t>({value_name})"),
        HostAbiType::Bool => format!("{value_name} == JNI_TRUE"),
        HostAbiType::F64 => format!("static_cast<double>({value_name})"),
        HostAbiType::StringRef => format!(
            "string_ref_from_java(env, static_cast<jstring>({value_name}), &runtime_string_storage)"
        ),
        HostAbiType::Named(name) if android_named_type_is_enum(module, name) => format!(
            "{}(env, {value_name})",
            android_cpp_decode_function_name(name)
        ),
        HostAbiType::Named(name) => format!(
            "{}(env, {value_name}, &runtime_string_storage)",
            android_cpp_decode_function_name(name)
        ),
        HostAbiType::NativeSlice(inner) => {
            let HostAbiType::Named(name) = inner.as_ref() else {
                panic!("unsupported Android JNI native slice element");
            };
            format!(
                "{}(env, {}, &runtime_string_storage)",
                android_cpp_decode_slice_function_name(name),
                value_name
            )
        }
        other => panic!("unsupported Android JNI decode for {:?}", other),
    }
}

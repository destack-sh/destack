use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use destack_dir::{EnumBackingType, IntType};

use crate::model::{
    BindingCatalog, BindingEntry, BindingEnumValue, BindingEnumVariant, BindingField, BindingParam,
    BindingType,
};

/// Catalog entry grouping bindings by extern name.
type BindingCatalogEntry = BTreeMap<String, BindingEntry>;

/// Named ABI types grouped by platform domain.
#[derive(Debug, Default, Clone)]
pub(crate) struct DomainAbiTypes {
    /// Newtype definitions keyed by name.
    pub newtypes: BTreeMap<String, BindingType>,
    /// Struct definitions keyed by name.
    pub structs: BTreeMap<String, BindingType>,
    /// Enum definitions keyed by name.
    pub enums: BTreeMap<String, BindingType>,
}

/// Usage flags for native stub bindings.
#[derive(Debug, Default, Clone, Copy)]
struct NativeUsage {
    /// Platform slices are referenced in the native ABI.
    uses_platform_slice: bool,
    /// Platform arrays are referenced in the native ABI.
    uses_platform_array: bool,
    /// Platform string references are referenced in the native ABI.
    uses_platform_string_ref: bool,
    /// Platform string slices are referenced in the native ABI.
    uses_platform_string_slice: bool,
}

/// Canonical binding descriptor information for rendering.
#[derive(Debug, Clone)]
struct BindingConst<'a> {
    /// Constant name for the binding descriptor.
    const_name: String,
    /// Fully qualified extern binding name.
    extern_name: &'a str,
    /// Binding metadata payload.
    entry: &'a BindingEntry,
}

/// Resolve the generated binding path for a runtime domain.
pub(crate) fn runtime_domain_bindings_path(domain: &str) -> PathBuf {
    let runtime_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let language_root = runtime_root
        .parent()
        .unwrap_or_else(|| panic!("missing language root for {runtime_root:?}"));
    language_root.join(format!(
        "runtime/src/platform/{domain}/bindings.generated.rs"
    ))
}

/// Resolve the native binding path for a runtime domain.
pub(crate) fn runtime_domain_native_path(domain: &str) -> PathBuf {
    let runtime_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let language_root = runtime_root
        .parent()
        .unwrap_or_else(|| panic!("missing language root for {runtime_root:?}"));
    language_root.join(format!("runtime/src/platform/{domain}/native.rs"))
}

/// Resolve the VM binding path for a runtime domain.
pub(crate) fn runtime_domain_vm_path(domain: &str) -> PathBuf {
    let runtime_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let language_root = runtime_root
        .parent()
        .unwrap_or_else(|| panic!("missing language root for {runtime_root:?}"));
    language_root.join(format!("runtime/src/platform/{domain}/vm.rs"))
}

/// Resolve the VM types binding path for a runtime domain.
pub(crate) fn runtime_domain_abi_types_path(domain: &str) -> PathBuf {
    let runtime_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let language_root = runtime_root
        .parent()
        .unwrap_or_else(|| panic!("missing language root for {runtime_root:?}"));
    language_root.join(format!("runtime/src/platform/{domain}/abi.generated.rs"))
}

/// Resolve the generated platform binding list path.
pub(crate) fn runtime_platform_generated_path() -> PathBuf {
    let runtime_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let language_root = runtime_root
        .parent()
        .unwrap_or_else(|| panic!("missing language root for {runtime_root:?}"));
    language_root.join("runtime/src/platform/generated.rs")
}

/// Render generated bindings for a runtime domain.
pub(crate) fn render_domain_bindings(domain: &str, bindings: &BindingCatalogEntry) -> String {
    // build a deterministic list of binding descriptors
    let consts = build_binding_consts(bindings);

    // render the generated file content
    let mut output = String::new();
    push_header(&mut output, domain, bindings);
    push_descriptor_consts(&mut output, &consts);
    push_bindings_slice(&mut output, domain, &consts);
    push_native_set(&mut output, domain, &consts);
    push_vm_register_fn(&mut output, domain, &consts);
    push_vm_set(&mut output, domain);

    output
}

/// Collect ABI types grouped by their owning domain.
pub(crate) fn collect_domain_abi_types(
    catalog: &BindingCatalog,
) -> BTreeMap<String, DomainAbiTypes> {
    let mut domains: BTreeMap<String, DomainAbiTypes> = BTreeMap::new();
    for bindings in catalog.values() {
        for entry in bindings.values() {
            collect_domain_types_for_binding(&entry.return_binding, &mut domains);
            for param in &entry.params {
                collect_domain_types_for_binding(&param.binding_type, &mut domains);
            }
        }
    }

    domains
}

/// Walk a binding type and collect any named types into the owning domain map.
fn collect_domain_types_for_binding(
    binding_type: &BindingType,
    domains: &mut BTreeMap<String, DomainAbiTypes>,
) {
    match binding_type {
        BindingType::Newtype {
            name,
            domain,
            inner,
        } => {
            let entry = domains.entry(domain.clone()).or_default();
            entry
                .newtypes
                .entry(name.clone())
                .or_insert(binding_type.clone());
            collect_domain_types_for_binding(inner, domains);
        }
        BindingType::Struct {
            name,
            domain,
            fields,
        } => {
            let entry = domains.entry(domain.clone()).or_default();
            entry
                .structs
                .entry(name.clone())
                .or_insert(binding_type.clone());
            for field in fields {
                collect_domain_types_for_binding(&field.binding_type, domains);
            }
        }
        BindingType::Enum { name, domain, .. } => {
            let entry = domains.entry(domain.clone()).or_default();
            entry
                .enums
                .entry(name.clone())
                .or_insert(binding_type.clone());
        }
        BindingType::Slice(inner) | BindingType::Array(inner) => {
            collect_domain_types_for_binding(inner, domains);
        }
        _ => {}
    }
}

/// Collect native usage flags for a binding catalog entry.
fn collect_native_usage(bindings: &BindingCatalogEntry) -> NativeUsage {
    let mut usage = NativeUsage::default();
    for entry in bindings.values() {
        collect_native_usage_for_binding(&entry.return_binding, &mut usage);
        for param in &entry.params {
            collect_native_usage_for_binding(&param.binding_type, &mut usage);
        }
    }

    usage
}

/// Record native ABI usage for a binding type.
fn collect_native_usage_for_binding(binding_type: &BindingType, usage: &mut NativeUsage) {
    match binding_type {
        BindingType::String => usage.uses_platform_string_ref = true,
        BindingType::StringSlice => usage.uses_platform_string_slice = true,
        BindingType::Slice(inner) => {
            usage.uses_platform_slice = true;
            collect_native_usage_for_binding(inner, usage);
        }
        BindingType::Array(inner) => {
            usage.uses_platform_array = true;
            collect_native_usage_for_binding(inner, usage);
        }
        BindingType::Newtype { inner, .. } => {
            collect_native_usage_for_binding(inner, usage);
        }
        BindingType::Struct { fields, .. } => {
            for field in fields {
                collect_native_usage_for_binding(&field.binding_type, usage);
            }
        }
        BindingType::Enum { .. } => {}
        _ => {}
    }
}

/// Render the generated platform binding lists.
pub(crate) fn render_platform_bindings_index(domains: &BTreeSet<String>) -> String {
    let mut output = String::new();
    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("use crate::platform::bindings::{NativeBindingSet, VmBindingSet};\n");

    if !domains.is_empty() {
        let imports = domains
            .iter()
            .map(|domain| sanitize_module_name(domain))
            .collect::<Vec<_>>()
            .join(", ");
        output.push_str(&format!("use crate::platform::{{{imports}}};\n\n"));
    } else {
        output.push_str("\n");
    }

    output.push_str("/// Native binding sets for all platform domains.\n");
    output.push_str("pub const PLATFORM_NATIVE_BINDINGS: &[NativeBindingSet] = &[\n");
    for domain in domains {
        let module = sanitize_module_name(domain);
        let set_name = native_set_name_for_domain(domain);
        output.push_str(&format!("    {module}::{set_name},\n"));
    }
    output.push_str("];\n\n");

    output.push_str("/// VM binding sets for all platform domains.\n");
    output.push_str("pub const PLATFORM_VM_BINDINGS: &[VmBindingSet] = &[\n");
    for domain in domains {
        let module = sanitize_module_name(domain);
        let set_name = vm_set_name_for_domain(domain);
        output.push_str(&format!("    {module}::{set_name},\n"));
    }
    output.push_str("];\n");

    output
}

/// Write generated bindings to the given path.
pub(crate) fn write_domain_bindings(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("failed to create output directory");
    }
    fs::write(path, contents).expect("failed to write binding catalog");
}

/// Render stub native bindings for a runtime domain.
pub(crate) fn render_native_stub(domain: &str, bindings: &BindingCatalogEntry) -> String {
    // build a deterministic list of binding descriptors
    let consts = build_binding_consts(bindings);

    // collect required imports for the native stub
    let usage = collect_native_usage(bindings);

    // collect domain-specific named types for imports
    let named_types = collect_native_named_types(domain, bindings);

    // render the stub file content
    let mut output = String::new();
    output.push_str("#![allow(clippy::missing_safety_doc)]\n");
    output.push_str("use crate::diagnostic::RuntimeError;\n");
    output.push_str("use crate::platform::{\n");
    output.push_str("    PlatformError,\n");
    if usage.uses_platform_slice {
        output.push_str("    PlatformSlice,\n");
    }
    if usage.uses_platform_array {
        output.push_str("    PlatformArray,\n");
    }
    output.push_str("    RuntimeStatus,\n");
    if usage.uses_platform_string_ref {
        output.push_str("    PlatformStringRef,\n");
    }
    if usage.uses_platform_string_slice {
        output.push_str("    PlatformStringSlice,\n");
    }
    output.push_str("};\n\n");
    if !named_types.is_empty() {
        let names = named_types.iter().cloned().collect::<Vec<_>>().join(", ");
        output.push_str(&format!("use crate::platform::{domain}::{{{names}}};\n\n"));
    }

    for binding in &consts {
        let entry = binding.entry;
        if !entry.return_is_result {
            panic!(
                "platform binding {} must return Result",
                binding.extern_name
            );
        }

        let function_name = native_fn_name(domain, binding.extern_name);
        let mut params = Vec::new();
        let mut unused = Vec::new();

        if entry.return_binding != BindingType::Void {
            let out_type = native_type_for_binding(domain, &entry.return_binding);
            params.push(format!("out: *mut {out_type}"));
            unused.push("out".to_string());
        }

        for (index, param) in entry.params.iter().enumerate() {
            let name = sanitize_param_name(&param.name, index);
            let ty = native_type_for_binding(domain, &param.binding_type);
            params.push(format!("{name}: {ty}"));
            unused.push(name);
        }

        output.push_str(&format!("/// Stub for {}.\n", binding.extern_name));
        output.push_str(&format!(
            "#[unsafe(export_name = \"{}\")]\n",
            binding.extern_name
        ));
        output.push_str(&format!(
            "pub unsafe extern \"C\" fn {function_name}({}) -> RuntimeStatus {{\n",
            params.join(", ")
        ));
        if let Some((first, rest)) = unused.split_first() {
            if rest.is_empty() {
                output.push_str(&format!("    let _ = {first};\n"));
            } else {
                output.push_str("    let _ = (");
                output.push_str(&unused.join(", "));
                output.push_str(");\n");
            }
        }
        output.push_str("    RuntimeStatus::from_error(\n");
        output.push_str(&format!(
            "        RuntimeError::platform(PlatformError::not_supported(\"{}\")).boxed(),\n",
            binding.extern_name
        ));
        output.push_str("        None,\n");
        output.push_str("    )\n");
        output.push_str("}\n\n");
    }

    output
}

/// Render stub VM bindings for a runtime domain.
pub(crate) fn render_vm_stub(domain: &str, bindings: &BindingCatalogEntry) -> String {
    // build a deterministic list of binding descriptors
    let consts = build_binding_consts(bindings);
    let vm_types = collect_vm_named_types(domain, bindings);
    let vm_usage = collect_vm_stub_usage(bindings);

    // render the stub file content
    let mut output = String::new();
    output.push_str("use destack_vm as vm;\n");
    output.push_str("use crate::diagnostic::{RuntimeError, RuntimeResult};\n");
    output.push_str("use crate::platform::PlatformError;\n");
    let mut vm_imports = Vec::new();
    if vm_usage.uses_vm_slice {
        vm_imports.push("VmSlice");
    }
    if vm_usage.uses_vm_array {
        vm_imports.push("VmArray");
    }
    if !vm_imports.is_empty() {
        output.push_str(&format!(
            "use crate::platform::{{{}}};\n",
            vm_imports.join(", ")
        ));
    }
    if !vm_types.is_empty() {
        let names = vm_types.iter().cloned().collect::<Vec<_>>().join(", ");
        output.push_str(&format!("use crate::platform::{domain}::{{{names}}};\n"));
    }
    output.push_str("use crate::runtime::RuntimeCallContext;\n\n");

    for binding in &consts {
        let entry = binding.entry;
        let method_name = vm_fn_name(domain, binding.extern_name);
        let return_type = render_return_type(domain, entry);
        let params = render_params(domain, entry);
        let mut unused = Vec::new();

        for (index, param) in entry.params.iter().enumerate() {
            unused.push(sanitize_param_name(&param.name, index));
        }

        output.push_str(&format!("/// Stub for {}.\n", binding.extern_name));
        output.push_str(&format!("pub(super) fn {method_name}(\n"));
        output.push_str("    _runtime: &RuntimeCallContext,\n");
        output.push_str("    _context: &mut vm::RuntimeContext<'_>,\n");
        for param in &params {
            output.push_str(&format!("    {param},\n"));
        }
        output.push_str(&format!(") -> RuntimeResult<{return_type}> {{\n"));
        if let Some((first, rest)) = unused.split_first() {
            if rest.is_empty() {
                output.push_str(&format!("    let _ = {first};\n"));
            } else {
                output.push_str("    let _ = (");
                output.push_str(&unused.join(", "));
                output.push_str(");\n");
            }
        }
        output.push_str("    Err(RuntimeError::platform(PlatformError::not_supported(\n");
        output.push_str(&format!(
            "        \"{} is not available in the VM yet\",\n",
            binding.extern_name
        ));
        output.push_str("    ))\n");
        output.push_str("    .boxed())\n");
        output.push_str("}\n\n");
    }

    output
}

/// Render ABI type definitions for a runtime domain.
pub(crate) fn render_abi_types(domain: &str, types: &DomainAbiTypes) -> String {
    let structs = &types.structs;
    let newtypes = &types.newtypes;
    let enums = &types.enums;
    let mut output = String::new();
    let needs_abi = structs
        .values()
        .any(|binding_type| binding_type_requires_abi(binding_type));

    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("#![allow(dead_code)]\n\n");
    if needs_abi {
        output.push_str("use crate::platform::abi::{BindingAbi, NativeAbi, VmAbi};\n\n");
    }
    if !newtypes.is_empty() {
        output.push_str("use serde::{Deserialize, Serialize};\n\n");
    }

    for (name, binding_type) in newtypes {
        let BindingType::Newtype { inner, .. } = binding_type else {
            panic!("expected newtype binding for {name}");
        };
        let inner_type = abi_newtype_inner_type(domain, inner);
        output.push_str(&format!("/// ABI newtype for {name}.\n"));
        output.push_str("#[repr(transparent)]\n");
        output.push_str(
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\n",
        );
        output.push_str(&format!("pub struct {name}(\n"));
        output.push_str(&format!("    /// Inner value.\n    pub {inner_type},\n"));
        output.push_str(");\n\n");
    }

    for (name, binding_type) in enums {
        let BindingType::Enum {
            backing, variants, ..
        } = binding_type
        else {
            panic!("expected enum binding for {name}");
        };
        let repr = enum_backing_repr(*backing);
        output.push_str(&format!("/// ABI enum for {name}.\n"));
        output.push_str(&format!("#[repr({repr})]\n"));
        output.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\n");
        output.push_str(&format!("pub enum {name} {{\n"));
        for variant in variants {
            if let BindingEnumValue::Int(value) = variant.value {
                output.push_str(&format!("    /// {0}.\n    {0} = {value},\n", variant.name));
            }
        }
        output.push_str("}\n\n");
    }

    for (struct_name, binding_type) in structs {
        let BindingType::Struct { fields, .. } = binding_type else {
            panic!("expected struct binding for {struct_name}");
        };
        let requires_abi = binding_type_requires_abi(binding_type);
        output.push_str(&format!("/// ABI struct for {struct_name}.\n"));
        output.push_str("#[repr(C)]\n");
        output.push_str("#[derive(Debug, Clone, Copy)]\n");
        if requires_abi {
            output.push_str(&format!("pub struct {struct_name}Abi<A: BindingAbi> {{\n"));
        } else {
            output.push_str(&format!("pub struct {struct_name} {{\n"));
        }
        for field in fields {
            let field_name = to_snake_case(&field.name);
            let field_type = abi_struct_field_type(domain, &field.binding_type);
            output.push_str(&format!("    /// The {field_name} field.\n"));
            output.push_str(&format!("    pub {field_name}: {field_type},\n"));
        }
        output.push_str("}\n\n");
        if requires_abi {
            output.push_str(&format!(
                "pub type {struct_name} = {struct_name}Abi<NativeAbi>;\n"
            ));
            output.push_str(&format!(
                "pub type {struct_name}Vm = {struct_name}Abi<VmAbi>;\n\n"
            ));
        } else {
            output.push_str(&format!("pub type {struct_name}Vm = {struct_name};\n\n"));
        }
    }

    output
}

/// Render stub VM bindings for a runtime domain.
/// Build a deterministic list of binding constants for a domain.
fn build_binding_consts<'a>(bindings: &'a BindingCatalogEntry) -> Vec<BindingConst<'a>> {
    // track constant names to avoid collisions
    let mut used_const_names = BTreeSet::new();
    let mut consts = Vec::new();

    // render each binding into a stable constant name
    for (extern_name, entry) in bindings {
        let mut const_name = const_name_for_extern(extern_name);
        if used_const_names.contains(&const_name) {
            let mut index = 2;
            let base = const_name.clone();
            while used_const_names.contains(&const_name) {
                const_name = format!("{base}_{index}");
                index += 1;
            }
        }

        used_const_names.insert(const_name.clone());
        consts.push(BindingConst {
            const_name,
            extern_name,
            entry,
        });
    }

    consts
}

/// Render the shared header for generated binding files.
fn push_header(output: &mut String, domain: &str, bindings: &BindingCatalogEntry) {
    let vm_usage = collect_vm_wrapper_usage(bindings);
    let named_types = collect_vm_named_types_from_params(domain, bindings);

    output.push_str("// generated by generate-bindings: do not edit\n\n");
    let needs_vm = bindings.values().any(|entry| {
        entry
            .params
            .iter()
            .any(|param| binding_type_requires_vm_for_decode(&param.binding_type))
            || binding_type_requires_vm_for_encode(&entry.return_binding)
    });
    if needs_vm {
        output.push_str("use destack_vm as vm;\n");
    }
    output.push_str("use destack_vm::Isolate;\n");
    output.push_str(
        "use crate::platform::bindings::{BindingDescriptor, BindingRegistry, NativeBinding, NativeBindingSet};\n",
    );
    output.push_str("use crate::vm_binding_set;\n");
    let needs_decode = bindings.values().any(|entry| !entry.params.is_empty());
    if needs_decode {
        output.push_str("use crate::diagnostic::RuntimeError;\n");
        output.push_str("use crate::platform::PlatformError;\n");
    }
    let mut vm_imports = Vec::new();
    if vm_usage.uses_vm_slice {
        vm_imports.push("VmSlice");
    }
    if vm_usage.uses_vm_array {
        vm_imports.push("VmArray");
    }
    if !vm_imports.is_empty() {
        output.push_str(&format!(
            "use crate::platform::{{{}}};\n",
            vm_imports.join(", ")
        ));
    }
    if !named_types.is_empty() {
        let names = named_types.iter().cloned().collect::<Vec<_>>().join(", ");
        output.push_str(&format!("use crate::platform::{domain}::{{{names}}};\n"));
    }
    output.push_str("use crate::runtime::with_runtime_call_context;\n");
    output.push_str("use crate::binding;\n\n");
    output.push_str(&format!("use crate::platform::{domain}::native;\n"));
    output.push_str(&format!(
        "use crate::platform::{domain}::vm as platform_vm;\n\n"
    ));
}

/// Render the binding descriptor constants for a domain.
fn push_descriptor_consts(output: &mut String, consts: &[BindingConst<'_>]) {
    for binding in consts {
        let signature = escape_rust_string(&binding.entry.signature);
        output.push_str(&format!(
            "/// Binding descriptor for {}.\n",
            binding.extern_name
        ));
        output.push_str(&format!(
            "pub const {}: BindingDescriptor = BindingDescriptor::recordable(\n",
            binding.const_name
        ));
        output.push_str(&format!("    \"{}\",\n", binding.extern_name));
        output.push_str(&format!("    \"{signature}\",\n"));
        output.push_str(");\n\n");
    }
}

/// Render the descriptor slice for a domain.
fn push_bindings_slice(output: &mut String, domain: &str, consts: &[BindingConst<'_>]) {
    output.push_str(&format!("/// Binding descriptors for {domain}.\n"));
    output.push_str("pub const BINDINGS: &[BindingDescriptor] = &[\n");
    for binding in consts {
        output.push_str(&format!("    {},\n", binding.const_name));
    }
    output.push_str("];\n\n");
}

/// Render the native binding set for a domain.
fn push_native_set(output: &mut String, domain: &str, consts: &[BindingConst<'_>]) {
    // compute the native binding set name
    let native_set_name = native_set_name_for_domain(domain);

    // render the binding set descriptor
    output.push_str(&format!("/// Native binding set for {domain}.\n"));
    output.push_str(&format!(
        "pub const {native_set_name}: NativeBindingSet = NativeBindingSet {{\n"
    ));
    output.push_str(&format!("    name: \"{domain}\",\n"));
    output.push_str("    bindings: &[\n");
    for binding in consts {
        let native_fn = native_fn_name(domain, binding.extern_name);
        output.push_str(&format!(
            "        NativeBinding::new({}, \"{}\", native::{native_fn} as *const ()),\n",
            binding.const_name, binding.extern_name
        ));
    }
    output.push_str("    ],\n};\n\n");
}

/// Render the binding registration function for a domain.
fn push_vm_register_fn(output: &mut String, domain: &str, consts: &[BindingConst<'_>]) {
    let register_fn = register_fn_name(domain);

    // render the registration function
    output.push_str(&format!("/// Register VM bindings for {domain}.\n"));
    output.push_str(&format!("pub fn {register_fn}(\n"));
    output.push_str("    registry: &mut BindingRegistry,\n");
    output.push_str("    isolate: &mut Isolate,\n");
    output.push_str(") {\n");
    for binding in consts {
        let method_name = vm_fn_name(domain, binding.extern_name);
        let args_ident = if binding.entry.params.is_empty() {
            "_args"
        } else {
            "args"
        };
        let binding_type = binding.entry.return_binding.clone();
        let decode = render_arg_decode(domain, binding.entry, args_ident);
        let invoke_args = render_invoke_args(binding.entry);
        let encode_lines = render_return_encode_lines(domain, &binding_type);

        // emit the binding wrapper closure
        output.push_str("    {\n");
        output.push_str(&format!(
            "        binding!(registry, isolate, {}, move |context, {args_ident}| {{\n",
            binding.const_name
        ));
        output.push_str("            with_runtime_call_context(|runtime| {\n");
        for line in &decode {
            output.push_str(&format!("                {line}\n"));
        }
        output.push_str(&format!(
            "                let result = platform_vm::{method_name}(runtime, context, {invoke_args});\n"
        ));
        for line in encode_lines {
            output.push_str(&format!("                {line}\n"));
        }
        output.push_str("            })\n");
        output.push_str("            .map_err(Into::into)\n");
        output.push_str("        });\n");
        output.push_str("    }\n");
    }
    output.push_str("}\n");
}

/// Render the VM binding set for a domain.
fn push_vm_set(output: &mut String, domain: &str) {
    let register_fn = register_fn_name(domain);
    let set_name = vm_set_name_for_domain(domain);
    let install_fn = install_fn_name(domain);

    output.push_str(&format!("/// Install VM bindings for {domain}.\n"));
    output.push_str(&format!("pub fn {install_fn}(\n"));
    output.push_str("    registry: &mut BindingRegistry,\n");
    output.push_str("    isolate: &mut Isolate,\n");
    output.push_str(") {\n");
    output.push_str(&format!("    {register_fn}(registry, isolate);\n"));
    output.push_str("}\n\n");

    output.push_str(&format!(
        "vm_binding_set!(pub {set_name}, \"{domain}\", {install_fn});\n\n"
    ));
}

/// Convert a domain into a module safe identifier.
fn sanitize_module_name(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("bindings");
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// Convert a binding name into a constant identifier.
fn const_name_for_extern(extern_name: &str) -> String {
    let tail = extern_name.rsplit('.').next().unwrap_or(extern_name);
    let mut out = String::new();
    for ch in snake_case(tail).chars() {
        out.push(ch.to_ascii_uppercase());
    }
    if out.is_empty() {
        out.push_str("BINDING");
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// Escape a string for embedding in Rust source.
fn escape_rust_string(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        for escaped in ch.escape_default() {
            out.push(escaped);
        }
    }
    out
}

/// Build the register function name for a domain.
fn register_fn_name(domain: &str) -> String {
    format!("register_{}_vm_bindings", sanitize_module_name(domain))
}

/// Build the VM binding set constant name for a domain.
fn vm_set_name_for_domain(domain: &str) -> String {
    format!("{}_VM_BINDINGS", const_name_for_extern(domain))
}

/// Build the install function name for a domain.
fn install_fn_name(domain: &str) -> String {
    format!("install_{}_vm_bindings", sanitize_module_name(domain))
}

/// Build the native binding set constant name for a domain.
fn native_set_name_for_domain(domain: &str) -> String {
    format!("{}_NATIVE_BINDINGS", const_name_for_extern(domain))
}

/// Build the handler method name for an extern binding.
/// Build the native symbol name for a binding.
fn native_fn_name(domain: &str, extern_name: &str) -> String {
    let tail = extern_name.rsplit('.').next().unwrap_or(extern_name);
    let tail = snake_case(tail);
    format!("destack_{domain}_{tail}")
}

/// Build the VM handler name for a binding.
fn vm_fn_name(domain: &str, extern_name: &str) -> String {
    native_fn_name(domain, extern_name)
}

/// Convert a string to snake case.
fn snake_case(name: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() {
                if prev_lower && !out.ends_with('_') {
                    out.push('_');
                }
                out.push(ch.to_ascii_lowercase());
                prev_lower = true;
            } else {
                out.push(ch.to_ascii_lowercase());
                prev_lower = true;
            }
        } else if !out.ends_with('_') {
            out.push('_');
            prev_lower = false;
        }
    }
    if out.is_empty() {
        out.push_str("binding");
    }
    out
}

/// Render the parameter list for a binding entry.
fn render_params(domain: &str, entry: &BindingEntry) -> Vec<String> {
    entry
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            let rust_type = vm_type_for_binding(domain, &param.binding_type);
            let name = sanitize_param_name(&param.name, index);
            format!("{name}: {rust_type}")
        })
        .collect()
}

/// Render the return type for a binding entry.
fn render_return_type(domain: &str, entry: &BindingEntry) -> String {
    vm_type_for_binding(domain, &entry.return_binding)
}

/// Build a qualified path for a named binding type.
fn named_type_path(domain: &str, type_domain: &str, name: &str) -> String {
    if type_domain == domain {
        name.to_string()
    } else {
        format!("crate::platform::{type_domain}::{name}")
    }
}

/// Build a qualified ABI struct path for a named binding type.
fn struct_abi_path(domain: &str, type_domain: &str, name: &str, abi: &str) -> String {
    if type_domain == domain {
        format!("{name}Abi<{abi}>")
    } else {
        format!("crate::platform::{type_domain}::{name}Abi<{abi}>")
    }
}

/// Build a qualified VM alias path for a struct type.
fn struct_vm_path(domain: &str, type_domain: &str, name: &str) -> String {
    let vm_name = format!("{name}Vm");
    named_type_path(domain, type_domain, vm_name.as_str())
}

/// Convert a binding type into a native ABI type.
fn native_type_for_binding(domain: &str, binding_type: &BindingType) -> String {
    match binding_type {
        BindingType::Void => "()".to_string(),
        BindingType::Bool => "bool".to_string(),
        BindingType::Int(8) => "i8".to_string(),
        BindingType::Int(16) => "i16".to_string(),
        BindingType::Int(32) => "i32".to_string(),
        BindingType::Int(64) => "i64".to_string(),
        BindingType::UInt(8) => "u8".to_string(),
        BindingType::UInt(16) => "u16".to_string(),
        BindingType::UInt(32) => "u32".to_string(),
        BindingType::UInt(64) => "u64".to_string(),
        BindingType::Float(32) => "f32".to_string(),
        BindingType::Float(64) => "f64".to_string(),
        BindingType::String => "PlatformStringRef".to_string(),
        BindingType::StringSlice => "PlatformStringSlice".to_string(),
        BindingType::Slice(inner) => {
            format!("PlatformSlice<{}>", native_type_for_binding(domain, inner))
        }
        BindingType::Array(inner) => {
            format!("PlatformArray<{}>", native_type_for_binding(domain, inner))
        }
        BindingType::Newtype {
            name,
            domain: type_domain,
            ..
        }
        | BindingType::Struct {
            name,
            domain: type_domain,
            ..
        }
        | BindingType::Enum {
            name,
            domain: type_domain,
            ..
        } => named_type_path(domain, type_domain, name),
        BindingType::Int(_) | BindingType::UInt(_) | BindingType::Float(_) => {
            panic!("unsupported numeric width for native bindings")
        }
    }
}

/// Collect named binding types referenced by bindings.
fn collect_native_named_types(domain: &str, bindings: &BindingCatalogEntry) -> BTreeSet<String> {
    // track unique names for imports
    let mut names = BTreeSet::new();

    // walk each binding signature
    for entry in bindings.values() {
        collect_signature_type_names(domain, &entry.return_binding, &mut names, "");
        for param in &entry.params {
            collect_signature_type_names(domain, &param.binding_type, &mut names, "");
        }
    }

    names
}

/// Collect named types referenced by binding parameters.
fn collect_vm_named_types_from_params(
    domain: &str,
    bindings: &BindingCatalogEntry,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for entry in bindings.values() {
        for param in &entry.params {
            collect_binding_type_names(domain, &param.binding_type, &mut names, "Vm");
        }
    }
    names
}

/// Walk a binding type and record any named types.
fn collect_binding_type_names(
    domain: &str,
    binding_type: &BindingType,
    names: &mut BTreeSet<String>,
    struct_suffix: &str,
) {
    match binding_type {
        BindingType::Slice(inner) | BindingType::Array(inner) => {
            collect_binding_type_names(domain, inner, names, struct_suffix);
        }
        BindingType::Newtype {
            name,
            domain: type_domain,
            inner,
        } => {
            if type_domain == domain {
                names.insert(name.clone());
            }
            collect_binding_type_names(domain, inner, names, struct_suffix);
        }
        BindingType::Struct {
            name,
            domain: type_domain,
            fields,
        } => {
            if type_domain == domain {
                names.insert(format!("{name}{struct_suffix}"));
            }
            for field in fields {
                collect_binding_type_names(domain, &field.binding_type, names, struct_suffix);
            }
        }
        BindingType::Enum {
            name,
            domain: type_domain,
            ..
        } => {
            if type_domain == domain {
                names.insert(name.clone());
            }
        }
        _ => {}
    }
}

/// Usage flags for VM stub imports.
#[derive(Debug, Default, Clone, Copy)]
struct VmStubUsage {
    /// VM slice types are referenced.
    uses_vm_slice: bool,
    /// VM array types are referenced.
    uses_vm_array: bool,
}

/// Collect VM stub usage flags from bindings.
fn collect_vm_stub_usage(bindings: &BindingCatalogEntry) -> VmStubUsage {
    let mut usage = VmStubUsage::default();
    for entry in bindings.values() {
        collect_vm_stub_usage_for_binding(&entry.return_binding, &mut usage);
        for param in &entry.params {
            collect_vm_stub_usage_for_binding(&param.binding_type, &mut usage);
        }
    }
    usage
}

/// Collect VM wrapper usage for binding parameter decoding.
fn collect_vm_wrapper_usage(bindings: &BindingCatalogEntry) -> VmStubUsage {
    let mut usage = VmStubUsage::default();
    for entry in bindings.values() {
        for param in &entry.params {
            collect_vm_stub_usage_for_binding(&param.binding_type, &mut usage);
        }
    }
    usage
}

/// Collect VM stub usage for a binding type.
fn collect_vm_stub_usage_for_binding(binding_type: &BindingType, usage: &mut VmStubUsage) {
    match binding_type {
        BindingType::StringSlice => usage.uses_vm_slice = true,
        BindingType::Slice(inner) => {
            usage.uses_vm_slice = true;
            collect_vm_stub_usage_for_binding(inner, usage);
        }
        BindingType::Array(inner) => {
            usage.uses_vm_array = true;
            collect_vm_stub_usage_for_binding(inner, usage);
        }
        BindingType::Newtype { inner, .. } => {
            collect_vm_stub_usage_for_binding(inner, usage);
        }
        BindingType::Struct { fields, .. } => {
            for field in fields {
                collect_vm_stub_usage_for_binding(&field.binding_type, usage);
            }
        }
        _ => {}
    }
}

/// Collect VM-visible named types.
fn collect_vm_named_types(domain: &str, bindings: &BindingCatalogEntry) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for entry in bindings.values() {
        collect_signature_type_names(domain, &entry.return_binding, &mut names, "Vm");
        for param in &entry.params {
            collect_signature_type_names(domain, &param.binding_type, &mut names, "Vm");
        }
    }
    names
}

/// Walk a binding type and record named types referenced in signatures.
fn collect_signature_type_names(
    domain: &str,
    binding_type: &BindingType,
    names: &mut BTreeSet<String>,
    struct_suffix: &str,
) {
    match binding_type {
        BindingType::Slice(inner) | BindingType::Array(inner) => {
            collect_signature_type_names(domain, inner, names, struct_suffix);
        }
        BindingType::Newtype {
            name,
            domain: type_domain,
            ..
        } => {
            if type_domain == domain {
                names.insert(name.clone());
            }
        }
        BindingType::Struct {
            name,
            domain: type_domain,
            ..
        } => {
            if type_domain == domain {
                names.insert(format!("{name}{struct_suffix}"));
            }
        }
        BindingType::Enum {
            name,
            domain: type_domain,
            ..
        } => {
            if type_domain == domain {
                names.insert(name.clone());
            }
        }
        _ => {}
    }
}

/// Return true if decoding a binding type requires the VM module.
fn binding_type_requires_vm_for_decode(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::String
        | BindingType::StringSlice
        | BindingType::Slice(_)
        | BindingType::Array(_)
        | BindingType::Struct { .. } => true,
        BindingType::Newtype { inner, .. } => binding_type_requires_vm_for_decode(inner),
        BindingType::Enum { backing, .. } => matches!(backing, EnumBackingType::String),
        _ => false,
    }
}

/// Return true if encoding a binding type requires the VM module.
fn binding_type_requires_vm_for_encode(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::Void
        | BindingType::Bool
        | BindingType::Int(_)
        | BindingType::UInt(_)
        | BindingType::Float(_) => true,
        BindingType::Newtype { inner, .. } => binding_type_requires_vm_for_encode(inner),
        BindingType::Enum { backing, .. } => matches!(backing, EnumBackingType::Int(_)),
        BindingType::Struct { fields, .. } => fields
            .iter()
            .any(|field| binding_type_requires_vm_for_encode(&field.binding_type)),
        _ => false,
    }
}

/// Return true if a binding type depends on the ABI parameter.
fn binding_type_requires_abi(binding_type: &BindingType) -> bool {
    match binding_type {
        BindingType::String
        | BindingType::StringSlice
        | BindingType::Slice(_)
        | BindingType::Array(_) => true,
        BindingType::Struct { fields, .. } => fields
            .iter()
            .any(|field| binding_type_requires_abi(&field.binding_type)),
        BindingType::Newtype { inner, .. } => binding_type_requires_abi(inner),
        _ => false,
    }
}

/// Return the Rust repr attribute for an enum backing type.
fn enum_backing_repr(backing: EnumBackingType) -> &'static str {
    match backing {
        EnumBackingType::Int(int_type) => match int_type.simplify() {
            IntType::Int8 => "i8",
            IntType::Int16 => "i16",
            IntType::Int32 => "i32",
            IntType::Int64 => "i64",
            IntType::Uint8 => "u8",
            IntType::Uint16 => "u16",
            IntType::Uint32 => "u32",
            IntType::Uint64 => "u64",
            _ => panic!("unsupported enum backing type: {int_type:?}"),
        },
        EnumBackingType::String => panic!("string-backed enums are not supported"),
    }
}

/// Render an ABI field type for a binding type.
fn abi_struct_field_type(domain: &str, binding_type: &BindingType) -> String {
    match binding_type {
        BindingType::Void => "()".to_string(),
        BindingType::Bool => "bool".to_string(),
        BindingType::Int(8) => "i8".to_string(),
        BindingType::Int(16) => "i16".to_string(),
        BindingType::Int(32) => "i32".to_string(),
        BindingType::Int(64) => "i64".to_string(),
        BindingType::Int(width) => panic!("unsupported int width for ABI type: {width}"),
        BindingType::UInt(8) => "u8".to_string(),
        BindingType::UInt(16) => "u16".to_string(),
        BindingType::UInt(32) => "u32".to_string(),
        BindingType::UInt(64) => "u64".to_string(),
        BindingType::UInt(width) => panic!("unsupported uint width for ABI type: {width}"),
        BindingType::Float(32) => "f32".to_string(),
        BindingType::Float(64) => "f64".to_string(),
        BindingType::Float(width) => panic!("unsupported float width for ABI type: {width}"),
        BindingType::String => "A::String".to_string(),
        BindingType::StringSlice => "A::StringSlice".to_string(),
        BindingType::Slice(inner) => format!("A::Slice<{}>", abi_struct_field_type(domain, inner)),
        BindingType::Array(inner) => format!("A::Array<{}>", abi_struct_field_type(domain, inner)),
        BindingType::Newtype {
            name,
            domain: type_domain,
            ..
        } => named_type_path(domain, type_domain, name),
        BindingType::Struct {
            name,
            domain: type_domain,
            ..
        } => {
            if binding_type_requires_abi(binding_type) {
                struct_abi_path(domain, type_domain, name, "A")
            } else {
                named_type_path(domain, type_domain, name)
            }
        }
        BindingType::Enum {
            name,
            domain: type_domain,
            ..
        } => named_type_path(domain, type_domain, name),
    }
}

/// Render a newtype inner ABI type.
fn abi_newtype_inner_type(domain: &str, binding_type: &BindingType) -> String {
    match binding_type {
        BindingType::Bool => "bool".to_string(),
        BindingType::Int(8) => "i8".to_string(),
        BindingType::Int(16) => "i16".to_string(),
        BindingType::Int(32) => "i32".to_string(),
        BindingType::Int(64) => "i64".to_string(),
        BindingType::UInt(8) => "u8".to_string(),
        BindingType::UInt(16) => "u16".to_string(),
        BindingType::UInt(32) => "u32".to_string(),
        BindingType::UInt(64) => "u64".to_string(),
        BindingType::Float(32) => "f32".to_string(),
        BindingType::Float(64) => "f64".to_string(),
        BindingType::Newtype {
            name,
            domain: type_domain,
            ..
        } => named_type_path(domain, type_domain, name),
        BindingType::Enum {
            name,
            domain: type_domain,
            ..
        } => named_type_path(domain, type_domain, name),
        BindingType::Struct { name, .. } => {
            panic!("struct newtypes are not supported in platform bindings: {name}");
        }
        BindingType::Void
        | BindingType::String
        | BindingType::StringSlice
        | BindingType::Slice(_)
        | BindingType::Array(_)
        | BindingType::Int(_)
        | BindingType::UInt(_)
        | BindingType::Float(_) => {
            panic!("unsupported newtype inner type for ABI generation")
        }
    }
}

/// Resolve the VM-facing type for a struct field.
/// Render the argument decoding lines for a binding entry.
fn render_arg_decode(domain: &str, entry: &BindingEntry, args_ident: &str) -> Vec<String> {
    entry
        .params
        .iter()
        .enumerate()
        .flat_map(|(index, param)| decode_arg_lines(domain, index, param, args_ident))
        .collect()
}

/// Render the argument list for invoking a handler.
fn render_invoke_args(entry: &BindingEntry) -> String {
    entry
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| sanitize_param_name(&param.name, index))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render the return encoding lines for a binding type.
fn render_return_encode_lines(domain: &str, binding_type: &BindingType) -> Vec<String> {
    if matches!(binding_type, BindingType::Void) {
        return vec!["result.map(|_| vm::Value::VOID)".to_string()];
    }

    let expr = render_encode_expr(domain, binding_type, "value");
    vec![format!("result.map(|value| {expr})")]
}

/// Render a VM value expression for an encoded binding value.
fn render_encode_expr(domain: &str, binding_type: &BindingType, value_expr: &str) -> String {
    match binding_type {
        BindingType::Void => "vm::Value::VOID".to_string(),
        BindingType::Bool => format!("vm::Value::bool({value_expr})"),
        BindingType::Int(64) => format!("vm::Value::int({value_expr}, 64)"),
        BindingType::Int(bits) => format!("vm::Value::int({value_expr} as i64, {bits})"),
        BindingType::UInt(64) => format!("vm::Value::uint({value_expr}, 64)"),
        BindingType::UInt(bits) => format!("vm::Value::uint({value_expr} as u64, {bits})"),
        BindingType::Float(32) => format!("vm::Value::float32({value_expr})"),
        BindingType::Float(64) => format!("vm::Value::float64({value_expr})"),
        BindingType::Float(width) => panic!("unsupported float width for VM binding: {width}"),
        BindingType::String => format!("{value_expr}.value()"),
        BindingType::StringSlice | BindingType::Slice(_) | BindingType::Array(_) => {
            format!("{value_expr}.to_value(context)")
        }
        BindingType::Newtype { inner, .. } => {
            let inner_expr = format!("{value_expr}.0");
            render_encode_expr(domain, inner, inner_expr.as_str())
        }
        BindingType::Enum {
            name,
            domain: enum_domain,
            backing,
            variants,
        } => match backing {
            EnumBackingType::Int(_) => render_encode_expr(
                domain,
                &enum_backing_binding_type(*backing),
                &format!("{value_expr} as {}", enum_backing_rust_type(*backing)),
            ),
            EnumBackingType::String => {
                let enum_path = named_type_path(domain, enum_domain, name);
                let mut arms = Vec::new();
                for variant in variants {
                    if let BindingEnumValue::String(value) = &variant.value {
                        arms.push(format!(
                            "{enum_path}::{} => context.intern_string(\"{value}\")",
                            variant.name
                        ));
                    }
                }
                format!(
                    "match {value_expr} {{ {} , _ => context.intern_string(\"\"), }}",
                    arms.join(", ")
                )
            }
        },
        BindingType::Struct { fields, .. } => {
            let mut encoded_fields = Vec::new();
            for field in fields {
                let field_name = to_snake_case(&field.name);
                let field_expr = format!("{value_expr}.{field_name}");
                encoded_fields.push(render_encode_expr(domain, &field.binding_type, &field_expr));
            }
            format!(
                "context.allocate_aggregate(vec![{}])",
                encoded_fields.join(", ")
            )
        }
    }
}

/// Convert a binding type into a VM-facing Rust type.
fn vm_type_for_binding(domain: &str, binding_type: &BindingType) -> String {
    match binding_type {
        BindingType::Void => "()".to_string(),
        BindingType::Bool => "bool".to_string(),
        BindingType::Int(8) => "i8".to_string(),
        BindingType::Int(16) => "i16".to_string(),
        BindingType::Int(32) => "i32".to_string(),
        BindingType::Int(64) => "i64".to_string(),
        BindingType::Int(width) => panic!("unsupported int width for VM binding: {width}"),
        BindingType::UInt(8) => "u8".to_string(),
        BindingType::UInt(16) => "u16".to_string(),
        BindingType::UInt(32) => "u32".to_string(),
        BindingType::UInt(64) => "u64".to_string(),
        BindingType::UInt(width) => panic!("unsupported uint width for VM binding: {width}"),
        BindingType::Float(32) => "f32".to_string(),
        BindingType::Float(64) => "f64".to_string(),
        BindingType::Float(width) => panic!("unsupported float width for VM binding: {width}"),
        BindingType::String => "vm::StringHandle".to_string(),
        BindingType::StringSlice => "VmSlice<vm::StringHandle>".to_string(),
        BindingType::Slice(inner) => format!("VmSlice<{}>", vm_type_for_binding(domain, inner)),
        BindingType::Array(inner) => format!("VmArray<{}>", vm_type_for_binding(domain, inner)),
        BindingType::Newtype {
            name,
            domain: type_domain,
            ..
        } => named_type_path(domain, type_domain, name),
        BindingType::Struct {
            name,
            domain: type_domain,
            ..
        } => struct_vm_path(domain, type_domain, name),
        BindingType::Enum {
            name,
            domain: type_domain,
            ..
        } => named_type_path(domain, type_domain, name),
    }
}

/// Decode binding arguments into typed locals.
fn decode_arg_lines(
    domain: &str,
    index: usize,
    param: &BindingParam,
    args_ident: &str,
) -> Vec<String> {
    let binding_type = &param.binding_type;
    let name = sanitize_param_name(&param.name, index);
    let expected = param
        .type_text
        .clone()
        .unwrap_or_else(|| "value".to_string());
    let mut lines = Vec::new();

    let arg_fetch = if index == 0 {
        format!("{args_ident}.first()")
    } else {
        format!("{args_ident}.get({index})")
    };
    lines.push(format!(
        "let {name}_value = *{arg_fetch}.ok_or_else(|| RuntimeError::platform(PlatformError::invalid_argument_type(\"{name}\", \"{expected}\")).boxed())?;"
    ));
    lines.extend(render_decode_value_lines(
        domain,
        binding_type,
        &name,
        &format!("{name}_value"),
        &expected,
    ));

    lines
}

/// Decode a binding value into a local variable.
fn render_decode_value_lines(
    domain: &str,
    binding_type: &BindingType,
    name: &str,
    value_expr: &str,
    expected: &str,
) -> Vec<String> {
    match binding_type {
        BindingType::Bool => vec![format!(
            "let {name} = {value_expr}.as_bool().ok_or_else(|| RuntimeError::platform(PlatformError::invalid_argument_type(\"{name}\", \"{expected}\")).boxed())?;"
        )],
        BindingType::Int(bits) if matches!(bits, 8 | 16 | 32 | 64) => {
            let mut lines = Vec::new();
            lines.push(format!(
                "let ({name}, width) = {value_expr}.as_int_with_width().ok_or_else(|| RuntimeError::platform(PlatformError::invalid_argument_type(\"{name}\", \"{expected}\")).boxed())?;"
            ));
            lines.push(format!(
                "if width != {bits} {{ return Err(RuntimeError::platform(PlatformError::invalid_argument_type(\"{name}\", \"{expected}\")).boxed()); }}"
            ));
            if *bits != 64 {
                lines.push(format!("let {name} = {name} as i{};", bits));
            }
            lines
        }
        BindingType::UInt(bits) if matches!(bits, 8 | 16 | 32 | 64) => {
            let mut lines = Vec::new();
            lines.push(format!(
                "let ({name}, width) = {value_expr}.as_uint_with_width().ok_or_else(|| RuntimeError::platform(PlatformError::invalid_argument_type(\"{name}\", \"{expected}\")).boxed())?;"
            ));
            lines.push(format!(
                "if width != {bits} {{ return Err(RuntimeError::platform(PlatformError::invalid_argument_type(\"{name}\", \"{expected}\")).boxed()); }}"
            ));
            if *bits != 64 {
                lines.push(format!("let {name} = {name} as u{};", bits));
            }
            lines
        }
        BindingType::Float(32) => vec![format!(
            "let {name} = {value_expr}.as_float32().ok_or_else(|| RuntimeError::platform(PlatformError::invalid_argument_type(\"{name}\", \"{expected}\")).boxed())?;"
        )],
        BindingType::Float(64) => vec![format!(
            "let {name} = {value_expr}.as_float64().ok_or_else(|| RuntimeError::platform(PlatformError::invalid_argument_type(\"{name}\", \"{expected}\")).boxed())?;"
        )],
        BindingType::String => vec![
            format!(
                "if {value_expr}.tag() != vm::ValueTag::String {{ return Err(RuntimeError::platform(PlatformError::invalid_argument_type(\"{name}\", \"{expected}\")).boxed()); }}"
            ),
            format!("let {name} = vm::StringHandle::new({value_expr});"),
        ],
        BindingType::StringSlice => vec![format!(
            "let {name} = VmSlice::<vm::StringHandle>::from_value(context, {value_expr}, \"{name}\", \"{expected}\")?;"
        )],
        BindingType::Slice(inner) => {
            let inner_type = vm_type_for_binding(domain, inner);
            vec![format!(
                "let {name} = VmSlice::<{inner_type}>::from_value(context, {value_expr}, \"{name}\", \"{expected}\")?;"
            )]
        }
        BindingType::Array(inner) => {
            let inner_type = vm_type_for_binding(domain, inner);
            vec![format!(
                "let {name} = VmArray::<{inner_type}>::from_value(context, {value_expr}, \"{name}\", \"{expected}\")?;"
            )]
        }
        BindingType::Newtype {
            name: type_name,
            domain: type_domain,
            inner,
        } => {
            let inner_name = format!("{name}_inner");
            let mut lines =
                render_decode_value_lines(domain, inner, &inner_name, value_expr, expected);
            let type_path = named_type_path(domain, type_domain, type_name);
            lines.push(format!("let {name} = {type_path}({inner_name});"));
            lines
        }
        BindingType::Enum {
            name: enum_name,
            domain: enum_domain,
            backing,
            variants,
        } => {
            let enum_path = named_type_path(domain, enum_domain, enum_name);
            render_decode_enum_lines(
                domain,
                name,
                value_expr,
                expected,
                enum_path.as_str(),
                *backing,
                variants,
            )
        }
        BindingType::Struct {
            name: struct_name,
            domain: struct_domain,
            fields,
        } => render_decode_struct_lines(
            domain,
            name,
            value_expr,
            expected,
            struct_name,
            struct_domain,
            fields,
        ),
        BindingType::Void => Vec::new(),
        _ => Vec::new(),
    }
}

/// Render enum decoding lines.
fn render_decode_enum_lines(
    domain: &str,
    name: &str,
    value_expr: &str,
    expected: &str,
    enum_name: &str,
    backing: EnumBackingType,
    variants: &[BindingEnumVariant],
) -> Vec<String> {
    let mut lines = Vec::new();
    let raw_name = format!("{name}_raw");
    let backing_type = enum_backing_binding_type(backing);
    lines.extend(render_decode_value_lines(
        domain,
        &backing_type,
        &raw_name,
        value_expr,
        expected,
    ));

    let match_expr = match backing {
        EnumBackingType::Int(int_type) => {
            let is_signed = int_type.is_signed();
            let mut arms = Vec::new();
            for variant in variants {
                if let BindingEnumValue::Int(value) = variant.value {
                    let literal = if is_signed {
                        format!("{value}")
                    } else {
                        format!("{value}u64")
                    };
                    arms.push(format!("{literal} => {enum_name}::{}", variant.name));
                }
            }
            format!(
                "match {raw_name} {{ {} , _ => return Err(RuntimeError::platform(PlatformError::invalid_argument_value(\"{name}\", \"unknown {enum_name} value\")).boxed()), }}",
                arms.join(", ")
            )
        }
        EnumBackingType::String => {
            lines.push(format!(
                "let {raw_name}_ref = context.string_ref({raw_name}).map_err(|error| RuntimeError::vm(error).boxed())?;"
            ));
            lines.push(format!("let {raw_name} = {raw_name}_ref.as_str();"));
            let mut arms = Vec::new();
            for variant in variants {
                if let BindingEnumValue::String(value) = &variant.value {
                    arms.push(format!("\"{value}\" => {enum_name}::{}", variant.name));
                }
            }
            format!(
                "match {raw_name} {{ {} , _ => return Err(RuntimeError::platform(PlatformError::invalid_argument_value(\"{name}\", \"unknown {enum_name} value\")).boxed()), }}",
                arms.join(", ")
            )
        }
    };
    lines.push(format!("let {name} = {match_expr};"));
    lines
}

/// Render struct decoding lines.
fn render_decode_struct_lines(
    domain: &str,
    name: &str,
    value_expr: &str,
    expected: &str,
    struct_name: &str,
    struct_domain: &str,
    fields: &[BindingField],
) -> Vec<String> {
    let mut lines = Vec::new();
    let struct_type = struct_vm_path(domain, struct_domain, struct_name);
    lines.push(format!("let {name} = {{"));
    lines.push(format!(
        "    if {value_expr}.tag() != vm::ValueTag::Aggregate {{ return Err(RuntimeError::platform(PlatformError::invalid_argument_type(\"{name}\", \"{expected}\")).boxed()); }}"
    ));
    lines.push(format!(
        "    let slots = context.aggregate_slots({value_expr}).map_err(|error| RuntimeError::vm(error).boxed())?;"
    ));
    lines.push(format!(
        "    if slots.len() != {} {{ return Err(RuntimeError::platform(PlatformError::invalid_argument_value(\"{name}\", \"expected {} fields\")).boxed()); }}",
        fields.len(),
        fields.len()
    ));

    let mut field_names = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        let field_name = to_snake_case(&field.name);
        let local_name = format!("{name}_{field_name}");
        field_names.push((field_name, local_name.clone()));
        let field_lines = render_decode_value_lines(
            domain,
            &field.binding_type,
            &local_name,
            &format!("slots[{index}]"),
            field.name.as_str(),
        );
        for line in field_lines {
            lines.push(format!("    {line}"));
        }
    }

    lines.push(format!("    {struct_type} {{"));
    for (field_name, local_name) in field_names {
        lines.push(format!("        {field_name}: {local_name},"));
    }
    lines.push("    }".to_string());
    lines.push("};".to_string());
    lines
}

/// Convert enum backing types into binding types.
fn enum_backing_binding_type(backing: EnumBackingType) -> BindingType {
    match backing {
        EnumBackingType::Int(int_type) => match int_type.simplify() {
            IntType::Int8 => BindingType::Int(8),
            IntType::Int16 => BindingType::Int(16),
            IntType::Int32 => BindingType::Int(32),
            IntType::Int64 => BindingType::Int(64),
            IntType::Uint8 => BindingType::UInt(8),
            IntType::Uint16 => BindingType::UInt(16),
            IntType::Uint32 => BindingType::UInt(32),
            IntType::Uint64 => BindingType::UInt(64),
            _ => panic!("unsupported enum backing width: {int_type:?}"),
        },
        EnumBackingType::String => BindingType::String,
    }
}

/// Convert enum backing types into a Rust primitive name.
fn enum_backing_rust_type(backing: EnumBackingType) -> &'static str {
    match backing {
        EnumBackingType::Int(int_type) => match int_type.simplify() {
            IntType::Int8 => "i8",
            IntType::Int16 => "i16",
            IntType::Int32 => "i32",
            IntType::Int64 => "i64",
            IntType::Uint8 => "u8",
            IntType::Uint16 => "u16",
            IntType::Uint32 => "u32",
            IntType::Uint64 => "u64",
            _ => panic!("unsupported enum backing width: {int_type:?}"),
        },
        EnumBackingType::String => "&str",
    }
}

/// Convert a field name to snake case.
fn to_snake_case(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_uppercase() {
            if !out.is_empty() {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// Normalize a parameter name into a safe identifier.
fn sanitize_param_name(name: &str, index: usize) -> String {
    let stripped = name.trim().trim_start_matches("...");
    let mut out = String::new();
    for ch in stripped.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str(&format!("arg{index}"));
    }
    if out.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

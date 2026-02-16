use super::*;

/// Render documentation comments for one generated stub implementation.
fn write_stub_docs(
    output: &mut String,
    entry: &BindingEntry,
    extern_name: &str,
    is_simulated: bool,
) {
    if let Some(documentation) = entry.documentation.as_deref() {
        for line in documentation.lines() {
            if line.trim().is_empty() {
                output.push_str("///\n");
            } else {
                output.push_str(&format!("/// {line}\n"));
            }
        }
    } else if is_simulated {
        output.push_str(&format!("/// Simulated binding for `{extern_name}`.\n"));
    } else {
        output.push_str(&format!("/// Binding for `{extern_name}`.\n"));
    }
}

/// Render stub native bindings for a runtime domain.
pub(crate) fn render_native_stub(domain: &str, bindings: &BindingCatalogEntry) -> String {
    render_native_like_stub(domain, bindings, "pub(crate)")
}

/// Render stub host bindings for a runtime domain.
pub(crate) fn render_host_stub(domain: &str, bindings: &BindingCatalogEntry) -> String {
    render_native_like_stub(domain, bindings, "pub(crate)")
}

/// Render stub host-like bindings for a runtime domain.
fn render_native_like_stub(
    domain: &str,
    bindings: &BindingCatalogEntry,
    function_visibility: &str,
) -> String {
    // build a deterministic list of binding descriptors
    let consts = build_binding_consts(domain, bindings);

    // collect required imports for the native stub
    let usage = collect_native_usage(bindings);

    // collect domain-specific named types for imports
    let mut named_types = collect_native_named_types(domain, bindings);
    let type_domains = collect_type_domains(domain, bindings);
    if domain == "error" {
        named_types.remove("PlatformError");
    }

    // render the stub file content
    let mut output = String::new();
    output.push_str("#![allow(dead_code)]\n");
    output.push_str("#![allow(unused_imports)]\n");
    output.push_str("#![allow(clippy::missing_safety_doc)]\n");
    output.push_str("use crate::diagnostic::{RuntimeError, RuntimeResult};\n");
    output.push_str(&format!(
        "use crate::platform::{domain}::bindings_generated as bindings;\n"
    ));
    output.push_str("use crate::platform::{\n");
    output.push_str("    PlatformError,\n");
    if usage.uses_platform_slice {
        output.push_str("    NativeSlice,\n");
    }
    if usage.uses_platform_array {
        output.push_str("    NativeArray,\n");
    }
    if usage.uses_platform_string_ref {
        output.push_str("    NativeStringRef,\n");
    }
    if usage.uses_platform_string_slice {
        output.push_str("    NativeStringSlice,\n");
    }
    output.push_str("};\n\n");
    output.push_str("use crate::runtime::RuntimeCallContext;\n");
    output.push_str("use bindings::*;\n\n");
    if !type_domains.is_empty() {
        let imports = type_domains
            .iter()
            .map(|domain| domain.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        output.push_str(&format!("use crate::platform::{{{imports}}};\n"));
    }
    if !named_types.is_empty() {
        let names = named_types.iter().cloned().collect::<Vec<_>>().join(", ");
        output.push_str(&format!("use crate::platform::{domain}::{{{names}}};\n\n"));
    }
    if domain == "error" {
        output.push_str("use crate::platform::error as platform_error;\n\n");
    }
    output.push('\n');

    for binding in &consts {
        let entry = binding.entry;
        if !entry.return_is_result {
            panic!(
                "platform binding {} must return Result",
                binding.extern_name
            );
        }

        let function_name = binding.implementation_fn_name.clone();
        let mut params = Vec::new();
        let mut unused = Vec::new();

        if entry.return_binding != BindingType::Void {
            let out_type = native_type_for_binding(domain, &entry.return_binding);
            params.push(format!("out: *mut {out_type}"));
            unused.push("out".to_string());
        }

        for (index, param) in entry.parameters.iter().enumerate() {
            let name = sanitize_param_name(&param.name, index);
            let ty = native_type_for_binding(domain, &param.binding_type);
            params.push(format!("{name}: {ty}"));
            unused.push(name);
        }

        write_stub_docs(&mut output, entry, binding.extern_name, false);
        output.push_str(&format!(
            "{function_visibility} unsafe fn {function_name}(context: &RuntimeCallContext{}) -> RuntimeResult<()> {{\n",
            if params.is_empty() {
                String::new()
            } else {
                format!(", {}", params.join(", "))
            }
        ));
        if entry.return_binding != BindingType::Void {
            output.push_str("    if out.is_null() {\n");
            output.push_str(
                "        return Err(RuntimeError::from(PlatformError::null_pointer(\"out\")).boxed());\n",
            );
            output.push_str("    }\n");
        }
        if let Some((first, rest)) = unused.split_first() {
            if rest.is_empty() {
                output.push_str(&format!("    let _ = {first};\n"));
            } else {
                output.push_str("    let _ = (");
                output.push_str(&unused.join(", "));
                output.push_str(");\n");
            }
        }
        output.push('\n');
        output.push_str("    Err(RuntimeError::from(PlatformError::not_supported(\n");
        output.push_str(&format!("        \"{}\",\n", binding.extern_name));
        output.push_str("    ))\n");
        output.push_str("    .boxed())\n");
        output.push_str("}\n\n");
    }

    output
}

/// Render stub VM bindings for a runtime domain.
pub(crate) fn render_vm_stub(domain: &str, bindings: &BindingCatalogEntry) -> String {
    // build a deterministic list of binding descriptors
    let consts = build_binding_consts(domain, bindings);
    let vm_types = collect_vm_named_types(domain, bindings);
    let type_domains = collect_type_domains(domain, bindings);
    let vm_usage = collect_vm_stub_usage(bindings);

    // render the stub file content
    let mut output = String::new();
    output.push_str("#![allow(dead_code)]\n");
    output.push_str("#![allow(unused_imports)]\n");
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
    if !type_domains.is_empty() {
        let imports = type_domains
            .iter()
            .map(|domain| domain.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        output.push_str(&format!("use crate::platform::{{{imports}}};\n"));
    }
    if !vm_types.is_empty() {
        let names = vm_types.iter().cloned().collect::<Vec<_>>().join(", ");
        output.push_str(&format!("use crate::platform::{domain}::{{{names}}};\n"));
    }
    output.push_str("use crate::runtime::RuntimeCallContext;\n\n");

    for binding in &consts {
        let entry = binding.entry;
        let method_name = binding.implementation_fn_name.clone();
        let return_type = render_return_type(domain, entry);
        let params = render_params(domain, entry);
        let mut unused = Vec::new();

        for (index, param) in entry.parameters.iter().enumerate() {
            unused.push(sanitize_param_name(&param.name, index));
        }

        write_stub_docs(&mut output, entry, binding.extern_name, false);
        output.push_str(&format!("pub(crate) fn {method_name}(\n"));
        output.push_str("    _runtime: &RuntimeCallContext,\n");
        output.push_str("    _context: &mut vm::ExternalCallContext<'_>,\n");
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
        output.push_str("    Err(RuntimeError::from(PlatformError::not_supported(\n");
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

/// Render stub simulated native bindings for a runtime domain.
pub(crate) fn render_simulated_native_stub(domain: &str, bindings: &BindingCatalogEntry) -> String {
    // build a deterministic list of binding descriptors
    let consts = build_binding_consts(domain, bindings);

    // collect required imports for the simulated native stub
    let usage = collect_native_usage(bindings);

    // collect domain-specific named types for imports
    let named_types = collect_native_stub_named_types(domain, bindings);
    let type_domains = collect_type_domains(domain, bindings);

    // render the stub file content
    let mut output = String::new();
    output.push_str("#![allow(dead_code)]\n");
    output.push_str("#![allow(unused_imports)]\n");
    output.push_str("#![allow(clippy::missing_safety_doc)]\n");
    output.push_str("use crate::diagnostic::{RuntimeError, RuntimeResult};\n");
    output.push_str("use crate::platform::PlatformError;\n");
    output.push_str("use crate::platform::{\n");
    if usage.uses_platform_slice {
        output.push_str("    NativeSlice,\n");
    }
    if usage.uses_platform_array {
        output.push_str("    NativeArray,\n");
    }
    if usage.uses_platform_string_ref {
        output.push_str("    NativeStringRef,\n");
    }
    if usage.uses_platform_string_slice {
        output.push_str("    NativeStringSlice,\n");
    }
    output.push_str("};\n\n");
    output.push_str("use crate::runtime::RuntimeCallContext;\n\n");
    if !type_domains.is_empty() {
        let imports = type_domains
            .iter()
            .map(|domain| domain.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        output.push_str(&format!("use crate::platform::{{{imports}}};\n"));
    }
    if !named_types.is_empty() {
        let names = named_types.iter().cloned().collect::<Vec<_>>().join(", ");
        output.push_str(&format!("use crate::platform::{domain}::{{{names}}};\n\n"));
    }
    if domain == "error" {
        output.push_str("use crate::platform::error as platform_error;\n\n");
    }
    output.push('\n');

    for binding in &consts {
        let entry = binding.entry;
        if !entry.return_is_result {
            panic!(
                "platform binding {} must return Result",
                binding.extern_name
            );
        }

        let function_name = binding.implementation_fn_name.clone();
        let mut params = Vec::new();
        let mut unused = Vec::new();

        if entry.return_binding != BindingType::Void {
            let out_type = native_type_for_binding(domain, &entry.return_binding);
            params.push(format!("out: *mut {out_type}"));
            unused.push("out".to_string());
        }

        for (index, param) in entry.parameters.iter().enumerate() {
            let name = sanitize_param_name(&param.name, index);
            let ty = native_type_for_binding(domain, &param.binding_type);
            params.push(format!("{name}: {ty}"));
            unused.push(name);
        }

        write_stub_docs(&mut output, entry, binding.extern_name, true);
        output.push_str(&format!(
            "pub(crate) unsafe fn {function_name}(context: &RuntimeCallContext{}) -> RuntimeResult<()> {{\n",
            if params.is_empty() {
                String::new()
            } else {
                format!(", {}", params.join(", "))
            }
        ));
        output.push_str("    let _ = context;\n");
        if let Some((first, rest)) = unused.split_first() {
            if rest.is_empty() {
                output.push_str(&format!("    let _ = {first};\n"));
            } else {
                output.push_str("    let _ = (");
                output.push_str(&unused.join(", "));
                output.push_str(");\n");
            }
        }
        output.push('\n');
        output.push_str("    Err(RuntimeError::from(PlatformError::not_supported(\n");
        output.push_str(&format!("        \"{}\",\n", binding.extern_name));
        output.push_str("    ))\n");
        output.push_str("    .boxed())\n");
        output.push_str("}\n\n");
    }

    output
}

/// Render stub simulated VM bindings for a runtime domain.
pub(crate) fn render_simulated_vm_stub(domain: &str, bindings: &BindingCatalogEntry) -> String {
    // build a deterministic list of binding descriptors
    let consts = build_binding_consts(domain, bindings);
    let vm_types = collect_vm_stub_named_types(domain, bindings);
    let type_domains = collect_type_domains(domain, bindings);
    let vm_usage = collect_vm_stub_usage(bindings);

    // render the stub file content
    let mut output = String::new();
    output.push_str("#![allow(dead_code)]\n");
    output.push_str("#![allow(unused_imports)]\n");
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
    if !type_domains.is_empty() {
        let imports = type_domains
            .iter()
            .map(|domain| domain.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        output.push_str(&format!("use crate::platform::{{{imports}}};\n"));
    }
    if !vm_types.is_empty() {
        let names = vm_types.iter().cloned().collect::<Vec<_>>().join(", ");
        output.push_str(&format!("use crate::platform::{domain}::{{{names}}};\n"));
    }
    output.push_str("use crate::runtime::RuntimeCallContext;\n\n");

    for binding in &consts {
        let entry = binding.entry;
        let method_name = binding.implementation_fn_name.clone();
        let return_type = render_return_type(domain, entry);
        let params = render_params(domain, entry);
        let mut unused = Vec::new();

        for (index, param) in entry.parameters.iter().enumerate() {
            unused.push(sanitize_param_name(&param.name, index));
        }

        write_stub_docs(&mut output, entry, binding.extern_name, true);
        output.push_str(&format!("pub(crate) fn {method_name}(\n"));
        output.push_str("    _runtime: &RuntimeCallContext,\n");
        output.push_str("    _context: &mut vm::ExternalCallContext<'_>,\n");
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
        output.push_str("    Err(RuntimeError::from(PlatformError::not_supported(\n");
        output.push_str(&format!("        \"{}\",\n", binding.extern_name));
        output.push_str("    ))\n");
        output.push_str("    .boxed())\n");
        output.push_str("}\n\n");
    }

    output
}

/// Render a simulated module re-export stub for a runtime domain.
pub(crate) fn render_simulated_mod_stub() -> String {
    let mut output = String::new();
    output.push_str("pub(crate) mod native;\n");
    output.push_str("pub(crate) mod vm;\n");
    output
}

/// Render a top-level module stub for a runtime domain.
pub(crate) fn render_domain_mod_stub(
    has_world_dispatch: bool,
    has_runtime_dispatch: bool,
) -> String {
    let mut output = String::new();
    output.push_str("#[path = \"abi.generated.rs\"]\n");
    output.push_str("mod abi_generated;\n");
    output.push_str("#[path = \"bindings.generated.rs\"]\n");
    output.push_str("mod bindings_generated;\n\n");
    output.push_str("#[allow(unused_imports, unreachable_pub)]\n");
    output.push_str("pub use abi_generated::*;\n");
    output.push_str("#[allow(unused_imports, unreachable_pub)]\n");
    output.push_str("pub use bindings_generated::*;\n");
    if has_world_dispatch {
        output.push_str("mod host;\n");
    }
    output.push_str("pub mod native;\n");
    if has_runtime_dispatch {
        output.push_str("pub(crate) mod runtime;\n");
    }
    if has_world_dispatch {
        output.push_str("pub(crate) mod simulated;\n");
    }
    output.push_str("pub mod vm;\n");
    output
}

/// Render a host router stub for world-dispatched domains.
pub(crate) fn render_host_router_stub() -> String {
    let mut output = String::new();
    output.push_str("#[cfg(unix)]\n");
    output.push_str("#[path = \"unix/mod.rs\"]\n");
    output.push_str("mod unix;\n");
    output.push_str("#[cfg(unix)]\n");
    output.push_str("#[allow(unused_imports)]\n");
    output.push_str("pub(crate) use unix::*;\n\n");
    output.push_str("#[cfg(windows)]\n");
    output.push_str("#[path = \"windows/mod.rs\"]\n");
    output.push_str("mod windows;\n");
    output.push_str("#[cfg(windows)]\n");
    output.push_str("#[allow(unused_imports)]\n");
    output.push_str("pub(crate) use windows::*;\n\n");
    output.push_str("#[cfg(not(any(unix, windows)))]\n");
    output.push_str("#[path = \"unsupported.rs\"]\n");
    output.push_str("mod unsupported;\n");
    output.push_str("#[cfg(not(any(unix, windows)))]\n");
    output.push_str("#[allow(unused_imports)]\n");
    output.push_str("pub(crate) use unsupported::*;\n");
    output
}

/// Render a unix or windows backend shim for world-dispatched domains.
pub(crate) fn render_os_backend_mod_stub() -> String {
    let mut output = String::new();
    output.push_str("#[path = \"../unsupported.rs\"]\n");
    output.push_str("mod unsupported;\n\n");
    output.push_str("#[allow(unused_imports)]\n");
    output.push_str("pub(crate) use unsupported::*;\n");
    output
}

/// Render a runtime module re-export stub for a runtime domain.
pub(crate) fn render_runtime_mod_stub() -> String {
    let mut output = String::new();
    output.push_str("pub(crate) mod native;\n");
    output.push_str("pub(crate) mod vm;\n");
    output
}

/// Collect runtime-scope bindings from one domain catalog.
fn collect_runtime_bindings(bindings: &BindingCatalogEntry) -> BindingCatalogEntry {
    let mut runtime_bindings = BindingCatalogEntry::new();

    for (extern_name, entry) in bindings {
        if entry.scope == BindingScope::Runtime {
            runtime_bindings.insert(extern_name.clone(), entry.clone());
        }
    }

    runtime_bindings
}

/// Render a runtime native forwarding stub for a runtime domain.
pub(crate) fn render_runtime_native_stub(domain: &str, bindings: &BindingCatalogEntry) -> String {
    let runtime_bindings = collect_runtime_bindings(bindings);
    let consts = build_binding_consts(domain, &runtime_bindings);
    let usage = collect_native_usage(&runtime_bindings);
    let named_types = collect_native_named_types(domain, &runtime_bindings);
    let type_domains = collect_type_domains(domain, &runtime_bindings);
    let native_alias = format!("{domain}_native");

    let mut output = String::new();
    output.push_str("use crate::diagnostic::RuntimeResult;\n");
    output.push_str("use crate::platform::{\n");
    if usage.uses_platform_slice {
        output.push_str("    NativeSlice,\n");
    }
    if usage.uses_platform_array {
        output.push_str("    NativeArray,\n");
    }
    if usage.uses_platform_string_ref {
        output.push_str("    NativeStringRef,\n");
    }
    if usage.uses_platform_string_slice {
        output.push_str("    NativeStringSlice,\n");
    }
    output.push_str("};\n");
    output.push_str("use crate::runtime::RuntimeCallContext;\n\n");
    if !type_domains.is_empty() {
        let imports = type_domains
            .iter()
            .map(|domain| domain.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        output.push_str(&format!("use crate::platform::{{{imports}}};\n"));
    }
    if !named_types.is_empty() {
        let names = named_types.iter().cloned().collect::<Vec<_>>().join(", ");
        output.push_str(&format!("use crate::platform::{domain}::{{{names}}};\n"));
    }
    output.push_str(&format!(
        "use crate::platform::{domain}::native as {native_alias};\n\n"
    ));

    for binding in &consts {
        let entry = binding.entry;
        if entry.scope != BindingScope::Runtime {
            continue;
        }
        if !entry.return_is_result {
            panic!(
                "platform binding {} must return Result",
                binding.extern_name
            );
        }

        let function_name = binding.implementation_fn_name.clone();
        let mut params = Vec::new();
        let mut call_args = vec!["context".to_string()];

        if entry.return_binding != BindingType::Void {
            let out_type = native_type_for_binding(domain, &entry.return_binding);
            params.push(format!("out: *mut {out_type}"));
            call_args.push("out".to_string());
        }

        for (index, param) in entry.parameters.iter().enumerate() {
            let name = sanitize_param_name(&param.name, index);
            let ty = native_type_for_binding(domain, &param.binding_type);
            params.push(format!("{name}: {ty}"));
            call_args.push(name);
        }

        write_stub_docs(&mut output, entry, binding.extern_name, false);
        output.push_str(&format!(
            "pub(crate) unsafe fn {function_name}(context: &RuntimeCallContext{}) -> RuntimeResult<()> {{\n",
            if params.is_empty() {
                String::new()
            } else {
                format!(", {}", params.join(", "))
            }
        ));
        output.push_str(&format!(
            "    unsafe {{ {native_alias}::{function_name}({}) }}\n",
            call_args.join(", ")
        ));
        output.push_str("}\n\n");
    }

    output
}

/// Render a runtime VM forwarding stub for a runtime domain.
pub(crate) fn render_runtime_vm_stub(domain: &str, bindings: &BindingCatalogEntry) -> String {
    let runtime_bindings = collect_runtime_bindings(bindings);
    let consts = build_binding_consts(domain, &runtime_bindings);
    let vm_types = collect_vm_named_types(domain, &runtime_bindings);
    let type_domains = collect_type_domains(domain, &runtime_bindings);
    let vm_usage = collect_vm_stub_usage(&runtime_bindings);
    let vm_alias = format!("{domain}_vm");

    let mut output = String::new();
    output.push_str("use destack_vm as vm;\n\n");
    output.push_str("use crate::diagnostic::RuntimeResult;\n");
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
    if !type_domains.is_empty() {
        let imports = type_domains
            .iter()
            .map(|domain| domain.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        output.push_str(&format!("use crate::platform::{{{imports}}};\n"));
    }
    if !vm_types.is_empty() {
        let names = vm_types.iter().cloned().collect::<Vec<_>>().join(", ");
        output.push_str(&format!("use crate::platform::{domain}::{{{names}}};\n"));
    }
    output.push_str(&format!(
        "use crate::platform::{domain}::vm as {vm_alias};\n"
    ));
    output.push_str("use crate::runtime::RuntimeCallContext;\n\n");

    for binding in &consts {
        let entry = binding.entry;
        if entry.scope != BindingScope::Runtime {
            continue;
        }

        let method_name = binding.implementation_fn_name.clone();
        let return_type = render_return_type(domain, entry);
        let params = render_params(domain, entry);
        let mut call_args = vec!["runtime".to_string(), "context".to_string()];
        for (index, param) in entry.parameters.iter().enumerate() {
            call_args.push(sanitize_param_name(&param.name, index));
        }

        write_stub_docs(&mut output, entry, binding.extern_name, false);
        output.push_str(&format!("pub(crate) fn {method_name}(\n"));
        output.push_str("    runtime: &RuntimeCallContext,\n");
        output.push_str("    context: &mut vm::ExternalCallContext<'_>,\n");
        for param in &params {
            output.push_str(&format!("    {param},\n"));
        }
        output.push_str(&format!(") -> RuntimeResult<{return_type}> {{\n"));
        output.push_str(&format!(
            "    {vm_alias}::{method_name}({})\n",
            call_args.join(", ")
        ));
        output.push_str("}\n\n");
    }

    output
}

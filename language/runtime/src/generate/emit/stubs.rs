use super::codegen::ModuleCodegen;
use super::docs::GeneratedDocumentation;
use super::*;
use crate::analyze::{BindingEntry, BindingType, CatalogBindingSimulation};

/// Stateful renderer for one native stub module.
struct NativeStubRenderer<'a> {
    /// The platform module being rendered.
    domain: &'a str,
    /// The binding catalog for this module.
    bindings: &'a ModuleBindings,
    /// The function visibility for generated entrypoints.
    function_visibility: &'a str,
    /// Whether this renderer emits simulation stubs.
    is_simulation: bool,
    /// Rust naming helpers for this module.
    codegen: ModuleCodegen<'a>,
    /// Rendered output buffer.
    output: String,
}

impl<'a> NativeStubRenderer<'a> {
    /// Create one native stub renderer.
    fn new(
        domain: &'a str,
        bindings: &'a ModuleBindings,
        function_visibility: &'a str,
        is_simulation: bool,
    ) -> Self {
        Self {
            domain,
            bindings,
            function_visibility,
            is_simulation,
            codegen: ModuleCodegen::new(domain),
            output: String::new(),
        }
    }

    /// Render the full native stub file.
    fn render(mut self) -> String {
        let bindings = if self.is_simulation {
            collect_simulation_bindings(self.bindings)
        } else {
            self.bindings.clone()
        };
        let consts = BindingSymbol::collect(&self.codegen, &bindings);
        let usage = NativeUsage::collect(&bindings);
        let type_domains = collect_type_domains(self.domain, &bindings);
        let mut named_types = if self.is_simulation {
            collect_native_stub_named_types(self.domain, &bindings)
        } else {
            collect_native_named_types(self.domain, &bindings)
        };

        // error stubs already import the shared platform error type explicitly
        if self.domain == "error" && !self.is_simulation {
            named_types.remove("PlatformError");
        }

        // file header
        self.output.push_str("#![allow(dead_code)]\n");
        self.output.push_str("#![allow(unused_imports)]\n");
        self.output
            .push_str("#![allow(clippy::missing_safety_doc)]\n");
        self.output
            .push_str("use crate::diagnostic::{RuntimeError, RuntimeResult};\n");
        if !self.is_simulation {
            self.output.push_str(&format!(
                "use crate::platform::{}::bindings_generated as bindings;\n",
                self.domain
            ));
        }
        self.output.push_str("use crate::platform::{\n");
        self.output.push_str("    PlatformError,\n");
        if usage.uses_platform_slice {
            self.output.push_str("    NativeSlice,\n");
        }
        if usage.uses_platform_array {
            self.output.push_str("    NativeArray,\n");
        }
        if usage.uses_platform_string_ref {
            self.output.push_str("    NativeStringRef,\n");
        }
        if usage.uses_platform_string_slice {
            self.output.push_str("    NativeStringSlice,\n");
        }
        self.output.push_str("};\n\n");
        self.output
            .push_str("use crate::runtime::BindingCallContext;\n");
        if !self.is_simulation {
            self.output.push_str("use bindings::*;\n");
        }
        self.output.push('\n');

        // platform imports
        if !type_domains.is_empty() {
            let imports = type_domains
                .iter()
                .map(|domain| domain.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            self.output
                .push_str(&format!("use crate::platform::{{{imports}}};\n"));
        }
        if !named_types.is_empty() {
            let names = named_types.iter().cloned().collect::<Vec<_>>().join(", ");
            self.output.push_str(&format!(
                "use crate::platform::{}::{{{names}}};\n",
                self.domain
            ));
        }
        if self.domain == "error" {
            self.output
                .push_str("use crate::platform::error as platform_error;\n");
        }
        self.output.push('\n');

        // binding stubs
        for binding in &consts {
            self.render_binding(binding);
            self.output.push('\n');
        }

        self.output
    }

    /// Render documentation for one native stub binding.
    fn write_docs(&mut self, entry: &BindingEntry, extern_name: &str) {
        let fallback = if self.is_simulation {
            format!("Simulation binding for `{extern_name}`.")
        } else {
            format!("Binding for `{extern_name}`.")
        };
        let docs = GeneratedDocumentation::render(entry.documentation.as_deref(), &fallback);

        self.output.push_str(&docs);
    }

    /// Render one native stub binding.
    fn render_binding(&mut self, binding: &BindingSymbol<'_>) {
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

        // native result out pointer
        if entry.return_binding != BindingType::Void {
            let out_type = self.codegen.native_type_for_binding(&entry.return_binding);
            params.push(format!("out: *mut {out_type}"));
            unused.push("out".to_string());
        }

        // native parameters
        for (index, param) in entry.parameters.iter().enumerate() {
            let name = self.codegen.sanitize_param_name(&param.name, index);
            let ty = self.codegen.native_type_for_binding(&param.binding_type);
            params.push(format!("{name}: {ty}"));
            unused.push(name);
        }

        // function header
        self.write_docs(entry, binding.extern_name);
        self.output.push_str(&format!(
            "{} unsafe fn {function_name}({}BindingCallContext{}) -> RuntimeResult<()> {{\n",
            self.function_visibility,
            if self.is_simulation {
                "_binding: &"
            } else {
                "binding: &"
            },
            if params.is_empty() {
                String::new()
            } else {
                format!(", {}", params.join(", "))
            }
        ));

        // result pointer validation
        if entry.return_binding != BindingType::Void {
            self.output.push_str("    if out.is_null() {\n");
            self.output.push_str(
                "        return Err(RuntimeError::from(PlatformError::null_pointer(\"out\")).boxed());\n",
            );
            self.output.push_str("    }\n");
        }

        // keep parameters marked as used in the stub
        if let Some((first, rest)) = unused.split_first() {
            if rest.is_empty() {
                self.output.push_str(&format!("    let _ = {first};\n"));
            } else {
                self.output.push_str("    let _ = (");
                self.output.push_str(&unused.join(", "));
                self.output.push_str(");\n");
            }
        }
        self.output.push('\n');

        // runtime fallback
        self.output
            .push_str("    Err(RuntimeError::from(PlatformError::not_supported(\n");
        if self.is_simulation {
            self.output
                .push_str(&format!("        \"{}\",\n", binding.extern_name));
        } else {
            self.output
                .push_str(&format!("        \"{}\",\n", binding.extern_name));
        }
        self.output.push_str("    ))\n");
        self.output.push_str("    .boxed())\n");
        self.output.push_str("}\n");
    }
}

/// Stateful renderer for one VM stub module.
struct VmStubRenderer<'a> {
    /// The platform module being rendered.
    domain: &'a str,
    /// The binding catalog for this module.
    bindings: &'a ModuleBindings,
    /// Whether this renderer emits simulation stubs.
    is_simulation: bool,
    /// Rust naming helpers for this module.
    codegen: ModuleCodegen<'a>,
    /// Rendered output buffer.
    output: String,
}

impl<'a> VmStubRenderer<'a> {
    /// Create one VM stub renderer.
    fn new(domain: &'a str, bindings: &'a ModuleBindings, is_simulation: bool) -> Self {
        Self {
            domain,
            bindings,
            is_simulation,
            codegen: ModuleCodegen::new(domain),
            output: String::new(),
        }
    }

    /// Render the full VM stub file.
    fn render(mut self) -> String {
        let bindings = if self.is_simulation {
            collect_simulation_bindings(self.bindings)
        } else {
            self.bindings.clone()
        };
        let consts = BindingSymbol::collect(&self.codegen, &bindings);
        let vm_types = if self.is_simulation {
            collect_vm_stub_named_types(self.domain, &bindings)
        } else {
            collect_vm_named_types(self.domain, &bindings)
        };
        let type_domains = collect_type_domains(self.domain, &bindings);
        let vm_usage = VmStubUsage::collect(&bindings);

        // file header
        self.output.push_str("#![allow(dead_code)]\n");
        self.output.push_str("#![allow(unused_imports)]\n");
        self.output.push_str("use destack_vm as vm;\n");
        self.output
            .push_str("use crate::diagnostic::{RuntimeError, RuntimeResult};\n");
        self.output
            .push_str("use crate::platform::PlatformError;\n");

        // VM helper imports
        let mut vm_imports = Vec::new();
        if vm_usage.uses_vm_slice {
            vm_imports.push("VmSlice");
        }
        if vm_usage.uses_vm_array {
            vm_imports.push("VmArray");
        }
        if !vm_imports.is_empty() {
            self.output.push_str(&format!(
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
            self.output
                .push_str(&format!("use crate::platform::{{{imports}}};\n"));
        }
        if !vm_types.is_empty() {
            let names = vm_types.iter().cloned().collect::<Vec<_>>().join(", ");
            self.output.push_str(&format!(
                "use crate::platform::{}::{{{names}}};\n",
                self.domain
            ));
        }
        self.output
            .push_str("use crate::runtime::BindingCallContext;\n\n");

        // binding stubs
        for binding in &consts {
            self.render_binding(binding);
            self.output.push('\n');
        }

        self.output
    }

    /// Render documentation for one VM stub binding.
    fn write_docs(&mut self, entry: &BindingEntry, extern_name: &str) {
        let fallback = if self.is_simulation {
            format!("Simulation binding for `{extern_name}`.")
        } else {
            format!("Binding for `{extern_name}`.")
        };
        let docs = GeneratedDocumentation::render(entry.documentation.as_deref(), &fallback);

        self.output.push_str(&docs);
    }

    /// Render one VM stub binding.
    fn render_binding(&mut self, binding: &BindingSymbol<'_>) {
        let entry = binding.entry;
        let function_name = binding.implementation_fn_name.clone();
        let return_type = self.codegen.render_return_type(entry);
        let params = self.codegen.render_params(entry);
        let mut unused = Vec::new();

        // VM parameters
        for (index, param) in entry.parameters.iter().enumerate() {
            unused.push(self.codegen.sanitize_param_name(&param.name, index));
        }

        // function header
        self.write_docs(entry, binding.extern_name);
        self.output
            .push_str(&format!("pub(crate) fn {function_name}(\n"));
        if self.is_simulation {
            self.output.push_str("    _binding: &BindingCallContext,\n");
        } else {
            self.output.push_str("    _binding: &BindingCallContext,\n");
        }
        self.output
            .push_str("    _context: &mut vm::ExternalCallContext<'_>,\n");
        for param in &params {
            self.output.push_str(&format!("    {param},\n"));
        }
        self.output
            .push_str(&format!(") -> RuntimeResult<{return_type}> {{\n"));

        // keep parameters marked as used in the stub
        if let Some((first, rest)) = unused.split_first() {
            if rest.is_empty() {
                self.output.push_str(&format!("    let _ = {first};\n"));
            } else {
                self.output.push_str("    let _ = (");
                self.output.push_str(&unused.join(", "));
                self.output.push_str(");\n");
            }
        }

        // runtime fallback
        self.output
            .push_str("    Err(RuntimeError::from(PlatformError::not_supported(\n");
        if self.is_simulation {
            self.output
                .push_str(&format!("        \"{}\",\n", binding.extern_name));
        } else {
            self.output.push_str(&format!(
                "        \"{} is not available in the VM yet\",\n",
                binding.extern_name
            ));
        }
        self.output.push_str("    ))\n");
        self.output.push_str("    .boxed())\n");
        self.output.push_str("}\n");
    }
}

/// Render stub native bindings for a runtime domain.
pub(crate) fn render_native_stub(domain: &str, bindings: &ModuleBindings) -> String {
    NativeStubRenderer::new(domain, bindings, "pub(crate)", false).render()
}

/// Render stub host bindings for a runtime domain.
pub(crate) fn render_host_stub(domain: &str, bindings: &ModuleBindings) -> String {
    NativeStubRenderer::new(domain, bindings, "pub(crate)", false).render()
}

/// Render stub VM bindings for a runtime domain.
pub(crate) fn render_vm_stub(domain: &str, bindings: &ModuleBindings) -> String {
    VmStubRenderer::new(domain, bindings, false).render()
}

/// Render stub simulation native bindings for a runtime domain.
pub(crate) fn render_simulation_native_stub(domain: &str, bindings: &ModuleBindings) -> String {
    NativeStubRenderer::new(domain, bindings, "pub(crate)", true).render()
}

/// Render stub simulation VM bindings for a runtime domain.
pub(crate) fn render_simulation_vm_stub(domain: &str, bindings: &ModuleBindings) -> String {
    VmStubRenderer::new(domain, bindings, true).render()
}

/// Collect simulation-capable bindings for one domain.
fn collect_simulation_bindings(bindings: &ModuleBindings) -> ModuleBindings {
    let mut simulation_bindings = ModuleBindings::new();

    // keep only simulation-capable bindings
    for (extern_name, entry) in bindings {
        if entry.simulation == CatalogBindingSimulation::Unsupported {
            continue;
        }
        simulation_bindings.insert(extern_name.clone(), entry.clone());
    }
    simulation_bindings
}

/// Stateful renderer for generated module-level stubs.
struct ModuleStubRenderer {
    /// Rendered output buffer.
    output: String,
}

impl ModuleStubRenderer {
    /// Create one empty module-stub renderer.
    fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    /// Render the simulation module stub.
    fn render_simulation_mod(mut self) -> String {
        // simulation exports
        self.output.push_str("pub(crate) mod native;\n");
        self.output.push_str("pub(crate) mod vm;\n");

        self.output
    }

    /// Render the top-level module stub.
    fn render_module_mod(
        mut self,
        has_world_dispatch: bool,
        has_simulation_dispatch: bool,
    ) -> String {
        // generated modules
        self.output.push_str("#[path = \"abi.generated.rs\"]\n");
        self.output.push_str("pub(crate) mod abi_generated;\n");
        self.output
            .push_str("#[path = \"bindings.generated.rs\"]\n");
        self.output.push_str("mod bindings_generated;\n\n");
        self.output
            .push_str("#[allow(unused_imports, unreachable_pub)]\n");
        self.output.push_str("pub use abi_generated::*;\n");
        self.output
            .push_str("#[allow(unused_imports, unreachable_pub)]\n");
        self.output.push_str("pub use bindings_generated::*;\n");

        // handwritten entrypoints
        if has_world_dispatch {
            self.output.push_str("mod host;\n");
        }
        self.output.push_str("pub mod native;\n");
        if has_simulation_dispatch {
            self.output.push_str("pub(crate) mod simulation;\n");
        }
        self.output.push_str("pub mod vm;\n");

        self.output
    }

    /// Render the host router stub.
    fn render_host_router(mut self) -> String {
        // unix route
        self.output.push_str("#[cfg(unix)]\n");
        self.output.push_str("#[path = \"unix/mod.rs\"]\n");
        self.output.push_str("mod unix;\n");
        self.output.push_str("#[cfg(unix)]\n");
        self.output.push_str("#[allow(unused_imports)]\n");
        self.output.push_str("pub(crate) use unix::*;\n\n");

        // windows route
        self.output.push_str("#[cfg(windows)]\n");
        self.output.push_str("#[path = \"windows/mod.rs\"]\n");
        self.output.push_str("mod windows;\n");
        self.output.push_str("#[cfg(windows)]\n");
        self.output.push_str("#[allow(unused_imports)]\n");
        self.output.push_str("pub(crate) use windows::*;\n\n");

        // unsupported route
        self.output.push_str("#[cfg(not(any(unix, windows)))]\n");
        self.output.push_str("#[path = \"unsupported.rs\"]\n");
        self.output.push_str("mod unsupported;\n");
        self.output.push_str("#[cfg(not(any(unix, windows)))]\n");
        self.output.push_str("#[allow(unused_imports)]\n");
        self.output.push_str("pub(crate) use unsupported::*;\n");

        self.output
    }

    /// Render the OS backend shim stub.
    fn render_os_backend_mod(mut self) -> String {
        // unsupported shim
        self.output.push_str("#[path = \"../unsupported.rs\"]\n");
        self.output.push_str("mod unsupported;\n\n");
        self.output.push_str("#[allow(unused_imports)]\n");
        self.output.push_str("pub(crate) use unsupported::*;\n");

        self.output
    }
}

/// Render a simulation module re-export stub for a runtime domain.
pub(crate) fn render_simulation_mod_stub() -> String {
    ModuleStubRenderer::new().render_simulation_mod()
}

/// Render a top-level module stub for a runtime domain.
pub(crate) fn render_module_mod_stub(
    has_world_dispatch: bool,
    has_simulation_dispatch: bool,
) -> String {
    ModuleStubRenderer::new().render_module_mod(has_world_dispatch, has_simulation_dispatch)
}

/// Render a host router stub for world-dispatched domains.
pub(crate) fn render_host_router_stub() -> String {
    ModuleStubRenderer::new().render_host_router()
}

/// Render a unix or windows backend shim for world-dispatched domains.
pub(crate) fn render_os_backend_mod_stub() -> String {
    ModuleStubRenderer::new().render_os_backend_mod()
}

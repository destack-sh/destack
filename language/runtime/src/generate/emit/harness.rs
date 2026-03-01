/// Render a tests module scaffold.
pub(crate) fn render_domain_tests_mod_stub() -> String {
    let mut output = String::new();
    output.push_str("#[cfg(any(unix, windows))]\n");
    output.push_str("mod basic;\n");
    output.push_str("#[cfg(any(unix, windows))]\n");
    output.push_str("mod tests;\n\n");
    output.push_str("#[cfg(any(unix, windows))]\n");
    output.push_str("pub(super) use tests::*;\n");
    output
}

/// Render a handwritten tests harness scaffold.
pub(crate) fn render_domain_tests_stub(domain: &str) -> String {
    let domain_pascal = to_pascal_case(domain);
    let context_name = format!("{domain_pascal}HarnessContext");
    let native_name = format!("Native{domain_pascal}Harness");
    let vm_name = format!("Vm{domain_pascal}Harness");
    let handle_name = format!("{domain_pascal}HarnessHandle");
    let harness_label = format!("{domain} harness call should succeed");

    let mut output = String::new();
    output.push_str("#![cfg_attr(windows, allow(dead_code, unused_imports))]\n\n");
    output.push_str("#[path = \"harness.rs\"]\n");
    output.push_str("mod harness;\n\n");
    output.push_str("use destack_vm as vm;\n\n");
    output.push_str("use crate::diagnostic::RuntimeResult;\n");
    output.push_str("use crate::runtime::BindingCallContext;\n");
    output.push_str("use crate::tests::runtime::TestRuntime;\n\n");
    output.push_str("/// Test harness context used by tests.\n");
    output.push_str(&format!("pub(crate) struct {context_name}<'call> {{\n"));
    output.push_str("    /// Runtime call context active for this operation.\n");
    output.push_str("    pub(super) call_context: &'call BindingCallContext,\n");
    output.push_str("    /// VM context when running VM bindings.\n");
    output.push_str("    pub(super) vm_context: Option<*mut ()>,\n");
    output.push_str("}\n\n");
    output.push_str(&format!("/// Native {domain} harness.\n"));
    output.push_str(&format!("pub(crate) struct {native_name} {{\n"));
    output.push_str("    /// Runtime that powers the harness.\n");
    output.push_str("    runtime: TestRuntime,\n");
    output.push_str("}\n\n");
    output.push_str(&format!("impl {native_name} {{\n"));
    output.push_str(&format!("    /// Create a new native {domain} harness.\n"));
    output.push_str("    pub(crate) fn new() -> Self {\n");
    output.push_str("        Self {\n");
    output.push_str("            runtime: TestRuntime::deterministic_random(),\n");
    output.push_str("        }\n");
    output.push_str("    }\n");
    output.push_str("}\n\n");
    output.push_str(&format!("/// VM {domain} harness.\n"));
    output.push_str(&format!("pub(crate) struct {vm_name} {{\n"));
    output.push_str("    /// Runtime that powers the harness.\n");
    output.push_str("    runtime: TestRuntime,\n");
    output.push_str("}\n\n");
    output.push_str(&format!("impl {vm_name} {{\n"));
    output.push_str(&format!("    /// Create a new VM {domain} harness.\n"));
    output.push_str("    pub(crate) fn new() -> Self {\n");
    output.push_str("        Self {\n");
    output.push_str("            runtime: TestRuntime::deterministic_random(),\n");
    output.push_str("        }\n");
    output.push_str("    }\n");
    output.push_str("}\n\n");
    output.push_str("/// Harness handle that dispatches to native or VM implementations.\n");
    output.push_str(&format!("pub(crate) enum {handle_name} {{\n"));
    output.push_str(&format!("    /// Native {domain} harness.\n"));
    output.push_str(&format!("    Native({native_name}),\n"));
    output.push_str(&format!("    /// VM {domain} harness.\n"));
    output.push_str(&format!("    Vm({vm_name}),\n"));
    output.push_str("}\n\n");
    output.push_str(&format!("impl {handle_name} {{\n"));
    output.push_str("    /// Run a native or VM call context around one callback.\n");
    output.push_str("    pub(crate) fn with_context<F, R>(&self, callback: F) -> R\n");
    output.push_str("    where\n");
    output.push_str(&format!(
        "        F: for<'call> FnOnce({context_name}<'call>) -> R,\n"
    ));
    output.push_str("    {\n");
    output.push_str("        match self {\n");
    output.push_str(&format!(
        "            {handle_name}::Native(harness) => {{\n"
    ));
    output.push_str("                harness.runtime.with_native_call_context(|call_context| {\n");
    output.push_str(&format!("                    callback({context_name} {{\n"));
    output.push_str("                        call_context,\n");
    output.push_str("                        vm_context: None,\n");
    output.push_str("                    })\n");
    output.push_str("                })\n");
    output.push_str("            }\n");
    output.push_str(&format!("            {handle_name}::Vm(harness) => {{\n"));
    output.push_str("                harness\n");
    output.push_str("                    .runtime\n");
    output.push_str("                    .with_vm_call_context(|call_context, vm_context| {\n");
    output.push_str(
        "                        let vm_context = vm_context as *mut vm::ExternalCallContext<'_> as *mut ();\n",
    );
    output.push_str(&format!(
        "                        callback({context_name} {{\n"
    ));
    output.push_str("                            call_context,\n");
    output.push_str("                            vm_context: Some(vm_context),\n");
    output.push_str("                        })\n");
    output.push_str("                    })\n");
    output.push_str("            }\n");
    output.push_str("        }\n");
    output.push_str("    }\n\n");
    output.push_str("    /// Run one callback that returns a runtime result.\n");
    output.push_str("    pub(crate) fn run<F>(&self, callback: F)\n");
    output.push_str("    where\n");
    output.push_str(&format!(
        "        F: for<'call> FnOnce({context_name}<'call>) -> RuntimeResult<()>,\n"
    ));
    output.push_str("    {\n");
    output.push_str("        self.with_context(callback)\n");
    output.push_str(&format!("            .expect(\"{harness_label}\");\n"));
    output.push_str("    }\n");
    output.push_str("}\n\n");
    output.push_str("/// Run one callback against both harnesses.\n");
    output.push_str("pub(crate) fn with_harnesses<F>(mut callback: F)\n");
    output.push_str("where\n");
    output.push_str(&format!("    F: FnMut(&{handle_name}),\n"));
    output.push_str("{\n");
    output.push_str(&format!(
        "    let native = {handle_name}::Native({native_name}::new());\n"
    ));
    output.push_str("    callback(&native);\n");
    output.push_str(&format!(
        "    let vm = {handle_name}::Vm({vm_name}::new());\n"
    ));
    output.push_str("    callback(&vm);\n");
    output.push_str("}\n\n");
    output.push_str("/// Run one callback against both harness contexts.\n");
    output.push_str("pub(crate) fn with_harness_context<F>(mut callback: F)\n");
    output.push_str("where\n");
    output.push_str(&format!(
        "    F: for<'call> FnMut({context_name}<'call>) -> RuntimeResult<()>,\n"
    ));
    output.push_str("{\n");
    output.push_str("    with_harnesses(|harness| {\n");
    output.push_str("        harness.run(&mut callback);\n");
    output.push_str("    });\n");
    output.push_str("}\n");
    output
}

/// Render a basic scaffold test for one domain.
pub(crate) fn render_domain_tests_basic_stub(domain: &str) -> String {
    let mut output = String::new();
    output.push_str("use super::with_harness_context;\n\n");
    output.push_str("#[cfg(any(unix, windows))]\n");
    output.push_str("#[test]\n");
    output.push_str(&format!("fn test_{domain}_basic_scaffold() {{\n"));
    output.push_str("    with_harness_context(|context| {\n");
    output.push_str("        let _ = context.call_context;\n");
    output.push_str("        let _ = context.vm_context;\n");
    output.push_str("        Ok(())\n");
    output.push_str("    });\n");
    output.push_str("}\n");
    output
}

/// Convert one snake-case domain name into a PascalCase identifier segment.
pub(crate) fn to_pascal_case(domain: &str) -> String {
    let mut out = String::new();

    for part in domain.split('_') {
        let mut chars = part.chars();
        let Some(first) = chars.next() else {
            continue;
        };
        out.push(first.to_ascii_uppercase());
        out.extend(chars);
    }

    out
}

use std::collections::BTreeSet;

use crate::model::{
    BindingEntry, BindingType, CatalogBindingReplayKind, CatalogEffectClass, CatalogReplayPayload,
    CatalogReplayPolicy,
};

use super::*;

impl<'a> DomainWriter<'a> {
    /// Render replay payload structs for bindings.
    pub(super) fn write_replay_payloads(&mut self) {
        let output = &mut self.output;
        let domain = self.spec.domain;
        let consts = &self.spec.consts;

        let mut wrote = false;
        for binding in consts {
            let entry = binding.entry;
            let CatalogEffectClass::External {
                replay: CatalogReplayPolicy::Recordable,
            } = entry.effect_class
            else {
                continue;
            };
            if entry.replay_kind != CatalogBindingReplayKind::Regular {
                continue;
            }

            let struct_name = replay_struct_name(&binding.const_name);
            let args_struct_name = format!("{struct_name}Args");
            let result_type = replay_result_type(domain, entry);
            let supports_args = matches!(
                entry.replay_payload,
                CatalogReplayPayload::ArgumentsAndResults
            );
            if supports_args {
                output.push_str(&format!(
                    "/// Replay argument payload for {}.\n",
                    binding.extern_name
                ));
                output.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
                output.push_str(&format!("struct {args_struct_name} {{\n"));
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = sanitize_param_name(&param.name, index);
                    let field_type = replay_type_for_binding(domain, &param.binding_type);
                    output.push_str(&format!("    /// Replay value for {name}.\n"));
                    output.push_str(&format!("    pub {name}: {field_type},\n"));
                }
                output.push_str("}\n\n");
            }
            output.push_str(&format!(
                "/// Replay payload for {}.\n",
                binding.extern_name
            ));
            output.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
            output.push_str(&format!("struct {struct_name} {{\n"));
            if supports_args {
                output.push_str("    /// Optional replay arguments.\n");
                output.push_str(&format!("    pub args: Option<{args_struct_name}>,\n"));
            }
            output.push_str("    /// Replay result payload.\n");
            output.push_str(&format!("    pub result: {result_type},\n"));
            output.push_str("}\n\n");
            wrote = true;
        }

        if wrote {
            output.push_str("\n");
        }
    }

    /// Render native replay helpers for a domain.
    pub(super) fn write_native_replay_helpers(&mut self) {
        let output = &mut self.output;
        let domain = self.spec.domain;
        let consts = &self.spec.consts;

        let bindings = consts.iter().filter(|binding| {
            matches!(
                binding.entry.effect_class,
                CatalogEffectClass::External {
                    replay: CatalogReplayPolicy::Recordable
                }
            ) && binding.entry.replay_kind == CatalogBindingReplayKind::Regular
        });

        let mut emitted_header = false;
        for binding in bindings {
            if !emitted_header {
                output.push_str(&format!(
                    "/// Native replay implementations for {domain} bindings.\n"
                ));
                emitted_header = true;
            }

            let entry = binding.entry;
            let fn_name = native_replay_fn_name(domain, binding.extern_name);
            let implementation_fn_name = &binding.implementation_fn_name;
            let supports_args = matches!(
                entry.replay_payload,
                CatalogReplayPayload::ArgumentsAndResults
            );
            let replay_struct = replay_struct_name(&binding.const_name);
            let replay_args_struct = format!("{replay_struct}Args");

            let mut params = Vec::new();
            let mut args = Vec::new();
            if entry.return_binding != BindingType::Void {
                let out_type = native_type_for_binding(domain, &entry.return_binding);
                params.push(format!("out: *mut {out_type}"));
                args.push("out".to_string());
            }
            for (index, param) in entry.parameters.iter().enumerate() {
                let name = sanitize_param_name(&param.name, index);
                let ty = native_type_for_binding(domain, &param.binding_type);
                params.push(format!("{name}: {ty}"));
                args.push(name);
            }

            output.push_str("#[inline]\n");
            output.push_str(&format!("fn {fn_name}(\n"));
            output.push_str("    context: &BindingCallContext,\n");
            if entry.scope != crate::model::CatalogBindingScope::Runtime {
                output.push_str("    world: RuntimeWorld,\n");
            }
            for param in &params {
                output.push_str(&format!("    {param},\n"));
            }
            output.push_str(") -> RuntimeResult<()> {\n");

            if !supports_args && !entry.parameters.is_empty() {
                let unused = entry
                    .parameters
                    .iter()
                    .enumerate()
                    .map(|(index, param)| sanitize_param_name(&param.name, index))
                    .collect::<Vec<_>>();
                if unused.len() == 1 {
                    output.push_str(&format!("    let _ = &{};\n\n", unused[0]));
                } else {
                    output.push_str("    let _ = (");
                    output.push_str(
                        &unused
                            .iter()
                            .map(|name| format!("&{name}"))
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                    output.push_str(");\n\n");
                }
            }

            output.push_str("    context.replay().run_binding_with_policy(\n");
            output.push_str(&format!("        {},\n", binding.const_name));
            output.push_str(&format!(
                "        context.replay_payload_for({})?,\n",
                binding.const_name
            ));
            let runtime_call = if args.is_empty() {
                format!(
                    "unsafe {{ platform_runtime_native::{}(context) }}",
                    implementation_fn_name
                )
            } else {
                format!(
                    "unsafe {{ platform_runtime_native::{}(context, {}) }}",
                    implementation_fn_name,
                    args.join(", ")
                )
            };
            let host_call = if args.is_empty() {
                format!(
                    "unsafe {{ platform_native::{}(context) }}",
                    implementation_fn_name
                )
            } else {
                format!(
                    "unsafe {{ platform_native::{}(context, {}) }}",
                    implementation_fn_name,
                    args.join(", ")
                )
            };
            if entry.scope == crate::model::CatalogBindingScope::Runtime {
                output.push_str(&format!("        || {runtime_call},\n"));
            } else {
                let simulation_call = if args.is_empty() {
                    format!(
                        "unsafe {{ platform_simulation_native::{}(context) }}",
                        implementation_fn_name
                    )
                } else {
                    format!(
                        "unsafe {{ platform_simulation_native::{}(context, {}) }}",
                        implementation_fn_name,
                        args.join(", ")
                    )
                };
                output.push_str("        || match world {\n");
                output.push_str(&format!("            RuntimeWorld::Host => {host_call},\n"));
                output.push_str(&format!(
                    "            RuntimeWorld::Simulation => {simulation_call},\n"
                ));
                output.push_str("        },\n");
            }
            output.push_str("        |result| {\n");

            if supports_args {
                output.push_str("            let record_args = matches!(\n");
                output.push_str(&format!(
                    "                context.replay_payload_for({})?,\n",
                    binding.const_name
                ));
                output.push_str("                BindingReplayPayload::ArgumentsAndResults,\n");
                output.push_str("            );\n");
                output.push_str("            let args = if record_args {\n");
                output.push_str("                // replay args\n");
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    for line in render_native_replay_encode_lines(
                        domain,
                        &param.binding_type,
                        &recorded_name,
                        &name,
                    ) {
                        output.push_str(&format!("                {line}\n"));
                    }
                }
                output.push_str(&format!("                Some({replay_args_struct} {{\n"));
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    output.push_str(&format!("                    {name}: {recorded_name},\n"));
                }
                output.push_str("                })\n");
                output.push_str("            } else {\n");
                output.push_str("                None\n");
                output.push_str("            };\n\n");
            }

            output.push_str("            if let Ok(()) = result {\n");
            if entry.return_binding != BindingType::Void {
                for line in render_native_replay_read_out_lines(
                    domain,
                    &entry.return_binding,
                    "out",
                    "result_value",
                ) {
                    output.push_str(&format!("                {line}\n"));
                }
                for line in render_native_replay_encode_lines(
                    domain,
                    &entry.return_binding,
                    "result_recorded",
                    "result_value",
                ) {
                    output.push_str(&format!("                {line}\n"));
                }
            } else {
                output.push_str("                let result_recorded = ();\n");
            }
            output.push_str(&format!(
                "                let payload = {replay_struct} {{\n"
            ));
            if supports_args {
                output.push_str("                    args,\n");
            }
            if entry.return_is_result {
                output.push_str("                    result: Ok(result_recorded),\n");
            } else {
                output.push_str("                    result: result_recorded,\n");
            }
            output.push_str("                };\n");
            output.push_str("                return Ok(Some(payload));\n");
            output.push_str("            }\n\n");

            if entry.return_is_result {
                output.push_str("            if let Err(error) = result {\n");
                output.push_str("                let payload = {\n");
                output.push_str(
                    "                    let result = Err(PlatformError::from(error.as_ref()));\n",
                );
                output.push_str(&format!("                    {replay_struct} {{\n"));
                if supports_args {
                    output.push_str("                        args,\n");
                }
                output.push_str("                        result,\n");
                output.push_str("                    }\n");
                output.push_str("                };\n");
                output.push_str("                return Ok(Some(payload));\n");
                output.push_str("            }\n\n");
            }

            output.push_str("            Ok(None)\n");
            output.push_str("        },\n");
            output.push_str("        |payload| {\n");

            if supports_args {
                output.push_str("            if let Some(args) = payload.args.as_ref() {\n");
                output.push_str("                // replay arg verification\n");
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    for line in render_native_replay_encode_lines(
                        domain,
                        &param.binding_type,
                        &recorded_name,
                        &name,
                    ) {
                        output.push_str(&format!("                {line}\n"));
                    }
                }
                let mut compare_counter = 0usize;
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    output.push_str(&format!(
                        "                let {name}_payload = &args.{name};\n"
                    ));
                    output.push_str(&format!(
                        "                let {name}_current = &{recorded_name};\n"
                    ));
                    let mismatch_stmt = format!(
                        "return Err(RuntimeError::ReplayMismatch {{ name: {}.name.to_string() }}.boxed());",
                        binding.const_name
                    );
                    let compare_lines = render_replay_compare_lines(
                        domain,
                        &param.binding_type,
                        &format!("{name}_payload"),
                        &format!("{name}_current"),
                        &mismatch_stmt,
                        &mut compare_counter,
                    );
                    for line in compare_lines {
                        output.push_str(&format!("                {line}\n"));
                    }
                }
                output.push_str("            }\n\n");
            }

            output.push_str("            // replay result\n");
            if entry.return_is_result {
                output.push_str("            match payload.result {\n");
                if entry.return_binding != BindingType::Void {
                    output.push_str("                Ok(value) => {\n");
                    for line in render_native_replay_store_lines(
                        domain,
                        &entry.return_binding,
                        "out",
                        "value",
                        "value_native",
                    ) {
                        output.push_str(&format!("                    {line}\n"));
                    }
                    output.push_str("                    Ok(())\n");
                    output.push_str("                }\n");
                } else {
                    output.push_str("                Ok(()) => Ok(()),\n");
                }
                output.push_str(
                    "                Err(error) => Err(RuntimeError::from(error).boxed()),\n",
                );
                output.push_str("            }\n");
            } else {
                if entry.return_binding != BindingType::Void {
                    for line in render_native_replay_store_lines(
                        domain,
                        &entry.return_binding,
                        "out",
                        "payload.result",
                        "result_native",
                    ) {
                        output.push_str(&format!("            {line}\n"));
                    }
                }
                output.push_str("            Ok(())\n");
            }
            output.push_str("        },\n");
            output.push_str("    )\n");
            output.push_str("}\n\n");
        }
    }
}

/// Render the replay payload struct name for a binding.
fn replay_struct_name(const_name: &str) -> String {
    let mut out = String::new();
    let mut upper = true;
    for ch in const_name.chars() {
        if ch == '_' {
            upper = true;
            continue;
        }
        if upper {
            out.push(ch.to_ascii_uppercase());
            upper = false;
        } else {
            out.push(ch.to_ascii_lowercase());
        }
    }
    if out.is_empty() {
        out.push_str("Binding");
    }
    out.push_str("Replay");
    out
}

/// Render the replay result type for a binding entry.
fn replay_result_type(domain: &str, entry: &BindingEntry) -> String {
    let inner = replay_type_for_binding(domain, &entry.return_binding);
    if entry.return_is_result {
        format!("Result<{inner}, PlatformError>")
    } else {
        inner
    }
}

impl<'a> DomainWriter<'a> {
    /// Render the VM replay helpers for bindings.
    pub(super) fn write_vm_replay_helpers(&mut self) {
        let output = &mut self.output;
        let domain = self.spec.domain;
        let consts = &self.spec.consts;

        let bindings = consts.iter().filter(|binding| {
            matches!(
                binding.entry.effect_class,
                CatalogEffectClass::External {
                    replay: CatalogReplayPolicy::Recordable
                }
            ) && binding.entry.replay_kind == CatalogBindingReplayKind::Regular
        });

        let mut emitted_header = false;
        for binding in bindings {
            if !emitted_header {
                output.push_str(&format!(
                    "/// VM replay implementations for {domain} bindings.\n"
                ));
                emitted_header = true;
            }

            let entry = binding.entry;
            let fn_name = vm_replay_fn_name(domain, binding.extern_name);
            let helper_base = vm_fn_name(domain, binding.extern_name);
            let implementation_fn_name = &binding.implementation_fn_name;
            let encode_helper = encode_helper_name(&helper_base);
            let supports_args = matches!(
                entry.replay_payload,
                CatalogReplayPayload::ArgumentsAndResults
            );
            let replay_struct = replay_struct_name(&binding.const_name);
            let replay_args_struct = format!("{replay_struct}Args");
            let invoke_args = render_invoke_args_with_prefix(entry);

            let mut params = Vec::new();
            for (index, param) in entry.parameters.iter().enumerate() {
                let name = sanitize_param_name(&param.name, index);
                let ty = vm_type_for_binding(domain, &param.binding_type);
                params.push(format!("{name}: {ty}"));
            }

            output.push_str("#[inline]\n");
            output.push_str(&format!("fn {fn_name}(\n"));
            output.push_str("    runtime: &BindingCallContext,\n");
            output.push_str("    context: &mut vm::ExternalCallContext<'_>,\n");
            if entry.scope != crate::model::CatalogBindingScope::Runtime {
                output.push_str("    world: RuntimeWorld,\n");
            }
            for param in &params {
                output.push_str(&format!("    {param},\n"));
            }
            output.push_str(") -> RuntimeResult<vm::Value> {\n");

            output.push_str("    let result = runtime.replay().run_binding_with_context_policy(\n");
            output.push_str(&format!("        {},\n", binding.const_name));
            output.push_str(&format!(
                "        runtime.replay_payload_for({})?,\n",
                binding.const_name
            ));
            output.push_str("        context,\n");
            if entry.scope == crate::model::CatalogBindingScope::Runtime {
                output.push_str(&format!(
                    "        |context| platform_runtime_vm::{}(runtime, context{invoke_args}),\n",
                    implementation_fn_name
                ));
            } else {
                let simulation_call = format!(
                    "platform_simulation_vm::{}(runtime, context{invoke_args})",
                    implementation_fn_name
                );
                output.push_str("        |context| {\n");
                output.push_str("            match world {\n");
                output.push_str(&format!(
                    "                RuntimeWorld::Host => platform_vm::{}(runtime, context{invoke_args}),\n",
                    implementation_fn_name
                ));
                output.push_str(&format!(
                    "                RuntimeWorld::Simulation => {simulation_call},\n"
                ));
                output.push_str("            }\n");
                output.push_str("        },\n");
            }
            output.push_str("        |context, result| {\n");
            output.push_str("            let _ = &context;\n");
            if supports_args {
                output.push_str("            let record_args = matches!(\n");
                output.push_str(&format!(
                    "                runtime.replay_payload_for({})?,\n",
                    binding.const_name
                ));
                output.push_str("                BindingReplayPayload::ArgumentsAndResults,\n");
                output.push_str("            );\n");
                output.push_str("            let args = if record_args {\n");
                output.push_str("                // replay args\n");
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    for line in render_replay_encode_lines(
                        domain,
                        &param.binding_type,
                        &recorded_name,
                        &name,
                    ) {
                        output.push_str(&format!("                {line}\n"));
                    }
                }
                output.push_str(&format!("                Some({replay_args_struct} {{\n"));
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    output.push_str(&format!("                    {name}: {recorded_name},\n"));
                }
                output.push_str("                })\n");
                output.push_str("            } else {\n");
                output.push_str("                None\n");
                output.push_str("            };\n\n");
            }

            if matches!(entry.return_binding, BindingType::Void) {
                output.push_str("            if let Ok(()) = result {\n");
                output.push_str("                let result_recorded = ();\n");
            } else {
                output.push_str("            if let Ok(value) = result {\n");
                let vm_result_type = vm_type_for_binding(domain, &entry.return_binding);
                output.push_str(&format!(
                    "                let result_value: {vm_result_type} = value.clone();\n"
                ));
                for line in render_replay_encode_lines(
                    domain,
                    &entry.return_binding,
                    "result_recorded",
                    "result_value",
                ) {
                    output.push_str(&format!("                {line}\n"));
                }
            }
            output.push_str(&format!(
                "                let payload = {replay_struct} {{\n"
            ));
            if supports_args {
                output.push_str("                    args,\n");
            }
            if entry.return_is_result {
                output.push_str("                    result: Ok(result_recorded),\n");
            } else {
                output.push_str("                    result: result_recorded,\n");
            }
            output.push_str("                };\n");
            output.push_str("                return Ok(Some(payload));\n");
            output.push_str("            }\n\n");

            if entry.return_is_result {
                output.push_str("            if let Err(error) = result {\n");
                output.push_str("                let payload = {\n");
                output.push_str(
                    "                    let result = Err(PlatformError::from(error.as_ref()));\n",
                );
                output.push_str(&format!("                    {replay_struct} {{\n"));
                if supports_args {
                    output.push_str("                        args,\n");
                }
                output.push_str("                        result,\n");
                output.push_str("                    }\n");
                output.push_str("                };\n");
                output.push_str("                return Ok(Some(payload));\n");
                output.push_str("            }\n\n");
            }

            output.push_str("            Ok(None)\n");
            output.push_str("        },\n");
            output.push_str("        |context, payload| {\n");
            output.push_str("            let _ = &context;\n");
            if supports_args {
                output.push_str("            if let Some(args) = payload.args.as_ref() {\n");
                output.push_str("                // replay arg verification\n");
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    for line in render_replay_encode_lines(
                        domain,
                        &param.binding_type,
                        &recorded_name,
                        &name,
                    ) {
                        output.push_str(&format!("                {line}\n"));
                    }
                }
                let mut compare_counter = 0usize;
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = sanitize_param_name(&param.name, index);
                    let recorded_name = format!("{name}_recorded");
                    output.push_str(&format!(
                        "                let {name}_payload = &args.{name};\n"
                    ));
                    output.push_str(&format!(
                        "                let {name}_current = &{recorded_name};\n"
                    ));
                    let mismatch_stmt = format!(
                        "return Err(RuntimeError::ReplayMismatch {{ name: {}.name.to_string() }}.boxed());",
                        binding.const_name
                    );
                    let compare_lines = render_replay_compare_lines(
                        domain,
                        &param.binding_type,
                        &format!("{name}_payload"),
                        &format!("{name}_current"),
                        &mismatch_stmt,
                        &mut compare_counter,
                    );
                    for line in compare_lines {
                        output.push_str(&format!("                {line}\n"));
                    }
                }
                output.push_str("            }\n\n");
            }

            output.push_str("            // replay result\n");
            if entry.return_is_result {
                output.push_str("            match payload.result {\n");
                if entry.return_binding != BindingType::Void {
                    output.push_str("                Ok(value) => {\n");
                    for line in render_replay_to_vm_binding_lines(
                        domain,
                        &entry.return_binding,
                        "vm_result",
                        "value",
                    ) {
                        output.push_str(&format!("                    {line}\n"));
                    }
                    output.push_str("                    Ok(vm_result)\n");
                    output.push_str("                }\n");
                } else {
                    output.push_str("                Ok(()) => Ok(()),\n");
                }
                output.push_str(
                    "                Err(error) => Err(RuntimeError::from(error).boxed()),\n",
                );
                output.push_str("            }\n");
            } else {
                if entry.return_binding != BindingType::Void {
                    for line in render_replay_to_vm_binding_lines(
                        domain,
                        &entry.return_binding,
                        "vm_result",
                        "payload.result",
                    ) {
                        output.push_str(&format!("            {line}\n"));
                    }
                    output.push_str("            Ok(vm_result)\n");
                } else {
                    output.push_str("            Ok(())\n");
                }
            }
            output.push_str("        },\n");
            output.push_str("    );\n");
            output.push_str(&format!(
                "    let result = {encode_helper}(context, result)?;\n"
            ));
            output.push_str("    Ok(result)\n");
            output.push_str("}\n\n");
        }
    }
}

/// Collect named types referenced by replay payloads.
pub(super) fn collect_replay_named_types(
    domain: &str,
    bindings: &BindingCatalogEntry,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for entry in bindings.values() {
        let CatalogEffectClass::External {
            replay: CatalogReplayPolicy::Recordable,
        } = entry.effect_class
        else {
            continue;
        };
        if entry.replay_kind != CatalogBindingReplayKind::Regular {
            continue;
        }

        if matches!(
            entry.replay_payload,
            CatalogReplayPayload::ArgumentsAndResults
        ) {
            for param in &entry.parameters {
                collect_replay_type_names(domain, &param.binding_type, &mut names);
            }
        }
        collect_replay_type_names(domain, &entry.return_binding, &mut names);
    }

    names
}

/// Collect VM-visible named types referenced by replay collection decoding.
pub(super) fn collect_replay_vm_named_types(
    domain: &str,
    bindings: &BindingCatalogEntry,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for entry in bindings.values() {
        let CatalogEffectClass::External {
            replay: CatalogReplayPolicy::Recordable,
        } = entry.effect_class
        else {
            continue;
        };
        if entry.replay_kind != CatalogBindingReplayKind::Regular {
            continue;
        }

        if matches!(
            entry.replay_payload,
            CatalogReplayPayload::ArgumentsAndResults
        ) {
            for param in &entry.parameters {
                collect_collection_vm_names(domain, &param.binding_type, &mut names);
            }
        }
        collect_collection_vm_names(domain, &entry.return_binding, &mut names);
    }

    names
}

/// Walk a binding type and record named types referenced by replay payloads.
fn collect_replay_type_names(
    domain: &str,
    binding_type: &BindingType,
    names: &mut BTreeSet<String>,
) {
    match binding_type {
        BindingType::Slice(inner) | BindingType::Array(inner) => {
            collect_replay_type_names(domain, inner, names);
        }
        BindingType::Optional(inner) => {
            collect_replay_type_names(domain, inner, names);
        }
        BindingType::Newtype { inner, .. } => {
            if binding_type_requires_abi(inner) {
                collect_replay_type_names(domain, inner, names);
            } else if let BindingType::Newtype {
                name,
                domain: type_domain,
                ..
            } = binding_type
            {
                names.insert(named_type_path(domain, type_domain.as_str(), name.as_str()));
            }
        }
        BindingType::Struct {
            name,
            domain: type_domain,
            fields,
        } => {
            if binding_type_requires_abi(binding_type) {
                let replay_name = replay_named_struct_name(name);
                names.insert(named_type_path(
                    domain,
                    type_domain.as_str(),
                    replay_name.as_str(),
                ));
            } else {
                names.insert(named_type_path(domain, type_domain.as_str(), name.as_str()));
            }

            for field in fields {
                collect_replay_type_names(domain, &field.binding_type, names);
            }
        }
        BindingType::Enum {
            name,
            domain: type_domain,
            ..
        } => {
            names.insert(named_type_path(domain, type_domain.as_str(), name.as_str()));
        }
        BindingType::TaggedUnion {
            name,
            domain: type_domain,
            variants,
        } => {
            let replay_name = replay_named_struct_name(name);
            names.insert(named_type_path(
                domain,
                type_domain.as_str(),
                replay_name.as_str(),
            ));

            for variant in variants {
                collect_replay_type_names(domain, &variant.binding_type, names);
            }
        }
        _ => {}
    }
}

/// Build the generated replay struct name for one named ABI struct.
fn replay_named_struct_name(name: &str) -> String {
    format!("{name}ReplayRecord")
}

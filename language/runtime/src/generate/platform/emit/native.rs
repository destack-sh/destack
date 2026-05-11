use crate::platform::model::{
    BindingType, CatalogBindingProvider, CatalogBindingReplayKind, CatalogEffectClass,
    CatalogEntropyKind, CatalogReplayPolicy,
};

use super::binding::BindingWriter;

impl<'spec, 'output> BindingWriter<'spec, 'output> {
    /// Render the native binding set for a domain.
    pub(super) fn write_native_set(&mut self) {
        let codegen = self.codegen();
        let domain = codegen.module();
        let output = &mut self.output;
        let consts = &self.spec.consts;
        let native_set_name = codegen.native_set_name();

        // render the binding set descriptor
        output.push_str(&format!("/// Native binding set for {domain}.\n"));
        output.push_str(&format!(
            "pub(crate) const {native_set_name}: NativeBindingSet = NativeBindingSet {{\n"
        ));
        output.push_str(&format!("    name: \"{domain}\",\n"));
        output.push_str("    bindings: &[\n");
        for binding in consts {
            let native_fn = codegen.native_fn_name(binding.extern_name);
            output.push_str(&format!(
                "        NativeBinding::new({}, \"{}\", {native_fn} as *const ()),\n",
                binding.const_name, binding.extern_name
            ));
        }
        output.push_str("    ],\n};\n\n");
    }

    /// Render native export wrappers for a domain.
    pub(super) fn write_native_exports(&mut self) {
        let codegen = self.codegen();
        let domain = codegen.module();
        let output = &mut self.output;
        let consts = &self.spec.consts;
        output.push_str(&format!(
            "/// Native export wrappers for {domain} bindings.\n"
        ));
        for binding in consts {
            let entry = binding.entry;
            let export_fn_name = codegen.native_fn_name(binding.extern_name);
            let implementation_fn_name = &binding.implementation_fn_name;
            let replay_fn_name = codegen.native_replay_fn_name(binding.extern_name);
            let is_recordable = matches!(
                entry.effect_class,
                CatalogEffectClass::External {
                    replay: CatalogReplayPolicy::Recordable
                }
            );
            let replay_kind = entry.replay_kind;
            let uses_binding_replay =
                is_recordable && replay_kind == CatalogBindingReplayKind::BindingCall;
            let mut params = Vec::new();
            let mut args = Vec::new();
            if entry.return_binding != BindingType::Void {
                let out_type = codegen.native_type_for_binding(&entry.return_binding);
                params.push(format!("out: *mut {out_type}"));
                args.push("out".to_string());
            }
            for (index, param) in entry.parameters.iter().enumerate() {
                let name = codegen.sanitize_param_name(&param.name, index);
                let ty = codegen.native_type_for_binding(&param.binding_type);
                params.push(format!("{name}: {ty}"));
                args.push(name);
            }

            output.push_str(&format!(
                "#[unsafe(export_name = \"{}\")]\n",
                binding.extern_name
            ));
            output.push_str(&format!(
                "pub(crate) unsafe extern \"C\" fn {export_fn_name}(\n"
            ));
            for param in &params {
                output.push_str(&format!("    {param},\n"));
            }
            output.push_str(") -> RuntimeStatus {\n");
            output.push_str("    native_call(|context| {\n");
            if entry.return_binding != BindingType::Void {
                output.push_str("        if out.is_null() {\n");
                output.push_str(
                    "            return Err(RuntimeError::from(PlatformError::null_pointer(\"out\")).boxed());\n",
                );
                output.push_str("        }\n");
            }
            if !args.is_empty() {
                let args_refs = args
                    .iter()
                    .map(|name| format!("&{name}"))
                    .collect::<Vec<_>>();
                if args_refs.len() == 1 {
                    output.push_str(&format!("        let _ = {};\n", args_refs[0]));
                } else {
                    output.push_str(&format!("        let _ = ({});\n", args_refs.join(", ")));
                }
            }
            output.push_str("\n");
            match replay_kind {
                CatalogBindingReplayKind::BindingCall => {
                    if uses_binding_replay {
                        if entry.provider == CatalogBindingProvider::Runtime {
                            output.push_str(&format!(
                                "        let _binding_hook_guard = context.on_before_binding({})?;\n",
                                binding.const_name
                            ));
                        } else {
                            output.push_str(&format!(
                                "        let (world, _binding_hook_guard) = context.on_before_binding_resolve_world({})?;\n",
                                binding.const_name
                            ));
                        }
                        if args.is_empty() {
                            if entry.provider == CatalogBindingProvider::Runtime {
                                output.push_str(&format!("        {replay_fn_name}(context)\n"));
                            } else {
                                output.push_str(&format!(
                                    "        {replay_fn_name}(context, world)\n"
                                ));
                            }
                        } else if entry.provider == CatalogBindingProvider::Runtime {
                            output.push_str(&format!(
                                "        {replay_fn_name}(context, {})\n",
                                args.join(", ")
                            ));
                        } else {
                            output.push_str(&format!(
                                "        {replay_fn_name}(context, world, {})\n",
                                args.join(", ")
                            ));
                        }
                    } else {
                        let call = codegen.render_native_world_dispatch(
                            &binding.const_name,
                            entry.provider,
                            binding.entry.simulation,
                            implementation_fn_name,
                            &args,
                        );
                        output.push_str(&format!("        {call}\n"));
                    }
                }
                CatalogBindingReplayKind::Entropy(kind) => {
                    let kind_value = codegen.render_entropy_kind(kind);
                    let subject_expr = format!("context.entropy_subject({})", binding.const_name);

                    match kind {
                        CatalogEntropyKind::TimeReadMonotonic
                        | CatalogEntropyKind::TimeReadWall => {
                            output.push_str("        let value = context.trace().run_time_read(\n");
                            output.push_str(&format!("            {kind_value},\n"));
                            output.push_str(&format!("            {subject_expr},\n"));
                            output.push_str("            || context.on_time_read(),\n");
                            output.push_str("            || {\n");
                            let call = codegen.render_native_world_dispatch(
                                &binding.const_name,
                                entry.provider,
                                binding.entry.simulation,
                                implementation_fn_name,
                                &args,
                            );
                            output.push_str(&format!("            {call}?;\n"));
                            output.push_str("            unsafe { Ok(*out) }\n");
                            output.push_str("            },\n");
                            output.push_str("        )?;\n");
                            output.push_str("        unsafe { *out = value; }\n");
                            output.push_str("        Ok(())\n");
                        }
                        CatalogEntropyKind::RandomStreamCreate => {
                            output.push_str(
                                "        let value = context.trace().run_random_stream(\n",
                            );
                            output.push_str(&format!("            {subject_expr},\n"));
                            output.push_str("            || context.on_random_read(),\n");
                            output.push_str("            || {\n");
                            let call = codegen.render_native_world_dispatch(
                                &binding.const_name,
                                entry.provider,
                                binding.entry.simulation,
                                implementation_fn_name,
                                &args,
                            );
                            output.push_str(&format!("            {call}?;\n"));
                            output.push_str("            unsafe { Ok((*out).0) }\n");
                            output.push_str("            },\n");
                            output.push_str("        )?;\n");
                            output.push_str("        unsafe { *out = RandomStream(value); }\n");
                            output.push_str("        Ok(())\n");
                        }
                        CatalogEntropyKind::RandomReadU64 => {
                            let stream_expr = Self::random_stream_id_expr(
                                &codegen,
                                entry,
                                "context.random_stream_id()",
                            );
                            output
                                .push_str("        let value = context.trace().run_random_u64(\n");
                            output.push_str(&format!("            {subject_expr},\n"));
                            output.push_str(&format!("            {stream_expr},\n"));
                            output.push_str("            || context.on_random_read(),\n");
                            output.push_str("            || {\n");
                            let call = codegen.render_native_world_dispatch(
                                &binding.const_name,
                                entry.provider,
                                binding.entry.simulation,
                                implementation_fn_name,
                                &args,
                            );
                            output.push_str(&format!("            {call}?;\n"));
                            output.push_str("            unsafe { Ok(*out) }\n");
                            output.push_str("            },\n");
                            output.push_str("        )?;\n");
                            output.push_str("        unsafe { *out = value; }\n");
                            output.push_str("        Ok(())\n");
                        }
                        CatalogEntropyKind::RandomReadBytes => {
                            let stream_expr = Self::random_stream_id_expr(
                                &codegen,
                                entry,
                                "context.random_stream_id()",
                            );
                            let buffer_name = Self::random_bytes_buffer_arg(&codegen, entry);
                            output.push_str("        context.trace().run_random_bytes(\n");
                            output.push_str(&format!("            {subject_expr},\n"));
                            output.push_str(&format!("            {stream_expr},\n"));
                            output.push_str(&format!("            {buffer_name}.len,\n"));
                            output.push_str("            || context.on_random_read(),\n");
                            output.push_str("            || {\n");
                            let call = codegen.render_native_world_dispatch(
                                &binding.const_name,
                                entry.provider,
                                binding.entry.simulation,
                                implementation_fn_name,
                                &args,
                            );
                            output.push_str(&format!("                {call}\n"));
                            output.push_str("            },\n");
                            output.push_str(&format!(
                                "            || {{\n                let slice = unsafe {{ {buffer_name}.as_slice()? }};\n                Ok(slice.to_vec())\n            }},\n"
                            ));
                            output.push_str(&format!(
                                "            |bytes| {{\n                let slice = unsafe {{ {buffer_name}.as_mut_slice()? }};\n                if slice.len() != bytes.len() {{\n                    return Err(RuntimeError::from(PlatformError::invalid_argument_value(\n                        \"buffer\",\n                        \"buffer length mismatch\",\n                    ))\n                    .boxed());\n                }}\n                slice.copy_from_slice(&bytes);\n                Ok(())\n            }},\n"
                            ));
                            output.push_str("        )\n");
                        }
                    }
                }
            }
            output.push_str("    })\n");
            output.push_str("}\n\n");
        }
    }
}

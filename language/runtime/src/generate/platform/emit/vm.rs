use crate::platform::model::{
    CatalogBindingProvider, CatalogBindingReplayKind, CatalogEffectClass, CatalogEntropyKind,
    CatalogReplayPolicy,
};

use super::binding::BindingWriter;
use super::{
    VmDecodeUsage, binding_type_requires_context_for_decode,
    binding_type_requires_context_for_encode,
};

impl<'spec, 'output> BindingWriter<'spec, 'output> {
    /// Render VM decode helper functions for a binding domain.
    pub(super) fn write_vm_decode_helpers(&mut self, usage: &VmDecodeUsage) {
        let output = &mut self.output;

        output.push_str("/// Read a positional argument value.\n");
        output.push_str("#[allow(dead_code)]\n");
        output.push_str("fn arg_value(\n");
        output.push_str("    args: &[vm::Word],\n");
        output.push_str("    index: usize,\n");
        output.push_str("    name: &'static str,\n");
        output.push_str("    expected: &'static str,\n");
        output.push_str(") -> RuntimeResult<vm::Word> {\n");
        output.push_str("    let value = args.get(index).copied().ok_or_else(|| {\n");
        output.push_str("        RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed()\n");
        output.push_str("    })?;\n");
        output.push_str("\n");
        output.push_str("    Ok(value)\n");
        output.push_str("}\n\n");

        if usage.uses_bool {
            output.push_str("/// Decode a boolean argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_bool(\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    _name: &'static str,\n");
            output.push_str("    _expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<bool> {\n");
            output.push_str("    Ok(value.as_bool())\n");
            output.push_str("}\n\n");
        }

        if usage.uses_int8 || usage.uses_int16 || usage.uses_int32 || usage.uses_int64 {
            output.push_str("/// Decode a signed integer argument with an explicit width.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_int(\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    _name: &'static str,\n");
            output.push_str("    _expected: &'static str,\n");
            output.push_str("    _bits: u8,\n");
            output.push_str(") -> RuntimeResult<i64> {\n");
            output.push_str("    Ok(value.as_int())\n");
            output.push_str("}\n\n");
        }

        if usage.uses_uint8 || usage.uses_uint16 || usage.uses_uint32 || usage.uses_uint64 {
            output.push_str("/// Decode an unsigned integer argument with an explicit width.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_uint(\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    _name: &'static str,\n");
            output.push_str("    _expected: &'static str,\n");
            output.push_str("    _bits: u8,\n");
            output.push_str(") -> RuntimeResult<u64> {\n");
            output.push_str("    Ok(value.as_uint())\n");
            output.push_str("}\n\n");
        }

        if usage.uses_int8 {
            output.push_str("/// Decode an i8 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_int8(\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<i8> {\n");
            output.push_str("    Ok(decode_int(value, name, expected, 8)? as i8)\n");
            output.push_str("}\n\n");
        }
        if usage.uses_int16 {
            output.push_str("/// Decode an i16 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_int16(\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<i16> {\n");
            output.push_str("    Ok(decode_int(value, name, expected, 16)? as i16)\n");
            output.push_str("}\n\n");
        }
        if usage.uses_int32 {
            output.push_str("/// Decode an i32 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_int32(\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<i32> {\n");
            output.push_str("    Ok(decode_int(value, name, expected, 32)? as i32)\n");
            output.push_str("}\n\n");
        }
        if usage.uses_int64 {
            output.push_str("/// Decode an i64 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_int64(\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<i64> {\n");
            output.push_str("    decode_int(value, name, expected, 64)\n");
            output.push_str("}\n\n");
        }

        if usage.uses_uint8 {
            output.push_str("/// Decode a u8 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_uint8(\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<u8> {\n");
            output.push_str("    Ok(decode_uint(value, name, expected, 8)? as u8)\n");
            output.push_str("}\n\n");
        }
        if usage.uses_uint16 {
            output.push_str("/// Decode a u16 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_uint16(\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<u16> {\n");
            output.push_str("    Ok(decode_uint(value, name, expected, 16)? as u16)\n");
            output.push_str("}\n\n");
        }
        if usage.uses_uint32 {
            output.push_str("/// Decode a u32 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_uint32(\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<u32> {\n");
            output.push_str("    Ok(decode_uint(value, name, expected, 32)? as u32)\n");
            output.push_str("}\n\n");
        }
        if usage.uses_uint64 {
            output.push_str("/// Decode a u64 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_uint64(\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<u64> {\n");
            output.push_str("    decode_uint(value, name, expected, 64)\n");
            output.push_str("}\n\n");
        }

        if usage.uses_float32 {
            output.push_str("/// Decode an f32 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_float32(\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<f32> {\n");
            output.push_str("    value\n");
            output.push_str("        .as_float32()\n");
            output.push_str("        .ok_or_else(|| RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed())\n");
            output.push_str("}\n\n");
        }
        if usage.uses_float64 {
            output.push_str("/// Decode an f64 argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_float64(\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<f64> {\n");
            output.push_str("    value\n");
            output.push_str("        .as_float64()\n");
            output.push_str("        .ok_or_else(|| RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed())\n");
            output.push_str("}\n\n");
        }

        if usage.uses_string {
            output.push_str("/// Decode a string argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_string(\n");
            output.push_str("    context: &vm::BindingRead<'_, '_>,\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<vm::StringHandle> {\n");
            output.push_str("    context\n");
            output.push_str("        .string_handle_from_value(value)\n");
            output.push_str("        .map_err(|_| RuntimeError::from(PlatformError::invalid_argument_type(name, expected)).boxed())\n");
            output.push_str("}\n\n");
        }

        if usage.uses_slice {
            output.push_str("/// Decode a slice argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_slice<T>(\n");
            output.push_str("    context: &vm::BindingRead<'_, '_>,\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<VmSlice<T>> {\n");
            output.push_str("    VmSlice::<T>::from_value(context, value, name, expected)\n");
            output.push_str("}\n\n");
        }

        if usage.uses_array {
            output.push_str("/// Decode an array argument.\n");
            output.push_str("#[allow(dead_code)]\n");
            output.push_str("fn decode_array<T>(\n");
            output.push_str("    context: &vm::BindingRead<'_, '_>,\n");
            output.push_str("    value: vm::Word,\n");
            output.push_str("    name: &'static str,\n");
            output.push_str("    expected: &'static str,\n");
            output.push_str(") -> RuntimeResult<VmArray<T>> {\n");
            output.push_str("    VmArray::<T>::from_value(context, value, name, expected)\n");
            output.push_str("}\n\n");
        }
    }

    /// Render per-binding VM argument and result helpers.
    pub(super) fn write_vm_binding_helpers(&mut self) {
        let codegen = self.codegen();
        let output = &mut self.output;
        let bindings = self.spec.bindings;

        for (extern_name, entry) in bindings {
            let helper_base = codegen.vm_fn_name(extern_name);
            if !entry.parameters.is_empty() {
                let decode_helper = codegen.decode_helper_name(&helper_base);
                let decode_uses_context = entry
                    .parameters
                    .iter()
                    .any(|param| binding_type_requires_context_for_decode(&param.binding_type));
                let decode_context_name = if decode_uses_context {
                    "context"
                } else {
                    "_context"
                };
                output.push_str(&format!("/// Decode arguments for {}.\n", extern_name));
                output.push_str("#[inline]\n");
                output.push_str(&format!(
                "fn {decode_helper}(\n    {decode_context_name}: &mut vm::BindingContext<'_>,\n    args: &[vm::Word],\n) -> RuntimeResult<{}> {{\n",
                codegen.vm_args_tuple_type(&entry.parameters)
                ));
                if decode_uses_context {
                    output.push_str("    let context = &context.read();\n");
                }
                for (index, param) in entry.parameters.iter().enumerate() {
                    let name = codegen.sanitize_param_name(&param.name, index);
                    let expected = param
                        .type_text
                        .clone()
                        .unwrap_or_else(|| "value".to_string());
                    output.push_str(&format!(
                    "    let {name}_value = arg_value(args, {index}, \"{name}\", \"{expected}\")?;\n"
                ));
                    for line in codegen.render_decode_value_lines(
                        &name,
                        &param.binding_type,
                        &format!("{name}_value"),
                        &expected,
                    ) {
                        output.push_str(&format!("    {line}\n"));
                    }
                }
                let tuple_values = entry
                    .parameters
                    .iter()
                    .enumerate()
                    .map(|(index, param)| codegen.sanitize_param_name(&param.name, index))
                    .collect::<Vec<_>>();
                if tuple_values.len() == 1 {
                    output.push_str(&format!("    Ok(({},))\n", tuple_values[0]));
                } else {
                    output.push_str(&format!("    Ok(({}))\n", tuple_values.join(", ")));
                }
                output.push_str("}\n\n");
            }

            let encode_helper = codegen.encode_helper_name(&helper_base);
            let encode_uses_context =
                binding_type_requires_context_for_encode(&entry.return_binding);
            let encode_context_name = if encode_uses_context {
                "context"
            } else {
                "_context"
            };
            output.push_str(&format!("/// Encode the result for {}.\n", extern_name));
            output.push_str("#[inline]\n");
            output.push_str(&format!(
            "fn {encode_helper}(\n    {encode_context_name}: &mut vm::BindingContext<'_>,\n    result: RuntimeResult<{}>,\n) -> RuntimeResult<vm::Word> {{\n",
            codegen.vm_return_type(&entry.return_binding)
        ));
            if encode_uses_context {
                output.push_str("    let context = &mut context.write();\n");
            }
            for line in codegen.render_return_encode_lines(&entry.return_binding) {
                output.push_str(&format!("    {line}\n"));
            }
            output.push_str("}\n\n");
        }
    }

    /// Render the binding registration function for a domain.
    pub(super) fn write_vm_register_fn(&mut self) {
        let codegen = self.codegen();
        let domain = codegen.module();
        let output = &mut self.output;
        let consts = &self.spec.consts;
        let register_fn = codegen.register_fn_name();

        // render the registration function
        output.push_str(&format!("/// Register VM bindings for {domain}.\n"));
        output.push_str(&format!("pub(crate) fn {register_fn}(\n"));
        output.push_str("    registry: &mut BindingRegistry,\n");
        output.push_str("    isolate: &mut Isolate,\n");
        output.push_str(") {\n");
        for binding in consts {
            let decode_base_name = codegen.vm_fn_name(binding.extern_name);
            let implementation_fn_name = &binding.implementation_fn_name;
            let args_ident = if binding.entry.parameters.is_empty() {
                "_args"
            } else {
                "args"
            };
            let is_recordable = matches!(
                binding.entry.effect_class,
                CatalogEffectClass::External {
                    replay: CatalogReplayPolicy::Recordable
                }
            );
            let replay_kind = binding.entry.replay_kind;
            let uses_binding_replay =
                is_recordable && replay_kind == CatalogBindingReplayKind::BindingCall;
            let invoke_args = codegen.render_invoke_args_with_prefix(binding.entry);
            let decode_helper = codegen.decode_helper_name(&decode_base_name);
            let encode_helper = codegen.encode_helper_name(&decode_base_name);

            // emit the binding wrapper closure
            output.push_str("    {\n");
            output.push_str(&format!(
                "        binding!(registry, isolate, {}, move |context, {args_ident}| {{\n",
                binding.const_name
            ));
            output.push_str("            with_binding_call_context(|binding| {\n");
            if !binding.entry.parameters.is_empty() {
                output.push_str("                // decode args\n");
                let arg_names = binding
                    .entry
                    .parameters
                    .iter()
                    .enumerate()
                    .map(|(index, param)| codegen.sanitize_param_name(&param.name, index))
                    .collect::<Vec<_>>();
                if arg_names.len() == 1 {
                    output.push_str(&format!(
                        "                let ({},) = {decode_helper}(context, {args_ident})?;\n",
                        arg_names[0]
                    ));
                } else {
                    output.push_str(&format!(
                        "                let ({}) = {decode_helper}(context, {args_ident})?;\n",
                        arg_names.join(", ")
                    ));
                }
                output.push_str("\n");
            }
            output.push_str("                // execute binding\n");
            match replay_kind {
                CatalogBindingReplayKind::BindingCall => {
                    if uses_binding_replay {
                        let replay_fn = codegen.vm_replay_fn_name(binding.extern_name);
                        if binding.entry.provider == CatalogBindingProvider::Runtime {
                            output.push_str(&format!(
                                "                let _binding_hook_guard = binding.on_before_binding({})?;\n",
                                binding.const_name
                            ));
                            output.push_str(&format!(
                                "                {replay_fn}(binding, context{invoke_args})\n"
                            ));
                        } else {
                            output.push_str(&format!(
                                "                let (world, _binding_hook_guard) = binding.on_before_binding_resolve_world({})?;\n",
                                binding.const_name
                            ));
                            output.push_str(&format!(
                                "                {replay_fn}(binding, context, world{invoke_args})\n"
                            ));
                        }
                    } else {
                        let call = codegen.render_vm_world_dispatch(
                            &binding.const_name,
                            binding.entry.provider,
                            binding.entry.simulation,
                            implementation_fn_name,
                            &invoke_args,
                        );
                        output.push_str(&format!("                let result = {call};\n"));
                        output.push_str(&format!(
                            "                {encode_helper}(context, result)\n"
                        ));
                    }
                }
                CatalogBindingReplayKind::Entropy(kind) => {
                    let kind_value = codegen.render_entropy_kind(kind);
                    let subject_expr = format!("binding.entropy_subject({})", binding.const_name);

                    match kind {
                        CatalogEntropyKind::TimeReadMonotonic
                        | CatalogEntropyKind::TimeReadWall => {
                            let call = codegen.render_vm_world_dispatch(
                                &binding.const_name,
                                binding.entry.provider,
                                binding.entry.simulation,
                                implementation_fn_name,
                                &invoke_args,
                            );
                            output.push_str(
                                "                let result = binding.trace().run_time_read(\n",
                            );
                            output.push_str(&format!("                    {kind_value},\n"));
                            output.push_str(&format!("                    {subject_expr},\n"));
                            output.push_str("                    || binding.on_time_read(),\n");
                            output.push_str("                    || {\n");
                            output.push_str(&format!("                    {call}\n"));
                            output.push_str("                    },\n");
                            output.push_str("                );\n");
                            output.push_str(&format!(
                                "                {encode_helper}(context, result)\n"
                            ));
                        }
                        CatalogEntropyKind::RandomStreamCreate => {
                            let call = codegen.render_vm_world_dispatch(
                                &binding.const_name,
                                binding.entry.provider,
                                binding.entry.simulation,
                                implementation_fn_name,
                                &invoke_args,
                            );
                            output.push_str(
                                "                let result = binding.trace().run_random_stream(\n",
                            );
                            output.push_str(&format!("                    {subject_expr},\n"));
                            output.push_str("                    || binding.on_random_read(),\n");
                            output.push_str("                    || {\n");
                            output.push_str(&format!(
                                "                    {call}.map(|stream| stream.0)\n"
                            ));
                            output.push_str("                    },\n");
                            output.push_str("                );\n");
                            output.push_str(
                                "                let result = result.map(RandomStream);\n",
                            );
                            output.push_str(&format!(
                                "                {encode_helper}(context, result)\n"
                            ));
                        }
                        CatalogEntropyKind::RandomReadU64 => {
                            let stream_expr = Self::random_stream_id_expr(
                                &codegen,
                                &binding.entry,
                                "binding.random_stream_id()",
                            );
                            let call = codegen.render_vm_world_dispatch(
                                &binding.const_name,
                                binding.entry.provider,
                                binding.entry.simulation,
                                implementation_fn_name,
                                &invoke_args,
                            );
                            output.push_str(
                                "                let result = binding.trace().run_random_u64(\n",
                            );
                            output.push_str(&format!("                    {subject_expr},\n"));
                            output.push_str(&format!("                    {stream_expr},\n"));
                            output.push_str("                    || binding.on_random_read(),\n");
                            output.push_str("                    || {\n");
                            output.push_str(&format!("                    {call}\n"));
                            output.push_str("                    },\n");
                            output.push_str("                );\n");
                            output.push_str(&format!(
                                "                {encode_helper}(context, result)\n"
                            ));
                        }
                        CatalogEntropyKind::RandomReadBytes => {
                            let stream_expr = Self::random_stream_id_expr(
                                &codegen,
                                &binding.entry,
                                "binding.random_stream_id()",
                            );
                            let buffer_name =
                                Self::random_bytes_buffer_arg(&codegen, &binding.entry);
                            output.push_str(
                                "                let context_ptr = context as *mut vm::BindingContext<'_>;\n",
                            );
                            output.push_str(
                                "                let result = binding.trace().run_random_bytes(\n",
                            );
                            output.push_str(&format!("                    {subject_expr},\n"));
                            output.push_str(&format!("                    {stream_expr},\n"));
                            output.push_str(&format!("                    {buffer_name}.len,\n"));
                            output.push_str("                    || binding.on_random_read(),\n");
                            output.push_str("                    || {\n");
                            let call = codegen
                                .render_vm_world_dispatch(
                                    &binding.const_name,
                                    binding.entry.provider,
                                    binding.entry.simulation,
                                    implementation_fn_name,
                                    &invoke_args,
                                )
                                .replace("context", "&mut *context_ptr");
                            output.push_str(&format!(
                                "                        unsafe {{ {call} }}\n"
                            ));
                            output.push_str("                    },\n");
                            output.push_str(&format!("                    || unsafe {{\n"));
                            output.push_str(
                                "                        let context = (&*context_ptr).read();\n",
                            );
                            output.push_str(&format!(
                                "                        {buffer_name}.read_bytes(&context)\n"
                            ));
                            output.push_str("                    },\n");
                            output.push_str(&format!("                    |bytes| unsafe {{\n"));
                            output.push_str(
                                "                        let mut context = (&mut *context_ptr).write();\n",
                            );
                            output.push_str(&format!(
                                "                        {buffer_name}.write_bytes(&mut context, &bytes)\n"
                            ));
                            output.push_str("                    },\n");
                            output.push_str("                );\n");
                            output.push_str(&format!(
                                "                {encode_helper}(context, result)\n"
                            ));
                        }
                    }
                }
            }
            output.push_str("            })\n");
            output.push_str("            .map_err(Into::into)\n");
            output.push_str("        });\n");
            output.push_str("    }\n");
        }
        output.push_str("}\n\n");
    }

    /// Render the VM binding set for a domain.
    pub(super) fn write_vm_set(&mut self) {
        let codegen = self.codegen();
        let output = &mut self.output;
        let domain = codegen.module();
        let register_fn = codegen.register_fn_name();
        let set_name = codegen.vm_set_name();
        let install_fn = codegen.install_fn_name();
        let register_aggregate_types_fn = codegen.register_aggregate_types_fn_name();

        output.push_str(&format!("/// Install VM bindings for {domain}.\n"));
        output.push_str(&format!("pub(crate) fn {install_fn}(\n"));
        output.push_str("    registry: &mut BindingRegistry,\n");
        output.push_str("    isolate: &mut Isolate,\n");
        output.push_str(") -> vm::Result<()> {\n");
        output.push_str(&format!(
            "    super::abi_generated::{register_aggregate_types_fn}(isolate)?;\n"
        ));
        output.push_str(&format!("    {register_fn}(registry, isolate);\n"));
        output.push_str("    Ok(())\n");
        output.push_str("}\n\n");

        output.push_str(&format!(
            "vm_binding_set!(pub(crate) {set_name}, \"{domain}\", {install_fn});\n\n"
        ));
    }
}

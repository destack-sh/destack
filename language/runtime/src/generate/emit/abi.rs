use super::*;
use crate::model::BindingTaggedUnionVariant;

/// Collect ABI types grouped by their owning domain.
pub(crate) fn collect_domain_abi_types(
    catalog: &BindingCatalog,
) -> BTreeMap<String, DomainAbiTypes> {
    let mut domains: BTreeMap<String, DomainAbiTypes> = BTreeMap::new();
    for bindings in catalog.values() {
        for entry in bindings.values() {
            collect_domain_types_for_binding(&entry.return_binding, &mut domains);
            for param in &entry.parameters {
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
        BindingType::TaggedUnion {
            name,
            domain,
            variants,
        } => {
            let entry = domains.entry(domain.clone()).or_default();
            entry
                .tagged_unions
                .entry(name.clone())
                .or_insert(binding_type.clone());
            for variant in variants {
                collect_domain_types_for_binding(&variant.binding_type, domains);
            }
        }
        BindingType::Slice(inner) | BindingType::Array(inner) => {
            collect_domain_types_for_binding(inner, domains);
        }
        BindingType::Optional(inner) => {
            collect_domain_types_for_binding(inner, domains);
        }
        _ => {}
    }
}

/// Remove unused local domain imports from a generated binding file.
pub(super) fn trim_unused_domain_imports(domain: &str, output: &mut String) {
    let prefix = format!("use crate::platform::{domain}::{{");
    let Some(start) = output.find(&prefix) else {
        return;
    };
    let Some(end) = output[start..].find("};") else {
        return;
    };
    let end = start + end + 2;
    let import_line = &output[start..end];
    if import_line.contains("native as platform_native") {
        return;
    }
    let Some(open_brace) = import_line.find('{') else {
        return;
    };
    let Some(close_brace) = import_line.rfind('}') else {
        return;
    };
    let names = &import_line[open_brace + 1..close_brace];
    let mut kept = Vec::new();
    for name in names
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        if name_used_in_output(output, name, end) {
            kept.push(name);
        }
    }
    let new_line = if kept.is_empty() {
        String::new()
    } else {
        format!("use crate::platform::{domain}::{{{}}};\n", kept.join(", "))
    };
    output.replace_range(start..end, &new_line);
}

/// Return true if a name appears outside the import line.
fn name_used_in_output(output: &str, name: &str, search_start: usize) -> bool {
    let mut index = search_start;
    while let Some(position) = output[index..].find(name) {
        let position = index + position;
        let before = output[..position].chars().rev().next();
        let after = output[position + name.len()..].chars().next();
        if is_word_boundary(before) && is_word_boundary(after) {
            return true;
        }
        index = position + name.len();
    }
    false
}

/// Return true if the character is a word boundary for type identifiers.
fn is_word_boundary(ch: Option<char>) -> bool {
    match ch {
        None => true,
        Some(value) => !(value.is_ascii_alphanumeric() || value == '_'),
    }
}

/// Compute deterministic and insertion-stable tags for one tagged union.
fn tagged_union_variant_tags(union_name: &str, variants: &[BindingTaggedUnionVariant]) -> Vec<u32> {
    let mut tags = Vec::with_capacity(variants.len());
    let mut seen = BTreeMap::<u32, String>::new();
    for variant in variants {
        let tag = tagged_union_variant_tag(union_name, variant.name.as_str());
        if let Some(existing) = seen.insert(tag, variant.name.clone()) {
            panic!(
                "tagged union {union_name} has colliding stable tags for variants {existing} and {}",
                variant.name
            );
        }
        tags.push(tag);
    }

    tags
}

/// Render ABI type definitions for a runtime domain.
pub(crate) fn render_abi_types(domain: &str, types: &DomainAbiTypes) -> String {
    let structs = &types.structs;
    let newtypes = &types.newtypes;
    let enums = &types.enums;
    let tagged_unions = &types.tagged_unions;
    let mut output = String::new();
    let needs_abi = newtypes.values().any(|binding_type| {
        let BindingType::Newtype { inner, .. } = binding_type else {
            return false;
        };
        binding_type_requires_abi(inner)
    }) || structs
        .values()
        .any(|binding_type| binding_type_requires_abi(binding_type))
        || tagged_unions
            .values()
            .any(|binding_type| binding_type_requires_abi(binding_type));
    let mut type_domains = BTreeSet::new();
    for binding_type in newtypes.values() {
        collect_binding_type_domains(domain, binding_type, &mut type_domains);
    }
    for binding_type in structs.values() {
        collect_binding_type_domains(domain, binding_type, &mut type_domains);
    }
    for binding_type in enums.values() {
        collect_binding_type_domains(domain, binding_type, &mut type_domains);
    }
    for binding_type in tagged_unions.values() {
        collect_binding_type_domains(domain, binding_type, &mut type_domains);
    }

    output.push_str("// generated by generate-bindings: do not edit\n\n");
    output.push_str("#![allow(dead_code)]\n");
    output.push_str("#![allow(unused_imports)]\n");
    output.push_str("#![allow(unreachable_pub)]\n\n");
    if needs_abi {
        output.push_str("use crate::platform::abi::{BindingAbi, NativeAbi, VmAbi};\n");
    }
    let needs_vm_value_codec = newtypes.values().any(|binding_type| {
        let BindingType::Newtype { inner, .. } = binding_type else {
            return false;
        };
        !binding_type_requires_abi(inner)
    }) || !enums.is_empty();
    let needs_vm_aggregate_codec = !structs.is_empty()
        || newtypes.values().any(|binding_type| {
            let BindingType::Newtype { inner, .. } = binding_type else {
                return false;
            };
            binding_type_requires_abi(inner)
        })
        || !tagged_unions.is_empty();
    if needs_vm_value_codec || needs_vm_aggregate_codec {
        output.push_str("use crate::diagnostic::RuntimeError;\n");
        output.push_str("use crate::diagnostic::RuntimeResult;\n");
        output.push_str("use crate::platform::PlatformError as AbiPlatformError;\n");
    }
    if needs_vm_value_codec {
        output.push_str("use crate::platform::VmValueCodec;\n");
    }
    if needs_vm_aggregate_codec {
        output.push_str("use crate::platform::VmAggregateCodec;\n");
        output.push_str("use crate::platform::{VmArray, VmSlice};\n");
    }
    if needs_vm_value_codec || needs_vm_aggregate_codec {
        output.push_str("use destack_vm as vm;\n");
    }
    if !newtypes.is_empty() || !enums.is_empty() || !structs.is_empty() || !tagged_unions.is_empty()
    {
        output.push_str("use serde::{Deserialize, Serialize};\n");
    }
    if !type_domains.is_empty() {
        let imports = type_domains
            .iter()
            .map(|domain| domain.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        output.push_str(&format!("use crate::platform::{{{imports}}};\n"));
    }
    let mut alias_domains = BTreeSet::new();
    alias_domains.insert(domain.to_string());
    alias_domains.extend(type_domains.iter().cloned());
    for alias_domain in alias_domains {
        let alias = platform_domain_alias(alias_domain.as_str());
        output.push_str(&format!(
            "use crate::platform::{alias_domain} as {alias};\n"
        ));
    }
    if needs_abi
        || !newtypes.is_empty()
        || !enums.is_empty()
        || !structs.is_empty()
        || !tagged_unions.is_empty()
    {
        output.push_str("\n");
    }

    for (name, binding_type) in newtypes {
        let BindingType::Newtype { inner, .. } = binding_type else {
            panic!("expected newtype binding for {name}");
        };
        let requires_abi = binding_type_requires_abi(inner);
        output.push_str(&format!("/// ABI newtype for {name}.\n"));
        if requires_abi {
            output.push_str("#[repr(C)]\n");
            output.push_str("#[derive(Debug, Clone, Copy)]\n");
            output.push_str(&format!("pub struct {name}Abi<A: BindingAbi>(\n"));
            output.push_str(&format!(
                "    /// Inner value.\n    pub {},\n",
                abi_struct_field_type(domain, inner)
            ));
            output.push_str(");\n\n");
            output.push_str(&format!("pub type {name} = {name}Abi<NativeAbi>;\n"));
            output.push_str(&format!("pub type {name}Vm = {name}Abi<VmAbi>;\n\n"));
            let inner_vm_type = vm_type_for_binding(domain, inner);
            output.push_str(&format!("impl VmAggregateCodec for {name}Abi<VmAbi> {{\n"));
            output.push_str(
                "    fn decode_with_context(context: &vm::ExternalCallContext<'_>, value: vm::Value) -> RuntimeResult<Self> {\n",
            );
            output.push_str(&format!(
                "        Ok(Self(<{inner_vm_type} as VmAggregateCodec>::decode_with_context(context, value)?))\n"
            ));
            output.push_str("    }\n\n");
            output.push_str(
                "    fn encode_with_context(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<vm::Value> {\n",
            );
            output.push_str(&format!(
                "        <{inner_vm_type} as VmAggregateCodec>::encode_with_context(self.0, context)\n"
            ));
            output.push_str("    }\n");
            output.push_str("}\n\n");
            continue;
        }

        let inner_type = abi_newtype_inner_type(domain, inner);
        output.push_str("#[repr(transparent)]\n");
        output.push_str(
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\n",
        );
        output.push_str(&format!("pub struct {name}(\n"));
        output.push_str(&format!("    /// Inner value.\n    pub {inner_type},\n"));
        output.push_str(");\n\n");
        output.push_str(&format!("pub type {name}Vm = {name};\n\n"));

        output.push_str(&format!("impl VmValueCodec for {name} {{\n"));
        output.push_str("    fn decode(value: vm::Value) -> RuntimeResult<Self> {\n");
        output.push_str(&format!(
            "        Ok(Self(<{inner_type} as VmValueCodec>::decode(value)?))\n"
        ));
        output.push_str("    }\n\n");
        output.push_str("    fn encode(self) -> vm::Value {\n");
        output.push_str(&format!(
            "        <{inner_type} as VmValueCodec>::encode(self.0)\n"
        ));
        output.push_str("    }\n");
        output.push_str("}\n\n");
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
        output.push_str(
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\n",
        );
        output.push_str(&format!("pub enum {name} {{\n"));
        for variant in variants {
            if let BindingEnumValue::Int(value) = variant.value {
                output.push_str(&format!("    /// {0}.\n    {0} = {value},\n", variant.name));
            }
        }
        output.push_str("}\n\n");

        if matches!(backing, EnumBackingType::Int(_)) {
            let backing_type = enum_backing_rust_type(*backing);
            let mut arms = Vec::new();
            for variant in variants {
                if let BindingEnumValue::Int(value) = variant.value {
                    let literal = format!("{value}{backing_type}");
                    arms.push(format!("{literal} => Self::{}", variant.name));
                }
            }
            let arms = arms.join(", ");
            output.push_str(&format!("impl VmValueCodec for {name} {{\n"));
            output.push_str("    fn decode(value: vm::Value) -> RuntimeResult<Self> {\n");
            output.push_str(&format!(
                "        let raw = <{backing_type} as VmValueCodec>::decode(value)?;\n"
            ));
            output.push_str("        let decoded = match raw {\n");
            output.push_str(&format!("            {arms},\n"));
            output.push_str(&format!(
                "            _ => return Err(RuntimeError::from(AbiPlatformError::invalid_argument_value(\"value\", \"unknown {name} value\")).boxed()),\n"
            ));
            output.push_str("        };\n");
            output.push_str("        Ok(decoded)\n");
            output.push_str("    }\n\n");
            output.push_str("    fn encode(self) -> vm::Value {\n");
            output.push_str(&format!(
                "        <{backing_type} as VmValueCodec>::encode(self as {backing_type})\n"
            ));
            output.push_str("    }\n");
            output.push_str("}\n\n");
        }
    }

    for (union_name, binding_type) in tagged_unions {
        let BindingType::TaggedUnion { variants, .. } = binding_type else {
            panic!("expected tagged union binding for {union_name}");
        };
        let variant_tags = tagged_union_variant_tags(union_name, variants);
        let requires_abi = binding_type_requires_abi(binding_type);
        output.push_str(&format!("/// ABI tagged union for {union_name}.\n"));
        if requires_abi {
            output.push_str(&format!("pub enum {union_name}Abi<A: BindingAbi> {{\n"));
        } else {
            output.push_str("#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]\n");
            output.push_str(&format!("pub enum {union_name} {{\n"));
        }
        for variant in variants {
            let variant_type = if requires_abi {
                abi_struct_field_type(domain, &variant.binding_type)
            } else {
                native_type_for_binding(domain, &variant.binding_type)
            };
            output.push_str(&format!("    /// {} variant.\n", variant.name));
            output.push_str(&format!("    {}({variant_type}),\n", variant.name));
        }
        output.push_str("}\n\n");

        if requires_abi {
            output.push_str(&format!(
                "pub type {union_name} = {union_name}Abi<NativeAbi>;\n"
            ));
            output.push_str(&format!(
                "pub type {union_name}Vm = {union_name}Abi<VmAbi>;\n\n"
            ));
            output.push_str(&format!(
                "impl<A: BindingAbi> std::fmt::Debug for {union_name}Abi<A> {{\n"
            ));
            output.push_str("    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n");
            output.push_str(&format!(
                "        formatter.debug_tuple(\"{union_name}Abi\").finish()\n"
            ));
            output.push_str("    }\n");
            output.push_str("}\n\n");
            output.push_str(&format!("impl Copy for {union_name}Abi<NativeAbi> {{}}\n"));
            output.push_str(&format!(
                "impl Clone for {union_name}Abi<NativeAbi> {{\n    fn clone(&self) -> Self {{ *self }}\n}}\n"
            ));
            output.push_str(&format!("impl Copy for {union_name}Abi<VmAbi> {{}}\n"));
            output.push_str(&format!(
                "impl Clone for {union_name}Abi<VmAbi> {{\n    fn clone(&self) -> Self {{ *self }}\n}}\n\n"
            ));
        } else {
            output.push_str(&format!("pub type {union_name}Vm = {union_name};\n\n"));
        }

        let vm_union_type = if requires_abi {
            format!("{union_name}Abi<VmAbi>")
        } else {
            union_name.clone()
        };
        output.push_str(&format!("impl VmAggregateCodec for {vm_union_type} {{\n"));
        output.push_str(
            "    fn decode_with_context(context: &vm::ExternalCallContext<'_>, value: vm::Value) -> RuntimeResult<Self> {\n",
        );
        output.push_str("        if value.tag() != vm::ValueTag::Aggregate {\n");
        output.push_str(&format!(
            "            return Err(RuntimeError::from(AbiPlatformError::invalid_argument_type(\"value\", \"{union_name}\")).boxed());\n"
        ));
        output.push_str("        }\n");
        output.push_str(
            "        let slots = context.aggregate_slots(value).map_err(|error| RuntimeError::from(error).boxed())?;\n",
        );
        output.push_str("        if slots.len() != 2 {\n");
        output.push_str(
            "            return Err(RuntimeError::from(AbiPlatformError::invalid_argument_value(\"value\", \"expected 2 fields\")).boxed());\n",
        );
        output.push_str("        }\n");
        output.push_str(
            "        let tag = <u32 as VmAggregateCodec>::decode_with_context(context, slots[0])?;\n",
        );
        output.push_str("        let decoded = match tag {\n");
        for (variant, tag) in variants.iter().zip(variant_tags.iter().copied()) {
            let variant_type = vm_type_for_binding(domain, &variant.binding_type);
            output.push_str(&format!(
                "            {tag}u32 => Self::{}(<{variant_type} as VmAggregateCodec>::decode_with_context(context, slots[1])?),\n",
                variant.name
            ));
        }
        output.push_str(&format!(
            "            _ => return Err(RuntimeError::from(AbiPlatformError::invalid_argument_value(\"value\", \"unknown {union_name} tag\")).boxed()),\n"
        ));
        output.push_str("        };\n");
        output.push_str("        Ok(decoded)\n");
        output.push_str("    }\n\n");
        output.push_str(
            "    fn encode_with_context(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<vm::Value> {\n",
        );
        output.push_str("        let slots = match self {\n");
        for (variant, tag) in variants.iter().zip(variant_tags.iter().copied()) {
            let variant_type = vm_type_for_binding(domain, &variant.binding_type);
            output.push_str(&format!(
                "            Self::{}(value) => {{\n",
                variant.name
            ));
            output.push_str(&format!(
                "                let tag_value = <u32 as VmAggregateCodec>::encode_with_context({tag}u32, context)?;\n"
            ));
            output.push_str(&format!(
                "                let payload_value = <{variant_type} as VmAggregateCodec>::encode_with_context(value, context)?;\n"
            ));
            output.push_str("                vec![tag_value, payload_value]\n");
            output.push_str("            }\n");
        }
        output.push_str("        };\n");
        output.push_str("        Ok(context.allocate_aggregate(slots))\n");
        output.push_str("    }\n");
        output.push_str("}\n\n");
    }

    for (struct_name, binding_type) in structs {
        let BindingType::Struct { fields, .. } = binding_type else {
            panic!("expected struct binding for {struct_name}");
        };
        let requires_abi = binding_type_requires_abi(binding_type);
        output.push_str(&format!("/// ABI struct for {struct_name}.\n"));
        output.push_str("#[repr(C)]\n");
        if requires_abi {
            output.push_str(&format!("pub struct {struct_name}Abi<A: BindingAbi> {{\n"));
        } else {
            output.push_str("#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]\n");
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
            output.push_str(&format!(
                "impl<A: BindingAbi> std::fmt::Debug for {struct_name}Abi<A> {{\n"
            ));
            output.push_str("    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n");
            output.push_str(&format!(
                "        formatter.debug_struct(\"{struct_name}Abi\").finish_non_exhaustive()\n"
            ));
            output.push_str("    }\n");
            output.push_str("}\n\n");
            output.push_str(&format!("impl Copy for {struct_name}Abi<NativeAbi> {{}}\n"));
            output.push_str(&format!(
                "impl Clone for {struct_name}Abi<NativeAbi> {{\n    fn clone(&self) -> Self {{ *self }}\n}}\n"
            ));
            output.push_str(&format!("impl Copy for {struct_name}Abi<VmAbi> {{}}\n"));
            output.push_str(&format!(
                "impl Clone for {struct_name}Abi<VmAbi> {{\n    fn clone(&self) -> Self {{ *self }}\n}}\n\n"
            ));

            output.push_str(&format!(
                "impl VmAggregateCodec for {struct_name}Abi<VmAbi> {{\n"
            ));
            output.push_str(
                "    fn decode_with_context(context: &vm::ExternalCallContext<'_>, value: vm::Value) -> RuntimeResult<Self> {\n",
            );
            output.push_str("        if value.tag() != vm::ValueTag::Aggregate {\n");
            output.push_str(&format!(
                "            return Err(RuntimeError::from(AbiPlatformError::invalid_argument_type(\"value\", \"{struct_name}\")).boxed());\n"
            ));
            output.push_str("        }\n");
            output.push_str(
                "        let slots = context.aggregate_slots(value).map_err(|error| RuntimeError::from(error).boxed())?;\n",
            );
            output.push_str(&format!("        if slots.len() != {} {{\n", fields.len()));
            output.push_str(&format!(
                "            return Err(RuntimeError::from(AbiPlatformError::invalid_argument_value(\"value\", \"expected {} fields\")).boxed());\n",
                fields.len()
            ));
            output.push_str("        }\n");
            for (index, field) in fields.iter().enumerate() {
                let field_name = to_snake_case(&field.name);
                let field_type = vm_type_for_binding(domain, &field.binding_type);
                let local_name = format!("field_{field_name}");
                output.push_str(&format!(
                    "        let {local_name} = <{field_type} as VmAggregateCodec>::decode_with_context(context, slots[{index}])?;\n"
                ));
            }
            output.push_str("        Ok(Self {\n");
            for field in fields {
                let field_name = to_snake_case(&field.name);
                let local_name = format!("field_{field_name}");
                output.push_str(&format!("            {field_name}: {local_name},\n"));
            }
            output.push_str("        })\n");
            output.push_str("    }\n\n");
            output.push_str(
                "    fn encode_with_context(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<vm::Value> {\n",
            );
            output.push_str("        let slots = vec![\n");
            for field in fields {
                let field_name = to_snake_case(&field.name);
                let field_type = vm_type_for_binding(domain, &field.binding_type);
                output.push_str(&format!(
                    "            <{field_type} as VmAggregateCodec>::encode_with_context(self.{field_name}, context)?,\n"
                ));
            }
            output.push_str("        ];\n");
            output.push_str("        Ok(context.allocate_aggregate(slots))\n");
            output.push_str("    }\n");
            output.push_str("}\n\n");
        } else {
            output.push_str(&format!("pub type {struct_name}Vm = {struct_name};\n\n"));

            output.push_str(&format!("impl VmAggregateCodec for {struct_name} {{\n"));
            output.push_str(
                "    fn decode_with_context(context: &vm::ExternalCallContext<'_>, value: vm::Value) -> RuntimeResult<Self> {\n",
            );
            output.push_str("        if value.tag() != vm::ValueTag::Aggregate {\n");
            output.push_str(&format!(
                "            return Err(RuntimeError::from(AbiPlatformError::invalid_argument_type(\"value\", \"{struct_name}\")).boxed());\n"
            ));
            output.push_str("        }\n");
            output.push_str(
                "        let slots = context.aggregate_slots(value).map_err(|error| RuntimeError::from(error).boxed())?;\n",
            );
            output.push_str(&format!("        if slots.len() != {} {{\n", fields.len()));
            output.push_str(&format!(
                "            return Err(RuntimeError::from(AbiPlatformError::invalid_argument_value(\"value\", \"expected {} fields\")).boxed());\n",
                fields.len()
            ));
            output.push_str("        }\n");
            for (index, field) in fields.iter().enumerate() {
                let field_name = to_snake_case(&field.name);
                let field_type = vm_type_for_binding(domain, &field.binding_type);
                let local_name = format!("field_{field_name}");
                output.push_str(&format!(
                    "        let {local_name} = <{field_type} as VmAggregateCodec>::decode_with_context(context, slots[{index}])?;\n"
                ));
            }
            output.push_str("        Ok(Self {\n");
            for field in fields {
                let field_name = to_snake_case(&field.name);
                let local_name = format!("field_{field_name}");
                output.push_str(&format!("            {field_name}: {local_name},\n"));
            }
            output.push_str("        })\n");
            output.push_str("    }\n\n");
            output.push_str(
                "    fn encode_with_context(self, context: &mut vm::ExternalCallContext<'_>) -> RuntimeResult<vm::Value> {\n",
            );
            output.push_str("        let slots = vec![\n");
            for field in fields {
                let field_name = to_snake_case(&field.name);
                let field_type = vm_type_for_binding(domain, &field.binding_type);
                output.push_str(&format!(
                    "            <{field_type} as VmAggregateCodec>::encode_with_context(self.{field_name}, context)?,\n"
                ));
            }
            output.push_str("        ];\n");
            output.push_str("        Ok(context.allocate_aggregate(slots))\n");
            output.push_str("    }\n");
            output.push_str("}\n\n");
        }
    }

    for (struct_name, binding_type) in structs {
        let BindingType::Struct { fields, .. } = binding_type else {
            continue;
        };
        if !binding_type_requires_abi(binding_type) {
            continue;
        }

        let replay_name = replay_struct_name(struct_name);
        output.push_str(&format!("/// Replay struct for {struct_name}.\n"));
        output.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
        output.push_str(&format!("pub struct {replay_name} {{\n"));
        for field in fields {
            let field_name = to_snake_case(&field.name);
            let field_type = replay_type_for_binding(domain, &field.binding_type);
            output.push_str(&format!("    /// The {field_name} field.\n"));
            output.push_str(&format!("    pub {field_name}: {field_type},\n"));
        }
        output.push_str("}\n\n");
    }

    for (union_name, binding_type) in tagged_unions {
        let BindingType::TaggedUnion { variants, .. } = binding_type else {
            continue;
        };
        if !binding_type_requires_abi(binding_type) {
            continue;
        }

        let replay_name = replay_tagged_union_name(union_name);
        output.push_str(&format!("/// Replay enum for {union_name}.\n"));
        output.push_str("#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]\n");
        output.push_str(&format!("pub enum {replay_name} {{\n"));
        for variant in variants {
            let variant_type = replay_type_for_binding(domain, &variant.binding_type);
            output.push_str(&format!("    /// {} variant.\n", variant.name));
            output.push_str(&format!("    {}({variant_type}),\n", variant.name));
        }
        output.push_str("}\n\n");
    }

    output
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::model::{BindingTaggedUnionVariant, BindingType};

    use super::tagged_union_variant_tags;

    /// Keep tagged union tags stable regardless of declaration order.
    #[test]
    fn test_tagged_union_variant_tags_are_order_independent() {
        let variants = vec![
            BindingTaggedUnionVariant {
                name: "OpenFile".to_string(),
                binding_type: BindingType::Void,
            },
            BindingTaggedUnionVariant {
                name: "OpenUrl".to_string(),
                binding_type: BindingType::Void,
            },
            BindingTaggedUnionVariant {
                name: "ShareText".to_string(),
                binding_type: BindingType::Void,
            },
        ];
        let mut reversed = variants.clone();
        reversed.reverse();

        let mut forward_tags = BTreeMap::new();
        for (variant, tag) in variants
            .iter()
            .zip(tagged_union_variant_tags("IntentEvent", &variants))
        {
            forward_tags.insert(variant.name.clone(), tag);
        }

        let mut reversed_tags = BTreeMap::new();
        for (variant, tag) in reversed
            .iter()
            .zip(tagged_union_variant_tags("IntentEvent", &reversed))
        {
            reversed_tags.insert(variant.name.clone(), tag);
        }

        assert_eq!(forward_tags, reversed_tags);
    }
}

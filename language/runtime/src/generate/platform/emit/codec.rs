use super::codegen::ModuleCodegen;
use crate::platform::model::{BindingEnumValue, BindingEnumVariant, BindingField, BindingType};
use destack_dir::EnumBackingType;

use super::binding_type_requires_abi;

impl<'a> ModuleCodegen<'a> {
    /// Render one native owned decode expression.
    pub(super) fn render_native_into_value(
        &self,
        binding_type: &BindingType,
        value_expr: &str,
    ) -> String {
        let native_type = self.native_type_for_binding(binding_type);

        format!("unsafe {{ <{native_type} as NativeAbiCodec>::into_value({value_expr})? }}")
    }

    /// Render one native owned encode expression.
    pub(super) fn render_native_from_value(
        &self,
        binding_type: &BindingType,
        value_expr: &str,
    ) -> String {
        let native_type = self.native_type_for_binding(binding_type);

        format!("<{native_type} as NativeAbiCodec>::from_value(binding, {value_expr})")
    }

    /// Render one VM owned decode expression.
    pub(super) fn render_vm_into_value(
        &self,
        binding_type: &BindingType,
        value_expr: &str,
    ) -> String {
        let vm_type = self.vm_type_for_binding(binding_type);

        format!("<{vm_type} as VmAbiCodec>::into_value({value_expr}, context)?")
    }

    /// Render one VM owned encode expression.
    pub(super) fn render_vm_from_value(
        &self,
        binding_type: &BindingType,
        value_expr: &str,
    ) -> String {
        let vm_type = self.vm_type_for_binding(binding_type);

        format!("<{vm_type} as VmAbiCodec>::from_value(context, {value_expr})?")
    }

    /// Render the return encoding lines for one binding type.
    pub(super) fn render_return_encode_lines(&self, binding_type: &BindingType) -> Vec<String> {
        if matches!(binding_type, BindingType::Void) {
            return vec!["result.map(|_| vm::Word::VOID)".to_string()];
        }

        let expr = self.render_encode_expr(binding_type, "value");
        vec![format!(
            "result.map(|value| {expr}).and_then(|value| value)"
        )]
    }

    /// Render one VM value expression for one encoded binding value.
    pub(super) fn render_encode_expr(
        &self,
        binding_type: &BindingType,
        value_expr: &str,
    ) -> String {
        match binding_type {
            BindingType::Void => "Ok(vm::Word::VOID)".to_string(),
            BindingType::Bool => format!("Ok(vm::Word::bool({value_expr}))"),
            BindingType::Int(64) => format!("Ok(vm::Word::int({value_expr}, 64))"),
            BindingType::Int(bits) => {
                format!("Ok(vm::Word::int({value_expr} as i64, {bits}))")
            }
            BindingType::UInt(64) => format!("Ok(vm::Word::uint({value_expr}, 64))"),
            BindingType::UInt(bits) => {
                format!("Ok(vm::Word::uint({value_expr} as u64, {bits}))")
            }
            BindingType::Float(32) => format!("Ok(vm::Word::float32({value_expr}))"),
            BindingType::Float(64) => format!("Ok(vm::Word::float64({value_expr}))"),
            BindingType::Float(width) => panic!("unsupported float width for VM binding: {width}"),
            BindingType::String => format!("Ok({value_expr}.value())"),
            BindingType::StringSlice | BindingType::Slice(_) | BindingType::Array(_) => {
                format!("{value_expr}.to_value(context)")
            }
            BindingType::Optional(inner) => {
                let some_expr = self.render_encode_expr(inner, "value");
                format!(
                    "match {value_expr} {{ Some(value) => {some_expr}, None => Ok(vm::Word::VOID) }}"
                )
            }
            BindingType::Newtype {
                name,
                domain: newtype_domain,
                inner,
            } => {
                if name == "ResourceKind" && newtype_domain == "resource" {
                    return format!("Ok({value_expr}.value())");
                }

                let inner_expr = format!("{value_expr}.0");
                self.render_encode_expr(inner, inner_expr.as_str())
            }
            BindingType::Enum {
                name,
                domain: enum_domain,
                backing,
                variants,
            } => match backing {
                EnumBackingType::Integer(_) => self.render_encode_expr(
                    &super::enum_backing_binding_type(*backing),
                    &format!("{value_expr} as {}", Self::enum_backing_rust_type(*backing)),
                ),
                EnumBackingType::String => {
                    let enum_path = self.named_type_path(enum_domain, name);
                    let mut arms = Vec::new();
                    for variant in variants {
                        if let BindingEnumValue::String(value) = &variant.value {
                            arms.push(format!(
                                "{enum_path}::{} => Ok(context.string_handle(\"{value}\").map_err(Box::<RuntimeError>::from)?.value())",
                                variant.name
                            ));
                        }
                    }

                    format!(
                        "match {value_expr} {{ {} , _ => Ok(context.string_handle(\"\").map_err(Box::<RuntimeError>::from)?.value()), }}",
                        arms.join(", ")
                    )
                }
            },
            BindingType::Struct { fields, .. } => {
                let mut lines = Vec::new();
                let mut encoded_fields = Vec::new();
                let BindingType::Struct { name, domain, .. } = binding_type else {
                    unreachable!();
                };
                let metadata_name = self.named_type_metadata_name(domain, name);

                // encode each field
                for (index, field) in fields.iter().enumerate() {
                    let field_name = self.to_snake_case(&field.name);
                    let field_expr = format!("{value_expr}.{field_name}");
                    let encoded_expr = self.render_encode_expr(&field.binding_type, &field_expr);
                    let local_name = format!("field_{index}");
                    lines.push(format!(
                        "let {local_name}: RuntimeResult<vm::Word> = {encoded_expr};"
                    ));
                    encoded_fields.push(format!("{local_name}?"));
                }

                lines.push(format!(
                    "let mut value_builder = context.begin_named_aggregate_builder(\"{}\").map_err(Box::<RuntimeError>::from)?;",
                    self.escape_rust_string(&metadata_name)
                ));
                for (index, local_name) in encoded_fields.iter().enumerate() {
                    lines.push(format!(
                        "value_builder.write_field({index}, {local_name}).map_err(Box::<RuntimeError>::from)?;"
                    ));
                }
                lines.push("value_builder.finish().map_err(Box::<RuntimeError>::from)".to_string());
                format!("{{ {} }}", lines.join(" "))
            }
            BindingType::TaggedUnion {
                name,
                domain: union_domain,
                variants,
            } => {
                let union_type = self.tagged_union_vm_path(union_domain, name);
                let metadata_name = self.named_type_metadata_name(union_domain, name);
                let mut arms = Vec::new();
                for variant in variants {
                    let tag = Self::tagged_union_variant_tag(name, variant.name.as_str());
                    let payload_expr = self.render_encode_expr(&variant.binding_type, "value");
                    arms.push(format!(
                        "{union_type}::{}(value) => {{ let tag_value = vm::Word::uint({tag}u64, 32); let payload_value = {payload_expr}?; let mut value_builder = context.begin_named_aggregate_builder(\"{}\").map_err(Box::<RuntimeError>::from)?; value_builder.write_field(0, tag_value).map_err(Box::<RuntimeError>::from)?; value_builder.write_field(1, payload_value).map_err(Box::<RuntimeError>::from)?; value_builder.finish().map_err(Box::<RuntimeError>::from) }}",
                        variant.name
                        ,
                        self.escape_rust_string(&metadata_name)
                    ));
                }
                format!("match {value_expr} {{ {} }}", arms.join(", "))
            }
        }
    }

    /// Render decode lines for one VM binding value.
    pub(super) fn render_decode_value_lines(
        &self,
        name: &str,
        binding_type: &BindingType,
        value_expr: &str,
        expected: &str,
    ) -> Vec<String> {
        match binding_type {
            BindingType::Bool => vec![format!(
                "let {name} = decode_bool({value_expr}, \"{name}\", \"{expected}\")?;"
            )],
            BindingType::Int(bits) if matches!(bits, 8 | 16 | 32 | 64) => {
                let decoder = match bits {
                    8 => "decode_int8",
                    16 => "decode_int16",
                    32 => "decode_int32",
                    64 => "decode_int64",
                    _ => unreachable!(),
                };
                vec![format!(
                    "let {name} = {decoder}({value_expr}, \"{name}\", \"{expected}\")?;"
                )]
            }
            BindingType::UInt(bits) if matches!(bits, 8 | 16 | 32 | 64) => {
                let decoder = match bits {
                    8 => "decode_uint8",
                    16 => "decode_uint16",
                    32 => "decode_uint32",
                    64 => "decode_uint64",
                    _ => unreachable!(),
                };
                vec![format!(
                    "let {name} = {decoder}({value_expr}, \"{name}\", \"{expected}\")?;"
                )]
            }
            BindingType::Float(32) => vec![format!(
                "let {name} = decode_float32({value_expr}, \"{name}\", \"{expected}\")?;"
            )],
            BindingType::Float(64) => vec![format!(
                "let {name} = decode_float64({value_expr}, \"{name}\", \"{expected}\")?;"
            )],
            BindingType::String => vec![format!(
                "let {name} = decode_string(context, {value_expr}, \"{name}\", \"{expected}\")?;"
            )],
            BindingType::StringSlice => vec![format!(
                "let {name} = decode_slice::<vm::StringHandle>(context, {value_expr}, \"{name}\", \"{expected}\")?;"
            )],
            BindingType::Slice(inner) => {
                let inner_type = self.vm_type_for_binding(inner);
                vec![format!(
                    "let {name} = decode_slice::<{inner_type}>(context, {value_expr}, \"{name}\", \"{expected}\")?;"
                )]
            }
            BindingType::Array(inner) => {
                let inner_type = self.vm_type_for_binding(inner);
                vec![format!(
                    "let {name} = decode_array::<{inner_type}>(context, {value_expr}, \"{name}\", \"{expected}\")?;"
                )]
            }
            BindingType::Optional(inner) => {
                let inner_name = format!("{name}_inner");
                let mut lines = Vec::new();
                lines.push(format!("let {name} = if {value_expr} == vm::Word::VOID {{"));
                lines.push("    None".to_string());
                lines.push("} else {".to_string());
                lines.extend(
                    self.render_decode_value_lines(&inner_name, inner, value_expr, expected)
                        .into_iter()
                        .map(|line| format!("    {line}")),
                );
                lines.push(format!("    Some({inner_name})"));
                lines.push("};".to_string());
                lines
            }
            BindingType::Newtype {
                name: type_name,
                domain: type_domain,
                inner,
            } => {
                let inner_name = format!("{name}_inner");
                let mut lines =
                    self.render_decode_value_lines(&inner_name, inner, value_expr, expected);
                let type_path = if binding_type_requires_abi(inner) {
                    self.newtype_abi_constructor_path(type_domain, type_name, "platform_abi::VmAbi")
                } else {
                    self.named_type_path(type_domain, type_name)
                };
                lines.push(format!("let {name} = {type_path}({inner_name});"));
                lines
            }
            BindingType::Enum {
                name: enum_name,
                domain: enum_domain,
                backing,
                variants,
            } => {
                let enum_path = self.named_type_path(enum_domain, enum_name);
                self.render_decode_enum_lines(
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
            } => self.render_decode_struct_lines(
                name,
                value_expr,
                expected,
                struct_name,
                struct_domain,
                fields,
            ),
            BindingType::TaggedUnion {
                name: union_name,
                domain: union_domain,
                ..
            } => {
                let union_type = self.tagged_union_vm_path(union_domain, union_name);
                vec![format!(
                    "let {name} = <{union_type} as VmAggregateCodec>::decode_with_context(context, {value_expr})?;"
                )]
            }
            BindingType::Void => Vec::new(),
            _ => Vec::new(),
        }
    }

    /// Render enum decode lines.
    pub(super) fn render_decode_enum_lines(
        &self,
        name: &str,
        value_expr: &str,
        expected: &str,
        enum_name: &str,
        backing: EnumBackingType,
        variants: &[BindingEnumVariant],
    ) -> Vec<String> {
        let raw_name = format!("{name}_raw");
        let backing_type = super::enum_backing_binding_type(backing);
        let mut lines =
            self.render_decode_value_lines(&raw_name, &backing_type, value_expr, expected);

        let match_expr = match backing {
            EnumBackingType::Integer(int_type) => {
                let suffix = Self::enum_backing_rust_type(EnumBackingType::Integer(int_type));
                let mut arms = Vec::new();
                for variant in variants {
                    if let BindingEnumValue::Int(value) = variant.value {
                        let literal = format!("{value}{suffix}");
                        arms.push(format!("{literal} => {enum_name}::{}", variant.name));
                    }
                }
                format!(
                    "match {raw_name} {{ {} , _ => return Err(RuntimeError::from(PlatformError::invalid_argument_value(\"{name}\", \"unknown {enum_name} value\")).boxed()), }}",
                    arms.join(", ")
                )
            }
            EnumBackingType::String => {
                lines.push(format!(
                    "let {raw_name}_ref = context.string_ref({raw_name}).map_err(|error| RuntimeError::from(error).boxed())?;"
                ));
                lines.push(format!("let {raw_name} = {raw_name}_ref.as_str();"));
                let mut arms = Vec::new();
                for variant in variants {
                    if let BindingEnumValue::String(value) = &variant.value {
                        arms.push(format!("\"{value}\" => {enum_name}::{}", variant.name));
                    }
                }
                format!(
                    "match {raw_name} {{ {} , _ => return Err(RuntimeError::from(PlatformError::invalid_argument_value(\"{name}\", \"unknown {enum_name} value\")).boxed()), }}",
                    arms.join(", ")
                )
            }
        };
        lines.push(format!("let {name} = {match_expr};"));
        lines
    }

    /// Render struct decode lines.
    pub(super) fn render_decode_struct_lines(
        &self,
        name: &str,
        value_expr: &str,
        _expected: &str,
        struct_name: &str,
        struct_domain: &str,
        _fields: &[BindingField],
    ) -> Vec<String> {
        let struct_type = self.struct_vm_path(struct_domain, struct_name);
        vec![format!(
            "let {name} = <{struct_type} as VmAggregateCodec>::decode_with_context(context, {value_expr})?;"
        )]
    }
}

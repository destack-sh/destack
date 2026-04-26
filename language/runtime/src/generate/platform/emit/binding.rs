use super::RenderSpec;
use super::codegen::ModuleCodegen;
use crate::platform::model::{BindingEntry, BindingType};

/// Stateful writer for one binding module.
pub(super) struct BindingWriter<'spec, 'output> {
    /// Render specification derived from bindings.
    pub(super) spec: &'spec RenderSpec<'spec>,
    /// Output buffer for generated content.
    pub(super) output: &'output mut String,
}

impl<'spec, 'output> BindingWriter<'spec, 'output> {
    /// Create a writer for one binding module.
    fn new(spec: &'spec RenderSpec<'spec>, output: &'output mut String) -> Self {
        Self { spec, output }
    }

    /// Return the Rust code generator for this module.
    pub(super) fn codegen(&self) -> ModuleCodegen<'spec> {
        self.spec.codegen()
    }

    /// Report whether one binding type is one random stream handle.
    fn is_random_stream_type(binding_type: &BindingType) -> bool {
        match binding_type {
            BindingType::Newtype { name, domain, .. } => {
                domain == "random" && (name == "RandomStream" || name == "RandomStreamId")
            }
            _ => false,
        }
    }

    /// Report whether one binding type is one byte buffer.
    fn is_byte_buffer_type(binding_type: &BindingType) -> bool {
        binding_type.is_byte_collection()
    }

    /// Locate one random stream parameter name.
    fn random_stream_arg(codegen: &ModuleCodegen<'_>, binding: &BindingEntry) -> Option<String> {
        binding
            .parameters
            .iter()
            .enumerate()
            .find(|(_, param)| Self::is_random_stream_type(&param.binding_type))
            .map(|(index, param)| codegen.sanitize_param_name(&param.name, index))
    }

    /// Render one random stream identifier expression.
    pub(super) fn random_stream_id_expr(
        codegen: &ModuleCodegen<'_>,
        binding: &BindingEntry,
        default_expr: &str,
    ) -> String {
        Self::random_stream_arg(codegen, binding).map_or_else(
            || default_expr.to_string(),
            |name| format!("RandomStreamId::new({name}.0)"),
        )
    }

    /// Locate one byte buffer parameter name for random byte bindings.
    pub(super) fn random_bytes_buffer_arg(
        codegen: &ModuleCodegen<'_>,
        binding: &BindingEntry,
    ) -> String {
        binding
            .parameters
            .iter()
            .enumerate()
            .find(|(_, param)| Self::is_byte_buffer_type(&param.binding_type))
            .map(|(index, param)| codegen.sanitize_param_name(&param.name, index))
            .unwrap_or_else(|| {
                panic!("random byte bindings must take a Slice<uint8> or Array<uint8> parameter")
            })
    }
}

impl<'a> RenderSpec<'a> {
    /// Render generated bindings for this module.
    pub(crate) fn render(&self) -> String {
        let mut output = String::new();
        let mut writer = BindingWriter::new(self, &mut output);

        writer.write_header();
        if self.usage.uses_binding_replay {
            writer.write_replay_payloads();
        }
        writer.write_descriptor_consts();
        writer.write_native_set();
        if self.usage.uses_binding_replay {
            writer.write_native_replay_helpers();
        }
        writer.write_native_exports();
        if self.usage.uses_binding_replay {
            writer.write_vm_replay_helpers();
        }
        writer.write_vm_register_fn();
        writer.write_vm_set();

        output
    }
}

#[cfg(test)]
mod tests {
    use crate::platform::model::{BindingTaggedUnionVariant, BindingType};

    use super::ModuleCodegen;

    /// Encode tagged unions with stable 32-bit tag values.
    #[test]
    fn test_render_encode_expr_tagged_union_uses_stable_u32_tags() {
        let binding_type = BindingType::TaggedUnion {
            name: "ExampleEvent".to_string(),
            domain: "example".to_string(),
            variants: vec![
                BindingTaggedUnionVariant {
                    name: "ExampleClosed".to_string(),
                    binding_type: BindingType::Void,
                },
                BindingTaggedUnionVariant {
                    name: "ExampleOpened".to_string(),
                    binding_type: BindingType::Void,
                },
            ],
        };

        let codegen = ModuleCodegen::new("example");
        let rendered = codegen.render_encode_expr(&binding_type, "value");
        let closed_tag = ModuleCodegen::tagged_union_variant_tag("ExampleEvent", "ExampleClosed");
        let opened_tag = ModuleCodegen::tagged_union_variant_tag("ExampleEvent", "ExampleOpened");

        // stable 32 bit tags
        assert!(rendered.contains(format!("vm::Word::uint({closed_tag}u64, 32)").as_str()));
        assert!(rendered.contains(format!("vm::Word::uint({opened_tag}u64, 32)").as_str()));

        // not ordinal tags
        assert!(!rendered.contains("vm::Word::uint(1, 8)"));
        assert!(!rendered.contains("vm::Word::uint(2, 8)"));
    }
}

use crate::analyze::{
    CatalogBindingAffinity, CatalogBindingBlocking, CatalogBindingReplayKind, CatalogBindingScope,
    CatalogEffectClass, CatalogReplayPayload, CatalogReplayPolicy,
};

use super::bindings::BindingWriter;
use super::codegen::ModuleCodegen;

impl<'spec, 'output> BindingWriter<'spec, 'output> {
    /// Render the binding descriptor constants for a domain.
    pub(super) fn write_descriptor_consts(&mut self) {
        let codegen = self.codegen();
        let output = &mut self.output;
        let consts = &self.spec.consts;
        for binding in consts {
            let signature = codegen.escape_rust_string(&binding.entry.signature);
            let (ctor, args) = Self::descriptor_ctor_for_binding(
                binding.entry.effect_class,
                binding.entry.replay_payload,
                binding.entry.replay_kind,
                &binding.entry.requires,
                binding.entry.scope,
                binding.entry.blocking,
                binding.entry.affinity,
                &codegen,
            );
            let host_platforms =
                codegen.render_binding_host_platforms(&binding.entry.host_platforms);
            output.push_str(&format!(
                "/// Binding descriptor for {}.\n",
                binding.extern_name
            ));
            output.push_str(&format!(
                "pub const {}: BindingDescriptor = BindingDescriptor::{}(\n",
                binding.const_name, ctor,
            ));
            output.push_str(&format!("    \"{}\",\n", binding.extern_name));
            output.push_str(&format!("    \"{signature}\",\n"));
            for arg in args {
                output.push_str(&format!("    {arg},\n"));
            }
            output.push_str(")");
            output.push_str(&format!("\n    .with_namespace(\"{}\")", self.spec.module));
            if let Some(host_platforms) = host_platforms {
                output.push_str(&format!("\n    .with_host_platforms({host_platforms})"));
            }
            output.push_str(";\n\n");
        }
    }

    /// Resolve the descriptor constructor for an effect class.
    fn descriptor_ctor_for_binding(
        effect_class: CatalogEffectClass,
        replay_payload: CatalogReplayPayload,
        replay_kind: CatalogBindingReplayKind,
        requires: &[String],
        scope: CatalogBindingScope,
        blocking: CatalogBindingBlocking,
        affinity: CatalogBindingAffinity,
        codegen: &ModuleCodegen<'_>,
    ) -> (&'static str, Vec<String>) {
        let requires_arg = codegen.render_binding_requires(requires);
        let scope_arg = codegen.render_binding_scope(scope);
        let blocking_arg = codegen.render_binding_blocking(blocking);
        let affinity_arg = codegen.render_binding_affinity(affinity);
        match effect_class {
            CatalogEffectClass::Pure => (
                "pure_with_requires_and_behavior",
                vec![requires_arg, scope_arg, blocking_arg, affinity_arg],
            ),
            CatalogEffectClass::Deterministic => (
                "deterministic_with_requires_and_behavior",
                vec![requires_arg, scope_arg, blocking_arg, affinity_arg],
            ),
            CatalogEffectClass::External { replay } => {
                let replay = match replay {
                    CatalogReplayPolicy::Recordable => "BindingReplayPolicy::Recordable",
                    CatalogReplayPolicy::NonRecordable => "BindingReplayPolicy::NonRecordable",
                };
                let replay_kind = codegen.render_binding_replay_kind(replay_kind);
                let payload_arg = match replay_payload {
                    CatalogReplayPayload::ResultsOnly => None,
                    CatalogReplayPayload::ArgumentsAndResults => {
                        Some("BindingReplayPayload::ArgumentsAndResults".to_string())
                    }
                };
                if let Some(payload_arg) = payload_arg {
                    (
                        "external_with_payload_with_requires",
                        vec![
                            replay.to_string(),
                            replay_kind,
                            payload_arg,
                            requires_arg,
                            scope_arg,
                            blocking_arg,
                            affinity_arg,
                        ],
                    )
                } else {
                    (
                        "external_with_requires_and_behavior",
                        vec![
                            replay.to_string(),
                            replay_kind,
                            requires_arg,
                            scope_arg,
                            blocking_arg,
                            affinity_arg,
                        ],
                    )
                }
            }
        }
    }

    /// Render the descriptor slice for a domain.
    pub(super) fn write_bindings_slice(&mut self) {
        let output = &mut self.output;
        let domain = self.spec.module;
        let consts = &self.spec.consts;
        output.push_str(&format!("/// Binding descriptors for {domain}.\n"));
        output.push_str("pub const BINDINGS: &[BindingDescriptor] = &[\n");
        for binding in consts {
            output.push_str(&format!("    {},\n", binding.const_name));
        }
        output.push_str("];\n\n");
    }
}

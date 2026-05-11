use crate::platform::model::{
    CatalogBindingAffinity, CatalogBindingProvider, CatalogBindingReplayKind, CatalogEffectClass,
    CatalogReplayPayload, CatalogReplayPolicy,
};

use super::binding::BindingWriter;
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
                binding.entry.provider,
                binding.entry.affinity,
                &codegen,
            );
            let platforms = codegen.render_binding_platforms(&binding.entry.platforms);
            let hosts = codegen.render_binding_hosts(&binding.entry.hosts);
            output.push_str(&format!(
                "/// Binding descriptor for {}.\n",
                binding.extern_name
            ));
            output.push_str(&format!(
                "pub(crate) const {}: BindingDescriptor = BindingDescriptor::{}(\n",
                binding.const_name, ctor,
            ));
            output.push_str(&format!("    \"{}\",\n", binding.extern_name));
            output.push_str(&format!("    \"{signature}\",\n"));
            for arg in args {
                output.push_str(&format!("    {arg},\n"));
            }
            output.push_str(")");
            output.push_str(&format!("\n    .with_namespace(\"{}\")", self.spec.module));
            if let Some(platforms) = platforms {
                output.push_str(&format!("\n    .with_platforms({platforms})"));
            }
            if let Some(hosts) = hosts {
                output.push_str(&format!("\n    .with_hosts({hosts})"));
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
        provider: CatalogBindingProvider,
        affinity: CatalogBindingAffinity,
        codegen: &ModuleCodegen<'_>,
    ) -> (&'static str, Vec<String>) {
        let requires_arg = codegen.render_binding_requires(requires);
        let provider_arg = codegen.render_binding_provider(provider);
        let affinity_arg = codegen.render_binding_affinity(affinity);
        match effect_class {
            CatalogEffectClass::Pure => (
                "pure_with_requires_and_dispatch",
                vec![requires_arg, provider_arg, affinity_arg],
            ),
            CatalogEffectClass::Deterministic => (
                "deterministic_with_requires_and_dispatch",
                vec![requires_arg, provider_arg, affinity_arg],
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
                            provider_arg,
                            affinity_arg,
                        ],
                    )
                } else {
                    (
                        "external_with_requires_and_dispatch",
                        vec![
                            replay.to_string(),
                            replay_kind,
                            requires_arg,
                            provider_arg,
                            affinity_arg,
                        ],
                    )
                }
            }
        }
    }
}

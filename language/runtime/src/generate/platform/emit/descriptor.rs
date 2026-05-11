use crate::platform::model::{
    CatalogBindingAffinity, CatalogBindingProvider, CatalogBindingReplayKind, CatalogEffect,
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
            let args = Self::descriptor_args_for_binding(
                binding.entry.effect,
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
                "pub(crate) const {}: BindingDescriptor = BindingDescriptor::new(\n",
                binding.const_name,
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

    /// Render descriptor arguments for one binding.
    fn descriptor_args_for_binding(
        effect: CatalogEffect,
        replay_payload: CatalogReplayPayload,
        replay_kind: CatalogBindingReplayKind,
        requires: &[String],
        provider: CatalogBindingProvider,
        affinity: CatalogBindingAffinity,
        codegen: &ModuleCodegen<'_>,
    ) -> Vec<String> {
        let effect_arg = match effect {
            CatalogEffect::Pure => "BindingEffect::Pure".to_string(),
            CatalogEffect::Deterministic => "BindingEffect::Deterministic".to_string(),
            CatalogEffect::External { replay } => match replay {
                CatalogReplayPolicy::Recordable => "BindingEffect::ExternalRecordable".to_string(),
                CatalogReplayPolicy::NonRecordable => {
                    "BindingEffect::ExternalNonRecordable".to_string()
                }
            },
        };
        let replay_kind_arg = codegen.render_binding_replay_kind(replay_kind);
        let replay_payload_arg = match replay_payload {
            CatalogReplayPayload::ResultsOnly => "BindingReplayPayload::Results".to_string(),
            CatalogReplayPayload::ArgumentsAndResults => {
                "BindingReplayPayload::ArgumentsAndResults".to_string()
            }
        };
        let requires_arg = codegen.render_binding_requires(requires);
        let provider_arg = codegen.render_binding_provider(provider);
        let affinity_arg = codegen.render_binding_affinity(affinity);

        vec![
            effect_arg,
            replay_kind_arg,
            replay_payload_arg,
            requires_arg,
            provider_arg,
            affinity_arg,
        ]
    }
}

use destack_artifact::ArtifactEvent;

use crate::check::{
    DumpContext, MatchCase, Obligation, ObligationId, PatternCoverage, PlaceTarget,
};

impl Obligation {
    /// Render this obligation as one trace event.
    pub(in crate::check) fn render_event(
        &self,
        id: ObligationId,
        finished: bool,
        context: &DumpContext<'_, '_>,
    ) -> ArtifactEvent {
        let event = ArtifactEvent::new("obligation.check")
            .debug()
            .text("id", context.obligation_label(id))
            .text("kind", self.kind_label())
            .text("source", context.node_label(self.source()))
            .text("at", context.node_source_label(self.source()))
            .text(
                "condition",
                super::constraint::condition_label(self.condition(), context),
            )
            .bool("finished", finished);

        match self {
            Self::PatternCoverage(obligation) => event
                .text("value", context.type_label(obligation.value))
                .text(
                    "coverage",
                    pattern_coverage_label(&obligation.coverage, context),
                ),
            Self::TryPropagation(obligation) => event
                .text("value", context.type_label(obligation.value))
                .text(
                    "return",
                    obligation
                        .return_type
                        .map(|ty| context.type_label(ty))
                        .unwrap_or_else(|| "none".to_string()),
                ),
            Self::WritablePlace(obligation) => event
                .text(
                    "place",
                    place_target_label(&obligation.place.target, context),
                )
                .text("type", context.type_label(obligation.place.ty)),
            Self::Representation(obligation) => {
                event.text("type", context.type_label(obligation.ty))
            }
            Self::DynamicSafety(obligation) => {
                event.text("type", context.type_label(obligation.ty))
            }
            Self::ExtensionConformance(obligation) => {
                event.text("symbol", context.symbol_label(obligation.symbol))
            }
            Self::ImplementationCoherence(obligation) => {
                event.text("symbol", context.symbol_label(obligation.symbol))
            }
            Self::DeclarationHeritage(obligation) => {
                event.text("symbol", context.symbol_label(obligation.symbol))
            }
        }
    }

    /// Return the compact obligation kind label.
    fn kind_label(&self) -> &'static str {
        match self {
            Self::PatternCoverage(_) => "pattern.coverage",
            Self::TryPropagation(_) => "try.propagation",
            Self::WritablePlace(_) => "writable.place",
            Self::Representation(_) => "representation",
            Self::DynamicSafety(_) => "dynamic.safety",
            Self::ExtensionConformance(_) => "extension.conformance",
            Self::ImplementationCoherence(_) => "implementation.coherence",
            Self::DeclarationHeritage(_) => "declaration.heritage",
        }
    }
}

/// Render one pattern coverage payload.
fn pattern_coverage_label(coverage: &PatternCoverage, context: &DumpContext<'_, '_>) -> String {
    match coverage {
        PatternCoverage::Match { cases } => {
            let cases = cases
                .iter()
                .map(|case| match case {
                    MatchCase::Default => "default".to_string(),
                    MatchCase::Pattern { pattern, guard } => {
                        let pattern = context.node_label(pattern.into_any());
                        let Some(guard) = guard else {
                            return pattern;
                        };

                        format!("{pattern} if {}", context.type_label(*guard))
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");

            format!("match[{cases}]")
        }
        PatternCoverage::Binding { pattern } => {
            format!("binding({})", context.node_label(pattern.into_any()))
        }
        PatternCoverage::Catch { pattern } => {
            format!("catch({})", context.node_label(pattern.into_any()))
        }
    }
}

/// Render one place target compactly.
fn place_target_label(target: &PlaceTarget, context: &DumpContext<'_, '_>) -> String {
    match target {
        PlaceTarget::Binding { symbol } => {
            format!("binding({})", context.symbol_label(*symbol))
        }
        PlaceTarget::Member { owner, key } => {
            format!(
                "member({}.{})",
                context.type_label(*owner),
                context.static_key_label(key)
            )
        }
        PlaceTarget::Index { receiver, index } => {
            format!(
                "index({}[{}])",
                context.type_label(*receiver),
                context.type_label(*index)
            )
        }
        PlaceTarget::Dereference => "dereference".to_string(),
    }
}

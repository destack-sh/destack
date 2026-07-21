use destack_artifact::ArtifactEvent;
use destack_dir as dir;

use crate::check::{DumpContext, Obligation, ObligationId, PatternCoverage};

impl Obligation {
    /// Render this obligation as one trace event.
    pub(in crate::check) fn render_event(
        &self,
        id: ObligationId,
        finished: bool,
        context: &DumpContext<'_, '_>,
    ) -> ArtifactEvent {
        let event = ArtifactEvent::new("obligation.checked")
            .debug()
            .text("id", context.obligation_label(id))
            .text("kind", self.kind_label())
            .text("source", context.node_label(self.source()))
            .text("at", context.node_source_label(self.source()))
            .bool("finished", finished);

        match self {
            Self::PatternCoverage(obligation) => event
                .text("value", context.expected_type_label(obligation.value))
                .text(
                    "coverage",
                    pattern_coverage_label(&obligation.coverage, context),
                ),
            Self::WritablePlace(obligation) => event
                .text("place", place_label(&obligation.place.storage, context))
                .text("source", context.node_label(obligation.place.source)),
            Self::Representation(obligation) => {
                event.text("type", context.type_label(obligation.ty))
            }
            Self::RuntimePredicate(obligation) => event.text(
                "predicate",
                runtime_predicate_label(&obligation.predicate, context),
            ),
            Self::ForInSource(obligation) => event.text("type", context.type_label(obligation.ty)),
            Self::ExtensionConformance(obligation) => {
                event.text("symbol", context.symbol_label(obligation.symbol))
            }
            Self::ImplementationCoherence(obligation) => {
                event.text("symbol", context.symbol_label(obligation.symbol))
            }
            Self::DeclarationHeritage(obligation) => {
                event.text("symbol", context.symbol_label(obligation.symbol))
            }
            Self::ClassInitialization(obligation) => event
                .text("symbol", context.symbol_label(obligation.symbol))
                .text("receiver", context.type_label(obligation.receiver))
                .usize(
                    "constructor_branches",
                    obligation.constructor_branches.len(),
                ),
            Self::WellFormedType(obligation) => {
                event.text("type", context.type_label(obligation.ty))
            }
            Self::ParameterUse(obligation) => {
                event.text("symbol", context.symbol_label(obligation.symbol))
            }
        }
    }

    /// Return the compact obligation kind label.
    fn kind_label(&self) -> &'static str {
        match self {
            Self::PatternCoverage(_) => "pattern.coverage",
            Self::WritablePlace(_) => "writable.place",
            Self::Representation(_) => "representation",
            Self::RuntimePredicate(_) => "runtime.predicate",
            Self::ForInSource(_) => "for.in.source",
            Self::ExtensionConformance(_) => "extension.conformance",
            Self::ImplementationCoherence(_) => "implementation.coherence",
            Self::DeclarationHeritage(_) => "declaration.heritage",
            Self::ClassInitialization(_) => "class.initialization",
            Self::WellFormedType(_) => "wellformed.type",
            Self::ParameterUse(_) => "parameter.use",
        }
    }
}

/// Render one runtime predicate payload.
fn runtime_predicate_label(
    predicate: &dir::GuardResolution,
    context: &DumpContext<'_, '_>,
) -> String {
    match predicate {
        dir::GuardResolution::Is(predicate) => {
            let value = context.type_label(predicate.value_type);
            let target = context.type_label(predicate.target_type);

            format!("{value} is {target}")
        }
        dir::GuardResolution::InstanceOf(predicate) => {
            let value = context.type_label(predicate.value_type);
            let target = context.type_label(predicate.target_type);

            format!("{value} instanceof {target}")
        }
        dir::GuardResolution::In(predicate) => {
            let key = context.type_label(predicate.key_type);
            let receiver = context.type_label(predicate.receiver_type);

            format!("{key} in {receiver}")
        }
    }
}

/// Render one pattern coverage payload.
fn pattern_coverage_label(coverage: &PatternCoverage, context: &DumpContext<'_, '_>) -> String {
    match coverage {
        PatternCoverage::Match { arms } => {
            let arms = arms
                .iter()
                .map(|arm| {
                    let pattern = context.node_label(arm.pattern.into_any());

                    if arm.is_guarded {
                        format!("{pattern} if")
                    } else {
                        pattern
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");

            format!("match[{arms}]")
        }
        PatternCoverage::Binding { pattern } => {
            format!("binding({})", context.node_label(pattern.into_any()))
        }
        PatternCoverage::Catch { pattern } => {
            format!("catch({})", context.node_label(pattern.into_any()))
        }
    }
}

/// Render one place compactly.
fn place_label(place: &dir::Storage, context: &DumpContext<'_, '_>) -> String {
    match place {
        dir::Storage::Binding { symbol } => {
            format!("binding({})", context.symbol_label(*symbol))
        }
        dir::Storage::Field {
            receiver,
            field: dir::ProjectionField::Key(key),
        } => {
            format!(
                "field({}.{})",
                context.type_label(*receiver),
                context.static_key_label(key)
            )
        }
        dir::Storage::Field {
            field: dir::ProjectionField::Member(symbol),
            ..
        } => {
            format!("field({})", context.symbol_label(*symbol))
        }
        dir::Storage::Property { .. } => "property".to_string(),
        dir::Storage::Subscript { .. } => "subscript".to_string(),
        dir::Storage::Dereference { .. } => "dereference".to_string(),
    }
}

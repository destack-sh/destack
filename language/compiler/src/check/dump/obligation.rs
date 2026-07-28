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
            Self::UseAfterMove(obligation) => {
                event.text("symbol", context.symbol_label(obligation.symbol))
            }
            Self::PatternCoverage(obligation) => event
                .text("value", context.expected_type_label(obligation.value))
                .text(
                    "coverage",
                    pattern_coverage_label(&obligation.coverage, context),
                ),
            Self::WritableTarget(obligation) => event
                .text("target", place_label(&obligation.target.write, context))
                .text("source", context.node_label(obligation.target.source)),
            Self::Representation(obligation) => {
                event.text("type", context.type_label(obligation.ty))
            }
            Self::RuntimePredicate(obligation) => event.text(
                "predicate",
                runtime_predicate_label(&obligation.predicate, context),
            ),
            Self::ForInSource(obligation) => event.text("type", context.type_label(obligation.ty)),
            Self::InterfaceConformance(obligation) => {
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
            Self::UseAfterMove(_) => "use.after.move",
            Self::WritableTarget(_) => "writable.target",
            Self::Representation(_) => "representation",
            Self::RuntimePredicate(_) => "runtime.predicate",
            Self::ForInSource(_) => "for.in.source",
            Self::InterfaceConformance(_) => "interface.conformance",
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

/// Render one writable place compactly.
fn place_label(place: &dir::WriteResolution, context: &DumpContext<'_, '_>) -> String {
    match place {
        dir::WriteResolution::Binding { symbol, .. } => {
            format!("binding({})", context.symbol_label(*symbol))
        }
        dir::WriteResolution::Member(member) => member_place_label(member, context),
        dir::WriteResolution::Subscript(_) => "subscript".to_string(),
        dir::WriteResolution::Dereference(_) => "dereference".to_string(),
    }
}

/// Render one writable member compactly.
fn member_place_label(member: &dir::MemberResolution, context: &DumpContext<'_, '_>) -> String {
    match member {
        dir::OperationResolution::One(access) => member_access_place_label(access, context),
        dir::OperationResolution::Union { arms, .. } => {
            let labels = arms
                .iter()
                .map(|access| member_access_place_label(access, context))
                .collect::<Vec<_>>();

            format!("union({})", labels.join(", "))
        }
    }
}

/// Render one singular writable member compactly.
fn member_access_place_label(access: &dir::MemberAccess, context: &DumpContext<'_, '_>) -> String {
    member_target_place_label(&access.target, context)
}

/// Render one writable member target compactly.
fn member_target_place_label(target: &dir::MemberTarget, context: &DumpContext<'_, '_>) -> String {
    match target {
        dir::MemberTarget::Field(dir::FieldResolution {
            receiver,
            target: dir::FieldTarget::Structural { key, .. },
            ..
        }) => format!(
            "field({}.{})",
            context.type_label(receiver.source()),
            context.static_key_label(key)
        ),
        dir::MemberTarget::Field(dir::FieldResolution {
            target: dir::FieldTarget::Member { symbol, .. },
            ..
        }) => format!("field({})", context.symbol_label(*symbol)),
        dir::MemberTarget::Call(_) => "property".to_string(),
        dir::MemberTarget::Intersection(targets) => {
            let labels = targets
                .iter()
                .map(|target| member_target_place_label(target, context))
                .collect::<Vec<_>>();

            format!("intersection({})", labels.join(", "))
        }
        dir::MemberTarget::Projection { .. }
        | dir::MemberTarget::Index(_)
        | dir::MemberTarget::Symbol(_)
        | dir::MemberTarget::Existential(_) => "member".to_string(),
    }
}

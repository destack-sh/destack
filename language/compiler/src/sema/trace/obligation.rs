use tspp_dir as dir;
use tspp_repository::TraceEvent;

use crate::sema::{CheckId, EventFormatter, Obligation, PatternCoverage};

impl EventFormatter<'_, '_> {
    /// Format this obligation as one trace event.
    pub(super) fn format_obligation(
        &self,
        obligation: &Obligation,
        id: CheckId,
        finished: bool,
    ) -> TraceEvent {
        let event = TraceEvent::new("obligation.checked")
            .text("id", self.check_label(id))
            .text("kind", self.obligation_kind_label(obligation))
            .text("source", self.node_label(obligation.source()))
            .text("at", self.node_source_label(obligation.source()))
            .bool("finished", finished);

        // add the fields each obligation carries
        match obligation {
            Obligation::PatternCoverage(obligation) => event
                .text("value", self.expected_type_label(obligation.value))
                .text(
                    "coverage",
                    self.pattern_coverage_label(&obligation.coverage),
                ),
            Obligation::WritableTarget(obligation) => event
                .text("target", self.write_label(&obligation.target.write))
                .text("source", self.node_label(obligation.target.source)),
            Obligation::RuntimePredicate(obligation) => event.text(
                "predicate",
                self.runtime_predicate_label(&obligation.predicate),
            ),
            Obligation::WellFormedType(obligation) => {
                event.text("type", self.type_label(obligation.ty))
            }
            Obligation::RangeElement(obligation) => {
                event.text("element", self.type_label(obligation.element))
            }
            Obligation::RestParameter(obligation) => {
                event.text("type", self.type_label(obligation.ty))
            }
            Obligation::SharedStorage(obligation) => {
                event.text("type", self.type_label(obligation.ty))
            }
        }
    }

    /// Return the compact obligation kind label.
    fn obligation_kind_label(&self, obligation: &Obligation) -> &'static str {
        match obligation {
            Obligation::PatternCoverage(_) => "pattern.coverage",
            Obligation::WritableTarget(_) => "writable.target",
            Obligation::RuntimePredicate(_) => "runtime.predicate",
            Obligation::WellFormedType(_) => "wellformed.type",
            Obligation::RangeElement(_) => "range.element",
            Obligation::RestParameter(_) => "rest.parameter",
            Obligation::SharedStorage(_) => "shared.storage",
        }
    }

    /// Return the compact runtime predicate label.
    fn runtime_predicate_label(&self, predicate: &dir::GuardDecision) -> String {
        match predicate {
            dir::GuardDecision::Is(predicate) => {
                let value = self.type_label(predicate.value_type);
                let target = self.type_label(predicate.target_type);

                format!("{value} is {target}")
            }
            dir::GuardDecision::InstanceOf(predicate) => {
                let value = self.type_label(predicate.value_type);
                let target = self.type_label(predicate.target_type);

                format!("{value} instanceof {target}")
            }
            dir::GuardDecision::In(predicate) => {
                let key = self.type_label(predicate.key_type);
                let receiver = self.type_label(predicate.receiver_type);

                format!("{key} in {receiver}")
            }
        }
    }

    /// Return the compact pattern coverage label.
    fn pattern_coverage_label(&self, coverage: &PatternCoverage) -> String {
        match coverage {
            PatternCoverage::Match { arms } => {
                let arms = arms
                    .iter()
                    .map(|arm| {
                        let pattern = self.node_label(arm.pattern.into_any());

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
                format!("binding({})", self.node_label(pattern.into_any()))
            }
            PatternCoverage::Catch { pattern } => {
                format!("catch({})", self.node_label(pattern.into_any()))
            }
        }
    }

    /// Return the compact write label.
    fn write_label(&self, write: &dir::WriteResolution) -> String {
        match write {
            dir::WriteResolution::Binding { symbol, .. } => {
                format!("binding({})", self.symbol_label(*symbol))
            }
            dir::WriteResolution::Member(member) => self.member_label(member),
            dir::WriteResolution::Subscript(_) => "subscript".to_string(),
            dir::WriteResolution::Dereference(_) => "dereference".to_string(),
        }
    }

    /// Return the compact writable member label.
    fn member_label(&self, member: &dir::MemberDecision) -> String {
        match member {
            dir::OperationResolution::One(access) => self.member_access_label(access),
            dir::OperationResolution::Union { arms, .. } => {
                let labels = arms
                    .iter()
                    .map(|access| self.member_access_label(access))
                    .collect::<Vec<_>>();

                format!("union({})", labels.join(", "))
            }
        }
    }

    /// Return the compact writable member access label.
    fn member_access_label(&self, access: &dir::MemberAccess) -> String {
        self.member_target_label(&access.target)
    }

    /// Return the compact writable member target label.
    fn member_target_label(&self, target: &dir::MemberTarget) -> String {
        match target {
            dir::MemberTarget::Field(dir::FieldResolution {
                receiver,
                target: dir::FieldTarget::Structural { key, .. },
                ..
            }) => format!(
                "field({}.{})",
                self.type_label(receiver.source()),
                self.static_key_label(key)
            ),
            dir::MemberTarget::Field(dir::FieldResolution {
                target: dir::FieldTarget::Member { symbol, .. },
                ..
            }) => format!("field({})", self.symbol_label(*symbol)),
            dir::MemberTarget::Call(_) => "property".to_string(),
            dir::MemberTarget::Intersection(targets) => {
                let labels = targets
                    .iter()
                    .map(|target| self.member_target_label(target))
                    .collect::<Vec<_>>();

                format!("intersection({})", labels.join(", "))
            }
            dir::MemberTarget::Projection { .. }
            | dir::MemberTarget::Index(_)
            | dir::MemberTarget::Symbol(_)
            | dir::MemberTarget::OverloadSet(_) => "member".to_string(),
        }
    }
}

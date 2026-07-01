use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Dependency, FlowPointId, FlowSite, Origin, PlaceUse, answer,
};

impl CheckState<'_> {
    /// Select one literal pattern from its closed expression value.
    pub(in crate::check) fn select_literal_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        input: dir::GlobalTypeId,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let value_node = value.into_global_any(module);
        let ty = answer!(self.infer_node_type(
            FlowSite {
                node: value_node,
                flow,
            },
            PlaceUse::Read
        )?);
        let ty = answer!(self.reduce_type_head(origin, ty)?);

        // closed literal values select literal predicates
        let literal = match self.ty(ty)? {
            dir::Type::Literal(literal) => Some(*literal),
            dir::Type::Null => Some(dir::ScalarLiteral::Null),
            dir::Type::Undefined => Some(dir::ScalarLiteral::Undefined),
            _ => None,
        };
        match literal {
            Some(value) => {
                let predicate = dir::Predicate::unary(
                    dir::PredicateOperand::new(input),
                    dir::PredicateCondition::Literal(value),
                )
                .with_narrowed(ty);

                self.commit_pattern(
                    node,
                    dir::PatternResolution::Test(dir::PatternPredicateResolution { predicate }),
                )
            }
            None => {
                self.report_expression_pattern_not_literal(module, node.local_id.into_any());

                self.commit_pattern(node, dir::PatternResolution::Ignore)
            }
        }
    }

    /// Select one range pattern from its closed bounds.
    pub(in crate::check) fn select_range_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        input: dir::GlobalTypeId,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
        end_kind: dir::RangeEnd,
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;

        // reduce both written bounds to literals
        let mut bounds = [None, None];
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for (slot, bound) in [start, end].into_iter().enumerate() {
            let Some(bound) = bound else {
                continue;
            };
            let bound_node = bound.into_global_any(module);
            let ty = answer!(self.infer_node_type(
                FlowSite {
                    node: bound_node,
                    flow,
                },
                PlaceUse::Read
            )?);
            let ty = match self.reduce_type_head(origin, ty)? {
                Answer::Ready(ty) => ty,
                Answer::Pending(dependencies) => {
                    blockers.extend(dependencies);

                    continue;
                }
            };
            if let dir::Type::Literal(literal) = self.ty(ty)? {
                bounds[slot] = Some(*literal);
            }
        }
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        // narrow successful matches to the represented interval
        let domain = input;
        let written = dir::RangeType::new(bounds[0], bounds[1], end_kind);
        let narrowed =
            self.range_pattern_narrowed_type(module, node.local_id.into_any(), domain, &written)?;
        let predicate = dir::Predicate::unary(
            dir::PredicateOperand::new(domain),
            dir::PredicateCondition::Range(dir::PredicateRange {
                domain,
                start: bounds[0],
                end: bounds[1],
                end_bound: end_kind,
            }),
        )
        .with_narrowed(narrowed);

        self.commit_pattern(
            node,
            dir::PatternResolution::Test(dir::PatternPredicateResolution { predicate }),
        )
    }

    /// Return the type visible after a successful range pattern.
    fn range_pattern_narrowed_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        domain: dir::GlobalTypeId,
        written: &dir::RangeType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let narrowed = match self.ty(domain)? {
            // intersect nested intervals exactly
            dir::Type::Range(domain) => domain
                .intersection(written)
                .map(dir::Type::Range)
                .unwrap_or(dir::Type::Never),
            // intersect fixed-width integer primitives when their bounds fit DIR ranges
            dir::Type::Primitive(dir::PrimitiveType::Integer(integer)) => integer
                .finite_interval()
                .and_then(|domain| domain.intersection(written))
                .map(dir::Type::Range)
                .unwrap_or_else(|| dir::Type::Range(written.clone())),
            // otherwise the written interval is the strongest represented test type
            _ => dir::Type::Range(written.clone()),
        };

        self.push_type(module, narrowed, source)
    }
}

use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::CompilerResult;
use crate::sema::{CheckState, FlowPointId, FlowSite, Origin, PlaceUse};

impl CheckState<'_> {
    /// Select one literal pattern from its closed expression value.
    pub(in crate::sema) fn select_literal_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        origin: Origin,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
        value: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        let module = node.module_id;
        let value_node = value.into_global_any(module);

        // select declaration-backed variants from the matched input
        if let Some(case) = self.variant_expression_case(module, value)? {
            // decide bound qualifier segments before reading their references
            if let dir::Expression::Member { left, .. } = self.module(module).view().get(value) {
                let left = *left;
                self.decide_qualifier_segments(module, left)?;
            }

            return self.select_variant_pattern(node, origin, case, &[]);
        }

        // infer ordinary closed pattern expressions
        let ty = self.infer_node_type(
            FlowSite {
                node: value_node,
                flow,
                scope,
            },
            PlaceUse::Read,
        )?;

        // closed literal values select literal predicates
        let written = self.shallow_resolve(ty)?;
        let literal = self.ty(written)?.singleton_literal();
        match literal {
            // test the matched input against the selected literal
            Some(literal) => {
                let predicate = dir::Predicate::unary(
                    dir::PredicateOperand::direct(input),
                    dir::PredicateCondition::Literal(literal),
                )
                .with_narrowed(ty);

                self.commit_pattern(
                    node,
                    dir::PatternDecision::Test(Box::new(dir::PatternPredicateResolution {
                        predicate,
                    })),
                )
            }
            // error values already explain themselves
            None if self.has_error_operand(&[ty])? => {
                self.commit_pattern(node, dir::PatternDecision::Ignore)
            }
            // report every other pattern value
            None => {
                self.report_expression_pattern_not_literal(module, node.local_id.into_any());

                self.commit_pattern(node, dir::PatternDecision::Ignore)
            }
        }
    }

    /// Select one range pattern from its closed bounds.
    pub(in crate::sema) fn select_range_pattern(
        &mut self,
        node: dir::GlobalNodeId<dir::Pattern>,
        flow: FlowPointId,
        scope: Option<dir::GlobalGenericTemplateId>,
        input: dir::GlobalTypeId,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
        end_kind: dir::RangeEnd,
    ) -> CompilerResult<()> {
        let module = node.module_id;

        // read both written bounds as literals
        let mut bounds = [None, None];
        for (slot, bound) in [start, end].into_iter().enumerate() {
            let Some(bound) = bound else {
                continue;
            };
            let bound_node = bound.into_global_any(module);
            let ty = self.infer_node_type(
                FlowSite {
                    node: bound_node,
                    flow,
                    scope,
                },
                PlaceUse::Read,
            )?;
            if let dir::Type::Literal(literal) = self.ty(ty)? {
                bounds[slot] = Some(literal);
            }
        }

        // narrow successful matches to the represented interval
        let domain = input;
        let written = dir::RangeType::new(bounds[0], bounds[1], end_kind);
        let narrowed = self.range_pattern_narrowed_type(module, domain, &written)?;
        let predicate = dir::Predicate::unary(
            dir::PredicateOperand::direct(domain),
            dir::PredicateCondition::Range(dir::PredicateRange {
                domain,
                start: bounds[0],
                end: bounds[1],
                end_bound: end_kind,
            }),
        )
        .with_narrowed(narrowed);

        // commit the selected test
        self.commit_pattern(
            node,
            dir::PatternDecision::Test(Box::new(dir::PatternPredicateResolution { predicate })),
        )
    }

    /// Return the type visible after a successful range pattern.
    fn range_pattern_narrowed_type(
        &mut self,
        _module: ModuleId,
        domain: dir::GlobalTypeId,
        written: &dir::RangeType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // intersect the written interval with the domain
        let narrowed = match self.ty(domain)? {
            // intersect nested intervals exactly
            dir::Type::Range(domain) => domain
                .intersection(written)
                .map(dir::Type::Range)
                .unwrap_or(dir::Type::Never),
            // intersect fixed-width integer primitives when their bounds fit DIR ranges
            dir::Type::Primitive(dir::PrimitiveType::Integer(integer)) => {
                match integer.finite_interval() {
                    Some(domain) => domain
                        .intersection(written)
                        .map(dir::Type::Range)
                        .unwrap_or(dir::Type::Never),
                    // unrepresentable widths keep the written interval
                    None => dir::Type::Range(*written),
                }
            }
            // otherwise the written interval is the strongest represented test type
            _ => dir::Type::Range(*written),
        };

        self.intern_type(narrowed)
    }
}

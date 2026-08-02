use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, DecisionKind, ObligationCheck, ObligationFailure, Origin, Relation,
    UncoveredValue, answer,
};
use crate::{CompilerError, CompilerResult};

/// Scalar interval coverage represented by one pattern.
#[derive(Debug, Clone, PartialEq)]
enum IntervalCoverage {
    /// Every interval value is covered.
    All,
    /// Specific intervals are covered.
    Intervals(Vec<dir::RangeType>),
}

/// One statically known range pattern bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StaticRangeBound {
    /// The bound is omitted.
    Open,
    /// The bound is a scalar literal.
    Literal(dir::ScalarLiteral),
}

impl StaticRangeBound {
    /// Return the literal value of this bound.
    fn literal(self) -> Option<dir::ScalarLiteral> {
        match self {
            Self::Open => None,
            Self::Literal(literal) => Some(literal),
        }
    }
}

impl CheckState<'_> {
    /// Check whether one pattern is irrefutable for its matched value type.
    pub(in crate::check) fn check_irrefutable_pattern(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        value: dir::GlobalTypeId,
        failure: impl FnOnce(dir::GlobalNodeIdAny, UncoveredValue) -> ObligationFailure,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let decision = self.decide_pattern_covers(origin, pattern, value)?;
        let check = if answer!(decision) {
            ObligationCheck::holds()
        } else {
            let missing = answer!(self.uncovered_value(origin, &[pattern], value)?);

            ObligationCheck::fail(failure(source, missing))
        };

        Ok(Answer::Ready(check))
    }

    /// Decide whether any pattern alternative covers one value type.
    pub(in crate::check) fn decide_patterns_cover(
        &mut self,
        origin: Origin,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let value = answer!(self.strip_form(origin, value)?);

        // match untagged newtypes through their backing
        if answer!(self.variant_discriminant_domain(origin, value)?).is_none()
            && let Some(instance) = self.decompose_newtype(origin, value)?
        {
            let backing = instance.backing;
            return self.decide_patterns_cover(origin, patterns, backing);
        }

        // cover unions arm by arm
        if let dir::Type::Union(union) = self.ty(value)? {
            let elements: SmallVec<[_; 4]> =
                SmallVec::from_slice(self.type_ids(value.module_id, union.elements)?);
            let mut decision = Answer::Ready(true);
            for element in elements {
                decision = decision.and(self.decide_patterns_cover(origin, patterns, element)?);
                if decision.is_ready_false() {
                    return Ok(decision);
                }
            }

            return Ok(decision);
        }

        // cover variant domains case by case
        if let Some(domain) = answer!(self.variant_discriminant_domain(origin, value)?) {
            let mut decision = Answer::Ready(true);
            for discriminant in domain {
                decision =
                    decision.and(self.decide_patterns_cover_variant_case(patterns, discriminant)?);
                if decision.is_ready_false() {
                    return Ok(decision);
                }
            }

            return Ok(decision);
        }

        // cover finite scalar domains value by value
        if let Some(domain) = self.ty(value)?.finite_literals() {
            let mut decision = Answer::Ready(true);
            for literal in domain {
                let element = self.intern_type(dir::Type::Literal(literal))?;
                decision =
                    decision.and(self.decide_patterns_cover_value(origin, patterns, element)?);
                if decision.is_ready_false() {
                    return Ok(decision);
                }
            }

            return Ok(decision);
        }

        // cover scalar intervals by subtracting pattern intervals
        if let dir::Type::Range(domain) = self.ty(value)? {
            return self.decide_patterns_cover_range(origin, patterns, &domain);
        }

        self.decide_patterns_cover_value(origin, patterns, value)
    }

    /// Return one uncovered value for a failed coverage check.
    pub(in crate::check) fn uncovered_value(
        &mut self,
        origin: Origin,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<UncoveredValue>> {
        let value = answer!(self.strip_form(origin, value)?);

        // test untagged newtypes through their backing
        if let Answer::Ready(None) = self.variant_discriminant_domain(origin, value)?
            && let Some(instance) = self.decompose_newtype(origin, value)?
        {
            let backing = instance.backing;
            return self.uncovered_value(origin, patterns, backing);
        }

        // descend into the first uncovered union element
        if let dir::Type::Union(union) = self.ty(value)? {
            let elements: SmallVec<[_; 4]> =
                SmallVec::from_slice(self.type_ids(value.module_id, union.elements)?);
            for element in elements {
                if !answer!(self.decide_patterns_cover(origin, patterns, element)?) {
                    return self.uncovered_value(origin, patterns, element);
                }
            }

            return Ok(Answer::Ready(UncoveredValue::Type(value)));
        }

        // name the first uncovered variant case
        let variant_domain = answer!(self.variant_discriminant_domain(origin, value)?);
        if let Some(domain) = variant_domain {
            for discriminant in domain {
                if !answer!(self.decide_patterns_cover_variant_case(patterns, discriminant)?) {
                    if let Some(key) = self.enum_case_key_from_discriminant(value, discriminant)? {
                        return Ok(Answer::Ready(UncoveredValue::VariantCase {
                            ty: value,
                            key,
                        }));
                    }
                    if let Some(key) =
                        answer!(self.tagged_case_key_from_type(origin, value, discriminant)?)
                    {
                        return Ok(Answer::Ready(UncoveredValue::VariantCase {
                            ty: value,
                            key,
                        }));
                    }

                    let literal = self.intern_type(dir::Type::Literal(discriminant))?;
                    return Ok(Answer::Ready(UncoveredValue::Type(literal)));
                }
            }
        }

        // name the first uncovered finite scalar value
        if let Some(domain) = self.ty(value)?.finite_literals() {
            for literal in domain {
                let element = self.intern_type(dir::Type::Literal(literal))?;
                if !answer!(self.decide_patterns_cover(origin, patterns, element)?) {
                    return Ok(Answer::Ready(UncoveredValue::Type(element)));
                }
            }
        }

        // name the first uncovered scalar interval
        if let dir::Type::Range(domain) = self.ty(value)?
            && let Some(range) = answer!(self.uncovered_range(origin, patterns, &domain)?)
        {
            let ty = match range.singleton_literal() {
                Some(literal) => dir::Type::Literal(literal),
                None => dir::Type::Range(range),
            };
            let range = self.intern_type(ty)?;

            return Ok(Answer::Ready(UncoveredValue::Type(range)));
        }

        Ok(Answer::Ready(UncoveredValue::Type(value)))
    }

    /// Decide whether one pattern covers every value in one type.
    pub(in crate::check) fn decide_pattern_covers(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        self.decide_patterns_cover(origin, &[pattern], value)
    }

    /// Decide whether any pattern alternative covers one closed value.
    fn decide_patterns_cover_value(
        &mut self,
        origin: Origin,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(false);
        for pattern in patterns {
            decision = decision.or(self.decide_pattern_node_covers(origin, *pattern, value)?);
            if decision.is_ready_true() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether any pattern alternative covers one tagged discriminant.
    fn decide_patterns_cover_variant_case(
        &mut self,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        discriminant: dir::ScalarLiteral,
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(false);
        for pattern in patterns {
            decision =
                decision.or(self.decide_pattern_covers_variant_case(*pattern, discriminant)?);
            if decision.is_ready_true() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether one pattern covers one variant discriminant.
    fn decide_pattern_covers_variant_case(
        &mut self,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        discriminant: dir::ScalarLiteral,
    ) -> CompilerResult<Answer<bool>> {
        let kind = self.decision_kind(pattern.into_any());
        let resolution = self
            .resolutions(pattern.module_id)
            .pattern_resolution(pattern.into_any())
            .cloned();
        let resolution = match (kind, resolution) {
            (Some(DecisionKind::Pattern), Some(resolution)) => resolution,
            (Some(DecisionKind::Rejected), None) => return Ok(Answer::Ready(false)),
            (Some(kind), resolution) => {
                return Err(CompilerError::Internal {
                    message: format!(
                        "coverage pattern {pattern:?} decided as {kind:?} with resolution {resolution:?}"
                    ),
                });
            }
            (None, _) => {
                return Err(CompilerError::Internal {
                    message: format!("coverage pattern {pattern:?} is undecided"),
                });
            }
        };

        let covers = match &resolution {
            // wildcard shapes cover every discriminant
            dir::PatternResolution::Ignore
            | dir::PatternResolution::Bind(dir::PatternBindingResolution {
                pattern: None, ..
            }) => true,

            // variants cover their selected discriminant
            dir::PatternResolution::Variant(resolution) => matches!(
                &resolution.predicate.test,
                dir::PredicateTest::Unary(test)
                    if matches!(
                        test.condition,
                        dir::PredicateCondition::Literal(selected) if selected == discriminant
                    )
            ),

            // discriminant tests cover their tested literal
            dir::PatternResolution::Test(resolution) => matches!(
                &resolution.predicate.test,
                dir::PredicateTest::Unary(test)
                    if matches!(
                        &test.input,
                        dir::PredicateOperand::Projected(projection)
                            if matches!(projection.as_ref(), dir::Projection::VariantTag { .. })
                    ) && matches!(
                        test.condition,
                        dir::PredicateCondition::Literal(selected) if selected == discriminant
                    )
            ),

            // defaulted patterns cover through their inner pattern
            dir::PatternResolution::Default(resolution) => {
                let inner = resolution.pattern.into_typed();

                answer!(self.decide_pattern_covers_variant_case(inner, discriminant)?)
            }

            // or patterns cover when any branch covers
            dir::PatternResolution::Or(or) => {
                let mut covered = false;
                for branch in &or.patterns {
                    let branch = branch.into_typed();
                    if answer!(self.decide_pattern_covers_variant_case(branch, discriminant)?) {
                        covered = true;

                        break;
                    }
                }

                covered
            }

            _ => false,
        };

        Ok(Answer::Ready(covers))
    }

    /// Decide whether one pattern node covers one non-union value type.
    fn decide_pattern_node_covers(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let module = pattern.module_id;
        if let Some(decision) = self.decide_pattern_resolution_covers(origin, pattern, value)? {
            return Ok(decision);
        }

        let view = self.module(module).view();
        match view.get(pattern.local_id) {
            // wildcards and bare bindings always cover
            dir::Pattern::Wildcard | dir::Pattern::Binding { pattern: None, .. } => {
                Ok(Answer::Ready(true))
            }
            // pattern forms cover through their contained pattern
            dir::Pattern::Binding {
                pattern: Some(inner),
                ..
            }
            | dir::Pattern::Must(inner)
            | dir::Pattern::BorrowOf { right: inner, .. }
            | dir::Pattern::MoveOf { right: inner, .. }
            | dir::Pattern::DereferenceOf { right: inner } => {
                let inner = *inner;

                self.decide_pattern_covers(origin, inner.into_global(module), value)
            }
            // defaults cover absent values before testing the nested pattern
            dir::Pattern::Default { pattern: inner, .. } => {
                if self.ty(value)?.is_undefined() {
                    Ok(Answer::Ready(true))
                } else {
                    self.decide_pattern_covers(origin, inner.into_global(module), value)
                }
            }
            // expression patterns cover values their type absorbs
            dir::Pattern::Expression { value: expression } => {
                let expression = *expression;
                let expected = self.require_node_type(expression.into_global_any(module))?;

                self.decide_relation(origin, Relation::Assignable, value, expected)
            }
            // range patterns cover scalar values inside their interval
            dir::Pattern::Range {
                start,
                end,
                end_kind,
            } => {
                let (start, end, end_kind) = (*start, *end, *end_kind);

                self.decide_range_pattern_covers(origin, module, start, end, end_kind, value)
            }
            // field patterns must each cover their projections
            dir::Pattern::Tuple { fields }
            | dir::Pattern::Sequence { fields }
            | dir::Pattern::Object { fields } => {
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.decide_fields_cover(origin, module, &fields, value)
            }
            // nominal patterns check the tag then their payload fields
            dir::Pattern::NominalTuple { fields, .. }
            | dir::Pattern::NominalObject { fields, .. } => {
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();
                let resolution = self
                    .resolutions(module)
                    .pattern_resolution(pattern.into_any())
                    .cloned()
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!("nominal coverage pattern {pattern:?} has no resolution"),
                    })?;
                // tagged variants decide through their selected predicate and payload
                if let dir::PatternResolution::Variant(variant) = &resolution {
                    if !answer!(self.decide_predicate_covers(origin, &variant.predicate, value)?) {
                        return Ok(Answer::Ready(false));
                    }
                    let Some(projection) = &variant.predicate.projection else {
                        return Ok(Answer::Ready(fields.is_empty()));
                    };
                    let payload = projection.ty();

                    return self.decide_fields_cover(origin, module, &fields, payload);
                }
                let dir::PatternResolution::Destructure(resolution) = resolution else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "nominal coverage pattern {pattern:?} has non-nominal resolution: {resolution:?}"
                        ),
                    });
                };
                let dir::PatternDestructureResolution::Nominal(nominal) = resolution.as_ref()
                else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "nominal coverage pattern {pattern:?} has non-nominal destructuring"
                        ),
                    });
                };

                // one constructor covers every instantiation of its symbol
                let value_head = answer!(self.reduce_type_head(origin, value)?);
                match self.ty(value_head)? {
                    dir::Type::Application(value_instance) => {
                        // inherited constructors cover through heritage
                        if value_instance.symbol != nominal.symbol {
                            let closure = answer!(self.heritage_closure(
                                origin,
                                value_head.module_id,
                                &value_instance,
                            )?);
                            let mut inherits = false;
                            for application in &closure.applications {
                                let (_, instance) =
                                    self.require_nominal_application(application.ty)?;
                                if instance.symbol == nominal.symbol {
                                    inherits = true;
                                    break;
                                }
                            }
                            if !inherits {
                                return Ok(Answer::Ready(false));
                            }
                        }
                    }
                    _ => return Ok(Answer::Ready(false)),
                }

                // project the newtype payload behind the tag when present
                let payload = match self.decompose_newtype(origin, value)? {
                    Some(instance) => {
                        answer!(self.reduce_type_head(origin, instance.backing)?)
                    }
                    None => value,
                };

                self.decide_fields_cover(origin, module, &fields, payload)
            }
            dir::Pattern::Union { patterns } => {
                let patterns = patterns
                    .iter()
                    .map(|pattern| pattern.into_global(module))
                    .collect::<SmallVec<[_; 4]>>();

                self.decide_patterns_cover(origin, &patterns, value)
            }
        }
    }

    /// Decide coverage directly from one selected pattern resolution.
    fn decide_pattern_resolution_covers(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Answer<bool>>> {
        let resolution = self
            .resolutions(pattern.module_id)
            .pattern_resolution(pattern.into_any())
            .cloned();

        match resolution {
            Some(dir::PatternResolution::Test(resolution)) => {
                let decision =
                    self.decide_predicate_covers(origin, &resolution.predicate, value)?;

                Ok(Some(decision))
            }
            Some(dir::PatternResolution::Variant(resolution)) => {
                let decision =
                    self.decide_predicate_covers(origin, &resolution.predicate, value)?;

                Ok(Some(decision))
            }
            _ => Ok(None),
        }
    }

    /// Decide whether one predicate-backed pattern covers one closed value.
    fn decide_predicate_covers(
        &mut self,
        origin: Origin,
        predicate: &dir::Predicate,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        match &predicate.test {
            dir::PredicateTest::Unary(unary) => {
                self.decide_unary_predicate_covers(origin, unary, value)
            }
            dir::PredicateTest::Any(predicates) => {
                let mut decision = Answer::Ready(false);
                for predicate in predicates {
                    decision = decision.or(self.decide_predicate_covers(origin, predicate, value)?);
                    if decision.is_ready_true() {
                        return Ok(decision);
                    }
                }

                Ok(decision)
            }
            dir::PredicateTest::Membership(_) => Ok(Answer::Ready(false)),
        }
    }

    /// Decide whether one unary predicate-backed pattern covers one closed value.
    fn decide_unary_predicate_covers(
        &mut self,
        origin: Origin,
        predicate: &dir::PredicateUnaryTest,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let value = answer!(self.reduce_type_head(origin, value)?);
        let decision = match &predicate.condition {
            dir::PredicateCondition::Always => true,
            dir::PredicateCondition::Never => false,
            dir::PredicateCondition::Literal(literal) => {
                self.type_scalar_literal(value)? == Some(*literal)
            }
            dir::PredicateCondition::Range(range) => {
                let Some(literal) = self.type_scalar_literal(value)? else {
                    return Ok(Answer::Ready(false));
                };

                range.contains_literal(literal)
            }
            dir::PredicateCondition::Primitive(_)
            | dir::PredicateCondition::Type(_)
            | dir::PredicateCondition::Subtype(_) => false,
        };

        Ok(Answer::Ready(decision))
    }

    /// Decide whether field patterns each cover their projected values.
    fn decide_fields_cover(
        &mut self,
        origin: Origin,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);

        for field in fields {
            let field = self.module(module).view().get(*field).clone();
            let field_decision = match field {
                // bare fields and elisions always cover
                dir::PatternField::Named { pattern: None, .. }
                | dir::PatternField::Rest { pattern: None }
                | dir::PatternField::Elision => Answer::Ready(true),
                // named fields cover their projected member values
                dir::PatternField::Named {
                    name,
                    pattern: Some(pattern),
                    ..
                } => {
                    let key = name.static_key();
                    let is_defaulted = matches!(
                        self.module(module).view().get(pattern),
                        dir::Pattern::Default { .. }
                    );
                    let lookup = answer!(self.body().lookup_member(
                        origin,
                        module,
                        value,
                        dir::MemberSpace::Instance,
                        key
                    )?);
                    let member = self.body().member_read_type(origin, &lookup)?;

                    match member {
                        Some(member) => {
                            self.decide_pattern_covers(origin, pattern.into_global(module), member)?
                        }
                        None => {
                            if is_defaulted {
                                let undefined = self.intern_type(dir::Type::Undefined)?;

                                self.decide_pattern_covers(
                                    origin,
                                    pattern.into_global(module),
                                    undefined,
                                )?
                            } else {
                                Answer::Ready(false)
                            }
                        }
                    }
                }
                // inner fields cover through the whole value
                dir::PatternField::Computed { pattern, .. }
                | dir::PatternField::Positional { pattern }
                | dir::PatternField::Rest {
                    pattern: Some(pattern),
                } => self.decide_pattern_covers(origin, pattern.into_global(module), value)?,
            };

            decision = decision.and(field_decision);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether pattern alternatives cover one scalar interval.
    fn decide_patterns_cover_range(
        &mut self,
        origin: Origin,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        domain: &dir::RangeType,
    ) -> CompilerResult<Answer<bool>> {
        let uncovered = answer!(self.uncovered_range(origin, patterns, domain)?);

        Ok(Answer::Ready(uncovered.is_none()))
    }

    /// Return the first uncovered interval left after pattern subtraction.
    fn uncovered_range(
        &mut self,
        origin: Origin,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        domain: &dir::RangeType,
    ) -> CompilerResult<Answer<Option<dir::RangeType>>> {
        let mut uncovered = vec![*domain];

        // subtract each pattern's interval coverage
        for pattern in patterns {
            let Some(coverage) = answer!(self.pattern_range_coverage(origin, *pattern)?) else {
                continue;
            };
            let intervals = match coverage {
                IntervalCoverage::All => return Ok(Answer::Ready(None)),
                IntervalCoverage::Intervals(intervals) => intervals,
            };

            // apply every covered interval to the current remainder
            for interval in intervals {
                uncovered = subtract_intervals(uncovered, &interval);
                if uncovered.is_empty() {
                    return Ok(Answer::Ready(None));
                }
            }
        }

        Ok(Answer::Ready(uncovered.into_iter().next()))
    }

    /// Return scalar interval coverage represented by one pattern.
    fn pattern_range_coverage(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeId<dir::Pattern>,
    ) -> CompilerResult<Answer<Option<IntervalCoverage>>> {
        let module = pattern.module_id;
        let pattern_node = self.module(module).view().get(pattern.local_id).clone();

        let coverage = match pattern_node {
            // wildcards and bare bindings cover every interval value
            dir::Pattern::Wildcard | dir::Pattern::Binding { pattern: None, .. } => {
                Some(IntervalCoverage::All)
            }
            // pattern forms cover through their contained pattern
            dir::Pattern::Binding {
                pattern: Some(inner),
                ..
            }
            | dir::Pattern::Must(inner)
            | dir::Pattern::BorrowOf { right: inner, .. }
            | dir::Pattern::MoveOf { right: inner, .. }
            | dir::Pattern::DereferenceOf { right: inner }
            | dir::Pattern::Default { pattern: inner, .. } => {
                let inner = inner.into_global(module);

                return self.pattern_range_coverage(origin, inner);
            }
            // expression patterns cover literal points
            dir::Pattern::Expression { value } => {
                let ty = self.require_node_type(value.into_global_any(module))?;
                let ty = answer!(self.reduce_type_head(origin, ty)?);
                self.type_scalar_literal(ty)?.map(|literal| {
                    IntervalCoverage::Intervals(vec![dir::RangeType {
                        start: Some(literal),
                        end: Some(literal),
                        is_inclusive: true,
                    }])
                })
            }
            // range patterns cover their written interval
            dir::Pattern::Range {
                start,
                end,
                end_kind,
            } => {
                let Some(range) = answer!(
                    self.static_range_pattern_interval(origin, module, start, end, end_kind)?
                ) else {
                    return Ok(Answer::Ready(None));
                };

                Some(IntervalCoverage::Intervals(vec![range]))
            }
            // union patterns cover the union of their child coverages
            dir::Pattern::Union { patterns } => {
                let mut intervals = Vec::new();
                for pattern in patterns {
                    let pattern = pattern.into_global(module);
                    let Some(coverage) = answer!(self.pattern_range_coverage(origin, pattern)?)
                    else {
                        return Ok(Answer::Ready(None));
                    };

                    match coverage {
                        IntervalCoverage::All => {
                            return Ok(Answer::Ready(Some(IntervalCoverage::All)));
                        }
                        IntervalCoverage::Intervals(covered) => intervals.extend(covered),
                    }
                }

                Some(IntervalCoverage::Intervals(intervals))
            }
            // destructuring patterns do not describe scalar interval coverage
            dir::Pattern::Tuple { .. }
            | dir::Pattern::Sequence { .. }
            | dir::Pattern::Object { .. }
            | dir::Pattern::NominalTuple { .. }
            | dir::Pattern::NominalObject { .. } => None,
        };

        Ok(Answer::Ready(coverage))
    }

    /// Decide whether one range pattern covers one scalar value type.
    fn decide_range_pattern_covers(
        &mut self,
        origin: Origin,
        module: ModuleId,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
        end_kind: dir::RangeEnd,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // require a scalar literal value to test an interval
        let dir::Type::Literal(literal) = self.ty(value)? else {
            return Ok(Answer::Ready(false));
        };

        // closed range patterns can be used for static coverage
        let Some(range) =
            answer!(self.static_range_pattern_interval(origin, module, start, end, end_kind)?)
        else {
            return Ok(Answer::Ready(false));
        };

        let covered = range.contains_literal(literal);

        Ok(Answer::Ready(covered))
    }

    /// Return one statically known range pattern interval.
    fn static_range_pattern_interval(
        &mut self,
        origin: Origin,
        module: ModuleId,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
        end_kind: dir::RangeEnd,
    ) -> CompilerResult<Answer<Option<dir::RangeType>>> {
        let Some(start) = answer!(self.static_range_bound(origin, module, start)?) else {
            return Ok(Answer::Ready(None));
        };
        let Some(end) = answer!(self.static_range_bound(origin, module, end)?) else {
            return Ok(Answer::Ready(None));
        };

        let range = dir::RangeType::new(start.literal(), end.literal(), end_kind);

        Ok(Answer::Ready(Some(range)))
    }

    /// Return one statically known range pattern bound.
    fn static_range_bound(
        &mut self,
        origin: Origin,
        module: ModuleId,
        bound: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Answer<Option<StaticRangeBound>>> {
        let Some(bound) = bound else {
            return Ok(Answer::Ready(Some(StaticRangeBound::Open)));
        };
        let ty = self.require_node_type(bound.into_global_any(module))?;

        let reduced = answer!(self.reduce_type_head(origin, ty)?);
        match self.ty(reduced)? {
            dir::Type::Literal(literal) => {
                Ok(Answer::Ready(Some(StaticRangeBound::Literal(literal))))
            }
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Return the scalar literal represented by one closed scalar value type.
    fn type_scalar_literal(
        &self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::ScalarLiteral>> {
        let literal = match self.ty(value)? {
            dir::Type::Literal(literal) => Some(literal),
            dir::Type::Null => Some(dir::ScalarLiteral::Null),
            dir::Type::Undefined => Some(dir::ScalarLiteral::Undefined),
            _ => None,
        };

        Ok(literal)
    }
}

/// Subtract one interval from interval remainders.
fn subtract_intervals(
    intervals: Vec<dir::RangeType>,
    removed: &dir::RangeType,
) -> Vec<dir::RangeType> {
    let mut remaining = Vec::with_capacity(intervals.len() + 1);

    // subtract from each current interval independently
    for interval in intervals {
        for remainder in interval.subtract_range(removed).into_iter().flatten() {
            remaining.push(remainder);
        }
    }

    remaining
}

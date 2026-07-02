use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckError, CheckState, Decision, Dependency, Origin, Relation, answer,
};
use crate::{CompilerResult, DiagnosticAnchor};

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
    /// Return the literal value carried by this bound.
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
        source: dir::GlobalNodeIdAny,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        value: dir::GlobalTypeId,
        diagnostic: impl FnOnce(DiagnosticAnchor, ModuleId, String) -> DiagnosticBuilder<CheckError>,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let origin = Origin::Node(source);
        let decision = self.decide_pattern_covers(origin, pattern, value)?;
        let diagnostic = if answer!(decision) {
            None
        } else {
            let missing = self.uncovered_witness(origin, &[pattern], value)?;
            let (module, anchor) = self.source_anchor(source);

            let error = diagnostic(anchor, module, missing);

            Some(error)
        };

        Ok(Answer::Ready(diagnostic))
    }

    /// Decide whether any pattern alternative covers one value type.
    pub(in crate::check) fn decide_patterns_cover(
        &mut self,
        origin: Origin,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let value = answer!(self.reduce_type_head(origin, value)?);

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

        // cover tagged domains case by case
        if let Some(domain) = answer!(self.tagged_discriminant_domain(origin, value)?) {
            let mut decision = Answer::Ready(true);
            for discriminant in domain {
                decision = decision.and(self.decide_patterns_cover_tagged_case(
                    origin,
                    patterns,
                    discriminant,
                )?);
                if decision.is_ready_false() {
                    return Ok(decision);
                }
            }

            return Ok(decision);
        }

        // cover finite scalar domains value by value
        if let Some(domain) = self.finite_scalar_domain(value)? {
            let source = self.origin_source_node(origin)?;
            let mut decision = Answer::Ready(true);
            for literal in domain {
                let element = self.intern_type(origin.module(), dir::Type::Literal(literal))?;
                decision = decision.and(self.decide_patterns_cover(origin, patterns, element)?);
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

        let mut decision = Answer::Ready(false);

        // accept any covering pattern alternative
        for pattern in patterns {
            decision = decision.or(self.decide_pattern_node_covers(origin, *pattern, value)?);
            if decision.is_ready_true() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return one uncovered value written form for a failed coverage check.
    /// Mirrors the union and finite domain splits to find one witness.
    pub(in crate::check) fn uncovered_witness(
        &mut self,
        origin: Origin,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        value: dir::GlobalTypeId,
    ) -> CompilerResult<String> {
        let value = match self.reduce_type_head(origin, value)? {
            Answer::Ready(value) => value,
            Answer::Pending(_) => return Ok(self.format_type(value)),
        };

        // descend into the first uncovered union element
        if let dir::Type::Union(union) = self.ty(value)? {
            let elements: SmallVec<[_; 4]> =
                SmallVec::from_slice(self.type_ids(value.module_id, union.elements)?);
            for element in elements {
                if self
                    .decide_patterns_cover(origin, patterns, element)?
                    .is_ready_false()
                {
                    return self.uncovered_witness(origin, patterns, element);
                }
            }

            return Ok(self.format_type(value));
        }

        // name the first uncovered tagged case
        let tagged_domain = match self.tagged_discriminant_domain(origin, value)? {
            Answer::Ready(domain) => domain,
            Answer::Pending(_) => None,
        };
        if let Some(domain) = tagged_domain {
            let source = self.origin_source_node(origin)?;
            for discriminant in domain {
                if self
                    .decide_patterns_cover_tagged_case(origin, patterns, discriminant)?
                    .is_ready_false()
                {
                    if let Some(key) =
                        self.tagged_case_key_from_discriminant(origin.module(), discriminant)
                    {
                        return Ok(self.format_variant_case(value, key));
                    }

                    let literal =
                        self.intern_type(origin.module(), dir::Type::Literal(discriminant))?;
                    return Ok(self.format_type(literal));
                }
            }
        }

        // name the first uncovered finite scalar value
        if let Some(domain) = self.finite_scalar_domain(value)? {
            let source = self.origin_source_node(origin)?;
            for literal in domain {
                let element = self.intern_type(origin.module(), dir::Type::Literal(literal))?;
                if self
                    .decide_patterns_cover(origin, patterns, element)?
                    .is_ready_false()
                {
                    return Ok(self.format_type(element));
                }
            }
        }

        // name the first uncovered scalar interval
        if let dir::Type::Range(domain) = self.ty(value)? {
            match self.uncovered_range(origin, patterns, &domain)? {
                Answer::Ready(Some(range)) => {
                    let source = self.origin_source_node(origin)?;
                    let ty = match range.singleton_literal() {
                        Some(literal) => dir::Type::Literal(literal),
                        None => dir::Type::Range(range),
                    };
                    let range = self.intern_type(origin.module(), ty)?;

                    return Ok(self.format_type(range));
                }
                Answer::Ready(None) => {}
                Answer::Pending(_) => return Ok(self.format_type(value)),
            }
        }

        Ok(self.format_type(value))
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

    /// Decide whether any pattern alternative covers one tagged discriminant.
    fn decide_patterns_cover_tagged_case(
        &self,
        origin: Origin,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        discriminant: dir::ScalarLiteral,
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(false);
        for pattern in patterns {
            decision = decision.or(self.decide_pattern_covers_tagged_case(
                origin,
                *pattern,
                discriminant,
            )?);
            if decision.is_ready_true() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether one pattern covers one tagged discriminant.
    /// Undecided patterns park on their decision instead of denying coverage.
    fn decide_pattern_covers_tagged_case(
        &self,
        origin: Origin,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        discriminant: dir::ScalarLiteral,
    ) -> CompilerResult<Answer<bool>> {
        let Some(decision) = self.decision(pattern.into_any()) else {
            return Ok(Answer::pending([Dependency::Decision(pattern.into_any())]));
        };

        let covers = match decision {
            // wildcard shapes cover every discriminant
            Decision::Pattern(dir::PatternResolution::Ignore)
            | Decision::Pattern(dir::PatternResolution::Bind(dir::PatternBindingResolution {
                pattern: None,
                ..
            })) => true,

            // variant destructures cover their selected discriminant
            Decision::Pattern(dir::PatternResolution::Destructure(
                dir::PatternDestructureResolution::Variant(resolution),
            )) => match &resolution.projection {
                dir::Projection::VariantPayload {
                    discriminant: selected,
                    ..
                } => *selected == discriminant,
                _ => false,
            },

            // defaulted patterns cover through their inner pattern
            Decision::Pattern(dir::PatternResolution::Default(resolution)) => {
                let inner = resolution.pattern.into_typed();

                answer!(self.decide_pattern_covers_tagged_case(origin, inner, discriminant)?)
            }

            // or patterns cover when any branch covers
            Decision::Pattern(dir::PatternResolution::Or(or)) => {
                let mut covered = false;
                for branch in &or.patterns {
                    let branch = branch.into_typed();
                    if answer!(self.decide_pattern_covers_tagged_case(
                        origin,
                        branch,
                        discriminant
                    )?) {
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
                let expected =
                    answer!(self.committed_node_type(expression.into_global_any(module))?);

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
            dir::Pattern::NominalTuple { ty, fields }
            | dir::Pattern::NominalObject { ty, fields } => {
                let ty = *ty;
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();
                let tag = answer!(self.committed_node_type(ty.into_global_any(module))?);
                let tag_decision =
                    self.decide_relation(origin, Relation::Assignable, value, tag)?;
                if !tag_decision.is_ready_true() {
                    return Ok(tag_decision);
                }

                // project the newtype payload behind the tag when present
                let payload = answer!(self.newtype_payload(origin, value)?);

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
        let Some(decision) = self.decision(pattern.into_any()).cloned() else {
            return Ok(None);
        };

        match decision {
            Decision::Pattern(dir::PatternResolution::Test(resolution)) => {
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
            dir::PredicateTest::Has(_) | dir::PredicateTest::Call(_) => Ok(Answer::Ready(false)),
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
                | dir::PatternField::Spread { pattern: None }
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
                    let lookup = answer!(self.lookup_member(
                        origin,
                        module,
                        value,
                        dir::MemberSpace::Instance,
                        key
                    )?);
                    let member = lookup.value_type();

                    match member {
                        Some(member) => {
                            self.decide_pattern_covers(origin, pattern.into_global(module), member)?
                        }
                        None => {
                            if is_defaulted {
                                let source = self.origin_source_node(origin)?;
                                let undefined = self.intern_type(module, dir::Type::Undefined)?;

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
                | dir::PatternField::Spread {
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

    /// Project the substituted newtype payload behind one nominal value.
    /// Values without a newtype backing keep themselves as the payload.
    fn newtype_payload(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let instance = match self.ty(value)? {
            dir::Type::Instance(instance) => instance,
            _ => return Ok(Answer::Ready(value)),
        };
        let backing = match self.definition(instance.symbol) {
            Some(dir::Definition::Newtype(definition)) => definition.value,
            _ => return Ok(Answer::Ready(value)),
        };

        // substitute applied arguments through the backing
        let substitution = self.instance_substitution(value.module_id, &instance)?;
        let backing = self.substitute_type(origin.module(), backing, &substitution)?;

        self.reduce_type_head(origin, backing)
    }

    /// Return the finite scalar domain of one closed type when it has one.
    fn finite_scalar_domain(
        &self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::ScalarLiteral>>> {
        let domain = match self.ty(value)?.scalar_domain() {
            Some(dir::ScalarDomain::Boolean) => vec![
                dir::ScalarLiteral::Boolean(false),
                dir::ScalarLiteral::Boolean(true),
            ],
            _ => return Ok(None),
        };

        Ok(Some(domain))
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
                let ty = answer!(self.committed_node_type(value.into_global_any(module))?);
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
        // only scalar literal values sit inside intervals
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
        let ty = answer!(self.committed_node_type(bound.into_global_any(module))?);

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

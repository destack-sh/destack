use smallvec::SmallVec;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{
    CheckState, ObligationCheck, ObligationFailure, Origin, Relation, UncoveredValue, Verdict,
};
use crate::{CompilerError, CompilerResult};

/// One coverage case a splittable value domain decomposes into.
#[derive(Debug, Clone, Copy)]
enum CoverageCase {
    /// The case covers one whole type.
    Type(dir::GlobalTypeId),
    /// The case covers one variant discriminant.
    Discriminant(dir::Literal),
}

/// One arm's remaining pattern fields over the coverage columns.
type CoverageArm = SmallVec<[CoverageField; 4]>;

/// One field position within a coverage arm.
#[derive(Debug, Clone, Copy)]
enum CoverageField {
    /// The field is one pattern node.
    Pattern(dir::GlobalNodeId<dir::Pattern>),
    /// The field covers any value.
    Wildcard,
}

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
    Literal(dir::Literal),
}

impl StaticRangeBound {
    /// Return the literal value of this bound.
    fn literal(self) -> Option<dir::Literal> {
        match self {
            Self::Open => None,
            Self::Literal(literal) => Some(literal),
        }
    }
}

impl CheckState<'_> {
    /// Check whether one pattern is irrefutable for its matched value type.
    pub(in crate::sema) fn check_irrefutable_pattern(
        &mut self,
        origin: Origin,
        source: dir::GlobalNodeIdAny,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        value: dir::GlobalTypeId,
        failure: impl FnOnce(dir::GlobalNodeIdAny, UncoveredValue) -> ObligationFailure,
    ) -> CompilerResult<ObligationCheck> {
        // an undecided cover reports the value as uncovered, matching today's collapse
        let covered = self.decide_pattern_covers(origin, pattern, value)?;
        let check = if covered.holds() {
            ObligationCheck::holds()
        } else {
            let missing = self.uncovered_value(origin, &[pattern], value)?;

            ObligationCheck::fail(failure(source, missing))
        };

        Ok(check)
    }

    /// Decide whether any pattern alternative covers one value type.
    pub(in crate::sema) fn decide_patterns_cover(
        &mut self,
        origin: Origin,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let value = self.strip_form(origin, value)?;

        // match newtypes through their backing
        if self.variant_discriminant_domain(value)?.is_none()
            && let Some(instance) = self.decompose_newtype(origin, value)?
        {
            let backing = instance.backing;

            return self.decide_patterns_cover(origin, patterns, backing);
        }

        // cover unions arm by arm
        if let dir::Type::Union(union) = self.ty(value)? {
            let elements: SmallVec<[_; 4]> =
                SmallVec::from_slice(self.type_ids(value.module_id, union.elements)?);
            let mut covered = Verdict::Holds;
            for element in elements {
                covered = covered.and(self.decide_patterns_cover(origin, patterns, element)?);
                if covered == Verdict::Fails {
                    break;
                }
            }

            return Ok(covered);
        }

        // cover variant domains case by case
        if let Some(domain) = self.variant_discriminant_domain(value)? {
            let mut covered = Verdict::Holds;
            for discriminant in domain {
                covered =
                    covered.and(self.decide_variant_case_cover(origin, patterns, discriminant)?);
                if covered == Verdict::Fails {
                    break;
                }
            }

            return Ok(covered);
        }

        // cover finite scalar domains value by value
        if let Some(domain) = self.ty(value)?.finite_literals() {
            let mut covered = Verdict::Holds;
            for literal in domain {
                let element = self.intern_type(dir::Type::Literal(literal))?;
                covered = covered.and(self.decide_patterns_cover_value(origin, patterns, element)?);
                if covered == Verdict::Fails {
                    break;
                }
            }

            return Ok(covered);
        }

        // cover scalar intervals by subtracting pattern intervals
        if let dir::Type::Range(domain) = self.ty(value)? {
            let (uncovered, unreadable) = self.uncovered_range(patterns, &domain)?;

            return Ok(Verdict::decided(uncovered.is_none()).join_undecided(unreadable));
        }

        // cover tuple domains by specializing pattern arms per element case
        if let dir::Type::Tuple(tuple) = self.ty(value)?
            && let Some(elements) = self.tuple_element_types(value, tuple.elements)?
        {
            let (arms, whole) = self.tuple_coverage_arms(origin, patterns, value, &elements)?;
            if whole.holds() {
                return Ok(Verdict::Holds);
            }

            let covered = self.decide_arms_cover(origin, arms, &elements)?;

            return Ok(covered.join_undecided(whole));
        }

        self.decide_patterns_cover_value(origin, patterns, value)
    }

    /// Build the coverage arms over one tuple's elements, with the verdict of covering it whole.
    fn tuple_coverage_arms(
        &mut self,
        origin: Origin,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        value: dir::GlobalTypeId,
        elements: &[dir::GlobalTypeId],
    ) -> CompilerResult<(Vec<CoverageArm>, Verdict)> {
        let mut arms = Vec::new();
        let mut whole = Verdict::Fails;
        for pattern in patterns {
            // take a matching-width tuple pattern's fields as one arm
            if let Some(arm) = self.nested_coverage_arm(*pattern, elements.len())? {
                arms.push(arm);

                continue;
            }

            // accept the whole domain from one covering pattern
            whole = whole.or(self.decide_pattern_node_covers(origin, *pattern, value)?);
            if whole.holds() {
                break;
            }
        }

        Ok((arms, whole))
    }

    /// Build one coverage arm from a tuple pattern of one width, or none for other pattern shapes.
    fn nested_coverage_arm(
        &mut self,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        width: usize,
    ) -> CompilerResult<Option<CoverageArm>> {
        let module = pattern.module_id;
        let fields = match self.module(module).view().get(pattern.local_id) {
            dir::Pattern::Tuple { fields } if fields.len() == width => Some(fields.clone()),
            _ => None,
        };

        // build the arm from a tuple of the requested width
        match fields {
            Some(fields) => self.coverage_arm(module, &fields),
            None => Ok(None),
        }
    }

    /// Build one coverage arm from tuple pattern fields, or none past a rest field.
    fn coverage_arm(
        &mut self,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
    ) -> CompilerResult<Option<CoverageArm>> {
        let mut arm = CoverageArm::new();
        for field in fields {
            match self.module(module).view().get(*field) {
                // positional and named fields contribute their inner pattern
                dir::PatternField::Positional { pattern } => {
                    arm.push(CoverageField::Pattern(pattern.into_global(module)));
                }
                dir::PatternField::Named {
                    pattern: Some(pattern),
                    ..
                } => {
                    arm.push(CoverageField::Pattern(pattern.into_global(module)));
                }
                // bare names and elisions cover their position
                dir::PatternField::Named { pattern: None, .. } | dir::PatternField::Elision => {
                    arm.push(CoverageField::Wildcard);
                }
                // rest and computed fields break positional coverage
                dir::PatternField::Rest { .. } | dir::PatternField::Computed { .. } => {
                    return Ok(None);
                }
            }
        }

        Ok(Some(arm))
    }

    /// Decide whether one arm field covers one head case, wildcards covering every case.
    fn decide_field_covers_case(
        &mut self,
        origin: Origin,
        field: CoverageField,
        case: CoverageCase,
    ) -> CompilerResult<Verdict> {
        match (field, case) {
            (CoverageField::Wildcard, _) => Ok(Verdict::Holds),
            (CoverageField::Pattern(pattern), CoverageCase::Type(ty)) => {
                self.decide_pattern_node_covers(origin, pattern, ty)
            }
            (CoverageField::Pattern(pattern), CoverageCase::Discriminant(literal)) => {
                self.decide_pattern_covers_variant_case(pattern, literal)
            }
        }
    }

    /// Decide whether the arms cover every combination over the remaining columns.
    fn decide_arms_cover(
        &mut self,
        origin: Origin,
        arms: Vec<CoverageArm>,
        columns: &[dir::GlobalTypeId],
    ) -> CompilerResult<Verdict> {
        // cover an exhausted column list with any surviving arm
        let Some((&head, rest)) = columns.split_first() else {
            return Ok(Verdict::decided(!arms.is_empty()));
        };

        // cover every head case through its specialized arms
        let mut covered = Verdict::Holds;
        for case in self.coverage_cases(origin, head)? {
            let (specialized, columns, dropped) =
                self.specialize_arms(origin, &arms, case, rest)?;

            // an escaping case leaves the whole combination uncovered
            let case_covered = match specialized.is_empty() {
                true => Verdict::Fails,
                false => self.decide_arms_cover(origin, specialized, &columns)?,
            };

            covered = covered.and(case_covered.join_undecided(dropped));
            if covered == Verdict::Fails {
                break;
            }
        }

        Ok(covered)
    }

    /// Specialize the arms against one head case, with the verdict of the arms it drops.
    fn specialize_arms(
        &mut self,
        origin: Origin,
        arms: &[CoverageArm],
        case: CoverageCase,
        rest: &[dir::GlobalTypeId],
    ) -> CompilerResult<(Vec<CoverageArm>, Vec<dir::GlobalTypeId>, Verdict)> {
        // expand a tuple-typed case's elements into the column list
        let expanded = match case {
            CoverageCase::Type(ty) => match self.ty(ty)? {
                dir::Type::Tuple(tuple) => self.tuple_element_types(ty, tuple.elements)?,
                _ => None,
            },
            CoverageCase::Discriminant(_) => None,
        };

        // put the expanded elements ahead of the remaining columns
        let columns = match &expanded {
            Some(elements) => elements.iter().chain(rest.iter()).copied().collect(),
            None => rest.to_vec(),
        };

        // specialize each arm against the case
        let mut specialized = Vec::new();
        let mut dropped = Verdict::Fails;
        for arm in arms {
            let Some((&field, tail)) = arm.split_first() else {
                return Err(CompilerError::Internal {
                    message: "coverage arm is shorter than its column list".to_string(),
                });
            };

            // refine an expanded case through the arm's own nested tuple pattern
            if let Some(elements) = &expanded
                && let CoverageField::Pattern(pattern) = field
                && let Some(nested) = self.nested_coverage_arm(pattern, elements.len())?
            {
                let mut arm = nested;
                arm.extend(tail.iter().copied());
                specialized.push(arm);

                continue;
            }

            // drop arms whose head field escapes the case, wildcards cover every case
            let covers = self.decide_field_covers_case(origin, field, case)?;
            if !covers.holds() {
                dropped = dropped.or(covers);

                continue;
            }

            // widen the covering field over the expanded columns
            if let Some(elements) = &expanded {
                let mut arm: CoverageArm =
                    std::iter::repeat_n(CoverageField::Wildcard, elements.len()).collect();
                arm.extend(tail.iter().copied());
                specialized.push(arm);
            }
            // keep the arm on its tail alone
            else {
                specialized.push(SmallVec::from_slice(tail));
            }
        }

        Ok((specialized, columns, dropped))
    }

    /// Decompose one value into its coverage cases, an indivisible type staying one case.
    fn coverage_cases(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[CoverageCase; 4]>> {
        let value = self.strip_form(origin, value)?;

        // decompose newtypes through their backing
        if self.variant_discriminant_domain(value)?.is_none()
            && let Some(instance) = self.decompose_newtype(origin, value)?
        {
            return self.coverage_cases(origin, instance.backing);
        }

        // flatten union elements into their own cases
        if let dir::Type::Union(union) = self.ty(value)? {
            let elements: SmallVec<[_; 4]> =
                SmallVec::from_slice(self.type_ids(value.module_id, union.elements)?);
            let mut cases = SmallVec::new();
            for element in elements {
                cases.extend(self.coverage_cases(origin, element)?);
            }

            return Ok(cases);
        }

        // split variant domains by discriminant
        if let Some(domain) = self.variant_discriminant_domain(value)? {
            return Ok(domain.into_iter().map(CoverageCase::Discriminant).collect());
        }

        // split finite scalar domains by literal
        if let Some(domain) = self.ty(value)?.finite_literals() {
            let mut cases = SmallVec::new();
            for literal in domain {
                let element = self.intern_type(dir::Type::Literal(literal))?;
                cases.push(CoverageCase::Type(element));
            }

            return Ok(cases);
        }

        Ok(SmallVec::from_elem(CoverageCase::Type(value), 1))
    }

    /// Return one uncovered value for a failed coverage check.
    ///
    /// An undecided cover counts as uncovered, matching the failed check that asks for a witness.
    pub(in crate::sema) fn uncovered_value(
        &mut self,
        origin: Origin,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        value: dir::GlobalTypeId,
    ) -> CompilerResult<UncoveredValue> {
        let value = self.strip_form(origin, value)?;

        // test newtypes through their backing
        if self.variant_discriminant_domain(value)?.is_none()
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
                if !self
                    .decide_patterns_cover(origin, patterns, element)?
                    .holds()
                {
                    return self.uncovered_value(origin, patterns, element);
                }
            }

            return Ok(UncoveredValue::Type(value));
        }

        // name the first uncovered variant case, by its enum case key where one exists
        if let Some(domain) = self.variant_discriminant_domain(value)? {
            for discriminant in domain {
                let covered = self.decide_patterns_cover_variant_case(patterns, discriminant)?;
                if covered.holds() {
                    continue;
                }

                if let Some(key) = self.enum_case_key_from_discriminant(value, discriminant)? {
                    return Ok(UncoveredValue::VariantCase { ty: value, key });
                }

                let literal = self.intern_type(dir::Type::Literal(discriminant))?;

                return Ok(UncoveredValue::Type(literal));
            }
        }

        // name the first uncovered finite scalar value
        if let Some(domain) = self.ty(value)?.finite_literals() {
            for literal in domain {
                let element = self.intern_type(dir::Type::Literal(literal))?;
                if !self
                    .decide_patterns_cover(origin, patterns, element)?
                    .holds()
                {
                    return Ok(UncoveredValue::Type(element));
                }
            }
        }

        // name the first uncovered scalar interval
        if let dir::Type::Range(domain) = self.ty(value)?
            && let (Some(range), _) = self.uncovered_range(patterns, &domain)?
        {
            let ty = match range.singleton_literal() {
                Some(literal) => dir::Type::Literal(literal),
                None => dir::Type::Range(range),
            };
            let range = self.intern_type(ty)?;

            return Ok(UncoveredValue::Type(range));
        }

        // name one uncovered tuple element combination
        if let dir::Type::Tuple(tuple) = self.ty(value)?
            && let Some(elements) = self.tuple_element_types(value, tuple.elements)?
        {
            let form = tuple.form;
            let (arms, whole) = self.tuple_coverage_arms(origin, patterns, value, &elements)?;
            if !whole.holds()
                && let Some(witness) = self.uncovered_arms_witness(origin, arms, &elements)?
            {
                // intern the witness combination as one tuple type
                let elements = witness
                    .into_iter()
                    .map(dir::TypeElement::new)
                    .collect::<SmallVec<[_; 4]>>();
                let elements = self.intern_elements(&elements)?;
                let witness =
                    self.intern_type(dir::Type::Tuple(dir::TupleType { form, elements }))?;

                return Ok(UncoveredValue::Type(witness));
            }
        }

        Ok(UncoveredValue::Type(value))
    }

    /// Return one uncovered element combination, or none when the arms cover every combination.
    fn uncovered_arms_witness(
        &mut self,
        origin: Origin,
        arms: Vec<CoverageArm>,
        columns: &[dir::GlobalTypeId],
    ) -> CompilerResult<Option<Vec<dir::GlobalTypeId>>> {
        // report an exhausted column list only without surviving arms
        let Some((&head, rest)) = columns.split_first() else {
            return Ok(arms.is_empty().then(Vec::new));
        };

        // walk the head cases in order, naming the first failing path
        for case in self.coverage_cases(origin, head)? {
            let case_type = match case {
                CoverageCase::Type(ty) => ty,
                CoverageCase::Discriminant(literal) => {
                    self.intern_type(dir::Type::Literal(literal))?
                }
            };

            // keep each arm whose head field covers the case
            let mut specialized = Vec::new();
            for arm in &arms {
                let Some((&field, tail)) = arm.split_first() else {
                    return Err(CompilerError::Internal {
                        message: "coverage arm is shorter than its column list".to_string(),
                    });
                };
                if self.decide_field_covers_case(origin, field, case)?.holds() {
                    specialized.push(SmallVec::from_slice(tail));
                }
            }

            // leave the remaining columns whole under an escaping case
            let tail = if specialized.is_empty() {
                Some(rest.to_vec())
            } else {
                self.uncovered_arms_witness(origin, specialized, rest)?
            };

            // compose the witness from this case and the failing tail
            if let Some(tail) = tail {
                let mut witness = vec![case_type];
                witness.extend(tail);

                return Ok(Some(witness));
            }
        }

        Ok(None)
    }

    /// Decide whether one pattern covers every value in one type.
    pub(in crate::sema) fn decide_pattern_covers(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        self.decide_patterns_cover(origin, &[pattern], value)
    }

    /// Decide whether any pattern alternative covers one closed value.
    fn decide_patterns_cover_value(
        &mut self,
        origin: Origin,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let mut covered = Verdict::Fails;
        for pattern in patterns {
            covered = covered.or(self.decide_pattern_node_covers(origin, *pattern, value)?);
            if covered.holds() {
                break;
            }
        }

        Ok(covered)
    }

    /// Decide whether the patterns jointly cover one variant case and its payload.
    fn decide_variant_case_cover(
        &mut self,
        origin: Origin,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        discriminant: dir::Literal,
    ) -> CompilerResult<Verdict> {
        // collect payload arms from the arms selecting this discriminant
        let mut arms = Vec::new();
        let mut payload = None;
        let mut selects = Verdict::Fails;
        for pattern in patterns {
            let selected = self.decide_pattern_covers_variant_case(*pattern, discriminant)?;
            selects = selects.or(selected);
            if !selected.holds() {
                continue;
            }

            // accept the whole case from an arm without payload destructuring
            let Some((fields, projected)) = self.variant_payload_fields(*pattern)? else {
                return Ok(Verdict::Holds);
            };

            // keep the arm's own field cover when its shapes build no arm
            let Some(arm) = self.coverage_arm(pattern.module_id, &fields)? else {
                if let Some(projected) = projected {
                    let covered =
                        self.decide_fields_cover(origin, pattern.module_id, &fields, projected)?;
                    if covered.holds() {
                        return Ok(Verdict::Holds);
                    }

                    // keep the case undecided while its field cover stays open
                    if covered == Verdict::Ambiguous {
                        selects = Verdict::Ambiguous;
                    }
                }

                continue;
            };
            payload = payload.or(projected);
            arms.push(arm);
        }

        // settle a payloadless case by any selecting arm
        let Some(payload) = payload else {
            return Ok(selects);
        };
        if arms.is_empty() {
            return Ok(Verdict::Fails.join_undecided(selects));
        }

        // cover the payload columns jointly, single payloads standing as one column
        let columns = match self.ty(payload)? {
            dir::Type::Tuple(tuple) => self.tuple_element_types(payload, tuple.elements)?,
            _ => Some(SmallVec::from_elem(payload, 1)),
        };
        let Some(columns) = columns else {
            return Ok(Verdict::Fails);
        };

        let covered = self.decide_arms_cover(origin, arms, &columns)?;

        Ok(covered.join_undecided(selects))
    }

    /// Return one variant arm's payload fields and projection, or none without destructuring.
    fn variant_payload_fields(
        &mut self,
        pattern: dir::GlobalNodeId<dir::Pattern>,
    ) -> CompilerResult<
        Option<(
            Vec<dir::LocalNodeId<dir::PatternField>>,
            Option<dir::GlobalTypeId>,
        )>,
    > {
        // only variant decisions with destructuring fields carry payload arms
        let Some(dir::Decision::Pattern(dir::PatternDecision::Variant(resolution))) =
            self.decision(pattern.into_any()).cloned()
        else {
            return Ok(None);
        };
        let module = pattern.module_id;
        let fields = match self.module(module).view().get(pattern.local_id) {
            dir::Pattern::NominalTuple { fields, .. } => fields.clone(),
            _ => return Ok(None),
        };
        if fields.is_empty() {
            return Ok(None);
        }
        let projected = resolution
            .predicate
            .projection
            .as_ref()
            .map(|projection| projection.ty());

        Ok(Some((fields, projected)))
    }

    /// Decide whether any pattern alternative covers one variant discriminant.
    fn decide_patterns_cover_variant_case(
        &mut self,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        discriminant: dir::Literal,
    ) -> CompilerResult<Verdict> {
        let mut covered = Verdict::Fails;
        for pattern in patterns {
            covered = covered.or(self.decide_pattern_covers_variant_case(*pattern, discriminant)?);
            if covered.holds() {
                break;
            }
        }

        Ok(covered)
    }

    /// Decide whether one pattern covers one variant discriminant.
    fn decide_pattern_covers_variant_case(
        &mut self,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        discriminant: dir::Literal,
    ) -> CompilerResult<Verdict> {
        let resolution = match self.decision(pattern.into_any()).cloned() {
            Some(dir::Decision::Pattern(resolution)) => resolution,
            // a rejected pattern carries no shape to decide against
            Some(dir::Decision::Rejected | dir::Decision::Poisoned) => {
                return Ok(Verdict::Ambiguous);
            }
            Some(decision) => {
                return Err(CompilerError::Internal {
                    message: format!("coverage pattern {pattern:?} decided as {decision:?}"),
                });
            }
            None => {
                return Err(CompilerError::Internal {
                    message: format!("coverage pattern {pattern:?} is undecided"),
                });
            }
        };

        // decide coverage by the pattern's own resolution
        let covers = match &resolution {
            // wildcard shapes cover every discriminant
            dir::PatternDecision::Ignore
            | dir::PatternDecision::Bind(dir::PatternBindingResolution { pattern: None, .. }) => {
                Verdict::Holds
            }

            // variants cover their selected discriminant
            dir::PatternDecision::Variant(resolution) => Verdict::decided(matches!(
                &resolution.predicate.test,
                dir::PredicateTest::Unary(test)
                    if matches!(
                        test.condition,
                        dir::PredicateCondition::Literal(selected) if selected == discriminant
                    )
            )),

            // discriminant tests cover their tested literal
            dir::PatternDecision::Test(resolution) => Verdict::decided(matches!(
                &resolution.predicate.test,
                dir::PredicateTest::Unary(test)
                    if matches!(
                        &test.input,
                        dir::PredicateOperand::Projected(projection)
                            if matches!(projection.as_ref(), dir::Projection::Discriminant { .. })
                    ) && matches!(
                        test.condition,
                        dir::PredicateCondition::Literal(selected) if selected == discriminant
                    )
            )),

            // defaulted patterns cover through their inner pattern
            dir::PatternDecision::Default(resolution) => {
                let inner = resolution.pattern.into_typed();

                self.decide_pattern_covers_variant_case(inner, discriminant)?
            }

            // or patterns cover when any branch covers
            dir::PatternDecision::Or(or) => {
                let mut covered = Verdict::Fails;
                for branch in &or.patterns {
                    let branch = branch.into_typed();
                    covered =
                        covered.or(self.decide_pattern_covers_variant_case(branch, discriminant)?);
                    if covered.holds() {
                        break;
                    }
                }

                covered
            }

            // TODO #Incomplete: bound patterns like `name @ Some(value)` skip their nested pattern
            _ => Verdict::Fails,
        };

        Ok(covers)
    }

    /// Decide whether one pattern node covers one non-union value type.
    fn decide_pattern_node_covers(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let module = pattern.module_id;
        if let Some(covered) = self.decide_pattern_decision_covers(pattern, value)? {
            return Ok(covered);
        }

        // decide coverage by the pattern's own syntax
        let view = self.module(module).view();
        match view.get(pattern.local_id) {
            // wildcards and bare bindings always cover
            dir::Pattern::Wildcard | dir::Pattern::Binding { pattern: None, .. } => {
                Ok(Verdict::Holds)
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
                let inner = *inner;
                let value_type = self.ty(value)?;
                if value_type.is_undefined() {
                    Ok(Verdict::Holds)
                } else {
                    self.decide_pattern_covers(origin, inner.into_global(module), value)
                }
            }
            // expression patterns cover values their type absorbs
            dir::Pattern::Expression { value: expression } => {
                let expression = *expression;
                let expected = self.require_node_type(expression.into_global_any(module))?;

                self.decide_relation(origin, Relation::Subtype, value, expected)
            }
            // range patterns cover scalar values inside their interval
            dir::Pattern::Range {
                start,
                end,
                end_kind,
            } => {
                let (start, end, end_kind) = (*start, *end, *end_kind);

                self.decide_range_pattern_covers(module, start, end, end_kind, value)
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
                    .decisions(module)
                    .pattern_decision(pattern.into_any())
                    .cloned()
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!("nominal coverage pattern {pattern:?} has no resolution"),
                    })?;

                // enum variants decide through their selected predicate
                if let dir::PatternDecision::Variant(variant) = &resolution {
                    let covers = self.decide_predicate_covers(&variant.predicate, value)?;
                    if !covers.holds() {
                        return Ok(covers);
                    }
                    let Some(projection) = &variant.predicate.projection else {
                        return Ok(Verdict::decided(fields.is_empty()));
                    };
                    let payload = projection.ty();

                    return self.decide_fields_cover(origin, module, &fields, payload);
                }

                // newtype patterns project their payload, nominal patterns destructure it
                let key = match &resolution {
                    dir::PatternDecision::Destructure(resolution) => {
                        let dir::PatternDestructureResolution::Nominal(nominal) =
                            resolution.as_ref()
                        else {
                            return Err(CompilerError::Internal {
                                message: format!(
                                    "nominal coverage pattern {pattern:?} has non-nominal destructuring"
                                ),
                            });
                        };

                        nominal.key.clone()
                    }
                    dir::PatternDecision::Project(projection)
                        if let dir::OperationResolution::One(dir::Projection::NewtypePayload {
                            key,
                            ..
                        }) = &projection.projection =>
                    {
                        key.clone()
                    }
                    resolution => {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "nominal coverage pattern {pattern:?} has non-nominal resolution: {resolution:?}"
                            ),
                        });
                    }
                };

                // cover every instantiation of a symbol with one constructor
                match self.ty(value)? {
                    dir::Type::Application(value_instance) => {
                        // inherited constructors cover through heritage
                        if value_instance.symbol != key.symbol {
                            let closure = self.heritage_closure(origin, value)?;
                            let mut inherits = false;
                            for application in &closure.applications {
                                let (_, instance) = self.nominal_application(application.ty)?;
                                if instance.symbol == key.symbol {
                                    inherits = true;
                                    break;
                                }
                            }
                            if !inherits {
                                return Ok(Verdict::Fails);
                            }
                        }
                    }
                    _ => return Ok(Verdict::Fails),
                }

                // project the newtype payload behind the tag when present
                let payload = match self.decompose_newtype(origin, value)? {
                    Some(instance) => instance.backing,
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
    fn decide_pattern_decision_covers(
        &mut self,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Verdict>> {
        let resolution = self
            .decisions(pattern.module_id)
            .pattern_decision(pattern.into_any())
            .cloned();

        // read coverage out of a predicate-backed resolution
        match resolution {
            Some(dir::PatternDecision::Test(resolution)) => {
                let covered = self.decide_predicate_covers(&resolution.predicate, value)?;

                Ok(Some(covered))
            }
            Some(dir::PatternDecision::Variant(resolution)) => {
                let covered = self.decide_predicate_covers(&resolution.predicate, value)?;

                Ok(Some(covered))
            }
            _ => Ok(None),
        }
    }

    /// Decide whether one predicate-backed pattern covers one closed value.
    fn decide_predicate_covers(
        &mut self,
        predicate: &dir::Predicate,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        match &predicate.test {
            dir::PredicateTest::Unary(unary) => self.decide_unary_predicate_covers(unary, value),
            dir::PredicateTest::Any(predicates) => {
                let mut covered = Verdict::Fails;
                for predicate in predicates {
                    covered = covered.or(self.decide_predicate_covers(predicate, value)?);
                    if covered.holds() {
                        break;
                    }
                }

                Ok(covered)
            }
            // membership tests carry no static coverage
            dir::PredicateTest::Membership(_) => Ok(Verdict::Ambiguous),
        }
    }

    /// Decide whether one unary predicate-backed pattern covers one closed value.
    fn decide_unary_predicate_covers(
        &mut self,
        predicate: &dir::PredicateUnaryTest,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let covers = match &predicate.condition {
            dir::PredicateCondition::Always => Verdict::Holds,
            dir::PredicateCondition::Never => Verdict::Fails,
            dir::PredicateCondition::Literal(literal) => {
                Verdict::decided(self.type_scalar_literal(value)? == Some(*literal))
            }
            dir::PredicateCondition::Range(range) => match self.type_scalar_literal(value)? {
                Some(literal) => Verdict::decided(range.contains_literal(literal)),
                None => Verdict::Fails,
            },
            // TODO #Incomplete: type tests decide coverage through their tested type
            dir::PredicateCondition::Primitive(_)
            | dir::PredicateCondition::Type(_)
            | dir::PredicateCondition::Subtype(_) => Verdict::Ambiguous,
        };

        Ok(covers)
    }

    /// Decide whether field patterns each cover their projected values.
    fn decide_fields_cover(
        &mut self,
        origin: Origin,
        module: ModuleId,
        fields: &[dir::LocalNodeId<dir::PatternField>],
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let mut covered = Verdict::Holds;
        for field in fields {
            let field = self.module(module).view().get(*field).clone();
            let field_covered = match field {
                // bare fields and elisions always cover
                dir::PatternField::Named { pattern: None, .. }
                | dir::PatternField::Rest { pattern: None }
                | dir::PatternField::Elision => Verdict::Holds,
                // named fields cover their projected member values
                dir::PatternField::Named {
                    name,
                    pattern: Some(pattern),
                    ..
                } => {
                    let key = name.into();
                    let is_defaulted = matches!(
                        self.module(module).view().get(pattern),
                        dir::Pattern::Default { .. }
                    );
                    let subject =
                        self.member_subject(origin, value, value, dir::MemberSpace::Instance)?;
                    let lookup = self.lookup_member(origin, module, subject, key)?;
                    let member = self.member_read_type(&lookup)?;

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
                                Verdict::Fails
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

            covered = covered.and(field_covered);
            if covered == Verdict::Fails {
                break;
            }
        }

        Ok(covered)
    }

    /// Return the first uncovered interval left after pattern subtraction.
    ///
    /// The verdict reports the patterns whose intervals stay unreadable.
    fn uncovered_range(
        &mut self,
        patterns: &[dir::GlobalNodeId<dir::Pattern>],
        domain: &dir::RangeType,
    ) -> CompilerResult<(Option<dir::RangeType>, Verdict)> {
        let mut uncovered = vec![*domain];
        let mut unreadable = Verdict::Fails;

        // subtract each pattern's interval coverage
        for pattern in patterns {
            // an unreadable pattern may still cover part of the domain
            let Some(coverage) = self.pattern_range_coverage(*pattern)? else {
                unreadable = Verdict::Ambiguous;

                continue;
            };
            let intervals = match coverage {
                IntervalCoverage::All => return Ok((None, unreadable)),
                IntervalCoverage::Intervals(intervals) => intervals,
            };

            // apply every covered interval to the current remainder
            for interval in intervals {
                uncovered = subtract_intervals(uncovered, &interval);
                if uncovered.is_empty() {
                    return Ok((None, unreadable));
                }
            }
        }

        Ok((uncovered.into_iter().next(), unreadable))
    }

    /// Return the scalar interval coverage one pattern represents, or none when it reads none.
    fn pattern_range_coverage(
        &mut self,
        pattern: dir::GlobalNodeId<dir::Pattern>,
    ) -> CompilerResult<Option<IntervalCoverage>> {
        let module = pattern.module_id;
        let pattern_node = self.module(module).view().get(pattern.local_id).clone();

        // read the interval coverage by the pattern's own syntax
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

                return self.pattern_range_coverage(inner);
            }
            // expression patterns cover literal points
            dir::Pattern::Expression { value } => {
                let ty = self.require_node_type(value.into_global_any(module))?;
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
                let Some(range) =
                    self.static_range_pattern_interval(module, start, end, end_kind)?
                else {
                    return Ok(None);
                };

                Some(IntervalCoverage::Intervals(vec![range]))
            }
            // union patterns cover the union of their child coverages
            dir::Pattern::Union { patterns } => {
                let mut intervals = Vec::new();
                for pattern in patterns {
                    let pattern = pattern.into_global(module);
                    let Some(coverage) = self.pattern_range_coverage(pattern)? else {
                        return Ok(None);
                    };

                    match coverage {
                        IntervalCoverage::All => {
                            return Ok(Some(IntervalCoverage::All));
                        }
                        IntervalCoverage::Intervals(covered) => intervals.extend(covered),
                    }
                }

                Some(IntervalCoverage::Intervals(intervals))
            }
            // destructuring patterns describe field coverage
            dir::Pattern::Tuple { .. }
            | dir::Pattern::Sequence { .. }
            | dir::Pattern::Object { .. }
            | dir::Pattern::NominalTuple { .. }
            | dir::Pattern::NominalObject { .. } => None,
        };

        Ok(coverage)
    }

    /// Decide whether one range pattern covers one scalar value type.
    fn decide_range_pattern_covers(
        &mut self,
        module: ModuleId,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
        end_kind: dir::RangeEnd,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // test the written interval against a scalar literal value
        // TODO #Incomplete: interval values need interval subtraction to decide
        let dir::Type::Literal(literal) = self.ty(value)? else {
            return Ok(Verdict::Fails);
        };

        // written bounds decide the interval only when they read statically
        let Some(range) = self.static_range_pattern_interval(module, start, end, end_kind)? else {
            return Ok(Verdict::Ambiguous);
        };

        Ok(Verdict::decided(range.contains_literal(literal)))
    }

    /// Return one statically known range pattern interval.
    fn static_range_pattern_interval(
        &mut self,
        module: ModuleId,
        start: Option<dir::LocalNodeId<dir::Expression>>,
        end: Option<dir::LocalNodeId<dir::Expression>>,
        end_kind: dir::RangeEnd,
    ) -> CompilerResult<Option<dir::RangeType>> {
        let Some(start) = self.static_range_bound(module, start)? else {
            return Ok(None);
        };
        let Some(end) = self.static_range_bound(module, end)? else {
            return Ok(None);
        };

        let range = dir::RangeType::new(start.literal(), end.literal(), end_kind);

        Ok(Some(range))
    }

    /// Return one statically known range pattern bound.
    fn static_range_bound(
        &mut self,
        module: ModuleId,
        bound: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Option<StaticRangeBound>> {
        let Some(bound) = bound else {
            return Ok(Some(StaticRangeBound::Open));
        };
        let ty = self.require_node_type(bound.into_global_any(module))?;

        // read a statically known bound from a scalar literal only
        match self.ty(ty)? {
            dir::Type::Literal(literal) => Ok(Some(StaticRangeBound::Literal(literal))),
            _ => Ok(None),
        }
    }

    /// Return the scalar literal represented by one closed scalar value type.
    fn type_scalar_literal(
        &self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::Literal>> {
        let literal = self.ty(value)?.singleton_literal();

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

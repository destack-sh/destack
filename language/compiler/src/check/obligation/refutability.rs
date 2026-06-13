use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckError, CheckState, MemberLookup, Origin, Relation};

impl CheckState<'_> {
    /// Check whether one pattern is irrefutable for its matched value type.
    pub(in crate::check) fn check_irrefutable_pattern(
        &mut self,
        source: dir::GlobalNodeIdAny,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let origin = Origin::Node(source);
        let decision = self.decide_pattern_covers(origin, pattern, value)?;
        let diagnostic = match decision {
            Answer::Ready(true) => None,
            Answer::Ready(false) => {
                let missing = self.uncovered_witness(origin, &[pattern], value)?;
                let (module, anchor) = self.source_anchor(source);

                let error = CheckError::RefutablePattern {
                    anchor,
                    module,
                    missing,
                };

                Some(error.help("handle the uncovered values with 'if let' or 'match'"))
            }
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
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
        let mut decision = Answer::Ready(false);

        // accept any covering pattern alternative
        for pattern in patterns {
            decision = decision.or(self.decide_pattern_covers(origin, *pattern, value)?);
            if decision == Answer::Ready(true) {
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
        let value = match self.evaluate_root(origin, value)? {
            Answer::Ready(value) => value,
            Answer::Pending(_) => return Ok(self.format_type(value)),
        };

        // descend into the first uncovered union element
        if let dir::Type::Union(union) = self.ty(value)? {
            let elements = union.elements.iter().copied().collect::<SmallVec<[_; 4]>>();
            for element in elements {
                if self.decide_patterns_cover(origin, patterns, element)? == Answer::Ready(false) {
                    return self.uncovered_witness(origin, patterns, element);
                }
            }

            return Ok(self.format_type(value));
        }

        // name the first uncovered finite domain value
        if let Some(domain) = self.finite_scalar_domain(value)? {
            let source = self.origin_source_node(origin)?;
            for literal in domain {
                let element =
                    self.push_type(origin.module(), dir::Type::Literal(literal), source)?;
                if self.decide_patterns_cover(origin, patterns, element)? == Answer::Ready(false) {
                    return Ok(self.format_type(element));
                }
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
        // close the matched value first
        let value = match self.evaluate_root(origin, value)? {
            Answer::Ready(value) => value,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // cover unions element-wise
        if let dir::Type::Union(union) = self.ty(value)? {
            let elements = union.elements.iter().copied().collect::<SmallVec<[_; 4]>>();
            let mut decision = Answer::Ready(true);
            for element in elements {
                decision = decision.and(self.decide_pattern_covers(origin, pattern, element)?);
                if decision == Answer::Ready(false) {
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
                let element =
                    self.push_type(origin.module(), dir::Type::Literal(literal), source)?;
                decision = decision.and(self.decide_pattern_covers(origin, pattern, element)?);
                if decision == Answer::Ready(false) {
                    return Ok(decision);
                }
            }

            return Ok(decision);
        }

        self.decide_pattern_node_covers(origin, pattern, value)
    }

    /// Decide whether one pattern node covers one non-union value type.
    fn decide_pattern_node_covers(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalNodeId<dir::Pattern>,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let module = pattern.module_id;
        let view = self.module(module).view();

        match view.get(pattern.local_id) {
            // wildcards and bare bindings always cover
            dir::Pattern::Wildcard | dir::Pattern::Binding { pattern: None, .. } => {
                Ok(Answer::Ready(true))
            }
            // wrappers cover through their inner pattern
            dir::Pattern::Binding {
                pattern: Some(inner),
                ..
            }
            | dir::Pattern::Must(inner)
            | dir::Pattern::BorrowOf { right: inner, .. }
            | dir::Pattern::MoveOf { right: inner, .. }
            | dir::Pattern::DereferenceOf { right: inner }
            | dir::Pattern::Assign { pattern: inner, .. } => {
                let inner = *inner;

                self.decide_pattern_covers(origin, inner.into_global(module), value)
            }
            // expression patterns cover values their type absorbs
            dir::Pattern::Expression { value: expression } => {
                let expression = *expression;
                let Some(expected) = self.inputs.node_type(expression.into_global_any(module))
                else {
                    return Ok(Answer::Ready(false));
                };

                self.decide_relation(origin, Relation::Assignable, value, expected)
            }
            // range patterns cover scalar values inside their interval
            dir::Pattern::Range {
                start,
                end,
                end_kind,
            } => {
                let (start, end, end_kind) = (*start, *end, *end_kind);

                self.decide_range_covers(origin, module, start, end, end_kind, value)
            }
            // type patterns cover values assignable to their target
            dir::Pattern::TypeExpression { value: target } => {
                let target = *target;
                let Some(target) = self.inputs.node_type(target.into_global_any(module)) else {
                    return Ok(Answer::Ready(false));
                };

                self.decide_relation(origin, Relation::Assignable, value, target)
            }
            // field patterns must each cover their projections
            dir::Pattern::Tuple { fields }
            | dir::Pattern::Sequence { fields }
            | dir::Pattern::Object { fields } => {
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();

                self.decide_fields_cover(origin, module, &fields, value)
            }
            // nominal patterns check the tag then their payload fields
            dir::Pattern::Newtype { ty, fields } | dir::Pattern::NominalObject { ty, fields } => {
                let ty = *ty;
                let fields = fields.iter().copied().collect::<SmallVec<[_; 4]>>();
                let Some(tag) = self.inputs.node_type(ty.into_global_any(module)) else {
                    return Ok(Answer::Ready(false));
                };
                let tag_decision =
                    self.decide_relation(origin, Relation::Assignable, value, tag)?;
                if tag_decision != Answer::Ready(true) {
                    return Ok(tag_decision);
                }

                // project the newtype payload behind the tag when present
                let payload = match self.newtype_payload(origin, value)? {
                    Answer::Ready(payload) => payload,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
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
            let view = self.module(module).view();
            let field_decision = match view.get(*field) {
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
                    let (key, pattern) = (name.static_key(), *pattern);
                    let lookup =
                        self.lookup_member(origin, module, value, dir::MemberSpace::Instance, key)?;
                    let member = match lookup {
                        MemberLookup::Field(ty) => Some(ty),
                        MemberLookup::Found(candidates) => {
                            candidates.first().map(|candidate| candidate.ty)
                        }
                        MemberLookup::Missing => None,
                        MemberLookup::Pending(blockers) => {
                            return Ok(Answer::Pending(blockers));
                        }
                    };

                    match member {
                        Some(member) => {
                            self.decide_pattern_covers(origin, pattern.into_global(module), member)?
                        }
                        None => Answer::Ready(false),
                    }
                }
                // inner fields cover through the whole value
                dir::PatternField::Computed { pattern, .. }
                | dir::PatternField::Positional { pattern }
                | dir::PatternField::Spread {
                    pattern: Some(pattern),
                } => {
                    let pattern = *pattern;

                    self.decide_pattern_covers(origin, pattern.into_global(module), value)?
                }
            };

            decision = decision.and(field_decision);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether one range pattern covers one scalar value type.
    fn decide_range_covers(
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
        let literal = *literal;

        // read solved literal bounds
        let start = match self.range_bound_literal(origin, module, start)? {
            Answer::Ready(start) => start,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let end = match self.range_bound_literal(origin, module, end)? {
            Answer::Ready(end) => end,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        let covered = literal
            .scalar_in_range(start.as_ref(), end.as_ref(), end_kind)
            .unwrap_or(false);

        Ok(Answer::Ready(covered))
    }

    /// Return one solved range bound literal.
    fn range_bound_literal(
        &mut self,
        origin: Origin,
        module: ModuleId,
        bound: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> CompilerResult<Answer<Option<dir::ScalarLiteral>>> {
        let Some(bound) = bound else {
            return Ok(Answer::Ready(None));
        };
        let Some(ty) = self.inputs.node_type(bound.into_global_any(module)) else {
            return Ok(Answer::Ready(None));
        };

        let reduced = match self.evaluate_root(origin, ty)? {
            Answer::Ready(reduced) => reduced,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        match self.ty(reduced)? {
            dir::Type::Literal(literal) => Ok(Answer::Ready(Some(*literal))),
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Project the substituted newtype payload behind one nominal value.
    /// Values without a newtype backing keep themselves as the payload.
    fn newtype_payload(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let instance = match self.ty(value)? {
            dir::Type::Reference(instance) => instance.clone(),
            _ => return Ok(Answer::Ready(value)),
        };
        let backing = match self.definition(instance.symbol) {
            Some(dir::Definition::Newtype(definition)) => definition.value,
            _ => return Ok(Answer::Ready(value)),
        };

        // substitute applied arguments through the backing
        let substitution = self.parameter_substitution(&instance)?;
        let backing = if substitution.is_empty() {
            backing
        } else {
            let source = self.origin_source_node(origin)?;

            self.fold_type(origin.module(), source, backing, substitution.rewrite())?
        };

        self.evaluate_root(origin, backing)
            .map(|answer| match answer {
                Answer::Ready(backing) => Answer::Ready(backing),
                Answer::Pending(blockers) => Answer::Pending(blockers),
            })
    }

    /// Return the finite scalar domain of one closed type when it has one.
    fn finite_scalar_domain(
        &self,
        value: dir::GlobalTypeId,
    ) -> CompilerResult<Option<Vec<dir::ScalarLiteral>>> {
        let domain = match self.ty(value)? {
            dir::Type::Primitive(dir::PrimitiveType::Boolean) => vec![
                dir::ScalarLiteral::Boolean(false),
                dir::ScalarLiteral::Boolean(true),
            ],
            _ => return Ok(None),
        };

        Ok(Some(domain))
    }
}

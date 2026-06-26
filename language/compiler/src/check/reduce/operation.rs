use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Dependency, MemberLookup, Origin, Relation, TypeRewrite, Widening, answer,
};

/// One broad property-key domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum KeyDomain {
    /// String property names.
    String,
    /// Positional numeric property names.
    Usize,
    /// Symbol property names.
    Symbol,
}

/// One reduced `keyof` result before it is written as a type.
#[derive(Debug, Clone, Default)]
struct KeySet {
    /// The exact known keys.
    keys: IndexSet<dir::StaticKey>,
    /// The broad key domains accepted by index signatures.
    domains: IndexSet<KeyDomain>,
}

impl KeySet {
    /// Insert one exact key.
    fn insert_key(&mut self, key: dir::StaticKey) {
        self.keys.insert(key);
    }

    /// Insert one key domain.
    fn insert_domain(&mut self, domain: KeyDomain) {
        self.domains.insert(domain);
    }

    /// Add every key from another set.
    fn extend(&mut self, other: Self) {
        self.keys.extend(other.keys);
        self.domains.extend(other.domains);
    }

    /// Return whether this set accepts one exact key.
    fn accepts(&self, key: dir::StaticKey) -> bool {
        self.keys.contains(&key) || self.domains.contains(&KeyDomain::from_key(key))
    }

    /// Return the intersection of two key sets.
    fn intersect(self, other: Self) -> Self {
        let mut keys = KeySet::default();

        // keep exact left keys accepted by the right side
        for key in self.keys.iter().copied() {
            if other.accepts(key) {
                keys.insert_key(key);
            }
        }

        // keep exact right keys accepted by left domains
        for key in other.keys.iter().copied() {
            if !keys.keys.contains(&key) && self.accepts(key) {
                keys.insert_key(key);
            }
        }

        // keep broad domains accepted by both sides
        for domain in self.domains.iter().copied() {
            if other.domains.contains(&domain) {
                keys.insert_domain(domain);
            }
        }

        keys
    }
}

impl KeyDomain {
    /// Return the broad key domain containing one exact key.
    fn from_key(key: dir::StaticKey) -> Self {
        match key {
            dir::StaticKey::Name(_) => Self::String,
            dir::StaticKey::Index(_) => Self::Usize,
            dir::StaticKey::Symbol(_) => Self::Symbol,
        }
    }

    /// Return the primitive type representing this key domain.
    fn primitive_type(self) -> dir::PrimitiveType {
        match self {
            Self::String => dir::PrimitiveType::String,
            Self::Usize => {
                dir::PrimitiveType::Integer(dir::IntegerType::Pointer { is_signed: false })
            }
            Self::Symbol => dir::PrimitiveType::Symbol,
        }
    }
}

impl CheckState<'_> {
    /// Reduce one type operation when its inputs allow.
    /// Returns ready none when the operation must stay symbolic.
    pub(super) fn reduce_operation(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        operation: &dir::TypeOperation,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        match operation {
            // conditionals select by the extends relation
            dir::TypeOperation::Conditional(conditional) => {
                self.reduce_conditional(origin, *conditional)
            }

            // guard narrowings filter runtime-tested sources
            dir::TypeOperation::Narrow(narrow) => self.reduce_narrow(origin, *narrow),

            // string mappings transform string literals
            dir::TypeOperation::StringMapping { mapping, target } => {
                let target = answer!(self.reduce_type_root(origin, *target)?);

                match self.ty(target)? {
                    dir::Type::Literal(dir::ScalarLiteral::String(value)) => {
                        let value = *value;
                        let mapped =
                            self.reduce_string_mapping(origin, id.module_id, *mapping, value)?;

                        Ok(Answer::Ready(Some(mapped)))
                    }
                    _ => Ok(Answer::Ready(None)),
                }
            }

            // indexed access projects element or member types
            dir::TypeOperation::Index(index) => self.reduce_index(origin, index),

            // keyof projects the key union of one closed type
            dir::TypeOperation::KeyOf(unary) => self.reduce_keyof(origin, id, unary.target),

            // inference barriers peel only after their target closes
            dir::TypeOperation::NoInfer(unary) => self.reduce_noinfer(origin, unary.target),

            // try projections split nullish parts and carrier channels
            dir::TypeOperation::TryOutput { value } => {
                self.reduce_try_projection(origin, *value, false)
            }
            dir::TypeOperation::TryResidual { value } => {
                self.reduce_try_projection(origin, *value, true)
            }

            // static operations evaluate over literal operands
            dir::TypeOperation::StaticBinary(binary) => {
                self.reduce_static_binary_operation(origin, *binary)
            }
            dir::TypeOperation::StaticUnary(unary) => {
                self.reduce_static_unary_operation(origin, *unary)
            }

            // template literals concatenate once every span closes
            dir::TypeOperation::TemplateLiteral(template) => {
                self.reduce_template_literal(origin, template)
            }

            // mapped types project their closed key sources field by field
            dir::TypeOperation::Mapped(mapped) => self.reduce_mapped(origin, mapped),

            // bare binders stay symbolic until applied
            dir::TypeOperation::Infer(_) => Ok(Answer::Ready(None)),
        }
    }

    /// Reduce one inference barrier after its target closes.
    fn reduce_noinfer(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let blockers = self
            .type_variables(target)?
            .into_iter()
            .map(Dependency::Variable)
            .collect::<SmallVec<[_; 2]>>();
        if !blockers.is_empty() {
            return Ok(Answer::Pending(blockers));
        }
        if self.type_contains_generic_parameter(target)? {
            return Ok(Answer::Ready(None));
        }

        let target = answer!(self.reduce_type_root(origin, target)?);

        Ok(Answer::Ready(Some(target)))
    }

    /// Return whether one type still references a declaration parameter.
    fn type_contains_generic_parameter(&self, target: dir::GlobalTypeId) -> CompilerResult<bool> {
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        let mut visited = indexmap::IndexSet::new();
        pending.push(target);

        // scan the written operation target without expanding references
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let ty = self.ty(id)?;
            if matches!(ty, dir::Type::Parameter(_)) {
                return Ok(true);
            }
            ty.for_each_child(|child| pending.push(child));
        }

        Ok(false)
    }

    /// Project the success or residual channel of one tried value.
    /// Nullish union members propagate directly, the remaining carrier
    /// contributes its `Output` and `Residual` associated types.
    /// Returns ready none while the value stays symbolic.
    fn reduce_try_projection(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        residual: bool,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // close the tried value first
        let value = answer!(self.reduce_type_root(origin, value)?);

        // split nullish members from the carrier part
        let mut nullish = Vec::new();
        let mut carriers = Vec::new();
        let elements = match self.ty(value)? {
            dir::Type::Union(union) => union.elements.iter().copied().collect::<SmallVec<[_; 4]>>(),
            dir::Type::Variable(_) | dir::Type::Parameter(_) => return Ok(Answer::Ready(None)),
            _ => {
                let mut single = SmallVec::new();
                single.push(value);

                single
            }
        };
        for element in elements {
            match self.ty(element)? {
                dir::Type::Null
                | dir::Type::Undefined
                | dir::Type::Literal(dir::ScalarLiteral::Null | dir::ScalarLiteral::Undefined) => {
                    nullish.push(element)
                }
                _ => carriers.push(element),
            }
        }

        let module = origin.module();
        let source = self.origin_source_node(origin)?;

        // rebuild the carrier part for member projection
        let carrier = match carriers.as_slice() {
            [] => None,
            [single] => Some(*single),
            _ => Some(self.push_type(
                module,
                dir::Type::Union(dir::UnionType { elements: carriers }),
                source,
            )?),
        };

        // project the requested channel through the try carrier
        let name = if residual { "Residual" } else { "Output" };
        let key = dir::StaticKey::Name(dir::StringId::for_text(name));
        let projected = match carrier {
            None => None,
            Some(carrier) => {
                let lookup =
                    self.lookup_member(origin, module, carrier, dir::MemberSpace::Static, key)?;

                match lookup {
                    MemberLookup::Pending(blockers) => {
                        return Ok(Answer::Pending(blockers));
                    }
                    // non-carriers keep their own value as the success channel
                    MemberLookup::Missing => {
                        if residual {
                            None
                        } else {
                            Some(carrier)
                        }
                    }
                    MemberLookup::Field(ty) => Some(ty),
                    MemberLookup::Found(candidates) => {
                        candidates.first().map(|candidate| candidate.ty)
                    }
                }
            }
        };

        // join the projected channel with the propagated nullish part
        let mut elements = Vec::new();
        if residual {
            elements.extend(nullish);
        }
        elements.extend(projected);
        let joined = match elements.as_slice() {
            [] => self.push_type(module, dir::Type::Never, source)?,
            [single] => *single,
            _ => self.push_type(
                module,
                dir::Type::Union(dir::UnionType { elements }),
                source,
            )?,
        };

        Ok(Answer::Ready(Some(joined)))
    }

    /// Evaluate one conditional type, distributing over union-valued
    /// left operands when the conditional is distributive.
    /// Returns ready none while the operands stay symbolic.
    fn reduce_conditional(
        &mut self,
        origin: Origin,
        conditional: dir::ConditionalType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // close the checked operand first
        let left = answer!(self.reduce_type_root(origin, conditional.left)?);

        // distribute over union-valued checked types
        let elements = match self.ty(left)? {
            dir::Type::Union(union) if conditional.is_distributive => {
                union.elements.iter().copied().collect::<SmallVec<[_; 4]>>()
            }
            // open composites stay symbolic until their leaves close
            dir::Type::Variable(_) | dir::Type::Parameter(_) => return Ok(Answer::Ready(None)),
            _ => {
                let mut single = SmallVec::new();
                single.push(left);

                single
            }
        };

        // collect infer binders declared by the extends pattern
        let binders = self.collect_infer_binders(conditional.right)?;

        // select each element's branch with the element substituted in
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let mut selected = Vec::with_capacity(elements.len());
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for element in elements {
            // match binder patterns under a probe
            let branch = if binders.is_empty() {
                let decision =
                    self.decide_relation(origin, Relation::Extends, element, conditional.right)?;

                match decision {
                    Answer::Ready(true) => conditional.then_type,
                    Answer::Ready(false) => conditional.else_type,
                    Answer::Pending(dependencies) => {
                        blockers.extend(dependencies);

                        continue;
                    }
                }
            } else {
                let probe = self.begin_probe();
                let matched = self.match_infer_pattern(
                    origin,
                    conditional.right,
                    conditional.then_type,
                    &binders,
                    element,
                );

                match matched {
                    Ok(Answer::Ready(Some(branch))) => {
                        self.commit_probe(probe);

                        branch
                    }
                    Ok(Answer::Ready(None)) => {
                        self.reject_probe(probe);

                        conditional.else_type
                    }
                    Ok(Answer::Pending(dependencies)) => {
                        self.reject_probe(probe);

                        // blockers that died with the probe mean no match
                        let live_blockers = self.live_blockers(dependencies);
                        if live_blockers.is_empty() {
                            conditional.else_type
                        } else {
                            blockers.extend(live_blockers);

                            continue;
                        }
                    }
                    Err(error) => {
                        self.reject_probe(probe);

                        return Err(error);
                    }
                }
            };

            // branch occurrences of the checked type become the element
            let rewrite = TypeRewrite::Replace {
                from: conditional.left,
                to: element,
            };
            selected.push(self.fold_type(module, source, branch, rewrite)?);
        }
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        // rebuild the distributed result, dropping never like any union
        let mut kept = Vec::with_capacity(selected.len());
        for branch in selected {
            let branch = answer!(self.reduce_type_root(origin, branch)?);
            if matches!(self.ty(branch)?, dir::Type::Never) {
                continue;
            }
            if !kept.contains(&branch) {
                kept.push(branch);
            }
        }
        let joined = match kept.as_slice() {
            [] => self.push_type(module, dir::Type::Never, source)?,
            [single] => *single,
            _ => self.push_type(
                module,
                dir::Type::Union(dir::UnionType { elements: kept }),
                source,
            )?,
        };

        Ok(Answer::Ready(Some(joined)))
    }

    /// Evaluate one runtime guard narrowing.
    fn reduce_narrow(
        &mut self,
        origin: Origin,
        narrow: dir::NarrowType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // close both operands before comparing arms
        let source = answer!(self.reduce_type_root(origin, narrow.source)?);
        let target = answer!(self.reduce_type_root(origin, narrow.target)?);

        // distribute over union-valued sources
        let elements = match self.ty(source)? {
            dir::Type::Union(union) => union.elements.iter().copied().collect::<SmallVec<[_; 4]>>(),
            dir::Type::Variable(_) | dir::Type::Parameter(_) => return Ok(Answer::Ready(None)),
            _ => {
                let mut single = SmallVec::new();
                single.push(source);

                single
            }
        };

        // filter each arm through the guard relation
        let module = origin.module();
        let source_node = self.origin_source_node(origin)?;
        let mut kept = Vec::with_capacity(elements.len());
        for element in elements {
            let narrowed =
                answer!(self.narrow_element(origin, element, target, narrow.is_positive)?);
            let narrowed = answer!(self.reduce_type_root(origin, narrowed)?);
            if matches!(self.ty(narrowed)?, dir::Type::Never) {
                continue;
            }
            if !kept.contains(&narrowed) {
                kept.push(narrowed);
            }
        }

        // rebuild the filtered result
        let joined = match kept.as_slice() {
            [] => self.push_type(module, dir::Type::Never, source_node)?,
            [single] => *single,
            _ => self.push_type(
                module,
                dir::Type::Union(dir::UnionType { elements: kept }),
                source_node,
            )?,
        };

        Ok(Answer::Ready(Some(joined)))
    }

    /// Narrow one source arm through one runtime target.
    fn narrow_element(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        is_positive: bool,
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let module = origin.module();
        let source_node = self.origin_source_node(origin)?;

        // erased values expose the checked target on matching branches
        if matches!(self.ty(source)?, dir::Type::Dynamic(_)) {
            let narrowed = if is_positive { target } else { source };

            return Ok(Answer::Ready(narrowed));
        }

        // disjoint arms can be decided without assignability
        if !answer!(self.types_may_overlap(origin, source, target)?) {
            let narrowed = if is_positive {
                self.push_type(module, dir::Type::Never, source_node)?
            } else {
                source
            };

            return Ok(Answer::Ready(narrowed));
        }

        // exact matches keep or remove the source arm
        if answer!(self.decide_relation(origin, Relation::Assignable, source, target)?) {
            let narrowed = if is_positive {
                source
            } else {
                self.push_type(module, dir::Type::Never, source_node)?
            };

            return Ok(Answer::Ready(narrowed));
        }

        // top-like source arms take the target on matching branches
        let is_top_like =
            answer!(self.decide_relation(origin, Relation::Assignable, target, source)?);
        let narrowed = if is_positive && is_top_like {
            target
        } else if is_positive {
            self.push_type(module, dir::Type::Never, source_node)?
        } else {
            source
        };

        Ok(Answer::Ready(narrowed))
    }

    /// Concatenate one template literal type over closed spans.
    /// Returns ready none while any span stays symbolic.
    fn reduce_template_literal(
        &mut self,
        origin: Origin,
        template: &dir::TemplateLiteralType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let module = origin.module();
        let strings = template.strings.clone();
        let spans = template.spans.clone();

        // close every interpolated span to a printable literal
        let mut printed = Vec::with_capacity(spans.len());
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for span in spans {
            let span = match self.reduce_type_root(origin, span)? {
                Answer::Ready(span) => span,
                Answer::Pending(dependencies) => {
                    blockers.extend(dependencies);

                    continue;
                }
            };

            let text = match self.ty(span)? {
                dir::Type::Literal(literal) => literal.template_text(&self.module(module).strings),
                dir::Type::Null => Some("null".to_string()),
                dir::Type::Undefined => Some("undefined".to_string()),
                _ => None,
            };
            match text {
                Some(text) => printed.push(text),
                // symbolic spans keep the template symbolic
                None => return Ok(Answer::Ready(None)),
            }
        }
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        // interleave the literal segments with the printed spans
        let mut joined = String::new();
        for (index, segment) in strings.iter().enumerate() {
            joined.push_str(self.module(module).strings.get(*segment));
            if let Some(text) = printed.get(index) {
                joined.push_str(text);
            }
        }
        let joined = self.module_mut(module).strings.intern(&joined);
        let literal = dir::Type::Literal(dir::ScalarLiteral::String(joined));
        let source = self.origin_source_node(origin)?;

        Ok(Answer::Ready(Some(
            self.push_type(module, literal, source)?,
        )))
    }

    /// Collect the infer binders declared by one extends pattern.
    fn collect_infer_binders(
        &self,
        pattern: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let mut binders = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        pending.push(pattern);

        while let Some(id) = pending.pop() {
            match self.ty(id)? {
                // record one binder without descending its constraint
                dir::Type::Operation(dir::TypeOperation::Infer(_)) => {
                    if !binders.contains(&id) {
                        binders.push(id);
                    }
                }
                // nested conditionals own their binders
                dir::Type::Operation(dir::TypeOperation::Conditional(_)) if id != pattern => {}
                ty => ty.for_each_child(|child| pending.push(child)),
            }
        }

        Ok(binders)
    }

    /// Match one element against an extends pattern with infer binders.
    fn match_infer_pattern(
        &mut self,
        origin: Origin,
        pattern: dir::GlobalTypeId,
        then_type: dir::GlobalTypeId,
        binders: &[dir::GlobalTypeId],
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;

        // open one variable per binder inside the pattern
        let mut replaced = pattern;
        let mut variables =
            SmallVec::<[(dir::GlobalTypeId, dir::TypeVariableId, dir::GlobalTypeId); 2]>::new();
        for binder in binders.iter().copied() {
            let variable = self.allocate_variable(module, origin, Widening::Preserve);
            let ty = self.push_variable_type(variable, source)?;

            // add declared binder constraints as upper bounds
            if let dir::Type::Operation(dir::TypeOperation::Infer(infer)) = self.ty(binder)?
                && let Some(constraint) = infer.constraint
            {
                self.push_upper_bound(variable, constraint)?;
            }
            replaced = self.fold_type(
                module,
                source,
                replaced,
                TypeRewrite::Replace {
                    from: binder,
                    to: ty,
                },
            )?;
            variables.push((binder, variable, ty));
        }
        // match the element against the binding pattern
        if !answer!(self.constrain(origin, Relation::Assignable, element, replaced)?) {
            return Ok(Answer::Ready(None));
        }

        // solve probe binders from their matched bounds
        if !answer!(self.solve_probe_variables(variables.iter().map(|(_, variable, _)| *variable))?)
        {
            return Ok(Answer::Ready(None));
        }

        // require every binder to solve
        for (_, variable, _) in &variables {
            if self.solver.solution(*variable)?.is_none() {
                return Ok(Answer::Ready(None));
            }
        }

        // substitute resolved binder solutions into the selected branch
        let mut branch = then_type;
        for (binder, _, ty) in &variables {
            let solution = self.fold_type(module, source, *ty, TypeRewrite::Resolve)?;
            branch = self.fold_type(
                module,
                source,
                branch,
                TypeRewrite::Replace {
                    from: *binder,
                    to: solution,
                },
            )?;
        }

        Ok(Answer::Ready(Some(branch)))
    }

    /// Evaluate one static binary operation over literal operands.
    fn reduce_static_binary_operation(
        &mut self,
        origin: Origin,
        binary: dir::StaticBinaryType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // close the left operand first for short-circuit logic
        let left = answer!(self.reduce_type_root(origin, binary.left)?);
        let left_literal = match self.ty(left)? {
            dir::Type::Literal(literal) => Some(*literal),
            _ => None,
        };

        // short-circuit logical joins on a decided left operand
        if let Some(dir::ScalarLiteral::Boolean(value)) = left_literal {
            match (binary.operator, value) {
                (dir::StaticBinaryOperator::And, false) | (dir::StaticBinaryOperator::Or, true) => {
                    return Ok(Answer::Ready(Some(left)));
                }
                (dir::StaticBinaryOperator::And, true) | (dir::StaticBinaryOperator::Or, false) => {
                    let right = answer!(self.reduce_type_root(origin, binary.right)?);

                    return match self.ty(right)? {
                        dir::Type::Literal(dir::ScalarLiteral::Boolean(_)) => {
                            Ok(Answer::Ready(Some(right)))
                        }
                        _ => Ok(Answer::Ready(None)),
                    };
                }
                _ => {}
            }
        }

        // close the right operand
        let right = answer!(self.reduce_type_root(origin, binary.right)?);
        let right_literal = match self.ty(right)? {
            dir::Type::Literal(literal) => Some(*literal),
            _ => None,
        };
        let (Some(left_literal), Some(right_literal)) = (left_literal, right_literal) else {
            return Ok(Answer::Ready(None));
        };

        // string concatenation interns through module state
        if let (
            dir::StaticBinaryOperator::Add,
            dir::ScalarLiteral::String(left_value),
            dir::ScalarLiteral::String(right_value),
        ) = (binary.operator, left_literal, right_literal)
        {
            let module = origin.module();
            let joined = {
                let strings = &self.module(module).strings;

                format!("{}{}", strings.get(left_value), strings.get(right_value))
            };
            let joined = self.module_mut(module).strings.intern(&joined);
            let literal = dir::Type::Literal(dir::ScalarLiteral::String(joined));
            let source = self.origin_source_node(origin)?;

            return Ok(Answer::Ready(Some(
                self.push_type(module, literal, source)?,
            )));
        }

        // every other operator evaluates purely
        match binary.operator.apply(left_literal, right_literal) {
            Ok(literal) => {
                let source = self.origin_source_node(origin)?;
                let id = self.push_type(origin.module(), dir::Type::Literal(literal), source)?;

                Ok(Answer::Ready(Some(id)))
            }
            Err(message) => {
                self.report_static_operation(origin, message)?;

                Ok(Answer::Ready(None))
            }
        }
    }

    /// Evaluate one static unary operation over a literal operand.
    fn reduce_static_unary_operation(
        &mut self,
        origin: Origin,
        unary: dir::StaticUnaryType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let target = answer!(self.reduce_type_root(origin, unary.target)?);
        let literal = match self.ty(target)? {
            dir::Type::Literal(literal) => *literal,
            _ => return Ok(Answer::Ready(None)),
        };

        let evaluated = match (unary.operator, literal) {
            (dir::StaticUnaryOperator::Not, dir::ScalarLiteral::Boolean(value)) => {
                Some(dir::ScalarLiteral::Boolean(!value))
            }
            (dir::StaticUnaryOperator::Negate, dir::ScalarLiteral::Integer(value)) => {
                match value.checked_neg() {
                    Some(negated) => Some(dir::ScalarLiteral::Integer(negated)),
                    None => {
                        self.report_static_operation(origin, "integer negation overflows")?;

                        None
                    }
                }
            }
            (dir::StaticUnaryOperator::Negate, dir::ScalarLiteral::Float(value)) => {
                Some(dir::ScalarLiteral::Float(-value))
            }
            (dir::StaticUnaryOperator::BitwiseNot, dir::ScalarLiteral::Integer(value)) => {
                Some(dir::ScalarLiteral::Integer(!value))
            }
            _ => None,
        };

        match evaluated {
            Some(literal) => {
                let source = self.origin_source_node(origin)?;
                let id = self.push_type(origin.module(), dir::Type::Literal(literal), source)?;

                Ok(Answer::Ready(Some(id)))
            }
            None => Ok(Answer::Ready(None)),
        }
    }

    /// Apply one compiler string mapping to a string literal.
    fn reduce_string_mapping(
        &mut self,
        origin: Origin,
        module: ModuleId,
        mapping: dir::StringMapping,
        value: dir::StringId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let text = self.module(module).strings.get(value).to_string();
        let mapped = mapping.apply(&text);
        let mapped = self.module_mut(module).strings.intern(&mapped);
        let literal = dir::Type::Literal(dir::ScalarLiteral::String(mapped));
        let source = self.origin_source_node(origin)?;

        self.push_type(module, literal, source)
    }

    /// Reduce one indexed access type with a closed key.
    fn reduce_index(
        &mut self,
        origin: Origin,
        index: &dir::IndexType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // close both operands first
        let left = answer!(self.reduce_type_root(origin, index.left)?);
        let key = answer!(self.reduce_type_root(origin, index.index)?);

        // union keys distribute their projections
        if let dir::Type::Union(union) = self.ty(key)? {
            let keys = union.elements.iter().copied().collect::<SmallVec<[_; 4]>>();
            let mut projections = Vec::with_capacity(keys.len());
            for key in keys {
                let projection = self.reduce_index(
                    origin,
                    &dir::IndexType {
                        left: index.left,
                        index: key,
                    },
                )?;
                let Some(projection) = answer!(projection) else {
                    return Ok(Answer::Ready(None));
                };
                projections.push(projection);
            }
            let source = self.origin_source_node(origin)?;
            let union = self.push_type(
                origin.module(),
                dir::Type::Union(dir::UnionType {
                    elements: projections,
                }),
                source,
            )?;
            return Ok(Answer::Ready(Some(union)));
        }

        // project string keys out of structural shapes
        let projected = match (self.ty(left)?, self.ty(key)?) {
            (dir::Type::Shape(shape), dir::Type::Literal(dir::ScalarLiteral::String(name))) => {
                let key = dir::StaticKey::Name(*name);

                shape
                    .fields
                    .iter()
                    .find(|field| field.key == key)
                    .map(|field| field.ty)
            }
            // project tuple elements by integer index
            (dir::Type::Tuple(tuple), dir::Type::Literal(dir::ScalarLiteral::Integer(value))) => {
                usize::try_from(*value)
                    .ok()
                    .and_then(|index| tuple.elements.get(index))
                    .map(|element| element.ty)
            }
            // project array elements by any integer key
            (dir::Type::Array(array), dir::Type::Literal(dir::ScalarLiteral::Integer(_))) => {
                Some(array.element)
            }
            _ => None,
        };

        Ok(Answer::Ready(projected))
    }

    /// Reduce keyof over one closed type to a key literal union.
    fn reduce_keyof(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // close the target first
        let target = answer!(self.reduce_type_root(origin, target)?);

        // collect exact keys and index domains
        let Some(keys) = answer!(self.keyof_set(origin, target)?) else {
            return Ok(Answer::Ready(None));
        };

        // create the key type union
        let module = id.module_id;
        let source = self.origin_source_node(origin)?;
        let elements = self.keyof_types(module, source, keys)?;
        let union = match elements.as_slice() {
            [] => self.push_type(module, dir::Type::Never, source)?,
            [single] => *single,
            _ => self.push_type(
                module,
                dir::Type::Union(dir::UnionType { elements }),
                source,
            )?,
        };

        Ok(Answer::Ready(Some(union)))
    }

    /// Collect the property-key set of one closed type.
    fn keyof_set(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<KeySet>>> {
        let set = match self.ty(target)?.clone() {
            // structural object keys come from fields and index signatures
            dir::Type::Shape(shape) => {
                let mut set = KeySet::default();
                for field in shape.fields {
                    set.insert_key(field.key);
                }
                for signature in shape.index_signatures {
                    answer!(self.insert_index_key_type(origin, &mut set, signature.key_type)?);
                }

                set
            }

            // nominal instance keys follow public instance members through heritage
            dir::Type::Instance(instance) => {
                answer!(self.instance_keyof_set(origin, instance.symbol)?)
            }

            // declaration references expose static declaration members
            dir::Type::Reference(reference) => {
                answer!(self.definition_key_set(
                    origin,
                    reference.symbol,
                    dir::MemberSpace::Static
                )?)
            }

            // wrapper forms preserve the key set of their payload
            dir::Type::Form(form) => {
                let value = answer!(self.reduce_type_root(origin, form.value)?);

                return self.keyof_set(origin, value);
            }

            // union keys are the keys present in every arm
            dir::Type::Union(union) => {
                let mut elements = union.elements.into_iter();
                let Some(first) = elements.next() else {
                    return Ok(Answer::Ready(Some(KeySet::default())));
                };
                let first = answer!(self.reduce_type_root(origin, first)?);
                let Some(mut keys) = answer!(self.keyof_set(origin, first)?) else {
                    return Ok(Answer::Ready(None));
                };
                for element in elements {
                    let element = answer!(self.reduce_type_root(origin, element)?);
                    let Some(other) = answer!(self.keyof_set(origin, element)?) else {
                        return Ok(Answer::Ready(None));
                    };
                    keys = keys.intersect(other);
                }

                keys
            }

            // intersection keys are keys from any constituent
            dir::Type::Intersection(intersection) => {
                let mut keys = KeySet::default();
                for element in intersection.elements {
                    let element = answer!(self.reduce_type_root(origin, element)?);
                    let Some(other) = answer!(self.keyof_set(origin, element)?) else {
                        return Ok(Answer::Ready(None));
                    };
                    keys.extend(other);
                }

                keys
            }

            // open and non-object types stay symbolic
            dir::Type::Variable(_) | dir::Type::Parameter(_) => return Ok(Answer::Ready(None)),
            _ => return Ok(Answer::Ready(None)),
        };

        Ok(Answer::Ready(Some(set)))
    }

    /// Collect the instance key set of one nominal declaration.
    fn instance_keyof_set(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Answer<KeySet>> {
        let mut keys = KeySet::default();
        let mut pending = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        let mut visited = SmallVec::<[dir::GlobalSymbolId; 4]>::new();
        pending.push(symbol);

        // walk instance members through nominal heritage
        while let Some(symbol) = pending.pop() {
            if visited.contains(&symbol) {
                continue;
            }
            visited.push(symbol);
            keys.extend(answer!(self.definition_key_set(
                origin,
                symbol,
                dir::MemberSpace::Instance
            )?));

            if let Some(definition) = self.definition(symbol) {
                for heritage in definition.bases() {
                    pending.push(heritage.symbol);
                }
            }
        }

        Ok(Answer::Ready(keys))
    }

    /// Collect the direct key set of one declaration member space.
    fn definition_key_set(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
    ) -> CompilerResult<Answer<KeySet>> {
        let mut keys = KeySet::default();
        let Some(definition) = self.definition(symbol) else {
            return Ok(Answer::Ready(keys));
        };
        let mut signatures = SmallVec::<[dir::GlobalTypeId; 2]>::new();

        // collect keyed fields, methods, and associated members
        for member in definition.members() {
            if member.space() != space {
                continue;
            }
            if let Some(key) = member.key() {
                keys.insert_key(key);
            }

            // index signatures contribute key domains rather than exact keys
            if let dir::DefinitionMember::IndexSignature(signature) = member {
                signatures.push(signature.ty);
            }
        }
        for signature in signatures {
            answer!(self.insert_index_signature_type(origin, &mut keys, signature)?);
        }

        Ok(Answer::Ready(keys))
    }

    /// Insert the key domain carried by one index-signature function type.
    fn insert_index_signature_type(
        &mut self,
        origin: Origin,
        keys: &mut KeySet,
        signature: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let signature = answer!(self.reduce_type_root(origin, signature)?);
        let dir::Type::FunctionSignature(signature) = self.ty(signature)?.clone() else {
            return Ok(Answer::Ready(()));
        };
        let Some(parameter) = signature.parameters.first() else {
            return Ok(Answer::Ready(()));
        };

        self.insert_index_key_type(origin, keys, parameter.ty)
    }

    /// Insert the key domain represented by one closed key type.
    fn insert_index_key_type(
        &mut self,
        origin: Origin,
        keys: &mut KeySet,
        key_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let key_type = answer!(self.reduce_type_root(origin, key_type)?);

        match self.ty(key_type)?.clone() {
            // union key domains contribute every alternative
            dir::Type::Union(union) => {
                for element in union.elements {
                    answer!(self.insert_index_key_type(origin, keys, element)?);
                }
            }

            // string index signatures accept numeric property names too
            dir::Type::Primitive(dir::PrimitiveType::String) => {
                keys.insert_domain(KeyDomain::String);
                keys.insert_domain(KeyDomain::Usize);
            }

            // numeric index signatures use the TS++ index domain
            dir::Type::Primitive(dir::PrimitiveType::Integer(_)) => {
                keys.insert_domain(KeyDomain::Usize);
            }

            // symbol index signatures accept all symbol keys
            dir::Type::Primitive(dir::PrimitiveType::Symbol | dir::PrimitiveType::UniqueSymbol) => {
                keys.insert_domain(KeyDomain::Symbol);
            }

            // literal key domains are exact keys
            _ => {
                if let Some(key) = self.static_key_from_type(key_type)? {
                    keys.insert_key(key);
                }
            }
        }

        Ok(Answer::Ready(()))
    }

    /// Write one key set as concrete type ids.
    fn keyof_types(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        keys: KeySet,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let mut elements = Vec::with_capacity(keys.keys.len() + keys.domains.len());
        for key in keys.keys {
            elements.push(self.push_static_key_type(module, source, key)?);
        }
        for domain in keys.domains {
            elements.push(self.key_domain_type(module, source, domain)?);
        }

        Ok(elements)
    }

    /// Write one key domain as its primitive type.
    fn key_domain_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        domain: KeyDomain,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.push_type(
            module,
            dir::Type::Primitive(domain.primitive_type()),
            source,
        )
    }

    /// Return the exact static key represented by one singleton key type.
    fn static_key_from_type(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let key = match self.ty(ty)? {
            dir::Type::Literal(dir::ScalarLiteral::String(name)) => dir::StaticKey::Name(*name),
            dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => {
                let Ok(index) = usize::try_from(*value) else {
                    return Ok(None);
                };

                dir::StaticKey::Index(index)
            }
            dir::Type::Instance(instance) => {
                if !self.is_unique_symbol_instance(ty, instance.symbol)? {
                    return Ok(None);
                }

                dir::StaticKey::Symbol(dir::SymbolKey::Unique(instance.symbol))
            }
            _ => return Ok(None),
        };

        Ok(Some(key))
    }

    /// Return whether one instance type is a unique-symbol singleton.
    fn is_unique_symbol_instance(
        &self,
        ty: dir::GlobalTypeId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        if self.static_value(symbol) == Some(ty) {
            return Ok(true);
        }

        let Some(symbol_type) = self.symbol_type_maybe(symbol) else {
            return Ok(false);
        };

        Ok(matches!(
            self.ty(symbol_type)?,
            dir::Type::Primitive(dir::PrimitiveType::UniqueSymbol)
        ))
    }

    /// Project one mapped type over its closed key source.
    fn reduce_mapped(
        &mut self,
        origin: Origin,
        mapped: &dir::MappedType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // homomorphic maps read source modifiers through their keyof target
        let homomorphic = match self.ty(mapped.parameter.constraint)? {
            dir::Type::Operation(dir::TypeOperation::KeyOf(unary)) => Some(unary.target),
            _ => None,
        };

        // close the key source first
        let closed = answer!(self.reduce_type_root(origin, mapped.parameter.constraint)?);
        let keys = match self.ty(closed)? {
            dir::Type::Union(union) => union.elements.iter().copied().collect::<SmallVec<[_; 8]>>(),
            dir::Type::Never => SmallVec::new(),
            dir::Type::Literal(_) | dir::Type::Primitive(_) => {
                let mut single = SmallVec::new();
                single.push(closed);

                single
            }
            // open key sources stay symbolic until they close
            _ => return Ok(Answer::Ready(None)),
        };

        // close the homomorphic source for modifier carry
        let source_shape = match homomorphic {
            Some(target) => {
                let target = answer!(self.reduce_type_root(origin, target)?);

                match self.ty(target)? {
                    dir::Type::Shape(shape) => Some(shape.clone()),
                    _ => None,
                }
            }
            None => None,
        };

        // project each key into one field
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let mut fields = Vec::with_capacity(keys.len());
        let mut index_signatures = Vec::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        let parameters = [mapped.parameter.parameter];
        for key in keys {
            // substitute the binder through the value and remap types
            let arguments = [key];
            let rewrite = TypeRewrite::Substitute {
                parameters: &parameters,
                arguments: &arguments,
                receiver: None,
            };
            let value = self.fold_type(module, source, mapped.value, rewrite)?;
            let value = match self.reduce_type_root(origin, value)? {
                Answer::Ready(value) => value,
                Answer::Pending(dependencies) => {
                    blockers.extend(dependencies);

                    continue;
                }
            };

            // remapped keys reduce after substitution
            let remapped = match mapped.parameter.key_remap {
                Some(remap) => {
                    let remap = self.fold_type(module, source, remap, rewrite)?;

                    match self.reduce_type_root(origin, remap)? {
                        Answer::Ready(remap) => remap,
                        Answer::Pending(dependencies) => {
                            blockers.extend(dependencies);

                            continue;
                        }
                    }
                }
                None => key,
            };

            // carried modifiers key by the pre-remap source field
            let carried = match (&source_shape, self.ty(key)?) {
                (Some(shape), dir::Type::Literal(dir::ScalarLiteral::String(name))) => {
                    let key = dir::StaticKey::Name(*name);

                    shape.fields.iter().find(|field| field.key == key)
                }
                _ => None,
            };
            let is_optional = match mapped.modifiers.optional {
                dir::MappedTypeModifier::Present | dir::MappedTypeModifier::Add => true,
                dir::MappedTypeModifier::Remove => false,
                dir::MappedTypeModifier::None => carried.is_some_and(|field| field.is_optional),
            };
            let is_readonly = match mapped.modifiers.readonly {
                dir::MappedTypeModifier::Present | dir::MappedTypeModifier::Add => true,
                dir::MappedTypeModifier::Remove => false,
                dir::MappedTypeModifier::None => carried.is_some_and(|field| field.is_readonly),
            };

            // primitive keys widen the projection to an index signature
            if matches!(self.ty(remapped)?, dir::Type::Primitive(_)) {
                index_signatures.push(dir::TypeIndexSignature {
                    name: mapped.parameter.name,
                    key_type: remapped,
                    value_type: value,
                    is_optional,
                    is_readonly,
                });

                continue;
            }

            let key = if matches!(self.ty(remapped)?, dir::Type::Never) {
                continue;
            } else if let Some(key) = self.static_key_from_type(remapped)? {
                key
            } else {
                return Ok(Answer::Ready(None));
            };
            fields.push(dir::TypeField {
                key,
                ty: value,
                is_optional,
                is_readonly,
            });
        }
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        let shape = dir::Type::Shape(dir::ShapeType {
            fields,
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures,
        });
        let projected = self.push_type(module, shape, source)?;

        Ok(Answer::Ready(Some(projected)))
    }
}

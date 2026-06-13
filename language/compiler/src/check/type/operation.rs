use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Dependency, MemberLookup, Origin, Relation, Rewrite, Task, Widening,
};

impl CheckState<'_> {
    /// Evaluate one type operation when its inputs allow.
    /// Returns ready none when the operation must stay symbolic.
    pub(super) fn evaluate_operation(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        operation: &dir::TypeOperation,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        match operation {
            // conditionals select by the extends relation
            dir::TypeOperation::Conditional(conditional) => {
                self.evaluate_conditional(origin, *conditional)
            }

            // string mappings transform string literals
            dir::TypeOperation::StringMapping { mapping, target } => {
                let target = match self.evaluate_root(origin, *target)? {
                    Answer::Ready(target) => target,
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                };

                match self.ty(target)? {
                    dir::Type::Literal(dir::ScalarLiteral::String(value)) => {
                        let value = *value;
                        let mapped =
                            self.evaluate_string_mapping(origin, id.module_id, *mapping, value)?;

                        Ok(Answer::Ready(Some(mapped)))
                    }
                    _ => Ok(Answer::Ready(None)),
                }
            }

            // indexed access projects element or member types
            dir::TypeOperation::Index(index) => self.evaluate_index(origin, index),

            // keyof projects the key union of one closed type
            dir::TypeOperation::KeyOf(unary) => self.evaluate_keyof(origin, id, unary.target),

            // try projections split nullish parts and carrier channels
            dir::TypeOperation::TryOutput { value } => {
                self.evaluate_try_projection(origin, *value, false)
            }
            dir::TypeOperation::TryResidual { value } => {
                self.evaluate_try_projection(origin, *value, true)
            }

            // static operations evaluate over literal operands
            dir::TypeOperation::StaticBinary(binary) => {
                self.evaluate_static_binary_operation(origin, *binary)
            }
            dir::TypeOperation::StaticUnary(unary) => {
                self.evaluate_static_unary_operation(origin, *unary)
            }

            // template literals concatenate once every span closes
            dir::TypeOperation::TemplateLiteral(template) => {
                self.evaluate_template_literal(origin, template)
            }

            // mapped types project their closed key sources field by field
            dir::TypeOperation::Mapped(mapped) => self.evaluate_mapped(origin, mapped),

            // bare binders stay symbolic until applied
            dir::TypeOperation::Infer(_) => Ok(Answer::Ready(None)),
        }
    }

    /// Project the success or residual channel of one tried value.
    /// Nullish union members propagate directly, the remaining carrier
    /// contributes its `Output` and `Residual` associated types.
    /// Returns ready none while the value stays symbolic.
    fn evaluate_try_projection(
        &mut self,
        origin: Origin,
        value: dir::GlobalTypeId,
        residual: bool,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // close the tried value first
        let value = match self.evaluate_root(origin, value)? {
            Answer::Ready(value) => value,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

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
    fn evaluate_conditional(
        &mut self,
        origin: Origin,
        conditional: dir::ConditionalType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // close the checked operand first
        let left = match self.evaluate_root(origin, conditional.left)? {
            Answer::Ready(left) => left,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

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
                    // the substituted branch escapes the probe, so its
                    // harvested allocations must survive the rollback
                    Ok(Answer::Ready(Some(branch))) => {
                        self.harvest_probe(probe)?;

                        branch
                    }
                    Ok(Answer::Ready(None)) => {
                        self.unwind_probe(probe)?;

                        conditional.else_type
                    }
                    Ok(Answer::Pending(dependencies)) => {
                        self.unwind_probe(probe)?;

                        // blockers that died with the probe mean no match
                        let survivors = self.surviving_blockers(dependencies);
                        if survivors.is_empty() {
                            conditional.else_type
                        } else {
                            blockers.extend(survivors);

                            continue;
                        }
                    }
                    Err(error) => {
                        self.unwind_probe(probe)?;

                        return Err(error);
                    }
                }
            };

            // branch occurrences of the checked type become the element
            let rewrite = Rewrite::Replace {
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
            let branch = match self.evaluate_root(origin, branch)? {
                Answer::Ready(branch) => branch,
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };
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

    /// Concatenate one template literal type over closed spans.
    /// Returns ready none while any span stays symbolic.
    fn evaluate_template_literal(
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
            let span = match self.evaluate_root(origin, span)? {
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

        // hypothesize one variable per binder inside the pattern
        let mut replaced = pattern;
        let mut variables =
            SmallVec::<[(dir::GlobalTypeId, dir::TypeVariableId, dir::GlobalTypeId); 2]>::new();
        for binder in binders.iter().copied() {
            let variable = self.allocate_variable(module, origin, Widening::Preserve);
            let ty = self.push_variable_type(variable, source)?;

            // seed declared binder constraints as upper bounds
            if let dir::Type::Operation(dir::TypeOperation::Infer(infer)) = self.ty(binder)?
                && let Some(constraint) = infer.constraint
            {
                self.push_upper_bound(variable, constraint)?;
            }
            replaced = self.fold_type(
                module,
                source,
                replaced,
                Rewrite::Replace {
                    from: binder,
                    to: ty,
                },
            )?;
            variables.push((binder, variable, ty));
        }

        // match the element against the binding pattern
        match self.constrain(origin, Relation::Assignable, element, replaced)? {
            Answer::Ready(true) => {}
            Answer::Ready(false) => return Ok(Answer::Ready(None)),
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        }

        // solve the hypothesized binders from their matched bounds
        let floor = self.queue.solve_count();
        for (_, variable, _) in &variables {
            self.queue_task(Task::Solve(*variable));
        }
        if !self.drain_probe_tasks(floor)? {
            return Ok(Answer::Ready(None));
        }

        // uninferable binders close to unknown
        for (_, variable, _) in &variables {
            if self.variables.solution(*variable)?.is_none() {
                let unknown = self.push_type(module, dir::Type::Unknown, source)?;
                self.set_solution(*variable, unknown)?;
            }
        }

        // substitute harvested binder solutions into the selected branch
        let mut branch = then_type;
        for (binder, _, ty) in &variables {
            let solution = self.harvest_type(module, source, *ty)?;
            branch = self.fold_type(
                module,
                source,
                branch,
                Rewrite::Replace {
                    from: *binder,
                    to: solution,
                },
            )?;
        }

        Ok(Answer::Ready(Some(branch)))
    }

    /// Evaluate one static binary operation over literal operands.
    fn evaluate_static_binary_operation(
        &mut self,
        origin: Origin,
        binary: dir::StaticBinaryType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // close the left operand first for short-circuit logic
        let left = match self.evaluate_root(origin, binary.left)? {
            Answer::Ready(left) => left,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
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
                    let right = match self.evaluate_root(origin, binary.right)? {
                        Answer::Ready(right) => right,
                        Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                    };

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
        let right = match self.evaluate_root(origin, binary.right)? {
            Answer::Ready(right) => right,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
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
    fn evaluate_static_unary_operation(
        &mut self,
        origin: Origin,
        unary: dir::StaticUnaryType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let target = match self.evaluate_root(origin, unary.target)? {
            Answer::Ready(target) => target,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
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

    /// Report one failed static evaluation.
    fn report_static_operation(&mut self, origin: Origin, message: &str) -> CompilerResult<()> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;
        let error = crate::CheckError::InvalidStaticOperation {
            anchor,
            module,
            message: message.to_string(),
        };
        self.module_mut(module).diagnostics.push(error.into());

        Ok(())
    }

    /// Apply one compiler string mapping to a string literal.
    fn evaluate_string_mapping(
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
    fn evaluate_index(
        &mut self,
        origin: Origin,
        index: &dir::IndexType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // close both operands first
        let left = match self.evaluate_root(origin, index.left)? {
            Answer::Ready(left) => left,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let key = match self.evaluate_root(origin, index.index)? {
            Answer::Ready(key) => key,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // union keys distribute their projections
        if let dir::Type::Union(union) = self.ty(key)? {
            let keys = union.elements.iter().copied().collect::<SmallVec<[_; 4]>>();
            let mut projections = Vec::with_capacity(keys.len());
            for key in keys {
                let projection = self.evaluate_index(
                    origin,
                    &dir::IndexType {
                        left: index.left,
                        index: key,
                    },
                )?;
                match projection {
                    Answer::Ready(Some(projection)) => projections.push(projection),
                    Answer::Ready(None) => return Ok(Answer::Ready(None)),
                    Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
                }
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
    fn evaluate_keyof(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        // close the target first
        let target = match self.evaluate_root(origin, target)? {
            Answer::Ready(target) => target,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // collect string keys from structural shapes
        let keys = match self.ty(target)? {
            dir::Type::Shape(shape) => shape
                .fields
                .iter()
                .filter_map(|field| match field.key {
                    dir::StaticKey::Name(name) => Some(name),
                    _ => None,
                })
                .collect::<SmallVec<[_; 4]>>(),
            _ => return Ok(Answer::Ready(None)),
        };

        // build the key literal union
        let module = id.module_id;
        let source = self.origin_source_node(origin)?;
        let mut elements = Vec::with_capacity(keys.len());
        for key in keys {
            let literal = dir::Type::Literal(dir::ScalarLiteral::String(key));
            elements.push(self.push_type(module, literal, source)?);
        }
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

    /// Project one mapped type over its closed key source.
    fn evaluate_mapped(
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
        let closed = match self.evaluate_root(origin, mapped.parameter.constraint)? {
            Answer::Ready(closed) => closed,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
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
            Some(target) => match self.evaluate_root(origin, target)? {
                Answer::Ready(target) => match self.ty(target)? {
                    dir::Type::Shape(shape) => Some(shape.clone()),
                    _ => None,
                },
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            },
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
            let rewrite = Rewrite::Substitute {
                parameters: &parameters,
                arguments: &arguments,
                receiver: None,
            };
            let value = self.fold_type(module, source, mapped.value, rewrite)?;
            let value = match self.evaluate_root(origin, value)? {
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

                    match self.evaluate_root(origin, remap)? {
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

            let key = match self.ty(remapped)? {
                dir::Type::Literal(dir::ScalarLiteral::String(name)) => dir::StaticKey::Name(*name),
                dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => {
                    match usize::try_from(*value) {
                        Ok(index) => dir::StaticKey::Index(index),
                        Err(_) => return Ok(Answer::Ready(None)),
                    }
                }
                // never-remapped keys drop out of the projection
                dir::Type::Never => continue,
                // other key shapes stay symbolic
                _ => return Ok(Answer::Ready(None)),
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

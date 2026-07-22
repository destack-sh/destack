use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CandidateOutcome, CheckState, Dependency, Origin, Relation, Variance, answer,
};

use super::substitute::InferSubstitution;

/// One binder declared by a conditional `infer` pattern.
#[derive(Debug, Clone, Copy)]
struct InferBinder {
    /// The infer pattern type.
    ty: dir::GlobalTypeId,
    /// The symbol referenced by the matched branch.
    symbol: Option<dir::GlobalSymbolId>,
}

/// Captures produced by one conditional `infer` pattern.
#[derive(Debug)]
struct InferMatch {
    /// The binders declared by the pattern in first-seen order.
    binders: SmallVec<[InferBinder; 2]>,
    /// The captured candidates for each binder.
    captures: SmallVec<[InferCapture; 2]>,
}

/// Candidates captured for one conditional `infer` binder.
#[derive(Debug, Clone, Default)]
struct InferCapture {
    /// Candidates captured from covariant positions.
    covariant: SmallVec<[dir::GlobalTypeId; 2]>,
    /// Candidates captured from contravariant positions.
    contravariant: SmallVec<[dir::GlobalTypeId; 2]>,
}

impl InferMatch {
    /// Create an empty capture table for one binder list.
    fn new(binders: &[InferBinder]) -> Self {
        let captures = binders.iter().map(|_| InferCapture::default()).collect();

        Self {
            binders: SmallVec::from_slice(binders),
            captures,
        }
    }

    /// Return the capture position for one binder type.
    fn binder_index_by_type(&self, binder: dir::GlobalTypeId) -> Option<usize> {
        self.binders
            .iter()
            .position(|candidate| candidate.ty == binder)
    }

    /// Return the capture position for one binder symbol.
    fn binder_index_by_symbol(&self, symbol: dir::GlobalSymbolId) -> Option<usize> {
        self.binders
            .iter()
            .position(|candidate| candidate.symbol == Some(symbol))
    }

    /// Return the capture position for one binder occurrence.
    fn binder_index(
        &self,
        binder: dir::GlobalTypeId,
        symbol: Option<dir::GlobalSymbolId>,
    ) -> Option<usize> {
        self.binder_index_by_type(binder)
            .or_else(|| symbol.and_then(|symbol| self.binder_index_by_symbol(symbol)))
    }

    /// Record one inferred candidate.
    fn bind(
        &mut self,
        binder: dir::GlobalTypeId,
        symbol: Option<dir::GlobalSymbolId>,
        captured: dir::GlobalTypeId,
        variance: Variance,
    ) -> Answer<bool> {
        let Some(index) = self.binder_index(binder, symbol) else {
            return Answer::Ready(false);
        };

        self.captures[index].push(variance, captured);

        Answer::Ready(true)
    }

    /// Return the covariant candidates already captured for one binder.
    fn captured(
        &self,
        pattern: dir::GlobalTypeId,
        symbol: Option<dir::GlobalSymbolId>,
    ) -> &[dir::GlobalTypeId] {
        match self.binder_index(pattern, symbol) {
            Some(index) => &self.captures[index].covariant,
            None => &[],
        }
    }

    /// Return the substitutions represented by captured binders.
    fn substitutions(
        &self,
        state: &mut CheckState<'_>,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) -> CompilerResult<SmallVec<[InferSubstitution; 2]>> {
        let mut substitutions = SmallVec::new();
        for (binder, capture) in self
            .binders
            .iter()
            .copied()
            .zip(self.captures.iter().cloned())
        {
            if let Some(symbol) = binder.symbol {
                let ty = capture.inferred_type(state, module, source)?;
                substitutions.push(InferSubstitution { symbol, ty });
            }
        }

        Ok(substitutions)
    }
}

impl InferCapture {
    /// Record one candidate from the active variance position.
    fn push(&mut self, variance: Variance, ty: dir::GlobalTypeId) {
        match variance {
            Variance::Bivariant | Variance::Covariant => self.covariant.push(ty),
            Variance::Contravariant => self.contravariant.push(ty),
            Variance::Invariant => {
                self.covariant.push(ty);
                self.contravariant.push(ty);
            }
        }
    }

    /// Return the inferred type produced by the recorded candidates.
    fn inferred_type(
        self,
        state: &mut CheckState<'_>,
        module: ModuleId,
        _source: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match self.covariant.as_slice() {
            [single] => return Ok(*single),
            [_, ..] => return state.normalized_union_type(module, self.covariant),
            [] => {}
        }

        match self.contravariant.as_slice() {
            [single] => Ok(*single),
            [_, ..] => {
                let elements = self.contravariant.into_iter().collect::<Vec<_>>();
                let elements = state.intern_type_ids(module, &elements)?;

                state.intern_type(
                    module,
                    dir::Type::Intersection(dir::IntersectionType { elements }),
                )
            }
            [] => state.intern_type(module, dir::Type::Never),
        }
    }
}

impl CheckState<'_> {
    /// Evaluate one conditional type.
    pub(super) fn reduce_conditional(
        &mut self,
        origin: Origin,
        conditional: dir::ConditionalType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let left = answer!(self.reduce_type_head(origin, conditional.left)?);

        // distribute over union-valued checked types
        let elements = match self.ty(left)? {
            dir::Type::Union(union) if conditional.is_distributive => {
                SmallVec::<[_; 4]>::from_slice(self.type_ids(left.module_id, union.elements)?)
            }
            dir::Type::Variable(_) | dir::Type::Parameter(_) => return Ok(Answer::Ready(None)),
            _ => SmallVec::from_slice(&[left]),
        };

        // collect infer binders declared by the extends pattern
        let binders = self.collect_infer_binders(conditional.right)?;

        // choose each element's branch with the element substituted in
        let module = origin.module();
        let mut branches = Vec::with_capacity(elements.len());
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for element in elements {
            let branch = if binders.is_empty() {
                self.conditional_branch(
                    origin,
                    element,
                    conditional.right,
                    conditional.then_type,
                    conditional.else_type,
                    &mut blockers,
                )?
            } else {
                self.inferred_conditional_branch(
                    origin,
                    element,
                    conditional.right,
                    conditional.then_type,
                    conditional.else_type,
                    &binders,
                    &mut blockers,
                )?
            };

            let Some(branch) = branch else {
                continue;
            };
            let branch = self.replace_type(module, branch, conditional.left, element)?;
            branches.push(branch);
        }
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        // rebuild the distributed result, dropping never like any union
        let mut kept = Vec::with_capacity(branches.len());
        for branch in branches {
            let branch = answer!(self.reduce_type_head(origin, branch)?);
            if matches!(self.ty(branch)?, dir::Type::Never) {
                continue;
            }
            if !kept.contains(&branch) {
                kept.push(branch);
            }
        }
        let joined = match kept.as_slice() {
            [] => self.intern_type(module, dir::Type::Never)?,
            [single] => *single,
            _ => self.normalized_union_type(module, kept)?,
        };

        Ok(Answer::Ready(Some(joined)))
    }

    /// Return the chosen branch for a conditional arm without binders.
    fn conditional_branch(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
        then_type: dir::GlobalTypeId,
        else_type: dir::GlobalTypeId,
        blockers: &mut SmallVec<[Dependency; 2]>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let decision = self.decide_relation(origin, Relation::Extends, left, right)?;

        match decision {
            Answer::Ready(true) => Ok(Some(then_type)),
            Answer::Ready(false) => Ok(Some(else_type)),
            Answer::Pending(dependencies) => {
                blockers.extend(dependencies);

                Ok(None)
            }
        }
    }

    /// Return the chosen branch for a conditional arm with infer binders.
    fn inferred_conditional_branch(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
        then_type: dir::GlobalTypeId,
        else_type: dir::GlobalTypeId,
        binders: &[InferBinder],
        blockers: &mut SmallVec<[Dependency; 2]>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let matched = self.confirm_candidate(|state| {
            match state.match_infer_pattern(origin, right, then_type, binders, left)? {
                Answer::Ready(Some(branch)) => {
                    Ok(Answer::Ready(CandidateOutcome::Accepted(branch)))
                }
                Answer::Ready(None) => Ok(Answer::Ready(CandidateOutcome::Rejected(()))),
                Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
            }
        })?;

        match matched {
            Answer::Ready(Some(branch)) => Ok(Some(branch)),
            Answer::Ready(None) => Ok(Some(else_type)),
            Answer::Pending(dependencies) => {
                blockers.extend(dependencies);

                Ok(None)
            }
        }
    }

    /// Collect the infer binders declared by one extends pattern.
    fn collect_infer_binders(
        &self,
        pattern: dir::GlobalTypeId,
    ) -> CompilerResult<SmallVec<[InferBinder; 2]>> {
        let mut binders = SmallVec::new();
        let mut pending = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        pending.push(pattern);

        while let Some(id) = pending.pop() {
            match self.ty(id)? {
                dir::Type::Operation(operation)
                    if let dir::TypeOperation::Infer(infer) =
                        self.type_operation(id.module_id, operation)? =>
                {
                    match infer.symbol {
                        Some(symbol)
                            if !binders
                                .iter()
                                .any(|binder: &InferBinder| binder.symbol == Some(symbol)) =>
                        {
                            binders.push(InferBinder {
                                ty: id,
                                symbol: Some(symbol),
                            });
                        }
                        Some(_) => {}
                        None => binders.push(InferBinder {
                            ty: id,
                            symbol: None,
                        }),
                    }
                }
                dir::Type::Operation(operation)
                    if id != pattern
                        && matches!(
                            self.type_operation(id.module_id, operation)?,
                            dir::TypeOperation::Conditional(_)
                        ) => {}
                ty => self.for_each_type_child(id.module_id, &ty, |child| pending.push(child))?,
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
        binders: &[InferBinder],
        element: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let mut captures = InferMatch::new(binders);

        // match the actual type against the pattern and capture binders
        if !answer!(self.match_infer_type(
            origin,
            &mut captures,
            Variance::Covariant,
            pattern,
            element,
        )?) {
            return Ok(Answer::Ready(None));
        }

        // substitute captured binders into the chosen branch
        let substitutions = captures.substitutions(self, module, source)?;
        let branch = self.substitute_infer_captures(origin.module(), then_type, &substitutions)?;

        Ok(Answer::Ready(Some(branch)))
    }

    /// Match one actual type against a conditional `infer` pattern.
    fn match_infer_type(
        &mut self,
        origin: Origin,
        captures: &mut InferMatch,
        variance: Variance,
        pattern: dir::GlobalTypeId,
        actual: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let pattern = answer!(self.reduce_type_head(origin, pattern)?);
        let actual = answer!(self.reduce_type_head(origin, actual)?);

        // capture direct infer binders
        if let Some(dir::TypeOperation::Infer(infer)) = self.operation_head(pattern)?
            && captures.binder_index(pattern, infer.symbol).is_some()
        {
            let symbol = infer.symbol;
            let constraint = infer.constraint;

            if let Some(constraint) = constraint
                && !answer!(self.decide_relation(origin, Relation::Extends, actual, constraint)?)
            {
                return Ok(Answer::Ready(false));
            }

            return Ok(captures.bind(pattern, symbol, actual, variance));
        }

        if pattern == actual {
            return Ok(Answer::Ready(true));
        }

        let pattern_module = pattern.module_id;
        let actual_module = actual.module_id;
        let pattern_type = self.ty(pattern)?;
        let actual_type = self.ty(actual)?;
        match (pattern_type, actual_type) {
            // template patterns split the actual text into span captures
            (
                dir::Type::Operation(operation),
                dir::Type::Literal(dir::ScalarLiteral::String(_))
                | dir::Type::Key(dir::StaticKey::Name(_)),
            ) if let dir::TypeOperation::TemplateLiteral(template) =
                self.type_operation(pattern_module, operation)? =>
            {
                let text = match actual_type {
                    dir::Type::Literal(dir::ScalarLiteral::String(text))
                    | dir::Type::Key(dir::StaticKey::Name(text)) => {
                        self.strings().get(text).to_string()
                    }
                    _ => unreachable!(),
                };
                let Some(parts) = answer!(self.split_template_captures(
                    origin,
                    &text,
                    pattern_module,
                    &template
                )?) else {
                    return Ok(Answer::Ready(false));
                };
                for (span, captured) in parts {
                    // fixed spans already matched during the split
                    if self.template_piece_text(span)?.is_some() {
                        continue;
                    }
                    match self.operation_head(span)? {
                        Some(dir::TypeOperation::Infer(infer)) => {
                            let captured = self.template_captured_type(
                                origin,
                                pattern_module,
                                span,
                                &captured,
                            )?;
                            // repeated binders must capture identical text
                            let previous = captures.captured(span, infer.symbol);
                            if !previous.is_empty() && previous != [captured] {
                                return Ok(Answer::Ready(false));
                            }
                            if !answer!(self.match_infer_type(
                                origin,
                                captures,
                                Variance::Covariant,
                                span,
                                captured
                            )?) {
                                return Ok(Answer::Ready(false));
                            }
                        }
                        _ => {
                            if !answer!(self.match_template_span(origin, &captured, span)?) {
                                return Ok(Answer::Ready(false));
                            }
                        }
                    }
                }

                Ok(Answer::Ready(true))
            }
            (dir::Type::Application(pattern), dir::Type::Application(actual))
                if pattern.symbol == actual.symbol =>
            {
                let pattern_arguments = self.type_ids(pattern_module, pattern.arguments)?.to_vec();
                let actual_arguments = self.type_ids(actual_module, actual.arguments)?.to_vec();

                self.match_infer_instance_arguments(
                    origin,
                    captures,
                    variance,
                    pattern.symbol,
                    &pattern_arguments,
                    &actual_arguments,
                )
            }
            (dir::Type::Array(pattern), dir::Type::Array(actual)) => {
                self.match_infer_type(origin, captures, variance, pattern.element, actual.element)
            }
            (dir::Type::Slice(pattern), dir::Type::Slice(actual)) => {
                self.match_infer_type(origin, captures, variance, pattern.element, actual.element)
            }
            (dir::Type::FixedArray(pattern), dir::Type::FixedArray(actual)) => {
                let pattern = [pattern.element, pattern.count];
                let actual = [actual.element, actual.count];

                self.match_infer_arguments(origin, captures, variance, &pattern, &actual)
            }
            (dir::Type::Tuple(pattern), dir::Type::Tuple(actual))
                if pattern.form == actual.form
                    && pattern.elements.len() == actual.elements.len() =>
            {
                let pattern_elements = self
                    .tuple_elements(pattern_module, pattern.elements)?
                    .to_vec();
                let actual_elements = self
                    .tuple_elements(actual_module, actual.elements)?
                    .to_vec();

                self.match_infer_tuple(
                    origin,
                    captures,
                    variance,
                    &pattern_elements,
                    &actual_elements,
                )
            }
            // class references match constructor patterns by their construct signatures
            (dir::Type::FunctionSignature(_), dir::Type::Reference(reference)) => {
                let candidates = self
                    .reference_construct_signatures(origin, reference)?
                    .to_vec();
                let mut matched = Answer::Ready(false);
                for candidate in candidates {
                    matched = matched
                        .or(self.match_infer_type(origin, captures, variance, pattern, candidate)?);
                    if matched.is_ready_true() {
                        break;
                    }
                }

                Ok(matched)
            }
            (dir::Type::Shape(pattern), dir::Type::Shape(actual)) => self.match_infer_shape(
                origin,
                captures,
                variance,
                pattern_module,
                pattern,
                actual_module,
                actual,
            ),
            (dir::Type::FunctionSignature(pattern), dir::Type::FunctionSignature(actual)) => {
                let pattern = self.type_signature(pattern_module, pattern)?;
                let actual = self.type_signature(actual_module, actual)?;

                self.match_infer_function(
                    origin,
                    captures,
                    variance,
                    pattern_module,
                    &pattern,
                    actual_module,
                    &actual,
                )
            }
            (dir::Type::Function(pattern), dir::Type::Function(actual)) => self.match_infer_type(
                origin,
                captures,
                variance,
                pattern.signature,
                actual.signature,
            ),
            (dir::Type::FunctionPointer(pattern), dir::Type::FunctionPointer(actual)) => self
                .match_infer_type(
                    origin,
                    captures,
                    variance,
                    pattern.signature,
                    actual.signature,
                ),
            (dir::Type::Form(pattern), dir::Type::Form(actual)) if pattern.form == actual.form => {
                self.match_infer_type(origin, captures, variance, pattern.value, actual.value)
            }
            _ => self.decide_relation(origin, Relation::Extends, actual, pattern),
        }
    }

    /// Match positional type arguments inside a conditional `infer` pattern.
    fn match_infer_arguments(
        &mut self,
        origin: Origin,
        captures: &mut InferMatch,
        variance: Variance,
        pattern: &[dir::GlobalTypeId],
        actual: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        if pattern.len() != actual.len() {
            return Ok(Answer::Ready(false));
        }

        let mut decision = Answer::Ready(true);
        for (pattern, actual) in pattern.iter().copied().zip(actual.iter().copied()) {
            decision =
                decision.and(self.match_infer_type(origin, captures, variance, pattern, actual)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Match structural shapes inside a conditional `infer` pattern.
    fn match_infer_shape(
        &mut self,
        origin: Origin,
        captures: &mut InferMatch,
        variance: Variance,
        pattern_module: ModuleId,
        pattern: dir::ShapeType,
        actual_module: ModuleId,
        actual: dir::ShapeType,
    ) -> CompilerResult<Answer<bool>> {
        let pattern_fields = self.shape_fields(pattern_module, pattern.fields)?.to_vec();
        let actual_fields = self.shape_fields(actual_module, actual.fields)?.to_vec();
        let fields = self.match_infer_shape_fields(
            origin,
            captures,
            variance,
            &pattern_fields,
            &actual_fields,
        )?;
        if !fields.is_ready_true() {
            return Ok(fields);
        }

        let pattern_calls = self
            .type_ids(pattern_module, pattern.call_signatures)?
            .to_vec();
        let actual_calls = self
            .type_ids(actual_module, actual.call_signatures)?
            .to_vec();
        let calls =
            self.match_infer_arguments(origin, captures, variance, &pattern_calls, &actual_calls)?;
        if !calls.is_ready_true() {
            return Ok(calls);
        }

        let pattern_constructs = self
            .type_ids(pattern_module, pattern.construct_signatures)?
            .to_vec();
        let actual_constructs = self
            .type_ids(actual_module, actual.construct_signatures)?
            .to_vec();
        let constructs = self.match_infer_arguments(
            origin,
            captures,
            variance,
            &pattern_constructs,
            &actual_constructs,
        )?;
        if !constructs.is_ready_true() {
            return Ok(constructs);
        }

        let pattern_indexes = self
            .shape_index_signatures(pattern_module, pattern.index_signatures)?
            .to_vec();
        let actual_indexes = self
            .shape_index_signatures(actual_module, actual.index_signatures)?
            .to_vec();
        self.match_infer_index_signatures(
            origin,
            captures,
            variance,
            &pattern_indexes,
            &actual_indexes,
        )
    }

    /// Match structural fields inside a conditional `infer` pattern.
    fn match_infer_shape_fields(
        &mut self,
        origin: Origin,
        captures: &mut InferMatch,
        variance: Variance,
        pattern: &[dir::TypeField],
        actual: &[dir::TypeField],
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for pattern_field in pattern {
            let actual_field = actual.iter().find(|field| field.key == pattern_field.key);
            let Some(actual_field) = actual_field else {
                if pattern_field.is_optional {
                    continue;
                }

                return Ok(Answer::Ready(false));
            };
            if actual_field.is_optional && !pattern_field.is_optional {
                return Ok(Answer::Ready(false));
            }

            decision = decision.and(self.match_infer_type(
                origin,
                captures,
                variance,
                pattern_field.ty,
                actual_field.ty,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Match index signatures inside a conditional `infer` pattern.
    fn match_infer_index_signatures(
        &mut self,
        origin: Origin,
        captures: &mut InferMatch,
        variance: Variance,
        pattern: &[dir::TypeIndexSignature],
        actual: &[dir::TypeIndexSignature],
    ) -> CompilerResult<Answer<bool>> {
        if pattern.len() != actual.len() {
            return Ok(Answer::Ready(false));
        }

        let mut decision = Answer::Ready(true);
        for (pattern, actual) in pattern.iter().zip(actual) {
            if pattern.is_optional != actual.is_optional
                || pattern.is_readonly != actual.is_readonly
            {
                return Ok(Answer::Ready(false));
            }

            decision = decision.and(self.match_infer_type(
                origin,
                captures,
                variance,
                pattern.key_type,
                actual.key_type,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }

            decision = decision.and(self.match_infer_type(
                origin,
                captures,
                variance,
                pattern.value_type,
                actual.value_type,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Match generic arguments inside a same-symbol application pattern.
    fn match_infer_instance_arguments(
        &mut self,
        origin: Origin,
        captures: &mut InferMatch,
        variance: Variance,
        symbol: dir::GlobalSymbolId,
        pattern: &[dir::GlobalTypeId],
        actual: &[dir::GlobalTypeId],
    ) -> CompilerResult<Answer<bool>> {
        if pattern.len() != actual.len() {
            return Ok(Answer::Ready(false));
        }
        let parameters = self
            .symbol_template(symbol)?
            .map(|template| self.generic_template_parameters(template));
        let mut decision = Answer::Ready(true);
        for (index, (pattern, actual)) in pattern
            .iter()
            .copied()
            .zip(actual.iter().copied())
            .enumerate()
        {
            let argument_variance = match &parameters {
                Some(parameters) => match parameters.get(index) {
                    Some(parameter) => {
                        let context = self.default_symbol_context(symbol);

                        variance.compose(self.parameter_variance(*parameter, context)?)
                    }
                    None => Variance::Invariant,
                },
                None => Variance::Invariant,
            };

            decision = decision.and(self.match_infer_type(
                origin,
                captures,
                argument_variance,
                pattern,
                actual,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Match tuple elements inside a conditional `infer` pattern.
    fn match_infer_tuple(
        &mut self,
        origin: Origin,
        captures: &mut InferMatch,
        variance: Variance,
        pattern: &[dir::TypeElement],
        actual: &[dir::TypeElement],
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for (pattern, actual) in pattern.iter().zip(actual) {
            if pattern.is_optional != actual.is_optional
                || pattern.is_readonly != actual.is_readonly
                || pattern.is_rest != actual.is_rest
            {
                return Ok(Answer::Ready(false));
            }
            decision = decision
                .and(self.match_infer_type(origin, captures, variance, pattern.ty, actual.ty)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Match function signatures inside a conditional `infer` pattern.
    fn match_infer_function(
        &mut self,
        origin: Origin,
        captures: &mut InferMatch,
        variance: Variance,
        pattern_module: ModuleId,
        pattern: &dir::FunctionSignatureType,
        actual_module: ModuleId,
        actual: &dir::FunctionSignatureType,
    ) -> CompilerResult<Answer<bool>> {
        if pattern.asynchrony != actual.asynchrony || pattern.is_generator != actual.is_generator {
            return Ok(Answer::Ready(false));
        }

        // match explicit receiver when the pattern names one
        let receiver = match (pattern.this_parameter, actual.this_parameter) {
            (Some(pattern), Some(actual)) => {
                self.match_infer_type(origin, captures, variance.flip(), pattern, actual)?
            }
            (Some(_), None) => return Ok(Answer::Ready(false)),
            (None, _) => Answer::Ready(true),
        };
        if !receiver.is_ready_true() {
            return Ok(receiver);
        }

        // match runtime parameters, including tuple capture from rest patterns
        let pattern_parameters = self
            .signature_parameters(pattern_module, pattern.parameters)?
            .to_vec();
        let actual_parameters = self
            .signature_parameters(actual_module, actual.parameters)?
            .to_vec();
        let parameters = self.match_infer_function_parameters(
            origin,
            captures,
            variance.flip(),
            &pattern_parameters,
            &actual_parameters,
        )?;
        if !parameters.is_ready_true() {
            return Ok(parameters);
        }

        match (pattern.return_type, actual.return_type) {
            (Some(pattern), Some(actual)) => {
                self.match_infer_type(origin, captures, variance, pattern, actual)
            }
            (Some(_), None) => Ok(Answer::Ready(false)),
            (None, _) => Ok(Answer::Ready(true)),
        }
    }

    /// Match function parameters inside a conditional `infer` pattern.
    fn match_infer_function_parameters(
        &mut self,
        origin: Origin,
        captures: &mut InferMatch,
        variance: Variance,
        pattern: &[dir::FunctionParameterType],
        actual: &[dir::FunctionParameterType],
    ) -> CompilerResult<Answer<bool>> {
        // one rest parameter pattern spans the full actual parameter tuple
        if let [rest] = pattern
            && rest.is_rest
        {
            return self.match_infer_rest_parameter(origin, captures, variance, *rest, actual);
        }

        if pattern.len() != actual.len() {
            return Ok(Answer::Ready(false));
        }

        let mut decision = Answer::Ready(true);
        for (pattern, actual) in pattern.iter().zip(actual) {
            if pattern.is_optional != actual.is_optional || pattern.is_rest != actual.is_rest {
                return Ok(Answer::Ready(false));
            }
            decision = decision
                .and(self.match_infer_type(origin, captures, variance, pattern.ty, actual.ty)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Match a rest parameter pattern against a full parameter list.
    fn match_infer_rest_parameter(
        &mut self,
        origin: Origin,
        captures: &mut InferMatch,
        variance: Variance,
        pattern: dir::FunctionParameterType,
        actual: &[dir::FunctionParameterType],
    ) -> CompilerResult<Answer<bool>> {
        let pattern_type = answer!(self.reduce_type_head(origin, pattern.ty)?);

        // infer rest parameters capture the actual parameter tuple
        if let Some(dir::TypeOperation::Infer(infer)) = self.operation_head(pattern_type)?
            && captures.binder_index(pattern_type, infer.symbol).is_some()
        {
            let symbol = infer.symbol;
            let captured = answer!(self.function_parameter_tuple(origin, actual)?);

            return Ok(captures.bind(pattern_type, symbol, captured, variance));
        }

        // non-infer rest parameters check every actual parameter
        let mut decision = Answer::Ready(true);
        for parameter in actual {
            let expected = if parameter.is_rest {
                pattern_type
            } else {
                self.spread_element_type(pattern_type)?
            };
            decision = decision.and(self.match_infer_type(
                origin,
                captures,
                variance,
                expected,
                parameter.ty,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return the tuple type represented by one function parameter list.
    fn function_parameter_tuple(
        &mut self,
        origin: Origin,
        parameters: &[dir::FunctionParameterType],
    ) -> CompilerResult<Answer<dir::GlobalTypeId>> {
        let mut elements = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            elements.push(dir::TypeElement {
                label: None,
                ty: parameter.ty,
                is_optional: parameter.is_optional,
                is_readonly: false,
                is_rest: parameter.is_rest,
            });
        }

        let module = origin.module();
        let elements = self.intern_elements(module, &elements)?;
        let tuple = self.intern_type(
            module,
            dir::Type::Tuple(dir::TupleType {
                form: dir::TupleForm::Tuple,
                elements,
            }),
        )?;

        Ok(Answer::Ready(tuple))
    }
}

use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation};

impl CheckState<'_> {
    /// Return whether one shape is the fresh walked type of an object literal.
    /// TODO #Suspicious: is "is_fresh_literal" really something we should derive from tree..?
    ///
    /// Freshness derives from provenance: the shape must still be its source
    /// literal's own walked type. Every rebuilt copy — widening, folding,
    /// harvesting — allocates a new id and loses freshness on its own.
    pub(in crate::check) fn is_fresh_literal(&self, id: dir::GlobalTypeId) -> CompilerResult<bool> {
        // only component working shapes can be fresh
        let Some(module) = self.modules.get(&id.module_id) else {
            return Ok(false);
        };
        if module.working.types.get_type_maybe(id.local_id).is_none() {
            return Ok(false);
        }

        // the type's source must be an object literal expression
        let source = module.working.types.get_type_source(id.local_id);
        if source.ty != dir::NodeType::Expression {
            return Ok(false);
        }
        let view = self.module(id.module_id).view();
        let expression = source.into_typed::<dir::Expression>();
        if !matches!(
            view.get(expression),
            dir::Expression::ObjectExpression { .. }
        ) {
            return Ok(false);
        }

        // the shape must still be the literal node's own walked type
        let node = dir::GlobalNodeIdAny {
            module_id: id.module_id,
            local_id: source,
        };
        let Some(node_type) = self.inputs.node_type(node) else {
            return Ok(false);
        };
        // peel the managed wrapper the walk added around the shape
        let node_type = match self.ty(node_type)? {
            dir::Type::Form(form) if form.form == dir::Form::Managed => form.value,
            _ => node_type,
        };

        Ok(node_type == id)
    }

    /// Return the element count of one fresh array literal type.
    pub(in crate::check) fn fresh_array_literal_length(
        &self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<Option<usize>> {
        // only component working arrays can be fresh
        let Some(module) = self.modules.get(&id.module_id) else {
            return Ok(None);
        };
        if module.working.types.get_type_maybe(id.local_id).is_none() {
            return Ok(None);
        }

        // the type's source must be a spread-free array literal
        let source = module.working.types.get_type_source(id.local_id);
        if source.ty != dir::NodeType::Expression {
            return Ok(None);
        }
        let view = self.module(id.module_id).view();
        let expression = source.into_typed::<dir::Expression>();
        let dir::Expression::ArrayExpression { elements } = view.get(expression) else {
            return Ok(None);
        };
        let mut length = 0usize;
        for element in elements {
            match view.get(*element) {
                dir::Argument::Spread { .. } | dir::Argument::Error => return Ok(None),
                _ => length += 1,
            }
        }

        // the array must still be the literal node's own walked type
        let node = dir::GlobalNodeIdAny {
            module_id: id.module_id,
            local_id: source,
        };
        let Some(node_type) = self.inputs.node_type(node) else {
            return Ok(None);
        };
        let node_type = match self.ty(node_type)? {
            dir::Type::Form(form) if form.form == dir::Form::Managed => form.value,
            _ => node_type,
        };

        Ok((node_type == id).then_some(length))
    }

    /// Decide exact equality of two tuple types.
    pub(in crate::check) fn decide_tuple_equal(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // compare element metadata and collect type pairs in one pure pass
        let pairs = {
            let (dir::Type::Tuple(left), dir::Type::Tuple(right)) =
                (self.ty(left)?, self.ty(right)?)
            else {
                return Ok(Answer::Ready(false));
            };
            if left.form != right.form || left.elements.len() != right.elements.len() {
                return Ok(Answer::Ready(false));
            }

            let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>::new();
            for (left, right) in left.elements.iter().zip(&right.elements) {
                if left.label != right.label
                    || left.is_optional != right.is_optional
                    || left.is_readonly != right.is_readonly
                    || left.is_rest != right.is_rest
                {
                    return Ok(Answer::Ready(false));
                }
                pairs.push((left.ty, right.ty));
            }

            pairs
        };

        self.decide_each(origin, Relation::Equal, &pairs)
    }

    /// Decide assignability of two tuple types.
    pub(in crate::check) fn decide_tuple_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // compare element metadata and collect type pairs in one pure pass
        let pairs = {
            let (dir::Type::Tuple(source), dir::Type::Tuple(target)) =
                (self.ty(source)?, self.ty(target)?)
            else {
                return Ok(Answer::Ready(false));
            };
            if source.form != target.form || source.elements.len() != target.elements.len() {
                return Ok(Answer::Ready(false));
            }

            let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>::new();
            for (source, target) in source.elements.iter().zip(&target.elements) {
                if source.is_rest != target.is_rest
                    || source.is_readonly && !target.is_readonly
                    || source.is_optional && !target.is_optional
                {
                    return Ok(Answer::Ready(false));
                }
                pairs.push((source.ty, target.ty));
            }

            pairs
        };

        self.decide_each(origin, Relation::Assignable, &pairs)
    }

    /// Decide exact equality of two structural shapes.
    pub(in crate::check) fn decide_shape_equal(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // compare member metadata and collect type pairs in one pure pass
        let pairs = {
            let (dir::Type::Shape(left), dir::Type::Shape(right)) =
                (self.ty(left)?, self.ty(right)?)
            else {
                return Ok(Answer::Ready(false));
            };

            // equal shapes need identical member counts
            if left.fields.len() != right.fields.len()
                || left.call_signatures.len() != right.call_signatures.len()
                || left.construct_signatures.len() != right.construct_signatures.len()
                || left.index_signatures.len() != right.index_signatures.len()
            {
                return Ok(Answer::Ready(false));
            }

            let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>::new();
            for (left, right) in left.fields.iter().zip(&right.fields) {
                if left.key != right.key
                    || left.is_optional != right.is_optional
                    || left.is_readonly != right.is_readonly
                {
                    return Ok(Answer::Ready(false));
                }
                pairs.push((left.ty, right.ty));
            }
            for (left, right) in left.call_signatures.iter().zip(&right.call_signatures) {
                pairs.push((*left, *right));
            }
            for (left, right) in left
                .construct_signatures
                .iter()
                .zip(&right.construct_signatures)
            {
                pairs.push((*left, *right));
            }
            for (left, right) in left.index_signatures.iter().zip(&right.index_signatures) {
                if left.is_optional != right.is_optional || left.is_readonly != right.is_readonly {
                    return Ok(Answer::Ready(false));
                }
                pairs.push((left.key_type, right.key_type));
                pairs.push((left.value_type, right.value_type));
            }

            pairs
        };

        self.decide_each(origin, Relation::Equal, &pairs)
    }

    /// Decide structural assignability of two shapes.
    pub(in crate::check) fn decide_shape_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // match members and collect demands in one pure pass
        let source_id = source;
        let (pairs, demands) = {
            let (dir::Type::Shape(source), dir::Type::Shape(target)) =
                (self.ty(source)?, self.ty(target)?)
            else {
                return Ok(Answer::Ready(false));
            };

            // fresh literals may only supply known properties
            if target.index_signatures.is_empty() && self.is_fresh_literal(source_id)? {
                for source_field in &source.fields {
                    if !target
                        .fields
                        .iter()
                        .any(|field| field.key == source_field.key)
                    {
                        return Ok(Answer::Ready(false));
                    }
                }
            }

            // require each target field from the source shape
            let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>::new();
            for target_field in &target.fields {
                let source_field = source
                    .fields
                    .iter()
                    .find(|source| source.key == target_field.key);

                match source_field {
                    // missing members satisfy optional targets only
                    None => {
                        if !target_field.is_optional {
                            return Ok(Answer::Ready(false));
                        }
                    }
                    Some(source_field) => {
                        // readonly sources cannot satisfy mutable targets
                        if source_field.is_readonly && !target_field.is_readonly {
                            return Ok(Answer::Ready(false));
                        }
                        // optional sources cannot satisfy required targets
                        if source_field.is_optional && !target_field.is_optional {
                            return Ok(Answer::Ready(false));
                        }

                        pairs.push((source_field.ty, target_field.ty));
                    }
                }
            }

            // require each target signature from any source signature
            let mut demands =
                SmallVec::<[(SmallVec<[dir::GlobalTypeId; 2]>, dir::GlobalTypeId); 2]>::new();
            for target_signature in target.call_signatures.iter().copied() {
                let candidates = source.call_signatures.iter().copied().collect();
                demands.push((candidates, target_signature));
            }
            for target_signature in target.construct_signatures.iter().copied() {
                let candidates = source.construct_signatures.iter().copied().collect();
                demands.push((candidates, target_signature));
            }

            (pairs, demands)
        };

        // decide matched field pairs
        let mut decision = self.decide_each(origin, Relation::Assignable, &pairs)?;
        if decision == Answer::Ready(false) {
            return Ok(decision);
        }

        // decide each signature demand against its candidates
        for (candidates, target_signature) in demands {
            let mut satisfied = Answer::Ready(false);
            for candidate in candidates {
                satisfied = satisfied.or(self.decide_relation(
                    origin,
                    Relation::Assignable,
                    candidate,
                    target_signature,
                )?);
                if satisfied == Answer::Ready(true) {
                    break;
                }
            }

            decision = decision.and(satisfied);
            if decision == Answer::Ready(false) {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide exact equality of two function types.
    pub(in crate::check) fn decide_function_equal(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // compare signature metadata and collect type pairs in one pure pass
        let pairs = {
            let (dir::Type::Function(left), dir::Type::Function(right)) =
                (self.ty(left)?, self.ty(right)?)
            else {
                return Ok(Answer::Ready(false));
            };

            // equal functions share asynchrony, generator shape, and arity
            if left.asynchrony != right.asynchrony
                || left.is_generator != right.is_generator
                || left.generic_parameters != right.generic_parameters
                || left.parameters.len() != right.parameters.len()
            {
                return Ok(Answer::Ready(false));
            }

            let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>::new();

            // compare receivers exactly
            match (left.this_parameter, right.this_parameter) {
                (Some(left), Some(right)) => pairs.push((left, right)),
                (None, None) => {}
                _ => return Ok(Answer::Ready(false)),
            }

            // compare parameters exactly
            for (left, right) in left.parameters.iter().zip(&right.parameters) {
                if left.is_optional != right.is_optional || left.is_rest != right.is_rest {
                    return Ok(Answer::Ready(false));
                }
                pairs.push((left.ty, right.ty));
            }

            // compare returns exactly
            match (left.return_type, right.return_type) {
                (Some(left), Some(right)) => pairs.push((left, right)),
                (None, None) => {}
                _ => return Ok(Answer::Ready(false)),
            }

            pairs
        };

        self.decide_each(origin, Relation::Equal, &pairs)
    }

    /// Decide assignability of two function types by signature variance.
    pub(in crate::check) fn decide_function_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // compare signature metadata and collect directed pairs in one pure pass
        let pairs = {
            let (dir::Type::Function(source), dir::Type::Function(target)) =
                (self.ty(source)?, self.ty(target)?)
            else {
                return Ok(Answer::Ready(false));
            };
            if source.asynchrony != target.asynchrony || source.is_generator != target.is_generator
            {
                return Ok(Answer::Ready(false));
            }

            let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>::new();

            // compare receiver input contravariantly
            match (source.this_parameter, target.this_parameter) {
                (Some(source), Some(target)) => pairs.push((target, source)),
                (None, _) => {}
                (Some(_), None) => return Ok(Answer::Ready(false)),
            }

            // require source parameters to accept every target call arity
            if !accepts_contextual_arities(&source.parameters, &target.parameters) {
                return Ok(Answer::Ready(false));
            }

            // compare runtime inputs contravariantly
            let shared = source.parameters.len().min(target.parameters.len());
            for (source, target) in source.parameters[..shared]
                .iter()
                .zip(&target.parameters[..shared])
            {
                if source.is_rest != target.is_rest {
                    return Ok(Answer::Ready(false));
                }
                pairs.push((target.ty, source.ty));
            }

            // compare outputs covariantly
            match (source.return_type, target.return_type) {
                (Some(source), Some(target)) => pairs.push((source, target)),
                (_, None) => {}
                (None, Some(_)) => return Ok(Answer::Ready(false)),
            }

            pairs
        };

        self.decide_each(origin, Relation::Assignable, &pairs)
    }
}

/// Return whether source parameters accept every target call arity.
fn accepts_contextual_arities(
    source: &[dir::FunctionParameterType],
    target: &[dir::FunctionParameterType],
) -> bool {
    // count required source parameters
    let required = source
        .iter()
        .filter(|parameter| !parameter.is_optional && !parameter.is_rest)
        .count();

    // the target must supply at least every required source parameter
    let supplied = target.len();
    let has_rest = source.iter().any(|parameter| parameter.is_rest);

    supplied >= required && (has_rest || supplied <= source.len())
}

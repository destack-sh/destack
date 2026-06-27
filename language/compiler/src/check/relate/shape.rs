use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation, SubscriptProtocol, answer};

impl CheckState<'_> {
    /// Return whether one type can be used as a property key.
    pub(in crate::check) fn is_property_key_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let ty = answer!(self.reduce_type_root(origin, ty)?);

        let result = match self.ty(ty)?.clone() {
            dir::Type::Any | dir::Type::Parameter(_) => true,
            dir::Type::Primitive(primitive) => matches!(
                primitive,
                dir::PrimitiveType::String
                    | dir::PrimitiveType::Symbol
                    | dir::PrimitiveType::UniqueSymbol
                    | dir::PrimitiveType::Integer(_)
            ),
            dir::Type::Literal(literal) => {
                matches!(
                    literal,
                    dir::ScalarLiteral::String(_) | dir::ScalarLiteral::Integer(_)
                )
            }
            dir::Type::Union(union) => {
                let mut is_key = true;
                for element in union.elements {
                    if !answer!(self.is_property_key_type(origin, element)?) {
                        is_key = false;
                        break;
                    }
                }

                is_key
            }
            _ => false,
        };

        Ok(Answer::Ready(result))
    }

    /// Return whether one type can be queried by a property key.
    pub(in crate::check) fn is_keyed_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let ty = answer!(self.reduce_type_root(origin, ty)?);

        let result = match self.ty(ty)?.clone() {
            dir::Type::Any | dir::Type::Object | dir::Type::Parameter(_) => true,
            dir::Type::Shape(_) => true,
            dir::Type::Dynamic(dynamic) => answer!(self.is_keyed_type(origin, dynamic.constraint)?),
            dir::Type::Instance(instance) => matches!(
                self.symbol_kind(instance.symbol),
                dir::SymbolKind::Class | dir::SymbolKind::Struct | dir::SymbolKind::Interface
            ),
            dir::Type::Form(form) => answer!(self.is_keyed_type(origin, form.value)?),
            dir::Type::Union(union) => {
                let mut is_keyed = true;
                for element in union.elements {
                    if !answer!(self.is_keyed_type(origin, element)?) {
                        is_keyed = false;
                        break;
                    }
                }

                is_keyed
            }
            _ => false,
        };

        Ok(Answer::Ready(result))
    }

    /// Decide exact equality of two tuple types.
    pub(in crate::check) fn decide_tuple_equal(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // compare element shapes and collect type pairs in one pure pass
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
        // compare element shapes and collect type pairs in one pure pass
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
        // compare member shapes and collect type pairs in one pure pass
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
        let (pairs, demands, index_signatures) = {
            let (dir::Type::Shape(source), dir::Type::Shape(target)) =
                (self.ty(source)?, self.ty(target)?)
            else {
                return Ok(Answer::Ready(false));
            };

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
            let index_signatures = target.index_signatures.clone();

            (pairs, demands, index_signatures)
        };

        // decide matched field pairs
        let mut decision = self.decide_each(origin, Relation::Assignable, &pairs)?;
        if decision.is_ready_false() {
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
                if satisfied.is_ready_true() {
                    break;
                }
            }

            decision = decision.and(satisfied);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        // require each target index signature from the source
        for signature in index_signatures {
            decision =
                decision.and(self.decide_index_signature_satisfied(origin, source, &signature)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether one source exposes an index signature.
    pub(in crate::check) fn decide_index_signature_satisfied(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: &dir::TypeIndexSignature,
    ) -> CompilerResult<Answer<bool>> {
        let source = answer!(self.reduce_type_root(origin, source)?);

        match self.ty(source)?.clone() {
            dir::Type::Shape(source) => {
                self.decide_shape_index_signature_satisfied(origin, &source, target)
            }
            _ => self.decide_protocol_index_signature_satisfied(origin, source, target),
        }
    }

    /// Decide whether one structural source exposes an index signature.
    fn decide_shape_index_signature_satisfied(
        &mut self,
        origin: Origin,
        source: &dir::ShapeType,
        target: &dir::TypeIndexSignature,
    ) -> CompilerResult<Answer<bool>> {
        // prefer declared index signatures when the source has one
        let mut decision = Answer::Ready(false);
        for source in &source.index_signatures {
            if source.is_optional && !target.is_optional {
                continue;
            }
            if source.is_readonly && !target.is_readonly {
                continue;
            }
            if !answer!(self.decide_relation(
                origin,
                Relation::Assignable,
                target.key_type,
                source.key_type,
            )?) {
                continue;
            }

            let value = self.decide_relation(
                origin,
                Relation::Assignable,
                source.value_type,
                target.value_type,
            )?;
            decision = decision.or(value);
            if decision.is_ready_true() {
                return Ok(decision);
            }
        }
        if matches!(decision, Answer::Pending(_)) || !target.is_readonly {
            return Ok(decision);
        }

        // readonly signatures also accept finite object views
        let module = origin.module();
        let source_node = self.origin_source_node(origin)?;
        for field in &source.fields {
            let key = self.push_static_key_type(module, source_node, field.key)?;
            if !answer!(self.decide_relation(origin, Relation::Assignable, key, target.key_type,)?)
            {
                continue;
            }

            decision = decision.and(self.decide_relation(
                origin,
                Relation::Assignable,
                field.ty,
                target.value_type,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision.or(Answer::Ready(true)))
    }

    /// Decide whether one nominal source exposes an index signature.
    fn decide_protocol_index_signature_satisfied(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: &dir::TypeIndexSignature,
    ) -> CompilerResult<Answer<bool>> {
        let module = origin.module();
        let read = {
            let method = SubscriptProtocol::Index;
            let arguments = SmallVec::<[dir::GlobalTypeId; 2]>::from_slice(&[target.key_type]);
            let sources = [dir::ArgumentSource::Omitted];
            let key = method.key(&self.module(module).strings);
            let protocol = method.protocol(self);
            let read_type = self.push_index_signature_read_type(origin, target.value_type)?;

            self.protocol_call_returns(
                origin, source, key, &protocol, &arguments, &sources, read_type,
            )?
        };
        if !read.is_ready_true() || target.is_readonly {
            return Ok(read);
        }

        let write = {
            let method = SubscriptProtocol::IndexSet;
            let arguments = SmallVec::<[dir::GlobalTypeId; 2]>::from_slice(&[
                target.key_type,
                target.value_type,
            ]);
            let sources = [dir::ArgumentSource::Omitted, dir::ArgumentSource::Omitted];
            let key = method.key(&self.module(module).strings);
            let protocol = method.protocol(self);
            let call = answer!(self.select_protocol_call(
                origin, source, source, key, &protocol, &arguments, &sources
            )?);

            Answer::Ready(call.is_some())
        };
        Ok(read.and(write))
    }

    /// Decide exact equality of two function types.
    pub(in crate::check) fn decide_function_equal(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // compare signature shapes and collect type pairs in one pure pass
        let pairs = {
            let (dir::Type::FunctionSignature(left), dir::Type::FunctionSignature(right)) =
                (self.ty(left)?, self.ty(right)?)
            else {
                return Ok(Answer::Ready(false));
            };

            // equal functions share asynchrony, generator shape, and arity
            let left_generics = self.signature_generic_parameters(left)?;
            let right_generics = self.signature_generic_parameters(right)?;
            if left.asynchrony != right.asynchrony
                || left.is_generator != right.is_generator
                || left_generics.len() != right_generics.len()
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
        // collect directed comparison pairs
        let pairs = {
            let (dir::Type::FunctionSignature(source), dir::Type::FunctionSignature(target)) =
                (self.ty(source)?, self.ty(target)?)
            else {
                return Ok(Answer::Ready(false));
            };

            let Some(pairs) = collect_function_assignability_pairs(source, target, true) else {
                return Ok(Answer::Ready(false));
            };
            pairs
        };

        self.decide_each(origin, Relation::Assignable, &pairs)
    }

    /// Decide assignability of two selected methods.
    pub(in crate::check) fn decide_method_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // collect directed comparison pairs
        let pairs = {
            let (dir::Type::FunctionSignature(source), dir::Type::FunctionSignature(target)) =
                (self.ty(source)?, self.ty(target)?)
            else {
                return Ok(Answer::Ready(false));
            };

            let Some(pairs) = collect_function_assignability_pairs(source, target, false) else {
                return Ok(Answer::Ready(false));
            };
            pairs
        };

        self.decide_each(origin, Relation::Assignable, &pairs)
    }
}

/// Collect directed function assignment pairs.
fn collect_function_assignability_pairs(
    source: &dir::FunctionSignatureType,
    target: &dir::FunctionSignatureType,
    is_receiver_compared: bool,
) -> Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>> {
    if source.asynchrony != target.asynchrony || source.is_generator != target.is_generator {
        return None;
    }

    let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>::new();

    // compare receiver input contravariantly for function values
    if is_receiver_compared {
        match (source.this_parameter, target.this_parameter) {
            (Some(source), Some(target)) => pairs.push((target, source)),
            (None, _) => {}
            (Some(_), None) => return None,
        }
    }

    // require source parameters to accept every target call arity
    if !accepts_contextual_arities(&source.parameters, &target.parameters) {
        return None;
    }

    // compare runtime inputs contravariantly
    let shared = source.parameters.len().min(target.parameters.len());
    for (source, target) in source.parameters[..shared]
        .iter()
        .zip(&target.parameters[..shared])
    {
        if source.is_rest != target.is_rest {
            return None;
        }
        pairs.push((target.ty, source.ty));
    }

    // compare outputs covariantly
    match (source.return_type, target.return_type) {
        (Some(source), Some(target)) => pairs.push((source, target)),
        (_, None) => {}
        (None, Some(_)) => return None,
    }

    Some(pairs)
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

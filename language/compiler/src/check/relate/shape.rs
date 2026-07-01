use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation, answer};

impl CheckState<'_> {
    /// Return whether one type can be used as a property key.
    pub(in crate::check) fn is_property_key_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);

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
            dir::Type::Key(_) => true,
            dir::Type::Instance(_) => self.static_key_from_type(ty)?.is_some(),
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
        let ty = answer!(self.reduce_type_head(origin, ty)?);

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
            if source.form != target.form {
                return Ok(Answer::Ready(false));
            }

            let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>::new();
            let mut source_index = 0usize;
            for target in &target.elements {
                // rest targets consume every remaining source element
                if target.is_rest {
                    let target_element = self.spread_element_type(target.ty)?;
                    while let Some(source) = source.elements.get(source_index) {
                        if source.is_readonly && !target.is_readonly {
                            return Ok(Answer::Ready(false));
                        }

                        let target = if source.is_rest {
                            target.ty
                        } else {
                            target_element
                        };
                        pairs.push((source.ty, target));
                        source_index += 1;
                    }

                    return self.decide_each(origin, Relation::Assignable, &pairs);
                }

                // omitted source elements satisfy optional target elements
                let Some(source) = source.elements.get(source_index) else {
                    if target.is_optional {
                        continue;
                    }

                    return Ok(Answer::Ready(false));
                };

                // open source rests cannot prove individual fixed elements
                if source.is_rest
                    || source.is_readonly && !target.is_readonly
                    || source.is_optional && !target.is_optional
                {
                    return Ok(Answer::Ready(false));
                }

                pairs.push((source.ty, target.ty));
                source_index += 1;
            }

            if source_index != source.elements.len() {
                return Ok(Answer::Ready(false));
            }

            pairs
        };

        self.decide_each(origin, Relation::Assignable, &pairs)
    }

    /// Return the item type yielded when one spread or rest container expands.
    pub(in crate::check) fn spread_element_type(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let element = match self.ty(ty)? {
            dir::Type::Array(array) => array.element,
            dir::Type::Slice(slice) => slice.element,
            _ => ty,
        };

        Ok(element)
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
        // match members and collect signature requirements
        let (pairs, signature_requirements, index_signatures) = {
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
            let mut signature_requirements =
                SmallVec::<[(SmallVec<[dir::GlobalTypeId; 2]>, dir::GlobalTypeId); 2]>::new();
            for target_signature in target.call_signatures.iter().copied() {
                let candidates = source.call_signatures.iter().copied().collect();
                signature_requirements.push((candidates, target_signature));
            }
            for target_signature in target.construct_signatures.iter().copied() {
                let candidates = source.construct_signatures.iter().copied().collect();
                signature_requirements.push((candidates, target_signature));
            }
            let index_signatures = target.index_signatures.clone();

            (pairs, signature_requirements, index_signatures)
        };

        // decide matched field pairs
        let mut decision = self.decide_each(origin, Relation::Assignable, &pairs)?;
        if decision.is_ready_false() {
            return Ok(decision);
        }

        // decide each signature requirement against its candidates
        for (candidates, target_signature) in signature_requirements {
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

    /// Decide assignability of a static declaration reference to a shape.
    pub(in crate::check) fn decide_reference_shape_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let reference = match self.ty(source)? {
            dir::Type::Reference(reference) => *reference,
            _ => return Ok(Answer::Ready(false)),
        };
        let target = match self.ty(target)? {
            dir::Type::Shape(target) => target.clone(),
            _ => return Ok(Answer::Ready(false)),
        };
        let module = origin.module();
        let mut decision = Answer::Ready(true);

        // require each target field from the static declaration
        for field in target.fields {
            let lookup = answer!(self.lookup_member(
                origin,
                module,
                source,
                dir::MemberSpace::Static,
                field.key
            )?);
            let found = lookup.value_type();
            let Some(found) = found else {
                if field.is_optional {
                    continue;
                }

                return Ok(Answer::Ready(false));
            };

            decision = decision.and(self.decide_relation(
                origin,
                Relation::Assignable,
                found,
                field.ty,
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        // require each target constructor from the class constructor set
        for target_signature in target.construct_signatures {
            let mut satisfied = Answer::Ready(false);
            for candidate in self.reference_construct_signatures(reference) {
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

        // call and index signatures are not part of nominal declaration values
        if !target.call_signatures.is_empty() || !target.index_signatures.is_empty() {
            return Ok(Answer::Ready(false));
        }

        Ok(decision)
    }

    /// Return constructor signatures exposed by one static declaration reference.
    fn reference_construct_signatures(
        &self,
        source: dir::TypeReference,
    ) -> SmallVec<[dir::GlobalTypeId; 2]> {
        match self.definition(source.symbol) {
            Some(dir::Definition::Class(class)) => class
                .constructors
                .iter()
                .map(|constructor| constructor.ty)
                .collect(),
            _ => SmallVec::new(),
        }
    }

    /// Decide whether one source exposes an index signature.
    pub(in crate::check) fn decide_index_signature_satisfied(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: &dir::TypeIndexSignature,
    ) -> CompilerResult<Answer<bool>> {
        let source = answer!(self.reduce_type_head(origin, source)?);

        match self.ty(source)?.clone() {
            dir::Type::Shape(source) => {
                self.decide_shape_index_signature_satisfied(origin, &source, target)
            }
            _ => self.decide_subscript_index_signature_satisfied(origin, source, target),
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

        // prove each finite field covered by the readonly key domain
        let module = origin.module();
        let source_node = self.origin_source_node(origin)?;
        let mut decision = Answer::Ready(true);
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

        Ok(decision)
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

            let Some(pairs) =
                function_assignability_pairs(source, target, ThisParameterComparison::Compare)
            else {
                return Ok(Answer::Ready(false));
            };
            pairs
        };

        self.decide_each(origin, Relation::Assignable, &pairs)
    }

    /// Decide assignability of two method signatures.
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

            let Some(pairs) =
                function_assignability_pairs(source, target, ThisParameterComparison::Skip)
            else {
                return Ok(Answer::Ready(false));
            };
            pairs
        };

        self.decide_each(origin, Relation::Assignable, &pairs)
    }
}

/// Whether function assignability compares the explicit `this` parameter.
enum ThisParameterComparison {
    /// Compare `this` as a contravariant input.
    Compare,
    /// Skip `this` because method receiver assignability was checked separately.
    Skip,
}

impl ThisParameterComparison {
    /// Return whether `this` participates in this comparison.
    fn includes_this(self) -> bool {
        matches!(self, Self::Compare)
    }
}

/// Return directed function assignment pairs.
fn function_assignability_pairs(
    source: &dir::FunctionSignatureType,
    target: &dir::FunctionSignatureType,
    this_parameter: ThisParameterComparison,
) -> Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>> {
    if source.asynchrony != target.asynchrony || source.is_generator != target.is_generator {
        return None;
    }

    let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>::new();

    // compare receiver input contravariantly for function values
    if this_parameter.includes_this() {
        match (source.this_parameter, target.this_parameter) {
            (Some(source), Some(target)) => pairs.push((target, source)),
            (None, _) => {}
            (Some(_), None) => return None,
        }
    }

    // require source parameters to accept every target call arity
    if !accepts_target_call_arities(&source.parameters, &target.parameters) {
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
fn accepts_target_call_arities(
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

use destack_dir as dir;
use destack_source::ModuleId;
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

        let result = match self.ty(ty)? {
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
                let elements = SmallVec::<[dir::GlobalTypeId; 8]>::from_slice(
                    self.type_ids(ty.module_id, union.elements)?,
                );
                let mut is_key = true;
                for element in elements {
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

        let result = match self.ty(ty)? {
            dir::Type::Any | dir::Type::Object | dir::Type::Parameter(_) => true,
            dir::Type::Shape(_) => true,
            dir::Type::Dynamic(dynamic) => answer!(self.is_keyed_type(origin, dynamic.constraint)?),
            dir::Type::Instance(instance) => matches!(
                self.symbol_kind(instance.symbol),
                dir::SymbolKind::Class | dir::SymbolKind::Struct | dir::SymbolKind::Interface
            ),
            dir::Type::Form(form) => answer!(self.is_keyed_type(origin, form.value)?),
            dir::Type::Union(union) => {
                let elements = SmallVec::<[dir::GlobalTypeId; 8]>::from_slice(
                    self.type_ids(ty.module_id, union.elements)?,
                );
                let mut is_keyed = true;
                for element in elements {
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

    /// Decide assignability of two tuple types.
    pub(in crate::check) fn decide_tuple_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // compare element shapes and collect type pairs in one pure pass
        let pairs = {
            let (dir::Type::Tuple(source_tuple), dir::Type::Tuple(target_tuple)) =
                (self.ty(source)?, self.ty(target)?)
            else {
                return Ok(Answer::Ready(false));
            };
            if source_tuple.form != target_tuple.form {
                return Ok(Answer::Ready(false));
            }

            let source_elements = self.tuple_elements(source.module_id, source_tuple.elements)?;
            let target_elements = self.tuple_elements(target.module_id, target_tuple.elements)?;
            let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>::new();
            let mut source_index = 0usize;
            for target in target_elements {
                // rest targets consume every remaining source element
                if target.is_rest {
                    let target_element = self.spread_element_type(target.ty)?;
                    while let Some(source) = source_elements.get(source_index) {
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
                let Some(source) = source_elements.get(source_index) else {
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

            if source_index != source_elements.len() {
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
            let (dir::Type::Shape(left_shape), dir::Type::Shape(right_shape)) =
                (self.ty(left)?, self.ty(right)?)
            else {
                return Ok(Answer::Ready(false));
            };

            // equal shapes need identical member counts
            if left_shape.fields.len() != right_shape.fields.len()
                || left_shape.call_signatures.len() != right_shape.call_signatures.len()
                || left_shape.construct_signatures.len() != right_shape.construct_signatures.len()
                || left_shape.index_signatures.len() != right_shape.index_signatures.len()
            {
                return Ok(Answer::Ready(false));
            }

            let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>::new();
            let left_fields = self.shape_fields(left.module_id, left_shape.fields)?;
            let right_fields = self.shape_fields(right.module_id, right_shape.fields)?;
            for (left, right) in left_fields.iter().zip(right_fields) {
                if left.key != right.key
                    || left.is_optional != right.is_optional
                    || left.is_readonly != right.is_readonly
                {
                    return Ok(Answer::Ready(false));
                }
                pairs.push((left.ty, right.ty));
            }

            let left_calls = self.type_ids(left.module_id, left_shape.call_signatures)?;
            let right_calls = self.type_ids(right.module_id, right_shape.call_signatures)?;
            for (left, right) in left_calls.iter().zip(right_calls) {
                pairs.push((*left, *right));
            }

            let left_constructs = self.type_ids(left.module_id, left_shape.construct_signatures)?;
            let right_constructs =
                self.type_ids(right.module_id, right_shape.construct_signatures)?;
            for (left, right) in left_constructs.iter().zip(right_constructs) {
                pairs.push((*left, *right));
            }

            let left_indexes =
                self.shape_index_signatures(left.module_id, left_shape.index_signatures)?;
            let right_indexes =
                self.shape_index_signatures(right.module_id, right_shape.index_signatures)?;
            for (left, right) in left_indexes.iter().zip(right_indexes) {
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
            let (dir::Type::Shape(source_shape), dir::Type::Shape(target_shape)) =
                (self.ty(source)?, self.ty(target)?)
            else {
                return Ok(Answer::Ready(false));
            };

            // require each target field from the source shape
            let source_fields = self.shape_fields(source.module_id, source_shape.fields)?;
            let target_fields = self.shape_fields(target.module_id, target_shape.fields)?;
            let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>::new();
            for target_field in target_fields {
                let source_field = source_fields
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
            let source_calls = self.type_ids(source.module_id, source_shape.call_signatures)?;
            let target_calls = self.type_ids(target.module_id, target_shape.call_signatures)?;
            let source_constructs =
                self.type_ids(source.module_id, source_shape.construct_signatures)?;
            let target_constructs =
                self.type_ids(target.module_id, target_shape.construct_signatures)?;
            let mut signature_requirements =
                SmallVec::<[(SmallVec<[dir::GlobalTypeId; 2]>, dir::GlobalTypeId); 2]>::new();
            for target_signature in target_calls.iter().copied() {
                let candidates = source_calls.iter().copied().collect();
                signature_requirements.push((candidates, target_signature));
            }
            for target_signature in target_constructs.iter().copied() {
                let candidates = source_constructs.iter().copied().collect();
                signature_requirements.push((candidates, target_signature));
            }
            let index_signatures = SmallVec::<[dir::TypeIndexSignature; 2]>::from_slice(
                self.shape_index_signatures(target.module_id, target_shape.index_signatures)?,
            );

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

    /// Return the first excess key in a direct property literal relation.
    pub(in crate::check) fn property_literal_excess_key(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let Some(keys) = self.property_literal_keys(origin, left)? else {
            return Ok(None);
        };

        // compare against the target's finite property set
        let Some(right) = self.reduce_type_head(origin, right)?.ready() else {
            return Ok(None);
        };
        let Some(accepted) = self.accepted_property_keys(origin, right)? else {
            return Ok(None);
        };

        for key in keys {
            if !accepted.contains(&key) {
                return Ok(Some(key));
            }
        }

        Ok(None)
    }

    /// Return the first missing key in a direct property literal relation.
    pub(in crate::check) fn property_literal_missing_key(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let Some(source_keys) = self.property_literal_keys(origin, left)? else {
            return Ok(None);
        };

        // compare against the target's required property set
        let Some(required) = self.required_property_keys(origin, right)? else {
            return Ok(None);
        };

        for key in required {
            if source_keys.contains(&key) {
                continue;
            }

            return Ok(Some(key));
        }

        Ok(None)
    }

    /// Return the explicit keys supplied by a direct property literal relation.
    fn property_literal_keys(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[dir::StaticKey; 8]>>> {
        let Some(expression) = origin.expression() else {
            return Ok(None);
        };
        if !self.is_property_literal_expression(expression) {
            return Ok(None);
        }

        // collect the literal's explicit keys
        let left = self.settled_root(left)?;
        let left = match self.ty(left)? {
            dir::Type::Form(form) if form.form == dir::Form::Managed => {
                self.settled_root(form.value)?
            }
            _ => left,
        };
        let dir::Type::Shape(source) = self.ty(left)? else {
            return Ok(None);
        };

        let keys = self
            .shape_fields(left.module_id, source.fields)?
            .iter()
            .map(|field| field.key)
            .collect::<SmallVec<[_; 8]>>();

        Ok(Some(keys))
    }

    /// Return the first writable index signature required by one target type.
    pub(in crate::check) fn first_writable_index_signature(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::TypeIndexSignature>> {
        let Some(target) = self.reduce_type_head(origin, target)?.ready() else {
            return Ok(None);
        };
        let dir::Type::Shape(shape) = self.ty(target)? else {
            return Ok(None);
        };

        let signature = self
            .shape_index_signatures(target.module_id, shape.index_signatures)?
            .iter()
            .find(|signature| !signature.is_readonly)
            .copied();

        Ok(signature)
    }

    /// Return whether one expression supplies literal properties.
    fn is_property_literal_expression(
        &self,
        expression: dir::GlobalNodeId<dir::Expression>,
    ) -> bool {
        matches!(
            self.module(expression.module_id)
                .view()
                .get(expression.local_id),
            dir::Expression::ObjectExpression { .. } | dir::Expression::StructExpression { .. }
        )
    }

    /// Collect the property keys one target requires, none when not statically enumerable.
    fn required_property_keys(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[dir::StaticKey; 8]>>> {
        let Some(target) = self.reduce_type_head(origin, target)?.ready() else {
            return Ok(None);
        };

        match self.ty(target)? {
            dir::Type::Shape(shape) => Ok(Some(
                self.shape_fields(target.module_id, shape.fields)?
                    .iter()
                    .filter(|field| !field.is_optional)
                    .map(|field| field.key)
                    .collect(),
            )),
            dir::Type::Form(form) => {
                let value = self.settled_root(form.value)?;

                self.required_property_keys(origin, value)
            }
            _ => Ok(None),
        }
    }

    /// Collect the property keys one target accepts, none when it accepts any.
    fn accepted_property_keys(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[dir::StaticKey; 8]>>> {
        match self.ty(target)? {
            dir::Type::Shape(shape) => {
                if !shape.index_signatures.is_empty() {
                    return Ok(None);
                }

                Ok(Some(
                    self.shape_fields(target.module_id, shape.fields)?
                        .iter()
                        .map(|field| field.key)
                        .collect(),
                ))
            }
            dir::Type::Instance(instance) => match self.definition(instance.symbol) {
                Some(dir::Definition::Interface(interface)) if !interface.is_nominal => {
                    Ok(Some(self.nominal_member_keys(instance.symbol)))
                }
                _ => Ok(None),
            },
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 4]> =
                    SmallVec::from_slice(self.type_ids(target.module_id, union.elements)?);
                let mut keys = SmallVec::new();
                for element in elements {
                    let Some(element) = self.reduce_type_head(origin, element)?.ready() else {
                        return Ok(None);
                    };
                    match self.accepted_property_keys(origin, element)? {
                        None => return Ok(None),
                        Some(element_keys) => keys.extend(element_keys),
                    }
                }

                Ok(Some(keys))
            }
            dir::Type::Form(form) => {
                let value = self.settled_root(form.value)?;

                self.accepted_property_keys(origin, value)
            }
            _ => Ok(None),
        }
    }

    /// Decide assignability of a static declaration reference to a shape.
    pub(in crate::check) fn decide_reference_shape_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let reference = match self.ty(source)? {
            dir::Type::Reference(reference) => reference,
            _ => return Ok(Answer::Ready(false)),
        };
        let target_shape = match self.ty(target)? {
            dir::Type::Shape(target_shape) => target_shape,
            _ => return Ok(Answer::Ready(false)),
        };
        let target_fields = SmallVec::<[dir::TypeField; 8]>::from_slice(
            self.shape_fields(target.module_id, target_shape.fields)?,
        );
        let target_constructs = SmallVec::<[dir::GlobalTypeId; 2]>::from_slice(
            self.type_ids(target.module_id, target_shape.construct_signatures)?,
        );
        let module = origin.module();
        let mut decision = Answer::Ready(true);

        // require each target field from the static declaration
        for field in target_fields {
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
        for target_signature in target_constructs {
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
        if !target_shape.call_signatures.is_empty() || !target_shape.index_signatures.is_empty() {
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

        match self.ty(source)? {
            dir::Type::Shape(shape) => {
                self.decide_shape_index_signature_satisfied(origin, source.module_id, shape, target)
            }
            _ => self.decide_subscript_index_signature_satisfied(origin, source, target),
        }
    }

    /// Decide whether one structural source exposes an index signature.
    fn decide_shape_index_signature_satisfied(
        &mut self,
        origin: Origin,
        module: ModuleId,
        source: dir::ShapeType,
        target: &dir::TypeIndexSignature,
    ) -> CompilerResult<Answer<bool>> {
        // prefer declared index signatures when the source has one
        let source_indexes = SmallVec::<[dir::TypeIndexSignature; 2]>::from_slice(
            self.shape_index_signatures(module, source.index_signatures)?,
        );
        let mut decision = Answer::Ready(false);
        for source in source_indexes {
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
        let key_module = origin.module();
        let source_fields =
            SmallVec::<[dir::TypeField; 8]>::from_slice(self.shape_fields(module, source.fields)?);
        let mut decision = Answer::Ready(true);
        for field in source_fields {
            let key = self.static_key_type(key_module, field.key)?;
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

    /// Decide assignability of two function types by signature variance.
    pub(in crate::check) fn decide_function_assignable(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // collect directed comparison pairs
        let pairs = {
            let Some(pairs) = self.function_assignability_pairs(
                source,
                target,
                ThisParameterComparison::Compare,
            )?
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
        // conformance quantifies universally: the found signature's own
        // generics bind by structurally matching the required signature,
        // so both sides compare over the same rigid parameters and
        // bound arguments must satisfy their declared constraints
        let mut source = answer!(self.reduce_type_head(origin, source)?);
        if !matches!(
            (self.ty(source)?, self.ty(target)?),
            (
                dir::Type::FunctionSignature(_),
                dir::Type::FunctionSignature(_)
            )
        ) {
            return self.decide_relation(origin, Relation::Assignable, source, target);
        }
        if let dir::Type::FunctionSignature(signature) = self.ty(source)? {
            let parameters = self.signature_generic_parameters(&signature)?;
            if !parameters.is_empty() {
                let Some(pairs) = self.signature_match_pairs(source, target)? else {
                    return Ok(Answer::Ready(false));
                };
                let substitution =
                    answer!(self.match_generic_pairs(origin, &parameters, &pairs)?);
                let Some(substitution) = substitution else {
                    return Ok(Answer::Ready(false));
                };
                source = self.substitute_type(origin.module(), source, &substitution)?;
            }
        }

        // collect directed comparison pairs
        let pairs = {
            let Some(pairs) =
                self.function_assignability_pairs(source, target, ThisParameterComparison::Skip)?
            else {
                return Ok(Answer::Ready(false));
            };
            pairs
        };

        self.decide_each(origin, Relation::Assignable, &pairs)
    }

    /// Return positional signature pairs for generic parameter matching.
    ///
    /// The pairs orient the found signature as the pattern: parameters
    /// and results pair positionally without variance, and the receiver
    /// stays out because method receivers relate separately.
    fn signature_match_pairs(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>>> {
        let (
            dir::Type::FunctionSignature(source_signature),
            dir::Type::FunctionSignature(target_signature),
        ) = (self.ty(source)?, self.ty(target)?)
        else {
            return Ok(None);
        };

        let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>::new();
        let source_parameters =
            self.signature_parameters(source.module_id, source_signature.parameters)?;
        let target_parameters =
            self.signature_parameters(target.module_id, target_signature.parameters)?;
        let shared = source_parameters.len().min(target_parameters.len());
        for (source, target) in source_parameters[..shared]
            .iter()
            .zip(&target_parameters[..shared])
        {
            pairs.push((source.ty, target.ty));
        }
        if let (Some(source), Some(target)) =
            (source_signature.return_type, target_signature.return_type)
        {
            pairs.push((source, target));
        }

        Ok(Some(pairs))
    }

    /// Return directed function assignment pairs, or none when the shapes cannot relate.
    fn function_assignability_pairs(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        this_parameter: ThisParameterComparison,
    ) -> CompilerResult<Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>>> {
        // require two function signatures with matching execution shape
        let (
            dir::Type::FunctionSignature(source_signature),
            dir::Type::FunctionSignature(target_signature),
        ) = (self.ty(source)?, self.ty(target)?)
        else {
            return Ok(None);
        };
        if source_signature.asynchrony != target_signature.asynchrony
            || source_signature.is_generator != target_signature.is_generator
        {
            return Ok(None);
        }

        let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>::new();

        // compare receiver input contravariantly for function values
        if this_parameter.includes_this() {
            match (
                source_signature.this_parameter,
                target_signature.this_parameter,
            ) {
                (Some(source), Some(target)) => pairs.push((target, source)),
                (None, _) => {}
                (Some(_), None) => return Ok(None),
            }
        }

        // require source parameters to accept every target call arity
        let source_parameters =
            self.signature_parameters(source.module_id, source_signature.parameters)?;
        let target_parameters =
            self.signature_parameters(target.module_id, target_signature.parameters)?;
        if !accepts_target_call_arities(source_parameters, target_parameters) {
            return Ok(None);
        }

        // compare runtime inputs contravariantly
        let shared = source_parameters.len().min(target_parameters.len());
        for (source, target) in source_parameters[..shared]
            .iter()
            .zip(&target_parameters[..shared])
        {
            if source.is_rest != target.is_rest {
                return Ok(None);
            }
            pairs.push((target.ty, source.ty));
        }

        // compare outputs covariantly
        match (source_signature.return_type, target_signature.return_type) {
            (Some(source), Some(target)) => pairs.push((source, target)),
            (_, None) => {}
            (None, Some(_)) => return Ok(None),
        }

        Ok(Some(pairs))
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

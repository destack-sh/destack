use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Relation, TypeSubstitution, answer};

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
            dir::Type::Application(instance) => matches!(
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
        relation: Relation,
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

                    return self.decide_each(origin, relation.interior(), &pairs);
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

        self.decide_each(origin, relation.interior(), &pairs)
    }

    /// Return the item type yielded when one spread or rest container expands.
    pub(in crate::check) fn spread_element_type(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let element = match self.ty(ty)? {
            dir::Type::Array(array) => array.element,
            dir::Type::Slice(slice) => slice.element,
            dir::Type::FixedArray(array) => array.element,
            _ => ty,
        };

        Ok(element)
    }

    /// Decide exact equality of two structural shapes.
    pub(in crate::check) fn decide_shape_equal(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // compare member shapes and collect type pairs in one pure pass
        let pairs = {
            let (dir::Type::Shape(source_shape), dir::Type::Shape(target_shape)) =
                (self.ty(source)?, self.ty(target)?)
            else {
                return Ok(Answer::Ready(false));
            };

            // equal shapes need identical member counts
            if source_shape.properties.len() != target_shape.properties.len()
                || source_shape.call_signatures.len() != target_shape.call_signatures.len()
                || source_shape.construct_signatures.len()
                    != target_shape.construct_signatures.len()
                || source_shape.index_signatures.len() != target_shape.index_signatures.len()
            {
                return Ok(Answer::Ready(false));
            }

            let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>::new();
            let source_fields = self.shape_properties(source.module_id, source_shape.properties)?;
            let target_fields = self.shape_properties(target.module_id, target_shape.properties)?;
            for (source, target) in source_fields.iter().zip(target_fields) {
                if source.key != target.key || source.is_optional != target.is_optional {
                    return Ok(Answer::Ready(false));
                }
                match (source.access, target.access) {
                    (
                        dir::PropertyAccess::Read(source_ty),
                        dir::PropertyAccess::Read(target_ty),
                    )
                    | (
                        dir::PropertyAccess::Write(source_ty),
                        dir::PropertyAccess::Write(target_ty),
                    ) => pairs.push((source_ty, target_ty)),
                    (
                        dir::PropertyAccess::ReadWrite {
                            read: source_read,
                            write: source_write,
                        },
                        dir::PropertyAccess::ReadWrite {
                            read: target_read,
                            write: target_write,
                        },
                    ) => {
                        pairs.push((source_read, target_read));
                        pairs.push((source_write, target_write));
                    }
                    _ => return Ok(Answer::Ready(false)),
                }
            }

            let source_calls = self.type_ids(source.module_id, source_shape.call_signatures)?;
            let target_calls = self.type_ids(target.module_id, target_shape.call_signatures)?;
            for (source, target) in source_calls.iter().zip(target_calls) {
                pairs.push((*source, *target));
            }

            let source_constructs =
                self.type_ids(source.module_id, source_shape.construct_signatures)?;
            let target_constructs =
                self.type_ids(target.module_id, target_shape.construct_signatures)?;
            for (source, target) in source_constructs.iter().zip(target_constructs) {
                pairs.push((*source, *target));
            }

            let source_indexes =
                self.shape_index_signatures(source.module_id, source_shape.index_signatures)?;
            let target_indexes =
                self.shape_index_signatures(target.module_id, target_shape.index_signatures)?;
            for (source, target) in source_indexes.iter().zip(target_indexes) {
                if source.is_optional != target.is_optional
                    || source.is_readonly != target.is_readonly
                {
                    return Ok(Answer::Ready(false));
                }
                pairs.push((source.key_type, target.key_type));
                pairs.push((source.value_type, target.value_type));
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
        self.decide_shape_relation(origin, Relation::Assignable, source, target)
    }

    /// Decide one structural pair under storage or read semantics.
    pub(in crate::check) fn decide_shape_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
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
            let source_fields = self.shape_properties(source.module_id, source_shape.properties)?;
            let target_fields = self.shape_properties(target.module_id, target_shape.properties)?;
            let mut pairs =
                SmallVec::<[(Relation, dir::GlobalTypeId, dir::GlobalTypeId); 8]>::new();
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
                        let Some(relations) =
                            self.shape_property_relations(relation, source_field, target_field)
                        else {
                            return Ok(Answer::Ready(false));
                        };
                        pairs.extend(relations);
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
        let mut decision = self.decide_shape_fields(origin, &pairs)?;
        if decision.is_ready_false() {
            return Ok(decision);
        }

        // decide each signature requirement against its candidates
        for (candidates, target_signature) in signature_requirements {
            let mut satisfied = Answer::Ready(false);
            for candidate in candidates {
                satisfied = satisfied.or(self.decide_relation(
                    origin,
                    relation,
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
            decision = decision
                .and(self.decide_index_signature_satisfied(origin, relation, source, &signature)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return the value relations one matched structural property demands.
    pub(in crate::check) fn shape_property_relations(
        &self,
        relation: Relation,
        source: &dir::TypeProperty,
        target: &dir::TypeProperty,
    ) -> Option<SmallVec<[(Relation, dir::GlobalTypeId, dir::GlobalTypeId); 2]>> {
        // optional sources cannot satisfy required targets
        if source.is_optional && !target.is_optional {
            return None;
        }

        let mut relations = SmallVec::new();
        match relation {
            // reads widen out and writes narrow back
            Relation::Assignable | Relation::Widens => {
                if let Some(target_read) = target.access.read() {
                    relations.push((Relation::Widens, source.access.read()?, target_read));
                }
                if let Some(target_write) = target.access.write() {
                    relations.push((Relation::Widens, target_write, source.access.write()?));
                }
            }
            // exact relations pair each supported operation
            _ => {
                match (source.access.read(), target.access.read()) {
                    (Some(source_read), Some(target_read)) => {
                        relations.push((relation, source_read, target_read));
                    }
                    (None, None) => {}
                    _ => return None,
                }
                match (source.access.write(), target.access.write()) {
                    (Some(source_write), Some(target_write)) => {
                        relations.push((relation, source_write, target_write));
                    }
                    (None, None) => {}
                    _ => return None,
                }
            }
        }

        Some(relations)
    }

    /// Decide each matched structural field with its required relation.
    pub(in crate::check) fn decide_shape_fields(
        &mut self,
        origin: Origin,
        fields: &[(Relation, dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for (relation, source, target) in fields.iter().copied() {
            decision = decision.and(self.decide_relation(origin, relation, source, target)?);
            if decision.is_ready_false() {
                break;
            }
        }

        Ok(decision)
    }

    /// Return the first writable index signature required by one target type.
    pub(in crate::check) fn first_writable_index_signature(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::TypeIndexSignature>>> {
        let target = answer!(self.reduce_type_head(origin, target)?);
        let dir::Type::Shape(shape) = self.ty(target)? else {
            return Ok(Answer::Ready(None));
        };

        let signature = self
            .shape_index_signatures(target.module_id, shape.index_signatures)?
            .iter()
            .find(|signature| !signature.is_readonly)
            .copied();

        Ok(Answer::Ready(signature))
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
        let target_fields = SmallVec::<[dir::TypeProperty; 8]>::from_slice(
            self.shape_properties(target.module_id, target_shape.properties)?,
        );
        let target_constructs = SmallVec::<[dir::GlobalTypeId; 2]>::from_slice(
            self.type_ids(target.module_id, target_shape.construct_signatures)?,
        );
        let module = origin.module();
        let mut decision = Answer::Ready(true);

        // require each target field from the static declaration
        for field in target_fields {
            let lookup = answer!(self.body().lookup_member(
                origin,
                module,
                source,
                dir::MemberSpace::Static,
                field.key
            )?);
            let found = self.body().member_read_type(origin, &lookup)?;
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
                field.access.store(),
            )?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        // require each target constructor from the class constructor set
        for target_signature in target_constructs {
            let mut satisfied = Answer::Ready(false);
            for candidate in self.reference_construct_signatures(origin, reference)? {
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
    pub(in crate::check) fn reference_construct_signatures(
        &mut self,
        origin: Origin,
        source: dir::TypeReference,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        let constructors: SmallVec<[dir::GlobalTypeId; 2]> = match self.definition(source.symbol)? {
            Some(dir::Definition::Class(class)) => class
                .constructors
                .iter()
                .map(|constructor| constructor.ty)
                .collect(),
            _ => SmallVec::new(),
        };

        // constructors return the declared instance in place of `this`
        let module = origin.module();
        let instance = self.declaration_instance(module, source.symbol)?;
        let instance = self.intern_type(module, dir::Type::Application(instance))?;
        let substitution = TypeSubstitution::default().with_receiver(instance);
        let mut signatures = SmallVec::new();
        for constructor in constructors {
            signatures.push(self.substitute_type(module, constructor, &substitution)?);
        }

        Ok(signatures)
    }

    /// Decide whether one source exposes an index signature.
    pub(in crate::check) fn decide_index_signature_satisfied(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: &dir::TypeIndexSignature,
    ) -> CompilerResult<Answer<bool>> {
        let source = answer!(self.reduce_type_head(origin, source)?);

        match self.ty(source)? {
            dir::Type::Shape(shape) => self.decide_shape_index_signature_satisfied(
                origin,
                relation,
                source.module_id,
                shape,
                target,
            ),
            _ => self
                .body()
                .decide_subscript_index_signature_satisfied(origin, relation, source, target),
        }
    }

    /// Decide whether one structural source exposes an index signature.
    fn decide_shape_index_signature_satisfied(
        &mut self,
        origin: Origin,
        relation: Relation,
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
            if !answer!(self.decide_relation(origin, relation, target.key_type, source.key_type,)?)
            {
                continue;
            }

            let value =
                self.decide_relation(origin, relation, source.value_type, target.value_type)?;
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
            SmallVec::<[dir::TypeProperty; 8]>::from_slice(
                self.shape_properties(module, source.properties)?,
            );
        let mut decision = Answer::Ready(true);
        for field in source_fields {
            let key = self.static_key_type(key_module, field.key)?;
            if !answer!(self.decide_relation(origin, relation, key, target.key_type)?) {
                continue;
            }

            decision = decision.and(self.decide_relation(
                origin,
                relation,
                field.access.store(),
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
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // collect directed comparison pairs
        let Some(pairs) =
            self.function_assignability_pairs(source, target, ThisParameterComparison::Compare)?
        else {
            return Ok(Answer::Ready(false));
        };

        // value interiors have no wrapper witness and widen by identity
        self.decide_each(origin, relation.interior(), &pairs)
    }

    /// Decide one relation between receiver-bound method signatures.
    pub(in crate::check) fn decide_method_relation(
        &mut self,
        origin: Origin,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // conformance quantifies universally: the found signature's own
        //  generics bind by structurally matching the required signature,
        //  so both sides compare over the same rigid parameters and
        //  bound arguments must satisfy their declared constraints
        let mut source = answer!(self.reduce_type_head(origin, source)?);
        if !matches!(
            (self.ty(source)?, self.ty(target)?),
            (
                dir::Type::FunctionSignature(_),
                dir::Type::FunctionSignature(_)
            )
        ) {
            return self.decide_relation(origin, relation, source, target);
        }
        if let Some(signature) = self.signature_head(source)?
            && let Some(template) = signature.template
        {
            let parameters = self.generic_template_parameters(template)?;
            if !parameters.is_empty() {
                let Some(pairs) = self.signature_match_pairs(source, target)? else {
                    return Ok(Answer::Ready(false));
                };
                let substitution =
                    answer!(self.match_generic_pairs(origin, &parameters, &pairs)?);
                let Some(substitution) = substitution else {
                    return Ok(Answer::Ready(false));
                };
                if !answer!(self.decide_substitution_constraints(
                    origin,
                    template,
                    &substitution,
                )?) {
                    return Ok(Answer::Ready(false));
                }
                source = self.substitute_type(origin.module(), source, &substitution)?;
            }
        }

        // collect directed comparison pairs
        let Some(pairs) =
            self.function_assignability_pairs(source, target, ThisParameterComparison::Skip)?
        else {
            return Ok(Answer::Ready(false));
        };

        self.decide_each(origin, relation, &pairs)
    }

    /// Return positional signature pairs for generic parameter matching.
    fn signature_match_pairs(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>>> {
        let (Some(source_signature), Some(target_signature)) =
            (self.signature_head(source)?, self.signature_head(target)?)
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
        // require two function signatures
        let (Some(source_signature), Some(target_signature)) =
            (self.signature_head(source)?, self.signature_head(target)?)
        else {
            return Ok(None);
        };

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

    // the target must supply every required source parameter; the source
    //  ignores extra target arguments
    target.len() >= required
}

use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{
    Cause, CauseId, CauseKind, CheckState, MemberRole, Origin, Relation, TypeSubstitution, Verdict,
};
use crate::{CompilerError, CompilerResult};

/// One directed function assignment pair with its signature slot.
type FunctionAssignabilityPair = (Option<CauseKind>, dir::GlobalTypeId, dir::GlobalTypeId);

/// One signature instantiated at its required signature.
pub(in crate::sema) struct SignatureInstantiation {
    /// The instantiated signature.
    pub(in crate::sema) signature: dir::GlobalTypeId,
    /// The complete generic arguments selecting the instance, or None while parameters stay open.
    pub(in crate::sema) arguments: Option<Vec<dir::GenericArgumentBinding>>,
}

impl SignatureInstantiation {
    /// Return one concrete signature passed through unchanged.
    fn concrete(signature: dir::GlobalTypeId) -> Self {
        Self {
            signature,
            arguments: Some(Vec::new()),
        }
    }
}

impl CheckState<'_> {
    /// Return whether one type can be used as a property key.
    pub(in crate::sema) fn is_property_key_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let result = match self.ty(ty)? {
            dir::Type::Any | dir::Type::Parameter(_) => true,
            dir::Type::Primitive(primitive) => matches!(
                primitive,
                dir::PrimitiveType::String | dir::PrimitiveType::Integer(_)
            ),
            dir::Type::Literal(literal) => {
                matches!(literal, dir::Literal::String(_) | dir::Literal::Integer(_))
            }
            dir::Type::Key(_) => true,
            // require every alternative of a union to key on its own
            dir::Type::Union(union) => {
                let elements = SmallVec::<[dir::GlobalTypeId; 8]>::from_slice(
                    self.type_ids(ty.module_id, union.elements)?,
                );
                let mut is_key = true;
                for element in elements {
                    if !self.is_property_key_type(element)? {
                        is_key = false;
                        break;
                    }
                }

                is_key
            }
            _ => false,
        };

        Ok(result)
    }

    /// Return whether one type can be queried by a property key.
    pub(in crate::sema) fn is_keyed_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // inspect the keyed shape over the normalized head
        let ty = self.normalize(origin, ty)?;

        let result = match self.ty(ty)? {
            dir::Type::Any | dir::Type::Parameter(_) => true,
            dir::Type::Object(_) => true,
            dir::Type::Dynamic(dynamic) => self.is_keyed_type(origin, dynamic.constraint)?,
            // arrays enumerate positionally
            dir::Type::Application(_) if self.array_element(ty)?.is_some() => false,
            dir::Type::Application(instance) => matches!(
                self.symbol_kind_maybe(instance.symbol)?,
                Some(dir::SymbolKind::Class | dir::SymbolKind::Struct | dir::SymbolKind::Interface)
            ),
            dir::Type::Form(form) => self.is_keyed_type(origin, form.value)?,
            // require every alternative of a union to be keyed on its own
            dir::Type::Union(union) => {
                let elements = SmallVec::<[dir::GlobalTypeId; 8]>::from_slice(
                    self.type_ids(ty.module_id, union.elements)?,
                );
                let mut is_keyed = true;
                for element in elements {
                    if !self.is_keyed_type(origin, element)? {
                        is_keyed = false;
                        break;
                    }
                }

                is_keyed
            }
            _ => false,
        };

        Ok(result)
    }

    /// Relate two tuple types under assignability.
    pub(in crate::sema) fn relate_tuple_assignable(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // compare element shapes and collect type pairs in one pure pass
        let pairs = {
            let (dir::Type::Tuple(source_tuple), dir::Type::Tuple(target_tuple)) =
                (self.ty(source)?, self.ty(target)?)
            else {
                return Ok(Verdict::Fails);
            };
            if source_tuple.form != target_tuple.form {
                return Ok(Verdict::Fails);
            }

            let source_elements = self
                .tuple_elements(source.module_id, source_tuple.elements)?
                .to_vec();
            let target_elements = self
                .tuple_elements(target.module_id, target_tuple.elements)?
                .to_vec();
            let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>::new();

            // consume source elements from the end for fixed targets after a rest
            let rest_index = target_elements.iter().position(|target| target.is_rest);
            let (target_elements, trailing_targets) = match rest_index {
                Some(rest_index) => target_elements.split_at(rest_index + 1),
                None => (target_elements.as_slice(), &[][..]),
            };
            let mut source_end = source_elements.len();
            for target in trailing_targets.iter().rev() {
                let Some(source) = source_end
                    .checked_sub(1)
                    .map(|index| &source_elements[index])
                else {
                    return Ok(Verdict::Fails);
                };
                if source.is_rest || source.is_optional || source.is_readonly && !target.is_readonly
                {
                    return Ok(Verdict::Fails);
                }

                pairs.push((source.ty, target.ty));
                source_end -= 1;
            }

            let mut source_index = 0usize;
            for target in target_elements {
                // rest targets consume every remaining source element
                if target.is_rest {
                    let remaining = &source_elements[source_index.min(source_end)..source_end];
                    if remaining
                        .iter()
                        .any(|source| source.is_readonly && !target.is_readonly)
                    {
                        return Ok(Verdict::Fails);
                    }

                    // bind the remaining elements as one tuple for an open rest binder
                    let rest = self.shallow_resolve(target.ty)?;
                    if matches!(self.ty(rest)?, dir::Type::Variable(_)) {
                        let tuple = self.intern_tuple(remaining)?;
                        pairs.push((tuple, rest));

                        return self.relate_each(origin, cause, relation.interior(), &pairs);
                    }

                    let target_element = self.spread_element_type(target.ty)?;
                    for source in remaining {
                        let target = if source.is_rest {
                            target.ty
                        } else {
                            target_element
                        };
                        pairs.push((source.ty, target));
                    }

                    return self.relate_each(origin, cause, relation.interior(), &pairs);
                }

                // omitted source elements satisfy optional target elements
                let Some(source) = source_elements[..source_end].get(source_index) else {
                    if target.is_optional {
                        continue;
                    }

                    return Ok(Verdict::Fails);
                };

                // reject an open source rest against individual fixed elements
                if source.is_rest
                    || source.is_readonly && !target.is_readonly
                    || source.is_optional && !target.is_optional
                {
                    return Ok(Verdict::Fails);
                }

                pairs.push((source.ty, target.ty));
                source_index += 1;
            }

            if source_index != source_end {
                return Ok(Verdict::Fails);
            }

            pairs
        };

        self.relate_each(origin, cause, relation.interior(), &pairs)
    }

    /// Return the item type yielded when one spread or rest container expands.
    pub(in crate::sema) fn spread_element_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the sequence head beneath ownership and access forms
        let mut value = self.shallow_resolve(ty)?;
        while let dir::Type::Form(form) = self.ty(value)? {
            value = self.shallow_resolve(form.value)?;
        }

        let element = match self.ty(value)? {
            dir::Type::Application(_) if let Some(element) = self.array_element(value)? => element,
            dir::Type::Slice(slice) => slice.element,
            dir::Type::FixedArray(array) => array.element,
            // erased sequences carry their element on the iterable constraint
            _ if self.is_erased_value(value)? => self.iterable_value_argument(value)?.unwrap_or(ty),
            _ => ty,
        };

        Ok(element)
    }

    /// Relate two structural shapes under exact equality.
    pub(in crate::sema) fn relate_shape_equal(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // compare member shapes and collect type pairs in one pure pass
        let pairs = {
            let (dir::Type::Object(source_shape), dir::Type::Object(target_shape)) =
                (self.ty(source)?, self.ty(target)?)
            else {
                return Ok(Verdict::Fails);
            };

            // require identical member counts for equal shapes
            if source_shape.properties.len() != target_shape.properties.len()
                || source_shape.call_signatures.len() != target_shape.call_signatures.len()
                || source_shape.construct_signatures.len()
                    != target_shape.construct_signatures.len()
                || source_shape.index_signatures.len() != target_shape.index_signatures.len()
            {
                return Ok(Verdict::Fails);
            }

            // pair the access slots of each property in turn
            let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>::new();
            let source_fields = self.shape_properties(source.module_id, source_shape.properties)?;
            let target_fields = self.shape_properties(target.module_id, target_shape.properties)?;
            for (source, target) in source_fields.iter().zip(target_fields) {
                if source.key != target.key || source.is_optional != target.is_optional {
                    return Ok(Verdict::Fails);
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
                    _ => return Ok(Verdict::Fails),
                }
            }

            // pair the call and construct signatures positionally
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

            // pair the key and value types of each index signature
            let source_indexes =
                self.shape_index_signatures(source.module_id, source_shape.index_signatures)?;
            let target_indexes =
                self.shape_index_signatures(target.module_id, target_shape.index_signatures)?;
            for (source, target) in source_indexes.iter().zip(target_indexes) {
                if source.is_optional != target.is_optional
                    || source.is_readonly != target.is_readonly
                {
                    return Ok(Verdict::Fails);
                }
                pairs.push((source.key_type, target.key_type));
                pairs.push((source.value_type, target.value_type));
            }

            pairs
        };

        self.relate_each(origin, cause, Relation::Equal, &pairs)
    }

    /// Relate one structural pair under storage or read rules.
    pub(in crate::sema) fn relate_shape(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // match members and collect signature requirements
        let (pairs, signature_requirements, index_signatures) = {
            let (dir::Type::Object(source_shape), dir::Type::Object(target_shape)) =
                (self.ty(source)?, self.ty(target)?)
            else {
                return Ok(Verdict::Fails);
            };

            // note a constructed source, whose entries adopt the target's storage
            let is_constructed = matches!(self.ty(source)?, dir::Type::Object(_));

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
                            return Ok(Verdict::Fails);
                        }
                    }
                    Some(source_field) => {
                        let Some(relations) = self.shape_property_relations(
                            relation,
                            is_constructed,
                            source_field,
                            target_field,
                        ) else {
                            return Ok(Verdict::Fails);
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

            // collect the index signatures the target requires
            let index_signatures = SmallVec::<[dir::TypeIndexSignature; 2]>::from_slice(
                self.shape_index_signatures(target.module_id, target_shape.index_signatures)?,
            );

            (pairs, signature_requirements, index_signatures)
        };

        // decide matched field pairs
        let mut verdict = self.relate_shape_fields(origin, cause, &pairs)?;
        if verdict == Verdict::Fails {
            return Ok(Verdict::Fails);
        }

        // decide each signature requirement against its candidates
        for (candidates, target_signature) in signature_requirements {
            let mut satisfied = Verdict::Fails;
            for candidate in candidates {
                satisfied = satisfied.or(self.constrain_type(
                    origin,
                    cause,
                    relation,
                    candidate,
                    target_signature,
                )?);
                if satisfied == Verdict::Holds {
                    break;
                }
            }
            verdict = verdict.and(satisfied);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        // require each target index signature from the source
        for signature in index_signatures {
            verdict = verdict
                .and(self.relate_index_signature(origin, cause, relation, source, &signature)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Return the structural property operations exposed by one member.
    pub(in crate::sema) fn property_access(
        &self,
        role: MemberRole,
        ty: dir::GlobalTypeId,
        is_readonly: bool,
    ) -> CompilerResult<Option<dir::PropertyAccess>> {
        let access = match role {
            MemberRole::Field | MemberRole::Method if is_readonly => dir::PropertyAccess::Read(ty),
            MemberRole::Field | MemberRole::Method => dir::PropertyAccess::ReadWrite {
                read: ty,
                write: ty,
            },
            MemberRole::Getter | MemberRole::Setter => {
                // skip an accessor whose signature is still an open variable
                if matches!(self.ty(ty)?, dir::Type::Variable(_)) {
                    return Ok(None);
                }

                let dir::Type::FunctionSignature(signature) = self.ty(ty)? else {
                    return Err(CompilerError::Internal {
                        message: format!("accessor has non-signature type {ty:?}"),
                    });
                };
                let signature = self.type_signature(ty.module_id, signature)?;

                // getters expose their return type
                if role == MemberRole::Getter {
                    let result = signature
                        .return_type
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!("getter signature {ty:?} has no return type"),
                        })?;

                    dir::PropertyAccess::Read(result)
                }
                // setters expose their single value parameter
                else {
                    let parameters =
                        self.signature_parameters(ty.module_id, signature.parameters)?;
                    let [parameter] = parameters else {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "setter signature {ty:?} has {} value parameters",
                                parameters.len(),
                            ),
                        });
                    };

                    dir::PropertyAccess::Write(parameter.ty)
                }
            }
            MemberRole::Associated | MemberRole::VariantValue => {
                return Ok(None);
            }
        };

        Ok(Some(access))
    }

    /// Return the value relations one matched structural property requires.
    pub(in crate::sema) fn shape_property_relations(
        &self,
        relation: Relation,
        is_constructed_source: bool,
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

                // compare reads alone for constructed entries, which adopt the target's storage
                if !is_constructed_source && let Some(target_write) = target.access.write() {
                    relations.push((Relation::Widens, target_write, source.access.write()?));
                }
            }
            // satisfies compares reads only and grants no writes
            Relation::Satisfies => {
                if let Some(target_read) = target.access.read() {
                    relations.push((Relation::Satisfies, source.access.read()?, target_read));
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

    /// Relate each matched structural field under its required relation.
    pub(in crate::sema) fn relate_shape_fields(
        &mut self,
        origin: Origin,
        cause: CauseId,
        fields: &[(Relation, dir::GlobalTypeId, dir::GlobalTypeId)],
    ) -> CompilerResult<Verdict> {
        let mut verdict = Verdict::Holds;
        for (relation, source, target) in fields.iter().copied() {
            verdict = verdict.and(self.constrain_type(origin, cause, relation, source, target)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Return the first writable index signature required by one target type.
    pub(in crate::sema) fn first_writable_index_signature(
        &mut self,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::TypeIndexSignature>> {
        let dir::Type::Object(shape) = self.ty(target)? else {
            return Ok(None);
        };

        let signature = self
            .shape_index_signatures(target.module_id, shape.index_signatures)?
            .iter()
            .find(|signature| !signature.is_readonly)
            .copied();

        Ok(signature)
    }

    /// Relate a static declaration reference to an object type under assignability.
    pub(in crate::sema) fn relate_reference_shape_assignable(
        &mut self,
        origin: Origin,
        cause: CauseId,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // require a declaration reference against an object type target
        if !matches!(self.ty(source)?, dir::Type::Reference(_)) {
            return Ok(Verdict::Fails);
        }
        let dir::Type::Object(target_shape) = self.ty(target)? else {
            return Ok(Verdict::Fails);
        };

        let target_fields = SmallVec::<[dir::TypeProperty; 8]>::from_slice(
            self.shape_properties(target.module_id, target_shape.properties)?,
        );
        let target_constructs = SmallVec::<[dir::GlobalTypeId; 2]>::from_slice(
            self.type_ids(target.module_id, target_shape.construct_signatures)?,
        );
        let module = origin.module();

        // require each target field from the static declaration
        let mut verdict = Verdict::Holds;
        for field in target_fields {
            let subject = dir::MemberSubject::new(source, source, dir::MemberSpace::Static);
            let lookup = self
                .body()
                .lookup_member(origin, module, subject, field.key)?;
            let found = self.body().member_read_type(&lookup)?;
            let Some(found) = found else {
                if field.is_optional {
                    continue;
                }

                return Ok(Verdict::Fails);
            };

            let store = field.access.store();
            verdict = verdict.and(self.constrain_type(
                origin,
                cause,
                Relation::Assignable,
                found,
                store,
            )?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        // require each target constructor from the class constructor set
        for target_signature in target_constructs {
            verdict = verdict.and(self.relate_reference_construct_assignable(
                origin,
                cause,
                Relation::Assignable,
                source,
                target_signature,
            )?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        // skip call and index signatures for nominal declaration values
        if !target_shape.call_signatures.is_empty() || !target_shape.index_signatures.is_empty() {
            return Ok(Verdict::Fails);
        }

        Ok(verdict)
    }

    /// Relate a static declaration reference to a required construct signature.
    pub(in crate::sema) fn relate_reference_construct_assignable(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // require a declaration reference against the construct target
        let dir::Type::Reference(reference) = self.ty(source)? else {
            return Ok(Verdict::Fails);
        };

        // satisfy the target from any one declared constructor
        let mut verdict = Verdict::Fails;
        for candidate in self.reference_construct_signatures(reference)? {
            verdict = verdict.or(self.constrain_type(origin, cause, relation, candidate, target)?);
            if verdict == Verdict::Holds {
                return Ok(Verdict::Holds);
            }
        }

        Ok(verdict)
    }

    /// Return constructor signatures exposed by one static declaration reference.
    pub(in crate::sema) fn reference_construct_signatures(
        &mut self,
        source: dir::TypeReference,
    ) -> CompilerResult<SmallVec<[dir::GlobalTypeId; 2]>> {
        // read constructors from a class declaration only
        let constructors: SmallVec<[dir::GlobalTypeId; 2]> = match self.definition(source.symbol)? {
            Some(dir::Definition::Class(class)) => class
                .constructors
                .iter()
                .map(|constructor| constructor.ty)
                .collect(),
            _ => SmallVec::new(),
        };

        // constructors return the declared instance in place of `this`
        let instance = self.declaration_instance(source.symbol)?;
        let instance = self.intern_type(dir::Type::Application(instance))?;
        let substitution = TypeSubstitution::default().with_receiver(instance);
        let mut signatures = SmallVec::new();
        for constructor in constructors {
            let constructor = self.substitute_type(constructor, &substitution)?;

            // present the declared signature in its construct form
            let Some(head) = self.signature_head(constructor)? else {
                continue;
            };
            signatures.push(self.intern_signature(dir::FunctionSignatureType {
                is_construct: true,
                return_type: head.return_type.or(Some(instance)),
                ..head
            })?);
        }

        Ok(signatures)
    }

    /// Relate one source against a required index signature.
    pub(in crate::sema) fn relate_index_signature(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: &dir::TypeIndexSignature,
    ) -> CompilerResult<Verdict> {
        match self.ty(source)? {
            // compare reads alone for constructed entries, which adopt the target's storage
            dir::Type::Object(shape) => {
                let relation = match relation {
                    Relation::Assignable | Relation::Widens | Relation::Equal => {
                        Relation::Satisfies
                    }
                    relation => relation,
                };

                self.relate_shape_index_signature(
                    origin,
                    cause,
                    relation,
                    source.module_id,
                    shape,
                    target,
                )
            }
            _ => self
                .body()
                .decide_subscript_index_signature_satisfied(origin, relation, source, target),
        }
    }

    /// Relate one structural source against a required index signature.
    fn relate_shape_index_signature(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        module: ModuleId,
        source: dir::ShapeType,
        target: &dir::TypeIndexSignature,
    ) -> CompilerResult<Verdict> {
        // hold covered storage to exactly the value type for keyed finds
        let value_relation = match relation {
            Relation::Satisfies => Relation::Satisfies,
            _ => Relation::Equal,
        };

        // decide a source with declared index signatures by those signatures
        let source_indexes = SmallVec::<[dir::TypeIndexSignature; 2]>::from_slice(
            self.shape_index_signatures(module, source.index_signatures)?,
        );
        if !source_indexes.is_empty() {
            let mut verdict = Verdict::Fails;
            for source in source_indexes {
                if source.is_optional && !target.is_optional {
                    continue;
                }
                if source.is_readonly && !target.is_readonly {
                    continue;
                }
                let key =
                    self.evaluate_relation(origin, relation, target.key_type, source.key_type)?;
                if key == Verdict::Fails {
                    continue;
                }

                let value = self.constrain_type(
                    origin,
                    cause,
                    value_relation,
                    source.value_type,
                    target.value_type,
                )?;
                verdict = verdict.or(key.and(value));
                if verdict == Verdict::Holds {
                    return Ok(Verdict::Holds);
                }
            }

            return Ok(verdict);
        }

        // prove each finite field covered by the key domain
        let source_fields = SmallVec::<[dir::TypeProperty; 8]>::from_slice(
            self.shape_properties(module, source.properties)?,
        );
        let mut verdict = Verdict::Holds;
        for field in source_fields {
            let key = self.static_key_type(field.key)?;
            let covered = self.evaluate_relation(origin, relation, key, target.key_type)?;
            if covered == Verdict::Fails {
                continue;
            }

            // require the field's write slot under writable domains
            if !target.is_readonly && field.access.write().is_none() {
                verdict = verdict.and(Verdict::Fails.join_undecided(covered));
                if verdict == Verdict::Fails {
                    return Ok(Verdict::Fails);
                }
                continue;
            }

            let store = field.access.store();
            let value =
                self.constrain_type(origin, cause, value_relation, store, target.value_type)?;
            verdict = verdict.and(value.join_undecided(covered));
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Relate two function types by signature variance.
    pub(in crate::sema) fn relate_function_assignable(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // normalize both signatures so splatted slots pair positionally
        let source = self.normalize(origin, source)?;
        let target = self.normalize(origin, target)?;

        // bind a polymorphic source against the required signature first
        let Some(instantiation) = self.instantiate_signature(origin, source, target)? else {
            return Ok(Verdict::Fails);
        };
        let source = instantiation.signature;

        // collect directed comparison pairs
        let Some(pairs) =
            self.function_assignability_pairs(source, target, ThisParameterComparison::Compare)?
        else {
            return Ok(Verdict::Fails);
        };

        // relate every interior slot under its own cause
        let relation = relation.interior();
        let mut verdict = Verdict::Holds;
        for (kind, source, target) in pairs {
            let child = match kind {
                Some(kind) => self.intern_cause(Cause::child(origin, kind, cause)),
                None => cause,
            };
            verdict = verdict.and(self.constrain_type(origin, child, relation, source, target)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Instantiate one polymorphic signature at its required signature.
    pub(in crate::sema) fn instantiate_signature(
        &mut self,
        origin: Origin,
        signature: dir::GlobalTypeId,
        required: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SignatureInstantiation>> {
        // peel callable carriers down to their matchable signatures
        let peeled = match self.ty(signature)? {
            dir::Type::Function(function) => function.signature,
            dir::Type::FunctionPointer(pointer) => pointer.signature,
            _ => signature,
        };
        let required = match self.ty(required)? {
            dir::Type::Function(function) => function.signature,
            dir::Type::FunctionPointer(pointer) => pointer.signature,
            _ => required,
        };

        // pass signatures without their own generics through unchanged
        let Some(head) = self.signature_head(peeled)? else {
            return Ok(Some(SignatureInstantiation::concrete(signature)));
        };
        let Some(template) = head.template else {
            return Ok(Some(SignatureInstantiation::concrete(signature)));
        };
        let parameters = self.generic_template_parameters(template)?;
        if parameters.is_empty() {
            return Ok(Some(SignatureInstantiation::concrete(signature)));
        }

        // reject shapes that expose no matchable pairs
        let Some(pairs) = self.signature_match_pairs(peeled, required)? else {
            return Ok(None);
        };

        // match the declared pairs and require the declared constraints
        let mut substitution = TypeSubstitution::default();
        if !self.extend_generic_substitution(origin, &parameters, &mut substitution, &pairs)? {
            return Ok(None);
        }
        if !self.relate_substitution_constraints(origin, template, &substitution)? {
            return Ok(None);
        }

        // extract the complete selection when every value parameter binds
        let mut arguments = Some(Vec::with_capacity(parameters.len()));
        for parameter in parameters.iter().copied() {
            // skip lifetimes, which erase from instance identity
            let is_lifetime = self.generic_parameter(parameter).is_some_and(|binding| {
                binding.memory_parameter() == Some(dir::MemoryParameter::Lifetime)
            });
            if is_lifetime {
                continue;
            }
            match substitution.argument(parameter) {
                Some(argument) => {
                    let argument = self.shallow_resolve(argument)?;
                    if let Some(arguments) = &mut arguments {
                        arguments.push(dir::GenericArgumentBinding::new(parameter, argument));
                    }
                }
                None => arguments = None,
            }
        }

        Ok(Some(SignatureInstantiation {
            signature: self.substitute_type(signature, &substitution)?,
            arguments,
        }))
    }

    /// Relate two receiver-bound method signatures over rigid parameters.
    pub(in crate::sema) fn relate_method(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        mut source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        receiver: Option<dir::GlobalTypeId>,
    ) -> CompilerResult<Verdict> {
        // accept when any overload of an intersected callable satisfies it
        if let dir::Type::Intersection(intersection) = self.ty(source)? {
            let elements: SmallVec<[_; 8]> = self
                .type_ids(source.module_id, intersection.elements)?
                .into();
            let mut verdict = Verdict::Fails;
            for element in elements {
                verdict = verdict
                    .or(self.relate_method(origin, cause, relation, element, target, receiver)?);
                if verdict == Verdict::Holds {
                    return Ok(Verdict::Holds);
                }
            }

            return Ok(verdict);
        }

        // decide anything but two signatures under the plain relation
        if !matches!(
            (self.ty(source)?, self.ty(target)?),
            (
                dir::Type::FunctionSignature(_),
                dir::Type::FunctionSignature(_)
            )
        ) {
            return self.constrain_type(origin, cause, relation, source, target);
        }

        // bind the source's own generics against the required signature
        if let Some(signature) = self.signature_head(source)?
            && let Some(template) = signature.template
        {
            let parameters = self.generic_template_parameters(template)?;
            if !parameters.is_empty() {
                let Some(written_pairs) = self.signature_match_pairs(source, target)? else {
                    return Ok(Verdict::Fails);
                };

                // reduce the required slots as a retry, shedding redundant forms
                let mut reduced_pairs = SmallVec::<[_; 8]>::new();
                for (source_slot, target_slot) in written_pairs.iter().copied() {
                    let reduced = self.normalize(origin, target_slot)?;
                    let reduced = self.reduce_redundant_forms(origin, reduced)?;
                    reduced_pairs.push((source_slot, reduced));
                }

                // bind over the written slots, falling back to the reduced ones
                let mut matched = None;
                for pairs in [written_pairs, reduced_pairs] {
                    // relate under the receiver, binding this-projected bounds
                    let mut substitution = TypeSubstitution::default();
                    if let Some(receiver) = receiver {
                        substitution = substitution.with_receiver(receiver);
                    }

                    // bind receiver slots when the two receiver shapes align
                    if let (Some(signature), Some(required)) =
                        (self.signature_head(source)?, self.signature_head(target)?)
                        && let (Some(source_this), Some(target_this)) =
                            (signature.this_parameter, required.this_parameter)
                    {
                        let mut scratch = substitution.clone();
                        let this_pairs = [(source_this, target_this)];
                        if self.extend_generic_substitution(
                            origin,
                            &parameters,
                            &mut scratch,
                            &this_pairs,
                        )? {
                            substitution = scratch;
                        }
                    }

                    // match the slot pairs, moving to the next round when they disagree
                    if !self.extend_generic_substitution(
                        origin,
                        &parameters,
                        &mut substitution,
                        &pairs,
                    )? {
                        continue;
                    }

                    // require the declared constraints of the bound arguments
                    if !self.relate_substitution_constraints(origin, template, &substitution)? {
                        return Ok(Verdict::Fails);
                    }

                    // keep the first substitution that binds every slot
                    matched = Some(substitution);
                    break;
                }

                let Some(substitution) = matched else {
                    return Ok(Verdict::Fails);
                };

                // compare the instantiated source from here on
                source = self.substitute_type(source, &substitution)?;
            }
        }

        // collect directed comparison pairs
        let Some(pairs) =
            self.function_assignability_pairs(source, target, ThisParameterComparison::Skip)?
        else {
            return Ok(Verdict::Fails);
        };

        // relate each pair with redundant forms shed and closed readonly borrows stripped
        let mut verdict = Verdict::Holds;
        for (_, source, target) in pairs {
            let source = self.reduce_redundant_forms(origin, source)?;
            let source = self.strip_borrows(origin, source)?;
            let target = self.normalize(origin, target)?;
            let target = self.reduce_redundant_forms(origin, target)?;
            let target = self.strip_borrows(origin, target)?;
            verdict = verdict.and(self.constrain_type(origin, cause, relation, source, target)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Strip readonly borrows from one type, mapping union arms one level deep.
    fn strip_borrows(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // strip union arms one by one, a single level deep
        if let dir::Type::Union(union) = self.ty(ty)? {
            let elements =
                SmallVec::<[_; 4]>::from_slice(self.type_ids(ty.module_id, union.elements)?);
            let mut stripped = Vec::with_capacity(elements.len());
            let mut changed = false;
            for element in elements {
                let payload = self.lent_borrow_payload(origin, element)?;
                changed |= payload != element;
                stripped.push(payload);
            }
            if !changed {
                return Ok(ty);
            }

            return self.normalized_union_type(stripped);
        }

        self.lent_borrow_payload(origin, ty)
    }

    /// Return the payload behind one closed readonly borrow of a copyable value.
    fn lent_borrow_payload(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // require a borrow form
        let dir::Type::Form(dir::FormType {
            form: dir::Form::Borrowed(borrow),
            value,
        }) = self.ty(ty)?
        else {
            return Ok(ty);
        };

        // require readonly access over a copyable payload
        let access = self.type_borrow(ty.module_id, borrow)?.access;
        if self.access_literal(origin, access)? != Some(dir::Access::Readonly) {
            return Ok(ty);
        }

        // keep the borrowed form as the compared shape while the copy is undecided
        if !self
            .satisfies_auto_interface(origin, value, dir::AutoInterface::Copy)?
            .holds()
        {
            return Ok(ty);
        }

        Ok(value)
    }

    /// Return positional signature pairs for generic parameter matching.
    pub(in crate::sema) fn signature_match_pairs(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>>> {
        let (Some(source_signature), Some(target_signature)) =
            (self.signature_head(source)?, self.signature_head(target)?)
        else {
            return Ok(None);
        };

        // pair the parameters both signatures declare
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

        // pair the returns when both signatures declare one
        if let (Some(source), Some(target)) =
            (source_signature.return_type, target_signature.return_type)
        {
            pairs.push((source, target));
        }

        Ok(Some(pairs))
    }

    /// Return directed function assignment pairs, or none when the shapes cannot relate.
    fn function_assignability_pairs(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
        this_parameter: ThisParameterComparison,
    ) -> CompilerResult<Option<SmallVec<[FunctionAssignabilityPair; 8]>>> {
        // require two function signatures
        let (Some(source_signature), Some(target_signature)) =
            (self.signature_head(source)?, self.signature_head(target)?)
        else {
            return Ok(None);
        };

        // only a constructor satisfies a construct signature
        if source_signature.is_construct != target_signature.is_construct {
            return Ok(None);
        }

        let mut pairs = SmallVec::<[FunctionAssignabilityPair; 8]>::new();

        // compare receiver input contravariantly for function values
        if this_parameter.includes_this() {
            match (
                source_signature.this_parameter,
                target_signature.this_parameter,
            ) {
                (Some(source), Some(target)) => pairs.push((None, target, source)),
                (None, _) => {}
                (Some(_), None) => return Ok(None),
            }
        }

        // require source parameters to accept every target call arity
        let source_slots = self.parameter_slots(source.module_id, source_signature.parameters)?;
        let target_slots = self.parameter_slots(target.module_id, target_signature.parameters)?;
        let required = source_slots
            .iter()
            .filter(|slot| !slot.is_optional && !slot.is_rest)
            .count();
        if target_slots.len() < required && !target_slots.iter().any(|slot| slot.is_rest) {
            return Ok(None);
        }

        // compare runtime inputs contravariantly, spreading rest slots over the other positions
        let mut source_index = 0usize;
        for (index, target) in target_slots.iter().enumerate() {
            let cause = Some(CauseKind::Parameter {
                index: index as u32,
            });

            // spread the target rest over every remaining source slot
            if target.is_rest {
                // bind the remaining source slots as one tuple for an open rest binder
                let rest = self.shallow_resolve(target.ty)?;
                if matches!(self.ty(rest)?, dir::Type::Variable(_)) {
                    let elements = source_slots[source_index..]
                        .iter()
                        .map(|slot| dir::TypeElement {
                            label: None,
                            ty: slot.ty,
                            is_optional: slot.is_optional,
                            is_readonly: false,
                            is_rest: slot.is_rest,
                        })
                        .collect::<SmallVec<[_; 4]>>();
                    let tuple = self.intern_tuple(&elements)?;
                    pairs.push((cause, tuple, rest));
                    break;
                }

                let element = self.rest_element_type(target.ty)?;
                while let Some(source) = source_slots.get(source_index) {
                    let target = if source.is_rest { target.ty } else { element };
                    let cause = Some(CauseKind::Parameter {
                        index: source_index as u32,
                    });
                    pairs.push((cause, target, source.ty));
                    source_index += 1;
                }
                break;
            }

            // stop at the target inputs past the source slots
            let Some(source) = source_slots.get(source_index) else {
                break;
            };

            // spread the source rest element over every remaining target slot
            if source.is_rest {
                let element = self.rest_element_type(source.ty)?;
                pairs.push((cause, target.ty, element));
                continue;
            }

            pairs.push((cause, target.ty, source.ty));
            source_index += 1;
        }

        // compare outputs covariantly, discarding results a void target ignores
        match (source_signature.return_type, target_signature.return_type) {
            (Some(source), Some(target)) => {
                let resolved = self.shallow_resolve(target)?;
                if !matches!(self.ty(resolved)?, dir::Type::Void) {
                    pairs.push((Some(CauseKind::ReturnSlot), source, target));
                }
            }
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
    /// Skip `this`, which the method receiver check compares separately.
    Skip,
}

impl ThisParameterComparison {
    /// Return whether `this` participates in this comparison.
    fn includes_this(self) -> bool {
        matches!(self, Self::Compare)
    }
}

/// One positional slot of a parameter list, with rest tuples spread in place.
pub(in crate::sema) struct ParameterSlot {
    /// The slot type, the whole container for a rest slot.
    pub(in crate::sema) ty: dir::GlobalTypeId,
    /// Whether a call may omit the slot.
    pub(in crate::sema) is_optional: bool,
    /// Whether the slot captures every remaining call argument.
    pub(in crate::sema) is_rest: bool,
}

impl CheckState<'_> {
    /// Expand one parameter list, spreading a rest tuple over its elements.
    pub(in crate::sema) fn parameter_slots(
        &self,
        module: ModuleId,
        parameters: dir::TypeListId,
    ) -> CompilerResult<SmallVec<[ParameterSlot; 6]>> {
        let mut slots = SmallVec::new();
        for parameter in self.signature_parameters(module, parameters)? {
            if !parameter.is_rest {
                slots.push(ParameterSlot {
                    ty: parameter.ty,
                    is_optional: parameter.is_optional,
                    is_rest: false,
                });
                continue;
            }
            let rest = self.shallow_resolve(parameter.ty)?;
            match self.ty(rest)? {
                dir::Type::Tuple(tuple) => {
                    for element in self.tuple_elements(rest.module_id, tuple.elements)? {
                        slots.push(ParameterSlot {
                            ty: element.ty,
                            is_optional: element.is_optional,
                            is_rest: element.is_rest,
                        });
                    }
                }
                _ => slots.push(ParameterSlot {
                    ty: rest,
                    is_optional: false,
                    is_rest: true,
                }),
            }
        }

        Ok(slots)
    }

    /// Return the type one rest container supplies to each position it spreads over.
    pub(in crate::sema) fn rest_element_type(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // read the sequence head beneath ownership and access forms
        let mut value = self.shallow_resolve(ty)?;
        while let dir::Type::Form(form) = self.ty(value)? {
            value = self.shallow_resolve(form.value)?;
        }

        let element = match self.ty(value)? {
            dir::Type::Application(_) if let Some(element) = self.array_element(value)? => element,
            dir::Type::Slice(slice) => slice.element,
            dir::Type::FixedArray(array) => array.element,
            _ => value,
        };

        Ok(element)
    }
}

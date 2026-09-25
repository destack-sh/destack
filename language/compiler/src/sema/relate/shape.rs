use smallvec::SmallVec;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{
    Cause, CauseId, CauseKind, CheckState, GenericParameterId, MemberRole, MemoryGrounding, Origin,
    Relation, TypeSubstitution, Verdict,
};
use crate::{CompilerError, CompilerResult};

/// Where the source properties of one shape relation come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum PropertySource {
    /// A literal's fields, stored once into the target's fields.
    Constructed,
    /// Stored fields, each read and assigned in place.
    Stored,
}

/// One directed function assignment pair with the cause it reports under.
type FunctionAssignabilityPair = (Option<CauseKind>, dir::GlobalTypeId, dir::GlobalTypeId);

/// One signature instantiated at its required signature.
pub(in crate::sema) struct SignatureInstantiation {
    /// The instantiated signature.
    pub(in crate::sema) signature: dir::GlobalTypeId,
    /// The value arguments selecting the instance, or None while parameters stay open.
    pub(in crate::sema) arguments: Option<Vec<dir::GenericArgumentBinding>>,
    /// The lifetime arguments selected for the declared signature.
    pub(in crate::sema) regions: Vec<dir::GenericArgumentBinding>,
}

impl SignatureInstantiation {
    /// Return one concrete signature passed through unchanged.
    fn concrete(signature: dir::GlobalTypeId) -> Self {
        Self {
            signature,
            arguments: Some(Vec::new()),
            regions: Vec::new(),
        }
    }
}

// TODO #Cleanup: compress relate/shape.rs

impl CheckState<'_> {
    /// Return whether two callables declare the same parameter list shape.
    pub(in crate::sema) fn signature_shapes_match(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let (Some(source_signature), Some(target_signature)) =
            (self.signature_head(source)?, self.signature_head(target)?)
        else {
            return Ok(true);
        };

        // compare the parameter lists slot by slot
        let source_parameters: SmallVec<[_; 4]> = self
            .signature_parameters(source.module_id, source_signature.parameters)?
            .into();
        let target_parameters: SmallVec<[_; 4]> = self
            .signature_parameters(target.module_id, target_signature.parameters)?
            .into();
        let matches = source_parameters.len() == target_parameters.len()
            && source_parameters
                .iter()
                .zip(target_parameters.iter())
                .all(|(source, target)| {
                    source.is_optional == target.is_optional && source.is_rest == target.is_rest
                });

        Ok(matches)
    }

    /// Return the value beneath one type's written forms.
    pub(in crate::sema) fn shallow_strip_forms(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // peel each form down to the value beneath it
        let mut ty = self.shallow_resolve(ty)?;
        while let dir::Type::Form(form) = self.ty(ty)? {
            let payload = self.shallow_resolve(form.value)?;
            if payload == ty {
                break;
            }
            ty = payload;
        }

        Ok(ty)
    }

    /// Return whether one type can be used as a property key.
    pub(in crate::sema) fn is_property_key_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // key a type by the head standing on it
        let result = match self.ty(ty)? {
            dir::Type::Parameter(_) => true,
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
            // reject every other type
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

        // key a type by the head standing on it
        let result = match self.ty(ty)? {
            dir::Type::Parameter(_) => true,
            dir::Type::Object(_) => true,
            dir::Type::Dynamic(dynamic) => self.is_keyed_type(origin, dynamic.constraint)?,
            // arrays enumerate positionally
            dir::Type::Application(_) if self.array_element(ty)?.is_some() => false,
            dir::Type::Application(instance) => matches!(
                self.symbol_kind(instance.symbol)?,
                dir::SymbolKind::Class | dir::SymbolKind::Struct | dir::SymbolKind::Interface
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
            // reject every other type
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
        // pair the elements, then relate each pair
        let Some(pairs) = self.tuple_pairs(source, target)? else {
            return Ok(Verdict::Fails);
        };

        self.relate_each(origin, cause, relation, &pairs)
    }

    /// Pair the elements of one tuple with the slots of another, `None` when their shapes disagree.
    pub(in crate::sema) fn tuple_pairs(
        &mut self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>>> {
        // require two tuples written in the same form
        let (dir::Type::Tuple(source_tuple), dir::Type::Tuple(target_tuple)) =
            (self.ty(source)?, self.ty(target)?)
        else {
            return Ok(None);
        };
        if source_tuple.form != target_tuple.form {
            return Ok(None);
        }

        // read both element lists
        let source_elements = self.tuple_elements(source.module_id, source_tuple.elements)?;
        let target_elements = self.tuple_elements(target.module_id, target_tuple.elements)?;
        let mut pairs = SmallVec::<[(dir::GlobalTypeId, dir::GlobalTypeId); 4]>::new();

        // consume source elements from the end for fixed targets after a rest
        let rest_index = target_elements.iter().position(|target| target.is_rest);
        let (target_elements, trailing_targets) = match rest_index {
            Some(rest_index) => target_elements.split_at(rest_index + 1),
            None => (target_elements, &[][..]),
        };
        let mut source_end = source_elements.len();
        for target in trailing_targets.iter().rev() {
            let Some(source) = source_end
                .checked_sub(1)
                .map(|index| &source_elements[index])
            else {
                return Ok(None);
            };
            if source.is_rest || source.is_optional || source.is_readonly && !target.is_readonly {
                return Ok(None);
            }

            pairs.push((source.ty, target.ty));
            source_end -= 1;
        }

        // pair the leading target elements with source elements in order
        let mut source_index = 0usize;
        for target in target_elements {
            // rest targets consume every remaining source element
            if target.is_rest {
                let remaining = &source_elements[source_index.min(source_end)..source_end];
                if remaining
                    .iter()
                    .any(|source| source.is_readonly && !target.is_readonly)
                {
                    return Ok(None);
                }

                // bind the remaining elements as one tuple for an open rest binder
                let rest = self.shallow_resolve(target.ty)?;
                if matches!(self.ty(rest)?, dir::Type::Variable(_)) {
                    let tuple = self.intern_tuple(remaining)?;
                    pairs.push((tuple, rest));

                    return Ok(Some(pairs));
                }

                let Some(target_element) = self.spread_element_type(target.ty)? else {
                    return Ok(None);
                };
                for source in remaining {
                    let target = if source.is_rest {
                        target.ty
                    } else {
                        target_element
                    };
                    pairs.push((source.ty, target));
                }

                return Ok(Some(pairs));
            }

            // omitted source elements satisfy optional target elements
            let Some(source) = source_elements[..source_end].get(source_index) else {
                if target.is_optional {
                    continue;
                }

                return Ok(None);
            };

            // reject an open source rest against individual fixed elements
            if source.is_rest
                || source.is_readonly && !target.is_readonly
                || source.is_optional && !target.is_optional
            {
                return Ok(None);
            }

            pairs.push((source.ty, target.ty));
            source_index += 1;
        }

        // reject source elements the target left unconsumed
        if source_index != source_end {
            return Ok(None);
        }

        Ok(Some(pairs))
    }

    /// Return the item type yielded when one spread or rest container expands.
    pub(in crate::sema) fn spread_element_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // read the sequence head beneath ownership and access forms
        let mut value = self.shallow_resolve(ty)?;
        while let dir::Type::Form(form) = self.ty(value)? {
            value = self.shallow_resolve(form.value)?;
        }

        // read the element the container yields
        let element = match self.ty(value)? {
            // preserve bottom in contravariant rest comparisons
            dir::Type::Never => Some(value),
            // erased sequences carry their element on the iterable constraint
            _ if self.is_erased_value(value)? => self.iterable_value_argument(value)?,
            _ => self.sequence_element_type(value)?,
        };

        Ok(element)
    }

    /// Relate two object shapes member by member over an exact member set.
    pub(in crate::sema) fn relate_shape(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        properties: PropertySource,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        // require two object shapes
        let (dir::Type::Object(source_shape), dir::Type::Object(target_shape)) =
            (self.ty(source)?, self.ty(target)?)
        else {
            return Ok(Verdict::Fails);
        };
        let is_directed = relation.is_directed();

        // pair each source property with the target property of its key
        let source_fields = self.object_properties(source.module_id, source_shape.properties)?;
        let target_fields = self.object_properties(target.module_id, target_shape.properties)?;
        let target_indexes = SmallVec::<[dir::TypeIndexSignature; 2]>::from_slice(
            self.object_index_signatures(target.module_id, target_shape.index_signatures)?,
        );
        let mut pairs = SmallVec::<[(Relation, dir::GlobalTypeId, dir::GlobalTypeId); 8]>::new();
        let mut has_extra = false;
        for source_field in source_fields {
            match target_fields
                .iter()
                .find(|field| field.key == source_field.key)
            {
                Some(target_field) => {
                    if source_field.is_optional && !target_field.is_optional
                        || (!is_directed && source_field.is_optional != target_field.is_optional)
                    {
                        return Ok(Verdict::Fails);
                    }

                    // keep the target's exact access for a stored object
                    if relation == Relation::Storable
                        && properties == PropertySource::Stored
                        && source_field.access.write().is_some()
                            != target_field.access.write().is_some()
                    {
                        return Ok(Verdict::Fails);
                    }
                    let Some(relations) = Self::shape_property_relations(
                        relation,
                        properties,
                        source_field,
                        target_field,
                    ) else {
                        return Ok(Verdict::Fails);
                    };
                    pairs.extend(relations);
                }
                None => has_extra = true,
            }
        }

        // require every target member, optional ones only under a directed relation
        for target_field in target_fields {
            let is_present = source_fields
                .iter()
                .any(|field| field.key == target_field.key);
            if !is_present && !(is_directed && target_field.is_optional) {
                return Ok(Verdict::Fails);
            }
        }

        // allow extra keys on an included value
        if has_extra && target_indexes.is_empty() && relation != Relation::Subtype {
            return Ok(Verdict::Fails);
        }

        // pair the call and construct signatures positionally
        let source_calls = self.type_ids(source.module_id, source_shape.call_signatures)?;
        let target_calls = self.type_ids(target.module_id, target_shape.call_signatures)?;
        let source_constructs =
            self.type_ids(source.module_id, source_shape.construct_signatures)?;
        let target_constructs =
            self.type_ids(target.module_id, target_shape.construct_signatures)?;
        // require the source signatures to cover the target set
        let is_covered = match is_directed {
            true => {
                source_calls.len() >= target_calls.len()
                    && source_constructs.len() >= target_constructs.len()
            }
            false => {
                source_calls.len() == target_calls.len()
                    && source_constructs.len() == target_constructs.len()
            }
        };
        if !is_covered {
            return Ok(Verdict::Fails);
        }

        // pair each signature with the one at its position
        for (source, target) in source_calls.iter().zip(target_calls) {
            pairs.push((relation, *source, *target));
        }
        for (source, target) in source_constructs.iter().zip(target_constructs) {
            pairs.push((relation, *source, *target));
        }

        // pair equal index signatures exactly
        let source_indexes = SmallVec::<[dir::TypeIndexSignature; 2]>::from_slice(
            self.object_index_signatures(source.module_id, source_shape.index_signatures)?,
        );
        if !is_directed {
            if source_indexes.len() != target_indexes.len() {
                return Ok(Verdict::Fails);
            }
            for (source, target) in source_indexes.iter().zip(&target_indexes) {
                if source.is_optional != target.is_optional
                    || source.is_readonly != target.is_readonly
                {
                    return Ok(Verdict::Fails);
                }
                pairs.push((relation, source.key_type, target.key_type));
                pairs.push((relation, source.value_type, target.value_type));
            }
        }

        // relate the paired members
        let mut verdict = self.relate_shape_fields(origin, cause, &pairs)?;

        // cover each target index signature under a directed relation
        if is_directed {
            for index in target_indexes {
                if verdict == Verdict::Fails {
                    break;
                }
                verdict = verdict
                    .and(self.relate_index_signature(origin, cause, relation, source, &index)?);
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
        // expose the operations each member role supports
        let access = match role {
            // readonly fields and methods read alone
            MemberRole::Field | MemberRole::Method if is_readonly => dir::PropertyAccess::Read(ty),
            // fields and methods read and write one type
            MemberRole::Field | MemberRole::Method => dir::PropertyAccess::ReadWrite {
                read: ty,
                write: ty,
            },
            // accessors expose the slots of their signature
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
            // associated members and variant values expose no property
            MemberRole::Associated | MemberRole::VariantValue => {
                return Ok(None);
            }
        };

        Ok(Some(access))
    }

    /// Return the value relations one matched structural property requires.
    pub(in crate::sema) fn shape_property_relations(
        relation: Relation,
        properties: PropertySource,
        source: &dir::TypeProperty,
        target: &dir::TypeProperty,
    ) -> Option<SmallVec<[(Relation, dir::GlobalTypeId, dir::GlobalTypeId); 2]>> {
        let is_stored = properties == PropertySource::Stored;
        let mut relations = SmallVec::new();

        // pair the reads and writes the relation requires
        match relation {
            // equal shapes pair each supported operation exactly
            Relation::Equal => {
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
            // pair stored fields exactly
            Relation::Storable
                if is_stored && is_field(source.access) && is_field(target.access) =>
            {
                let (Some(source_read), Some(target_read)) =
                    (source.access.read(), target.access.read())
                else {
                    return None;
                };
                relations.push((Relation::Equal, source_read, target_read));
            }
            // flow reads out and writes in
            Relation::Subtype | Relation::Storable => {
                if let Some(target_read) = target.access.read() {
                    let source_read = source.access.read()?;
                    relations.push((relation, source_read, target_read));
                }
                if let Some(target_write) = target.access.write()
                    && is_stored
                    && (relation == Relation::Storable || target.access.read().is_none())
                {
                    let source_write = source.access.write()?;
                    relations.push((relation, target_write, source_write));
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
        // relate each pair under the relation it carries
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
        // require an object shape
        let dir::Type::Object(shape) = self.ty(target)? else {
            return Ok(None);
        };

        // take the first index signature that writes
        let signature = self
            .object_index_signatures(target.module_id, shape.index_signatures)?
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

        // read the members the target declares
        let target_fields = SmallVec::<[dir::TypeProperty; 8]>::from_slice(
            self.object_properties(target.module_id, target_shape.properties)?,
        );
        let target_constructs = SmallVec::<[dir::GlobalTypeId; 2]>::from_slice(
            self.type_ids(target.module_id, target_shape.construct_signatures)?,
        );
        let module = origin.module();

        // require each target field from the static declaration
        let mut verdict = Verdict::Holds;
        for field in target_fields {
            let subject = self.member_subject(origin, source, source, dir::MemberSpace::Static)?;
            let lookup = self.lookup_member(origin, module, subject, field.key)?;
            let found = self.member_read_type(&lookup)?;
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
                Relation::Storable,
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
                Relation::Storable,
                source,
                target_signature,
            )?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        // reject call and index signatures on nominal declaration values
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
        if !matches!(self.ty(source)?, dir::Type::Reference(_)) {
            return Ok(Verdict::Fails);
        }

        // satisfy the target from any one declared constructor
        let mut verdict = Verdict::Fails;
        for candidate in self.construct_signatures(origin, source, MemoryGrounding::Open)? {
            verdict =
                verdict.or(self.constrain_type(origin, cause, relation, candidate.ty, target)?);
            if verdict == Verdict::Holds {
                return Ok(Verdict::Holds);
            }
        }

        Ok(verdict)
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
        // relate a structural source directly, every other source by subscript
        match self.ty(source)? {
            dir::Type::Object(shape) => self.relate_shape_index_signature(
                origin,
                cause,
                relation,
                source.module_id,
                shape,
                target,
            ),
            _ => self.decide_subscript_index_signature(origin, relation, source, target),
        }
    }

    /// Relate one structural source against a required index signature.
    fn relate_shape_index_signature(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        module: ModuleId,
        source: dir::ObjectType,
        target: &dir::TypeIndexSignature,
    ) -> CompilerResult<Verdict> {
        // prefer the source's own declared index signatures
        let source_indexes = SmallVec::<[dir::TypeIndexSignature; 2]>::from_slice(
            self.object_index_signatures(module, source.index_signatures)?,
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

                // cover one key domain by another through inclusion
                let key = self.decide_relation(
                    origin,
                    Relation::Subtype,
                    target.key_type,
                    source.key_type,
                )?;
                if key == Verdict::Fails {
                    continue;
                }

                // relate the value the two domains carry
                let value = self.constrain_type(
                    origin,
                    cause,
                    relation,
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

        // require each declared field covered by the key domain
        let source_fields = SmallVec::<[dir::TypeProperty; 8]>::from_slice(
            self.object_properties(module, source.properties)?,
        );
        let mut verdict = Verdict::Holds;
        for field in source_fields {
            // cover a declared key by the key domain through inclusion
            let key = self.static_key_type(field.key)?;
            let covered = self.decide_relation(origin, Relation::Subtype, key, target.key_type)?;
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

            // relate the field's stored type to the value domain
            let store = field.access.store();
            let value = self.constrain_type(origin, cause, relation, store, target.value_type)?;
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
        // normalize both signatures so spread parameters pair positionally
        let source = self.normalize(origin, source)?;
        let target = self.normalize(origin, target)?;

        // bind a polymorphic source against the required signature first
        let instantiation = self.instantiate_signature(origin, source, target)?;
        let Some(instantiation) = instantiation else {
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
        let mut verdict = Verdict::Holds;
        for (kind, left, right) in pairs {
            let child = match kind {
                Some(kind) => self.intern_cause(Cause::child(origin, kind, cause)),
                None => cause,
            };
            let own_slot = match kind {
                Some(CauseKind::ReturnSlot) => left,
                _ => right,
            };
            let slot_relation = match self.root_variable(own_slot)?.is_some() {
                true => Relation::Equal,
                false => relation,
            };
            verdict =
                verdict.and(self.constrain_type(origin, child, slot_relation, left, right)?);
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
        // peel function and pointer types down to their signatures
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
        let parameters = self.signature_generic_parameters(peeled.module_id, &head)?;
        if parameters.is_empty() {
            let bindings = self
                .signature_arguments(peeled.module_id, head.arguments)?
                .to_vec();
            let regions = self.resolved_region_bindings(&bindings)?;

            return Ok(Some(SignatureInstantiation {
                regions,
                ..SignatureInstantiation::concrete(signature)
            }));
        }

        // reject shapes that expose no matchable pairs
        let Some(pairs) = self.signature_match_pairs(peeled, required)? else {
            return Ok(None);
        };

        // match the declared pairs and require the declared constraints
        let fixed = self.signature_arguments(peeled.module_id, head.arguments)?;
        let mut substitution = TypeSubstitution::default().with_carried(fixed)?;
        self.bind_aligned_receivers(origin, &parameters, &mut substitution, peeled, required)?;
        if !self.extend_generic_substitution(origin, &parameters, &mut substitution, &pairs)? {
            return Ok(None);
        }
        if !self.relate_substitution_constraints(origin, template, &substitution, None)? {
            return Ok(None);
        }

        // extract the complete selection when every value parameter binds
        let parameters = self.generic_template_parameters(template)?;
        let mut arguments = Some(Vec::with_capacity(parameters.len()));
        let mut lifetimes = Vec::new();
        for parameter in parameters.iter().copied() {
            // skip lifetimes, which erase from instance identity
            let is_lifetime = self.generic_parameter(parameter)?.is_some_and(|binding| {
                binding.memory_parameter() == Some(dir::MemoryParameter::Region)
            });
            if is_lifetime {
                lifetimes.push(parameter);

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

        // substitute the bound arguments, keeping the signature's own binder regions
        let regions = self.resolved_region_bindings(&substitution.bindings)?;
        let mut bound = substitution;
        bound
            .bindings
            .retain(|binding| !lifetimes.contains(&binding.parameter));
        let substituted = self.substitute_type(signature, &bound)?;
        if substituted == signature {
            arguments = Some(Vec::new());
        }

        Ok(Some(SignatureInstantiation {
            signature: substituted,
            arguments,
            regions,
        }))
    }

    /// Bind one signature's generics through its receiver when both receiver shapes align.
    fn bind_aligned_receivers(
        &mut self,
        origin: Origin,
        parameters: &[GenericParameterId],
        substitution: &mut TypeSubstitution,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let (Some(signature), Some(required)) =
            (self.signature_head(source)?, self.signature_head(target)?)
        else {
            return Ok(());
        };
        let (Some(source_this), Some(target_this)) =
            (signature.this_parameter, required.this_parameter)
        else {
            return Ok(());
        };

        // keep the receiver bindings of matching shapes
        let mut scratch = substitution.clone();
        let pairs = [(source_this, target_this)];
        if self.extend_generic_substitution(origin, parameters, &mut scratch, &pairs)? {
            *substitution = scratch;
        }

        Ok(())
    }

    /// Relate two receiver-bound method signatures over rigid parameters.
    pub(in crate::sema) fn relate_method(
        &mut self,
        origin: Origin,
        cause: CauseId,
        relation: Relation,
        mut source: dir::GlobalTypeId,
        mut target: dir::GlobalTypeId,
        receiver: Option<dir::GlobalTypeId>,
        assumed: Option<&TypeSubstitution>,
    ) -> CompilerResult<Verdict> {
        // accept when any overload of an intersected callable satisfies it
        if let dir::Type::Intersection(intersection) = self.ty(source)? {
            let elements: SmallVec<[_; 8]> = self
                .type_ids(source.module_id, intersection.elements)?
                .into();
            let mut verdict = Verdict::Fails;
            for element in elements {
                verdict = verdict.or(self
                    .relate_method(origin, cause, relation, element, target, receiver, assumed)?);
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

        // the found signature reads `this` as the receiver it is checked at
        if let Some(receiver) = receiver {
            let receiver = TypeSubstitution::default().with_receiver(receiver);
            source = self.substitute_type(source, &receiver)?;
        }

        // open the required signature's induced memory parameters for the match to bind
        if let Some(required) = self.signature_head(target)? {
            let parameters = self.signature_generic_parameters(target.module_id, &required)?;
            let mut fresh = TypeSubstitution::default();
            for parameter in parameters {
                if let Some(kind) = self
                    .require_generic_parameter(parameter)?
                    .induced_memory_parameter()
                {
                    let variable = self.open_memory_type(origin, kind)?;
                    fresh.bind(parameter, variable)?;
                }
            }
            if !fresh.is_empty() {
                target = self.substitute_type(target, &fresh)?;
            }
        }

        // bind the source's own generics against the required signature
        if let Some(signature) = self.signature_head(source)?
            && let Some(template) = signature.template
        {
            let parameters = self.signature_generic_parameters(source.module_id, &signature)?;
            if !parameters.is_empty() {
                let Some(written_pairs) = self.signature_match_pairs(source, target)? else {
                    return Ok(Verdict::Fails);
                };

                // reduce the required slots, shedding redundant forms
                let mut pairs = SmallVec::<[_; 8]>::new();
                for (source_slot, target_slot) in written_pairs.iter().copied() {
                    let reduced = self.normalize(origin, target_slot)?;
                    let reduced = self.reduce_redundant_forms(origin, reduced)?;
                    pairs.push((source_slot, reduced));
                }

                // relate under the receiver, binding this-projected bounds
                let fixed = self.signature_arguments(source.module_id, signature.arguments)?;
                let mut substitution = TypeSubstitution::default().with_carried(fixed)?;
                if let Some(receiver) = receiver {
                    substitution = substitution.with_receiver(receiver);
                }

                // bind receivers when the two receiver shapes align
                self.bind_aligned_receivers(
                    origin,
                    &parameters,
                    &mut substitution,
                    source,
                    target,
                )?;

                // match the slot pairs and require the declared constraints of the bound arguments
                let is_bound = self.extend_generic_substitution(
                    origin,
                    &parameters,
                    &mut substitution,
                    &pairs,
                )?;
                let constrained = is_bound
                    && self.relate_substitution_constraints(
                        origin,
                        template,
                        &substitution,
                        assumed,
                    )?;
                if !constrained {
                    return Ok(Verdict::Fails);
                }

                // ground the memory parameters the match left unbound at their elided defaults
                self.ground_memory_parameters(
                    origin,
                    &parameters,
                    &mut substitution,
                    MemoryGrounding::Elided,
                )?;

                // compare the instantiated source from here on
                source = self.substitute_type(source, &substitution)?;
            }
        }

        // require the promised receiver to grant the implementation's demand
        if let (Some(signature), Some(required)) =
            (self.signature_head(source)?, self.signature_head(target)?)
            && let (Some(source_this), Some(target_this)) =
                (signature.this_parameter, required.this_parameter)
            && self.constrain_receiver_grant(origin, target_this, source_this)? == Verdict::Fails
        {
            return Ok(Verdict::Fails);
        }

        // collect directed comparison pairs
        let Some(pairs) =
            self.function_assignability_pairs(source, target, ThisParameterComparison::Skip)?
        else {
            return Ok(Verdict::Fails);
        };

        // relate each pair with redundant forms shed
        let mut verdict = Verdict::Holds;
        for (_, source, target) in pairs {
            let source = self.normalize(origin, source)?;
            let source = self.reduce_redundant_forms(origin, source)?;
            let target = self.normalize(origin, target)?;
            let target = self.reduce_redundant_forms(origin, target)?;
            let decided = self.constrain_type(origin, cause, relation, source, target)?;
            verdict = verdict.and(decided);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Require one promised receiver to grant one demanded receiver.
    fn constrain_receiver_grant(
        &mut self,
        origin: Origin,
        promise: dir::GlobalTypeId,
        demand: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let promise = self.receiver_shape(origin, promise)?;
        let demand = self.receiver_shape(origin, demand)?;

        match (promise, demand) {
            // a receiver-generic demand accepts every promise
            (_, None) => Ok(Verdict::Holds),
            // a whole receiver reborrows at every access
            (Some(ReceiverShape::Whole), Some(_)) => Ok(Verdict::Holds),
            // borrowed receivers grant by their access terms
            (Some(ReceiverShape::Borrowed(granted)), Some(ReceiverShape::Borrowed(requested))) => {
                self.constrain_access_assignable(origin, granted, requested)
            }
            // a borrow never satisfies a consuming demand
            (Some(ReceiverShape::Borrowed(_)), Some(ReceiverShape::Whole)) => Ok(Verdict::Fails),
            // an opaque promise stays with the pair machinery
            (None, Some(_)) => Ok(Verdict::Holds),
        }
    }

    /// Return the receiver form one `this` parameter type presents.
    fn receiver_shape(
        &mut self,
        origin: Origin,
        this: dir::GlobalTypeId,
    ) -> CompilerResult<Option<ReceiverShape>> {
        let this = self.shallow_resolve(this)?;
        if let Some(shape) = self.receiver_shape_of(this)? {
            return Ok(Some(shape));
        }

        // evaluate a computation-shaped receiver before deciding
        let this = self.normalize(origin, this)?;
        self.receiver_shape_of(this)
    }

    /// Read the receiver form one resolved `this` parameter type spells.
    pub(in crate::sema) fn receiver_shape_of(
        &mut self,
        this: dir::GlobalTypeId,
    ) -> CompilerResult<Option<ReceiverShape>> {
        match self.ty(this)? {
            dir::Type::This => Ok(Some(ReceiverShape::Whole)),
            dir::Type::Form(form) => match form.form {
                dir::Form::Borrowed(borrow) => {
                    let borrow = self.type_borrow(this.module_id, borrow)?;

                    Ok(Some(ReceiverShape::Borrowed(borrow.access)))
                }
                dir::Form::Owned => Ok(Some(ReceiverShape::Whole)),
                _ => Ok(None),
            },
            dir::Type::Application(application) => {
                // a WithAccess application spells a borrow at its access argument
                if self.language_item(application.symbol)? == Some(dir::LanguageItem::WithAccess) {
                    let access = self
                        .type_ids(this.module_id, application.arguments)?
                        .get(1)
                        .copied();

                    return Ok(access.map(ReceiverShape::Borrowed));
                }

                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Return the receiver mode one `this` parameter type takes, when its form decides it.
    pub(in crate::sema) fn this_parameter_mode(
        &mut self,
        this: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::ReceiverMode>> {
        let this = self.shallow_resolve(this)?;
        match self.ty(this)? {
            dir::Type::Form(form) => match form.form {
                dir::Form::Borrowed(borrow) => {
                    let borrow = self.type_borrow(this.module_id, borrow)?;

                    Ok(self
                        .access_of(borrow.access)?
                        .map(|access| dir::ReceiverMode::Borrowed { access }))
                }
                dir::Form::Owned => Ok(Some(dir::ReceiverMode::Owned)),
                _ => Ok(None),
            },
            dir::Type::This => Ok(Some(dir::ReceiverMode::Owned)),
            _ => Ok(None),
        }
    }

    /// Return positional signature pairs for generic parameter matching.
    pub(in crate::sema) fn signature_match_pairs(
        &self,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<SmallVec<[(dir::GlobalTypeId, dir::GlobalTypeId); 8]>>> {
        // require two signature heads
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

        // require both signatures to agree on construct
        if source_signature.is_construct != target_signature.is_construct {
            return Ok(None);
        }

        // collect the directed pairs the two signatures compare through
        let mut pairs = SmallVec::<[FunctionAssignabilityPair; 8]>::new();

        // compare receiver input contravariantly for function values
        if this_parameter.has_this() {
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
        let source_parameters =
            self.expand_parameters(source.module_id, source_signature.parameters)?;
        let target_parameters =
            self.expand_parameters(target.module_id, target_signature.parameters)?;
        let required = source_parameters
            .iter()
            .filter(|slot| !slot.is_optional && !slot.is_rest)
            .count();
        if target_parameters.len() < required && !target_parameters.iter().any(|slot| slot.is_rest)
        {
            return Ok(None);
        }

        // compare runtime inputs contravariantly, spreading rest parameters in place
        let mut source_index = 0usize;
        for (index, target) in target_parameters.iter().enumerate() {
            let cause = Some(CauseKind::Parameter {
                index: index as u32,
            });

            // spread the target rest over every remaining source slot
            if target.is_rest {
                // bind an open rest binder as one tuple of the remaining source slots
                let rest = self.shallow_resolve(target.ty)?;
                if matches!(self.ty(rest)?, dir::Type::Variable(_)) {
                    let elements = source_parameters[source_index..]
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
                    pairs.push((cause, rest, tuple));
                    break;
                }

                // spread the target rest element over every remaining source slot
                let Some(element) = self.spread_element_type(target.ty)? else {
                    return Ok(None);
                };
                while let Some(source) = source_parameters.get(source_index) {
                    let target = if source.is_rest { target.ty } else { element };
                    let cause = Some(CauseKind::Parameter {
                        index: source_index as u32,
                    });
                    pairs.push((cause, target, source.ty));
                    source_index += 1;
                }
                break;
            }

            // stop at the target inputs past the source parameters
            let Some(source) = source_parameters.get(source_index) else {
                break;
            };

            // spread the source rest element over every remaining target slot
            if source.is_rest {
                let Some(element) = self.spread_element_type(source.ty)? else {
                    return Ok(None);
                };
                pairs.push((cause, target.ty, element));
                continue;
            }

            // pair the written slot
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
    /// Skip `this`, which the method receiver grant compares separately.
    Skip,
}

impl ThisParameterComparison {
    /// Return whether `this` participates in this comparison.
    fn has_this(self) -> bool {
        matches!(self, Self::Compare)
    }
}

/// The receiver form one `this` parameter presents.
pub(in crate::sema) enum ReceiverShape {
    /// A whole receiver, reborrowing at every access.
    Whole,
    /// A borrowed receiver granting its access term.
    Borrowed(dir::GlobalTypeId),
}

/// One expanded parameter position, with rest tuples spread in place.
pub(in crate::sema) struct ExpandedParameter {
    /// The parameter type, the whole container for a rest parameter.
    pub(in crate::sema) ty: dir::GlobalTypeId,
    /// Whether a call may omit the argument.
    pub(in crate::sema) is_optional: bool,
    /// Whether the parameter captures every remaining call argument.
    pub(in crate::sema) is_rest: bool,
}

impl CheckState<'_> {
    /// Expand one parameter list, spreading a rest tuple over its elements.
    pub(in crate::sema) fn expand_parameters(
        &self,
        module: ModuleId,
        parameters: dir::TypeListId,
    ) -> CompilerResult<SmallVec<[ExpandedParameter; 6]>> {
        let mut expanded = SmallVec::new();
        for parameter in self.signature_parameters(module, parameters)? {
            // keep plain parameters in their written position
            if !parameter.is_rest {
                expanded.push(ExpandedParameter {
                    ty: parameter.ty,
                    is_optional: parameter.is_optional,
                    is_rest: false,
                });
                continue;
            }

            // spread a rest tuple over its elements, keeping other rests whole
            let rest = self.shallow_resolve(parameter.ty)?;
            match self.ty(rest)? {
                dir::Type::Tuple(tuple) => {
                    for element in self
                        .tuple_elements(rest.module_id, tuple.elements)?
                        .to_vec()
                    {
                        expanded.push(ExpandedParameter {
                            ty: element.ty,
                            is_optional: element.is_optional,
                            is_rest: element.is_rest,
                        });
                    }
                }
                _ => expanded.push(ExpandedParameter {
                    ty: rest,
                    is_optional: false,
                    is_rest: true,
                }),
            }
        }

        Ok(expanded)
    }
}

/// Return whether one property access stores a field, reading and writing one type.
fn is_field(access: dir::PropertyAccess) -> bool {
    matches!(access, dir::PropertyAccess::ReadWrite { read, write } if read == write)
}

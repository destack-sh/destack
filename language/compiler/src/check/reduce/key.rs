use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Dependency, MemberLookup, Origin, TypeSubstitution, answer,
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
    /// Reduce one member projection to its value type.
    pub(super) fn reduce_static_member_projection(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let space = match self.ty(owner)? {
            dir::Type::Reference(_) => dir::MemberSpace::Static,
            _ => dir::MemberSpace::Instance,
        };
        let lookup = answer!(self.lookup_member(origin, origin.module(), owner, space, key)?);

        match lookup {
            // a field contributes its value type
            MemberLookup::Field(ty) => Ok(Answer::Ready(Some(ty))),

            // a matching member contributes its static value or callable value type
            MemberLookup::Found(candidates) => match candidates.as_slice() {
                [candidate] => {
                    if let Some(written) = candidate.value_type {
                        return Ok(Answer::Ready(Some(written)));
                    }
                    if let Some(value) = candidate.value {
                        let ty = self.intern_type(origin.module(), dir::Type::Static(value))?;

                        return Ok(Answer::Ready(Some(ty)));
                    }

                    Ok(Answer::Ready(Some(candidate.ty)))
                }
                _ => Ok(Answer::Ready(None)),
            },

            // missing members leave the operation symbolic
            MemberLookup::Missing => Ok(Answer::Ready(None)),
        }
    }

    /// Reduce one indexed access type with a closed key.
    pub(super) fn reduce_index(
        &mut self,
        origin: Origin,
        index: &dir::IndexType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let left = answer!(self.reduce_type_head(origin, index.left)?);
        let key = answer!(self.reduce_type_head(origin, index.index)?);

        // union keys distribute their projections
        if let dir::Type::Union(union) = self.ty(key)? {
            let keys =
                SmallVec::<[_; 4]>::from_slice(self.type_ids(key.module_id, union.elements)?);
            let Some(union) =
                answer!(
                    self.reduce_distributed_operation(origin, keys, |state, key| {
                        state.reduce_index(
                            origin,
                            &dir::IndexType {
                                left: index.left,
                                index: key,
                            },
                        )
                    })?
                )
            else {
                return Ok(Answer::Ready(None));
            };

            return Ok(Answer::Ready(Some(union)));
        }

        // project string keys from static declaration references
        if matches!(self.ty(left)?, dir::Type::Reference(_))
            && let dir::Type::Literal(dir::ScalarLiteral::String(name)) = self.ty(key)?
        {
            let key = dir::StaticKey::Name(name);

            return self.reduce_static_member_projection(origin, left, key);
        }

        // project closed structural keys
        let projected = match (self.ty(left)?, self.ty(key)?) {
            (dir::Type::Shape(shape), dir::Type::Literal(dir::ScalarLiteral::String(name))) => {
                let key = dir::StaticKey::Name(name);

                self.shape_fields(left.module_id, shape.fields)?
                    .iter()
                    .find(|field| field.key == key)
                    .map(|field| field.ty)
            }
            (dir::Type::Tuple(tuple), dir::Type::Literal(dir::ScalarLiteral::Integer(value))) => {
                let elements = self.tuple_elements(left.module_id, tuple.elements)?;

                usize::try_from(value)
                    .ok()
                    .and_then(|index| elements.get(index))
                    .map(|element| element.ty)
            }
            (dir::Type::Array(array), dir::Type::Literal(dir::ScalarLiteral::Integer(_))) => {
                Some(array.element)
            }
            _ => None,
        };

        Ok(Answer::Ready(projected))
    }

    /// Reduce keyof over one closed type to a key literal union.
    pub(super) fn reduce_keyof(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let target = answer!(self.reduce_type_head(origin, target)?);

        // collect exact keys and index domains
        let Some(keys) = answer!(self.keyof_set(origin, target)?) else {
            return Ok(Answer::Ready(None));
        };

        // create the key type union
        let module = id.module_id;
        let elements = self.keyof_types(module, keys)?;
        let union = match elements.as_slice() {
            [] => self.intern_type(module, dir::Type::Never)?,
            [single] => *single,
            _ => self.normalized_union_type(module, elements)?,
        };

        Ok(Answer::Ready(Some(union)))
    }

    /// Collect the property-key set of one closed type.
    fn keyof_set(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<KeySet>>> {
        let set = match self.ty(target)? {
            // structural object keys come from fields and index signatures
            dir::Type::Shape(shape) => {
                let mut set = KeySet::default();
                let fields = self.shape_fields(target.module_id, shape.fields)?.to_vec();
                for field in fields {
                    set.insert_key(field.key);
                }
                let index_signatures = self
                    .shape_index_signatures(target.module_id, shape.index_signatures)?
                    .to_vec();
                for signature in index_signatures {
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
                    dir::MemberSpace::Static,
                )?)
            }

            // memory forms preserve the key set of their payload
            dir::Type::Form(form) => {
                let value = answer!(self.reduce_type_head(origin, form.value)?);

                return self.keyof_set(origin, value);
            }

            // union keys are the keys present in every arm
            dir::Type::Union(union) => {
                let mut elements = self
                    .type_ids(target.module_id, union.elements)?
                    .to_vec()
                    .into_iter();
                let Some(first) = elements.next() else {
                    return Ok(Answer::Ready(Some(KeySet::default())));
                };
                let first = answer!(self.reduce_type_head(origin, first)?);
                let Some(mut keys) = answer!(self.keyof_set(origin, first)?) else {
                    return Ok(Answer::Ready(None));
                };
                for element in elements {
                    let element = answer!(self.reduce_type_head(origin, element)?);
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
                let elements = self
                    .type_ids(target.module_id, intersection.elements)?
                    .to_vec();
                for element in elements {
                    let element = answer!(self.reduce_type_head(origin, element)?);
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
                dir::MemberSpace::Instance,
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
        let signature_id = answer!(self.reduce_type_head(origin, signature)?);
        let dir::Type::FunctionSignature(signature) = self.ty(signature_id)? else {
            return Ok(Answer::Ready(()));
        };
        let Some(parameter) = self
            .signature_parameters(signature_id.module_id, signature.parameters)?
            .first()
        else {
            return Ok(Answer::Ready(()));
        };
        let parameter_ty = parameter.ty;

        self.insert_index_key_type(origin, keys, parameter_ty)
    }

    /// Insert the key domain represented by one closed key type.
    fn insert_index_key_type(
        &mut self,
        origin: Origin,
        keys: &mut KeySet,
        key_type: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<()>> {
        let key_type = answer!(self.reduce_type_head(origin, key_type)?);

        match self.ty(key_type)? {
            // union key domains contribute every alternative
            dir::Type::Union(union) => {
                let elements = self.type_ids(key_type.module_id, union.elements)?.to_vec();
                for element in elements {
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
        keys: KeySet,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let mut elements = Vec::with_capacity(keys.keys.len() + keys.domains.len());
        for key in keys.keys {
            elements.push(self.static_key_type(module, key)?);
        }
        for domain in keys.domains {
            elements.push(self.key_domain_type(module, domain)?);
        }

        Ok(elements)
    }

    /// Write one key domain as its primitive type.
    fn key_domain_type(
        &mut self,
        module: ModuleId,
        domain: KeyDomain,
    ) -> CompilerResult<dir::GlobalTypeId> {
        self.intern_type(module, dir::Type::Primitive(domain.primitive_type()))
    }

    /// Return the exact static key represented by one singleton key type.
    pub(in crate::check) fn static_key_from_type(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let key = match self.ty(ty)? {
            dir::Type::Key(key) => key,
            dir::Type::Literal(dir::ScalarLiteral::String(name)) => dir::StaticKey::Name(name),
            dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => {
                let Ok(index) = usize::try_from(value) else {
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
    pub(super) fn reduce_mapped(
        &mut self,
        origin: Origin,
        mapped: &dir::MappedType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let homomorphic = match self.ty(mapped.parameter.constraint)? {
            dir::Type::Operation(dir::TypeOperation::KeyOf(unary)) => Some(unary.target),
            _ => None,
        };

        // close the key source first
        let closed = answer!(self.reduce_type_head(origin, mapped.parameter.constraint)?);
        let closed_type = self.ty(closed)?;
        let keys = match closed_type {
            dir::Type::Union(union) => {
                SmallVec::<[_; 8]>::from_slice(self.type_ids(closed.module_id, union.elements)?)
            }
            dir::Type::Never => SmallVec::new(),
            dir::Type::Literal(_) | dir::Type::Primitive(_) => {
                let mut single = SmallVec::new();
                single.push(closed);

                single
            }
            _ if self.static_key_from_type(closed)?.is_some() => {
                let mut single = SmallVec::new();
                single.push(closed);

                single
            }
            _ => return Ok(Answer::Ready(None)),
        };

        // close the homomorphic source for modifier carry
        let source_shape = match homomorphic {
            Some(target) => {
                let target = answer!(self.reduce_type_head(origin, target)?);

                match self.ty(target)? {
                    dir::Type::Shape(shape) => Some((target.module_id, shape)),
                    _ => None,
                }
            }
            None => None,
        };

        // project each key into one field
        let module = origin.module();
        let mut fields = Vec::with_capacity(keys.len());
        let mut index_signatures = Vec::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for key in keys {
            let mut substitution = TypeSubstitution::default();
            substitution.parameters.push(mapped.parameter.parameter);
            substitution.arguments.push(key);

            let value = self.substitute_type(module, mapped.value, &substitution)?;
            let value = match self.reduce_type_head(origin, value)? {
                Answer::Ready(value) => value,
                Answer::Pending(dependencies) => {
                    blockers.extend(dependencies);

                    continue;
                }
            };

            let remapped = match mapped.parameter.key_remap {
                Some(remap) => {
                    let remap = self.substitute_type(module, remap, &substitution)?;

                    match self.reduce_type_head(origin, remap)? {
                        Answer::Ready(remap) => remap,
                        Answer::Pending(dependencies) => {
                            blockers.extend(dependencies);

                            continue;
                        }
                    }
                }
                None => key,
            };

            let key_field = self.static_key_from_type(key)?;
            let carried = match (&source_shape, key_field) {
                (Some((shape_module, shape)), Some(key)) => self
                    .shape_fields(*shape_module, shape.fields)?
                    .iter()
                    .find(|field| field.key == key)
                    .copied(),
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

        let fields = self.intern_fields(module, &fields)?;
        let index_signatures = self.intern_index_signatures(module, &index_signatures)?;
        let shape = dir::Type::Shape(dir::ShapeType {
            fields,
            call_signatures: dir::TypeListId::EMPTY,
            construct_signatures: dir::TypeListId::EMPTY,
            index_signatures,
        });
        let projected = self.intern_type(module, shape)?;

        Ok(Answer::Ready(Some(projected)))
    }
}

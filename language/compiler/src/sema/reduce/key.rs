use smallvec::SmallVec;
use tspp_core::FxIndexSet;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{CheckState, MemberLookup, Origin, TypeSubstitution};
use crate::{CompilerError, CompilerResult};

/// One broad property-key domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum KeyDomain {
    /// String property names.
    String,
    /// Positional numeric property names.
    Usize,
}

/// One reduced `keyof` result before it is written as a type.
#[derive(Debug, Clone, Default)]
struct KeySet {
    /// The exact known keys.
    keys: FxIndexSet<dir::StaticKey>,
    /// The broad key domains accepted by index signatures.
    domains: FxIndexSet<KeyDomain>,
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

    /// Return whether this set covers one exact key.
    fn has_key(&self, key: dir::StaticKey) -> bool {
        self.keys.contains(&key) || self.domains.contains(&KeyDomain::from_key(key))
    }

    /// Return the intersection of two key sets.
    fn intersect(self, other: Self) -> Self {
        let mut keys = KeySet::default();

        // keep exact left keys the right side covers
        for key in self.keys.iter().copied() {
            if other.has_key(key) {
                keys.insert_key(key);
            }
        }

        // keep exact right keys the left domains cover
        for key in other.keys.iter().copied() {
            if !keys.keys.contains(&key) && self.has_key(key) {
                keys.insert_key(key);
            }
        }

        // keep broad domains both sides cover
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
        }
    }

    /// Return the primitive type representing this key domain.
    fn primitive_type(self) -> dir::PrimitiveType {
        match self {
            Self::String => dir::PrimitiveType::String,
            Self::Usize => {
                dir::PrimitiveType::Integer(dir::IntegerType::Pointer { is_signed: false })
            }
        }
    }
}

/// Outcome of reducing one written type operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::sema) enum OperationReduction {
    /// The operation projected to its value type.
    Projected(dir::GlobalTypeId),
    /// The operation stays symbolic over rigid operands.
    Rigid,
    /// The operation cannot hold on its closed operands.
    Invalid(InvalidOperation),
}

/// Reason one closed type operation is ill-formed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::sema) enum InvalidOperation {
    /// The receiver admits no index.
    IndexReceiver {
        /// The indexed receiver type.
        receiver: dir::GlobalTypeId,
    },
    /// The key projects from no receiver member.
    IndexKey {
        /// The indexed receiver type.
        receiver: dir::GlobalTypeId,
        /// The supplied key type.
        key: dir::GlobalTypeId,
    },
}

impl CheckState<'_> {
    /// Reduce one member projection to its value type.
    pub(super) fn reduce_static_member_projection(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        key: dir::StaticKey,
        key_type: dir::GlobalTypeId,
    ) -> CompilerResult<OperationReduction> {
        // resolve a stuck named head before selecting the member space
        let owner = self.shallow_resolve(owner)?;
        let head = self.structurally_normalize(origin, owner)?;
        let space = match self.ty(head)? {
            dir::Type::Reference(_) | dir::Type::Static(_) => dir::MemberSpace::Static,
            _ => dir::MemberSpace::Instance,
        };
        let subject = self.member_subject(origin, owner, head, space)?;
        let lookup = self.lookup_member(origin, origin.module(), subject, key)?;

        let reduction = self.reduce_member_lookup(owner, key_type, lookup)?;

        Ok(reduction)
    }

    /// Reduce one completed member lookup to its projected value type.
    fn reduce_member_lookup(
        &mut self,
        owner: dir::GlobalTypeId,
        key_type: dir::GlobalTypeId,
        lookup: MemberLookup,
    ) -> CompilerResult<OperationReduction> {
        // missing members reject the key on the closed receiver
        if lookup.is_empty() {
            return Ok(OperationReduction::Invalid(InvalidOperation::IndexKey {
                receiver: owner,
                key: key_type,
            }));
        }

        // project every runtime arm, joining several as one union
        let mut types = Vec::new();
        for group in lookup.arms() {
            // overloaded members stay symbolic
            let [candidate] = group.candidates.as_slice() else {
                return Ok(OperationReduction::Rigid);
            };

            // contribute a static value, a written value type, or the read type
            let declared = candidate.declaration();
            if let Some(written) = declared.and_then(|declared| declared.value_type) {
                types.push(written);
                continue;
            }
            if let Some(value) = declared.and_then(|declared| declared.value) {
                types.push(self.intern_type(dir::Type::Static(value))?);
                continue;
            }
            match (candidate.read_type(self)?, declared) {
                (Some(ty), _) => types.push(ty),
                // write-only properties project nothing readable
                (None, None) => {
                    return Ok(OperationReduction::Invalid(InvalidOperation::IndexKey {
                        receiver: owner,
                        key: key_type,
                    }));
                }
                (None, Some(_)) => return Ok(OperationReduction::Rigid),
            }
        }
        let ty = self.normalized_union_type(types)?;

        Ok(OperationReduction::Projected(ty))
    }

    /// Reduce one indexed access type over its reduced operands.
    pub(in crate::sema) fn reduce_index(
        &mut self,
        origin: Origin,
        index: &dir::IndexType,
    ) -> CompilerResult<OperationReduction> {
        let left = self.normalize(origin, index.left)?;
        let key = self.normalize(origin, index.index)?;

        // poisoned operands project their poison
        if matches!(self.ty(left)?, dir::Type::Error) || matches!(self.ty(key)?, dir::Type::Error) {
            let error = self.intern_type(dir::Type::Error)?;

            return Ok(OperationReduction::Projected(error));
        }

        // union keys distribute their projections
        if let dir::Type::Union(union) = self.ty(key)? {
            let keys =
                SmallVec::<[_; 4]>::from_slice(self.type_ids(key.module_id, union.elements)?);

            return self
                .reduce_distributed_index(origin, keys, |key| dir::IndexType { left, index: key });
        }

        // union bases distribute their projections
        if let dir::Type::Union(union) = self.ty(left)? {
            let bases =
                SmallVec::<[_; 4]>::from_slice(self.type_ids(left.module_id, union.elements)?);

            return self.reduce_distributed_index(origin, bases, |base| dir::IndexType {
                left: base,
                index: key,
            });
        }

        // memory forms project through their payload
        if let dir::Type::Form(form) = self.ty(left)? {
            return self.reduce_index(
                origin,
                &dir::IndexType {
                    left: form.value,
                    index: key,
                },
            );
        }

        // rigid operands keep the operation symbolic
        if self.is_rigid_index_operand(left)? || self.is_rigid_index_operand(key)? {
            return Ok(OperationReduction::Rigid);
        }

        // project static keys from receivers that declare members
        let static_key = self.static_key_from_type(key)?;
        let key_domain = self.index_key_domain(key)?;
        if let Some(static_key) = static_key
            && matches!(
                self.ty(left)?,
                dir::Type::Reference(_)
                    | dir::Type::Application(_)
                    | dir::Type::Refined(_)
                    | dir::Type::Static(_)
            )
        {
            return self.reduce_static_member_projection(origin, left, static_key, key);
        }

        // project closed structural keys
        let projected = match (self.ty(left)?, static_key) {
            (dir::Type::Object(shape), Some(static_key)) => {
                let field = self
                    .object_properties(left.module_id, shape.properties)?
                    .iter()
                    .find(|field| field.key == static_key)
                    .copied();

                // optional fields read as their value or undefined
                match field {
                    Some(field) if field.is_optional => {
                        let undefined = self.intern_type(dir::Type::Undefined)?;
                        let read = field.access.store();

                        Some(self.normalized_union_type(vec![read, undefined])?)
                    }
                    Some(field) => Some(field.access.store()),
                    // keyed index signatures cover missing exact fields
                    None => self.shape_signature_projection(origin, left, &shape, key)?,
                }
            }
            // primitive keys project matching index signatures
            (dir::Type::Object(shape), None) => {
                self.shape_signature_projection(origin, left, &shape, key)?
            }
            (dir::Type::Tuple(tuple), Some(dir::StaticKey::Index(index))) => self
                .tuple_elements(left.module_id, tuple.elements)?
                .get(index)
                .map(|element| element.ty),
            (dir::Type::FixedArray(array), Some(dir::StaticKey::Index(_))) => Some(array.element),
            // integer-domain keys project every positional element
            (dir::Type::Application(_), Some(dir::StaticKey::Index(_)))
                if let Some(element) = self.array_element(left)? =>
            {
                Some(element)
            }
            (dir::Type::Application(_), None)
                if key_domain == Some(KeyDomain::Usize)
                    && let Some(element) = self.array_element(left)? =>
            {
                Some(element)
            }
            (dir::Type::FixedArray(array), None) if key_domain == Some(KeyDomain::Usize) => {
                Some(array.element)
            }
            (dir::Type::Tuple(tuple), None) if key_domain == Some(KeyDomain::Usize) => {
                let elements = self
                    .tuple_elements(left.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect::<Vec<_>>();

                Some(self.normalized_union_type(elements)?)
            }
            // unprojected receivers that declare members stay symbolic
            (
                dir::Type::Application(_)
                | dir::Type::Reference(_)
                | dir::Type::Refined(_)
                | dir::Type::Intersection(_),
                _,
            ) => return Ok(OperationReduction::Rigid),
            // closed non-indexable receivers reject the operation
            (dir::Type::Tuple(_) | dir::Type::FixedArray(_), _) => None,
            _ => {
                return Ok(OperationReduction::Invalid(
                    InvalidOperation::IndexReceiver { receiver: left },
                ));
            }
        };

        // project the key, else report the access invalid
        match projected {
            Some(ty) => Ok(OperationReduction::Projected(ty)),
            None => Ok(OperationReduction::Invalid(InvalidOperation::IndexKey {
                receiver: left,
                key,
            })),
        }
    }

    /// Reduce one indexed access distributed over union operands.
    fn reduce_distributed_index(
        &mut self,
        origin: Origin,
        elements: SmallVec<[dir::GlobalTypeId; 4]>,
        index: impl Fn(dir::GlobalTypeId) -> dir::IndexType,
    ) -> CompilerResult<OperationReduction> {
        let mut projected = Vec::with_capacity(elements.len());

        // project each arm independently
        for element in elements {
            match self.reduce_index(origin, &index(element))? {
                OperationReduction::Projected(ty) => projected.push(ty),
                other => return Ok(other),
            }
        }
        let union = self.normalized_union_type(projected)?;

        Ok(OperationReduction::Projected(union))
    }

    /// Project one key through a shape's index signatures.
    fn shape_signature_projection(
        &mut self,
        origin: Origin,
        shape_id: dir::GlobalTypeId,
        shape: &dir::ObjectType,
        key: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(domain) = self.index_key_domain(key)? else {
            return Ok(None);
        };
        let signatures: SmallVec<[_; 4]> = self
            .object_index_signatures(shape_id.module_id, shape.index_signatures)?
            .into();

        // match the first signature covering the key domain
        for signature in signatures {
            let signature_key = self.normalize(origin, signature.key_type)?;
            let Some(signature_domain) = self.index_key_domain(signature_key)? else {
                continue;
            };

            // string signatures accept numeric property names too
            let covers = signature_domain == domain
                || (signature_domain == KeyDomain::String && domain == KeyDomain::Usize);
            if covers {
                return Ok(Some(signature.value_type));
            }
        }

        Ok(None)
    }

    /// Return the broad key domain of one closed key type.
    fn index_key_domain(&mut self, key: dir::GlobalTypeId) -> CompilerResult<Option<KeyDomain>> {
        if let Some(static_key) = self.static_key_from_type(key)? {
            return Ok(Some(KeyDomain::from_key(static_key)));
        }

        // read the domain the key type names
        let domain = match self.ty(key)? {
            dir::Type::Primitive(dir::PrimitiveType::String) => Some(KeyDomain::String),
            dir::Type::Primitive(dir::PrimitiveType::Integer(_)) => Some(KeyDomain::Usize),
            _ => None,
        };

        Ok(domain)
    }

    /// Return whether one reduced index operand has rigid material.
    fn is_rigid_index_operand(&self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        Ok(matches!(
            self.ty(ty)?,
            dir::Type::Variable(_)
                | dir::Type::Parameter(_)
                | dir::Type::This
                | dir::Type::Member(_)
                | dir::Type::Operation(_)
        ))
    }

    /// Reduce keyof over one closed type to a key literal union.
    pub(super) fn reduce_keyof(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // project the key domain from the operand's reduced shape
        let target = self.normalize(origin, target)?;

        // collect exact keys and index domains
        let Some(keys) = self.keyof_set(origin, target)? else {
            return Ok(None);
        };

        // create the key type union
        let module = id.module_id;
        let elements = self.keyof_types(module, keys)?;
        let union = match elements.as_slice() {
            [] => self.intern_type(dir::Type::Never)?,
            [single] => *single,
            _ => self.normalized_union_type(elements)?,
        };

        Ok(Some(union))
    }

    /// Collect the property-key set of one closed type.
    fn keyof_set(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<KeySet>> {
        // collect the keys by the head of the target
        let set = match self.ty(target)? {
            // structural object keys come from fields and index signatures
            dir::Type::Object(shape) => {
                let mut set = KeySet::default();
                let fields: SmallVec<[_; 4]> = self
                    .object_properties(target.module_id, shape.properties)?
                    .into();
                for field in fields {
                    set.insert_key(field.key);
                }
                let index_signatures: SmallVec<[_; 4]> = self
                    .object_index_signatures(target.module_id, shape.index_signatures)?
                    .into();
                for signature in index_signatures {
                    self.insert_index_key_type(origin, &mut set, signature.key_type)?;
                }

                set
            }

            // arrays key by the index domain
            _ if self.array_element(target)?.is_some() => {
                let mut set = KeySet::default();
                set.insert_domain(KeyDomain::Usize);

                set
            }
            // nominal instance keys follow public instance members through heritage
            dir::Type::Application(instance) => self.instance_keyof_set(origin, instance.symbol)?,

            // declaration references expose static declaration members
            dir::Type::Reference(reference) => {
                self.definition_key_set(origin, reference.symbol, dir::MemberSpace::Static)?
            }

            // memory forms preserve the key set of their payload
            dir::Type::Form(form) => {
                let value = self.normalize(origin, form.value)?;

                return self.keyof_set(origin, value);
            }

            // keep the union keys present in every arm
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(target.module_id, union.elements)?.into();
                let mut elements = elements.into_iter();
                let Some(first) = elements.next() else {
                    return Ok(Some(KeySet::default()));
                };
                let first = self.normalize(origin, first)?;
                let Some(mut keys) = self.keyof_set(origin, first)? else {
                    return Ok(None);
                };
                for element in elements {
                    let element = self.normalize(origin, element)?;
                    let Some(other) = self.keyof_set(origin, element)? else {
                        return Ok(None);
                    };
                    keys = keys.intersect(other);
                }

                keys
            }

            // collect intersection keys from any constituent
            dir::Type::Intersection(intersection) => {
                let mut keys = KeySet::default();
                let elements: SmallVec<[_; 8]> = self
                    .type_ids(target.module_id, intersection.elements)?
                    .into();
                for element in elements {
                    let element = self.normalize(origin, element)?;
                    let Some(other) = self.keyof_set(origin, element)? else {
                        return Ok(None);
                    };
                    keys.extend(other);
                }

                keys
            }

            // key tuples by element index, widening rest tails to the index domain
            dir::Type::Tuple(tuple) => {
                let mut set = KeySet::default();
                let elements: SmallVec<[_; 4]> = self
                    .tuple_elements(target.module_id, tuple.elements)?
                    .into();
                for (index, element) in elements.iter().enumerate() {
                    if element.is_rest {
                        set.insert_domain(KeyDomain::Usize);
                    } else {
                        set.insert_key(dir::StaticKey::Index(index));
                    }
                }

                set
            }

            // slices key by the index domain
            dir::Type::Slice(_) => {
                let mut set = KeySet::default();
                set.insert_domain(KeyDomain::Usize);

                set
            }

            // fixed arrays with a resolved count key by their exact indices
            dir::Type::FixedArray(array) => {
                let count = self.normalize(origin, array.count)?;
                let mut set = KeySet::default();
                match self.ty(count)? {
                    dir::Type::Literal(dir::Literal::Integer(count)) => {
                        for index in 0..count.max(0) as usize {
                            set.insert_key(dir::StaticKey::Index(index));
                        }
                    }
                    _ => set.insert_domain(KeyDomain::Usize),
                }

                set
            }

            // open and non-object types stay symbolic
            dir::Type::Variable(_) | dir::Type::Parameter(_) => return Ok(None),
            _ => return Ok(None),
        };

        Ok(Some(set))
    }

    /// Collect the instance key set of one nominal declaration.
    fn instance_keyof_set(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<KeySet> {
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
            keys.extend(self.definition_key_set(origin, symbol, dir::MemberSpace::Instance)?);

            if let Some(definition) = self.definition(symbol)? {
                let bases = definition
                    .bases()
                    .iter()
                    .map(|heritage| heritage.ty)
                    .collect::<SmallVec<[_; 2]>>();
                for heritage in bases {
                    let (_, base) = self.nominal_application(heritage)?;
                    pending.push(base.symbol);
                }
            }
        }

        Ok(keys)
    }

    /// Collect the direct key set of one declaration member space.
    fn definition_key_set(
        &mut self,
        origin: Origin,
        symbol: dir::GlobalSymbolId,
        space: dir::MemberSpace,
    ) -> CompilerResult<KeySet> {
        let mut keys = KeySet::default();
        let Some(definition) = self.definition(symbol)? else {
            return Ok(keys);
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

            // contribute key domains for index signatures
            if let dir::DefinitionMember::IndexSignature(signature) = member {
                signatures.push(signature.key_type);
            }
        }
        for key_type in signatures {
            self.insert_index_key_type(origin, &mut keys, key_type)?;
        }

        Ok(keys)
    }

    /// Insert the key domain represented by one closed key type.
    fn insert_index_key_type(
        &mut self,
        origin: Origin,
        keys: &mut KeySet,
        key_type: dir::GlobalTypeId,
    ) -> CompilerResult<()> {
        let key_type = self.normalize(origin, key_type)?;

        // collect the keys the domain names
        match self.ty(key_type)? {
            // union key domains contribute every alternative
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(key_type.module_id, union.elements)?.into();
                for element in elements {
                    self.insert_index_key_type(origin, keys, element)?;
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

            // insert literal key domains as exact keys
            _ => {
                if let Some(key) = self.static_key_from_type(key_type)? {
                    keys.insert_key(key);
                }
            }
        }

        Ok(())
    }

    /// Write one key set as concrete type ids.
    fn keyof_types(
        &mut self,
        _module: ModuleId,
        keys: KeySet,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let mut elements = Vec::with_capacity(keys.keys.len() + keys.domains.len());
        for key in keys.keys {
            elements.push(self.static_key_type(key)?);
        }
        for domain in keys.domains {
            elements.push(self.key_domain_type(domain)?);
        }

        Ok(elements)
    }

    /// Write one key domain as its primitive type.
    fn key_domain_type(&mut self, domain: KeyDomain) -> CompilerResult<dir::GlobalTypeId> {
        self.intern_type(dir::Type::Primitive(domain.primitive_type()))
    }

    /// Return the exact static key represented by one singleton key type.
    pub(in crate::sema) fn static_key_from_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let ty = self.shallow_resolve(ty)?;

        // read the exact key the type denotes
        let key = match self.ty(ty)? {
            dir::Type::Key(key) => key,
            // named static values preserve their exact key
            dir::Type::Application(instance) if instance.arguments.is_empty() => {
                return self.symbol_static_key(instance.symbol);
            }
            dir::Type::Literal(dir::Literal::String(name)) => dir::StaticKey::Name(name),
            dir::Type::Literal(dir::Literal::Integer(value)) => {
                let Ok(index) = usize::try_from(value) else {
                    return Ok(None);
                };

                dir::StaticKey::Index(index)
            }
            _ => return Ok(None),
        };

        Ok(Some(key))
    }

    /// Project one mapped type over its closed key source.
    pub(super) fn reduce_mapped(
        &mut self,
        origin: Origin,
        mapped: &dir::MappedType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let modifiers_type = mapped.parameter.modifiers_type.or(
            match self.operation_head(mapped.parameter.constraint)? {
                Some(dir::TypeOperation::KeyOf(unary)) => Some(unary.target),
                _ => None,
            },
        );

        // close the key source first
        let closed = self.normalize(origin, mapped.parameter.constraint)?;
        let closed_type = self.ty(closed)?;
        // read the keys the closed source enumerates
        let keys = match closed_type {
            dir::Type::Union(_) => {
                let leaves = self.union_leaves(origin, closed)?;

                leaves.ok_or_else(|| CompilerError::Internal {
                    message: "a union key source exposed no arms".to_string(),
                })?
            }
            dir::Type::Never => SmallVec::new(),
            dir::Type::Literal(_) | dir::Type::Primitive(_) => SmallVec::from_slice(&[closed]),
            _ if self.static_key_from_type(closed)?.is_some() => SmallVec::from_slice(&[closed]),
            _ => return Ok(None),
        };

        // close the modifier source for the carry
        let source_fields = match modifiers_type {
            Some(target) => {
                let target = self.normalize(origin, target)?;

                match self.ty(target)? {
                    dir::Type::Object(shape) => Some(
                        self.object_properties(target.module_id, shape.properties)?
                            .to_vec(),
                    ),
                    dir::Type::Application(_) => self.interface_instance_fields(target, target)?,
                    _ => None,
                }
            }
            None => None,
        };

        // project declared field types for top-level T[K] values
        let is_identity = match (self.operation_head(mapped.value)?, modifiers_type) {
            (Some(dir::TypeOperation::Index(index)), Some(target)) => {
                (index.left == target || Some(index.left) == mapped.parameter.modifiers_type)
                    && matches!(
                        self.ty(index.index)?,
                        dir::Type::Parameter(parameter) if parameter == mapped.parameter.parameter
                    )
            }
            _ => false,
        };

        // project each key into one field
        let mut fields = Vec::with_capacity(keys.len());
        let mut index_signatures = Vec::new();
        for key in keys {
            let mut substitution = TypeSubstitution::default();
            substitution.bind(mapped.parameter.parameter, key)?;

            let key_field = self.static_key_from_type(key)?;
            let carried = match (&source_fields, key_field) {
                (Some(fields), Some(key)) => fields.iter().find(|field| field.key == key).copied(),
                _ => None,
            };

            // identity projections carry the declared read type
            let value = match (is_identity, carried) {
                (true, Some(field)) => field.access.read().unwrap_or_else(|| field.access.store()),
                _ => {
                    let value = self.substitute_type(mapped.value, &substitution)?;

                    self.normalize(origin, value)?
                }
            };

            let remapped = match mapped.parameter.key_remap {
                Some(remap) => {
                    let remap = self.substitute_type(remap, &substitution)?;

                    self.normalize(origin, remap)?
                }
                None => key,
            };

            let is_optional = match mapped.modifiers.optional {
                dir::MappedTypeModifier::Present | dir::MappedTypeModifier::Add => true,
                dir::MappedTypeModifier::Remove => false,
                dir::MappedTypeModifier::None => carried.is_some_and(|field| field.is_optional),
            };
            let is_readonly = match mapped.modifiers.readonly {
                dir::MappedTypeModifier::Present | dir::MappedTypeModifier::Add => true,
                dir::MappedTypeModifier::Remove => false,
                dir::MappedTypeModifier::None => {
                    carried.is_some_and(|field| !field.access.is_writable())
                }
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
                return Ok(None);
            };
            let access = if is_readonly {
                dir::PropertyAccess::Read(value)
            } else {
                dir::PropertyAccess::ReadWrite {
                    read: value,
                    write: value,
                }
            };
            fields.push(dir::TypeProperty {
                key,
                access,
                is_optional,
            });
        }

        // intern the mapped shape
        let fields = self.intern_properties(&fields)?;
        let index_signatures = self.intern_index_signatures(&index_signatures)?;
        let shape = dir::Type::Object(dir::ObjectType {
            properties: fields,
            call_signatures: dir::TypeListId::EMPTY,
            construct_signatures: dir::TypeListId::EMPTY,
            index_signatures,
        });
        let projected = self.intern_type(shape)?;

        Ok(Some(projected))
    }
}

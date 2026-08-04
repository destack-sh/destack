use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
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

/// Outcome of reducing one written type operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum OperationReduction {
    /// The operation projected to its value type.
    Projected(dir::GlobalTypeId),
    /// The operation stays symbolic over rigid operands.
    Rigid,
    /// The operation cannot hold on its closed operands.
    Invalid(InvalidOperation),
}

/// Reason one closed type operation is ill-formed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum InvalidOperation {
    /// The receiver can never be indexed.
    IndexReceiver {
        /// The indexed receiver type.
        receiver: dir::GlobalTypeId,
    },
    /// The key does not project from the receiver.
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
    ) -> CompilerResult<Answer<OperationReduction>> {
        let space = match self.ty(owner)? {
            dir::Type::Reference(_) => dir::MemberSpace::Static,
            _ => dir::MemberSpace::Instance,
        };
        let lookup =
            answer!(
                self.body()
                    .lookup_member(origin, origin.module(), owner, space, key)?
            );

        let reduction = self.reduce_member_lookup(origin, owner, key_type, lookup)?;

        Ok(Answer::Ready(reduction))
    }

    /// Reduce one completed member lookup to its projected value type.
    fn reduce_member_lookup(
        &mut self,
        origin: Origin,
        owner: dir::GlobalTypeId,
        key_type: dir::GlobalTypeId,
        lookup: MemberLookup,
    ) -> CompilerResult<OperationReduction> {
        match lookup {
            // contribute a field's value type
            MemberLookup::Field(field) => {
                match field.read_type(origin.module(), &mut self.body())? {
                    Some(ty) => Ok(OperationReduction::Projected(ty)),
                    // write-only properties project nothing readable
                    None => Ok(OperationReduction::Invalid(InvalidOperation::IndexKey {
                        receiver: owner,
                        key: key_type,
                    })),
                }
            }

            // contribute a matching member's static value or callable value type
            MemberLookup::Found(candidates) => match candidates.as_slice() {
                [candidate] => {
                    if let Some(written) = candidate.value_type {
                        return Ok(OperationReduction::Projected(written));
                    }
                    if let Some(value) = candidate.value {
                        let ty = self.intern_type(dir::Type::Static(value))?;

                        return Ok(OperationReduction::Projected(ty));
                    }

                    match candidate.read_type(origin.module(), &mut self.body())? {
                        Some(ty) => Ok(OperationReduction::Projected(ty)),
                        None => Ok(OperationReduction::Rigid),
                    }
                }
                // overloaded members stay symbolic
                _ => Ok(OperationReduction::Rigid),
            },

            // union receivers project every runtime arm
            MemberLookup::Union(lookups) => {
                let mut types = Vec::with_capacity(lookups.len());
                for arm in lookups {
                    match self.reduce_member_lookup(origin, arm.receiver, key_type, arm.lookup)? {
                        OperationReduction::Projected(ty) => types.push(ty),
                        OperationReduction::Rigid => return Ok(OperationReduction::Rigid),
                        OperationReduction::Invalid(invalid) => {
                            return Ok(OperationReduction::Invalid(invalid));
                        }
                    }
                }
                let ty = self.normalized_union_type(types)?;

                Ok(OperationReduction::Projected(ty))
            }

            // intersection receivers impose every projected member type
            MemberLookup::Intersection(lookups) => {
                let mut types = Vec::with_capacity(lookups.len());
                for lookup in lookups {
                    match self.reduce_member_lookup(origin, owner, key_type, lookup)? {
                        OperationReduction::Projected(ty) => types.push(ty),
                        OperationReduction::Rigid => return Ok(OperationReduction::Rigid),
                        OperationReduction::Invalid(invalid) => {
                            return Ok(OperationReduction::Invalid(invalid));
                        }
                    }
                }
                let ty = self.normalized_intersection_type(types)?;

                Ok(OperationReduction::Projected(ty))
            }

            // missing members reject the key on the closed receiver
            MemberLookup::Missing => Ok(OperationReduction::Invalid(InvalidOperation::IndexKey {
                receiver: owner,
                key: key_type,
            })),
        }
    }

    /// Reduce one indexed access type over its reduced operands.
    pub(in crate::check) fn reduce_index(
        &mut self,
        origin: Origin,
        index: &dir::IndexType,
    ) -> CompilerResult<Answer<OperationReduction>> {
        let left = answer!(self.reduce_type_head(origin, index.left)?);
        let key = answer!(self.reduce_type_head(origin, index.index)?);

        // poisoned operands project their poison
        if self.ty(left)?.is_error() || self.ty(key)?.is_error() {
            let error = self.intern_type(dir::Type::Error)?;

            return Ok(Answer::Ready(OperationReduction::Projected(error)));
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
            return Ok(Answer::Ready(OperationReduction::Rigid));
        }

        // project static keys from member-bearing receivers
        let static_key = self.static_key_from_type(key)?;
        let key_domain = self.index_key_domain(key)?;
        if let Some(static_key) = static_key
            && matches!(
                self.ty(left)?,
                dir::Type::Reference(_) | dir::Type::Application(_) | dir::Type::Refined(_)
            )
        {
            return self.reduce_static_member_projection(origin, left, static_key, key);
        }

        // project closed structural keys
        let projected = match (self.ty(left)?, static_key) {
            (dir::Type::Shape(shape) | dir::Type::Object(shape), Some(static_key)) => {
                let field = self
                    .shape_properties(left.module_id, shape.properties)?
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
                    None => {
                        answer!(self.shape_signature_projection(origin, left, &shape, key)?)
                    }
                }
            }
            // primitive keys project matching index signatures
            (dir::Type::Shape(shape) | dir::Type::Object(shape), None) => {
                answer!(self.shape_signature_projection(origin, left, &shape, key)?)
            }
            (dir::Type::Tuple(tuple), Some(dir::StaticKey::Index(index))) => self
                .tuple_elements(left.module_id, tuple.elements)?
                .get(index)
                .map(|element| element.ty),
            (dir::Type::Array(array), Some(dir::StaticKey::Index(_))) => Some(array.element),
            (dir::Type::FixedArray(array), Some(dir::StaticKey::Index(_))) => Some(array.element),
            // integer-domain keys project every positional element
            (dir::Type::Array(array), None) if key_domain == Some(KeyDomain::Usize) => {
                Some(array.element)
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
            // unprojected member-bearing receivers stay symbolic
            (
                dir::Type::Application(_)
                | dir::Type::Reference(_)
                | dir::Type::Refined(_)
                | dir::Type::Intersection(_),
                _,
            ) => return Ok(Answer::Ready(OperationReduction::Rigid)),
            // closed non-indexable receivers reject the operation
            (dir::Type::Tuple(_) | dir::Type::Array(_) | dir::Type::FixedArray(_), _) => None,
            _ => {
                return Ok(Answer::Ready(OperationReduction::Invalid(
                    InvalidOperation::IndexReceiver { receiver: left },
                )));
            }
        };

        match projected {
            Some(ty) => Ok(Answer::Ready(OperationReduction::Projected(ty))),
            None => Ok(Answer::Ready(OperationReduction::Invalid(
                InvalidOperation::IndexKey {
                    receiver: left,
                    key,
                },
            ))),
        }
    }

    /// Reduce one indexed access distributed over union operands.
    fn reduce_distributed_index(
        &mut self,
        origin: Origin,
        elements: SmallVec<[dir::GlobalTypeId; 4]>,
        index: impl Fn(dir::GlobalTypeId) -> dir::IndexType,
    ) -> CompilerResult<Answer<OperationReduction>> {
        let mut projected = Vec::with_capacity(elements.len());

        // project each arm independently
        for element in elements {
            match answer!(self.reduce_index(origin, &index(element))?) {
                OperationReduction::Projected(ty) => projected.push(ty),
                other => return Ok(Answer::Ready(other)),
            }
        }
        let union = self.normalized_union_type(projected)?;

        Ok(Answer::Ready(OperationReduction::Projected(union)))
    }

    /// Project one key through a shape's index signatures.
    fn shape_signature_projection(
        &mut self,
        origin: Origin,
        shape_id: dir::GlobalTypeId,
        shape: &dir::ShapeType,
        key: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(domain) = self.index_key_domain(key)? else {
            return Ok(Answer::Ready(None));
        };
        let signatures = self
            .shape_index_signatures(shape_id.module_id, shape.index_signatures)?
            .to_vec();

        // match the first signature covering the key domain
        for signature in signatures {
            let signature_key = answer!(self.reduce_type_head(origin, signature.key_type)?);
            let Some(signature_domain) = self.index_key_domain(signature_key)? else {
                continue;
            };
            // string signatures accept numeric property names too
            let covers = signature_domain == domain
                || (signature_domain == KeyDomain::String && domain == KeyDomain::Usize);
            if covers {
                return Ok(Answer::Ready(Some(signature.value_type)));
            }
        }

        Ok(Answer::Ready(None))
    }

    /// Return the broad key domain of one closed key type.
    fn index_key_domain(&mut self, key: dir::GlobalTypeId) -> CompilerResult<Option<KeyDomain>> {
        if let Some(static_key) = self.static_key_from_type(key)? {
            return Ok(Some(KeyDomain::from_key(static_key)));
        }

        let domain = match self.ty(key)? {
            dir::Type::Primitive(dir::PrimitiveType::String) => Some(KeyDomain::String),
            dir::Type::Primitive(dir::PrimitiveType::Integer(_)) => Some(KeyDomain::Usize),
            dir::Type::Primitive(dir::PrimitiveType::Symbol | dir::PrimitiveType::UniqueSymbol) => {
                Some(KeyDomain::Symbol)
            }
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
            [] => self.intern_type(dir::Type::Never)?,
            [single] => *single,
            _ => self.normalized_union_type(elements)?,
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
            dir::Type::Shape(shape) | dir::Type::Object(shape) => {
                let mut set = KeySet::default();
                let fields = self
                    .shape_properties(target.module_id, shape.properties)?
                    .to_vec();
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
            dir::Type::Application(instance) => {
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

            // tuple keys are the element indices, rest tails widen to the index domain
            dir::Type::Tuple(tuple) => {
                let mut set = KeySet::default();
                let elements = self
                    .tuple_elements(target.module_id, tuple.elements)?
                    .to_vec();
                for (index, element) in elements.iter().enumerate() {
                    if element.is_rest {
                        set.insert_domain(KeyDomain::Usize);
                    } else {
                        set.insert_key(dir::StaticKey::Index(index));
                    }
                }

                set
            }

            // arrays and slices key by the index domain
            dir::Type::Array(_) | dir::Type::Slice(_) => {
                let mut set = KeySet::default();
                set.insert_domain(KeyDomain::Usize);

                set
            }

            // fixed arrays with a settled count key by their exact indices
            dir::Type::FixedArray(array) => {
                let count = answer!(self.reduce_type_head(origin, array.count)?);
                let mut set = KeySet::default();
                match self.ty(count)? {
                    dir::Type::Literal(dir::ScalarLiteral::Integer(count)) => {
                        for index in 0..count.max(0) as usize {
                            set.insert_key(dir::StaticKey::Index(index));
                        }
                    }
                    _ => set.insert_domain(KeyDomain::Usize),
                }

                set
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
        let Some(definition) = self.definition(symbol)? else {
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
                signatures.push(signature.key_type);
            }
        }
        for key_type in signatures {
            answer!(self.insert_index_key_type(origin, &mut keys, key_type)?);
        }

        Ok(Answer::Ready(keys))
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
    pub(in crate::check) fn static_key_from_type(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::StaticKey>> {
        let key = match self.ty(ty)? {
            dir::Type::Key(key) => key,
            // unique symbol references key by their declaration identity
            dir::Type::Application(instance) if instance.arguments.is_empty() => {
                return self.symbol_static_key(instance.symbol);
            }
            dir::Type::Literal(dir::ScalarLiteral::String(name)) => dir::StaticKey::Name(name),
            dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => {
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
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let modifiers_type = mapped.parameter.modifiers_type.or(
            match self.operation_head(mapped.parameter.constraint)? {
                Some(dir::TypeOperation::KeyOf(unary)) => Some(unary.target),
                _ => None,
            },
        );

        // close the key source first
        let closed = answer!(self.reduce_type_head(origin, mapped.parameter.constraint)?);
        let closed_type = self.ty(closed)?;
        let keys = match closed_type {
            dir::Type::Union(union) => {
                SmallVec::<[_; 8]>::from_slice(self.type_ids(closed.module_id, union.elements)?)
            }
            dir::Type::Never => SmallVec::new(),
            dir::Type::Literal(_) | dir::Type::Primitive(_) => SmallVec::from_slice(&[closed]),
            _ if self.static_key_from_type(closed)?.is_some() => SmallVec::from_slice(&[closed]),
            _ => return Ok(Answer::Ready(None)),
        };

        // close the modifier source for the carry
        let source_fields = match modifiers_type {
            Some(target) => {
                let target = answer!(self.reduce_type_head(origin, target)?);

                match self.ty(target)? {
                    dir::Type::Shape(shape) | dir::Type::Object(shape) => Some(
                        self.shape_properties(target.module_id, shape.properties)?
                            .to_vec(),
                    ),
                    dir::Type::Application(_) => {
                        answer!(self.interface_instance_fields(origin, target, target)?)
                    }
                    _ => None,
                }
            }
            None => None,
        };

        // top-level T[K] values project declared field types, not reads
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
        let module = origin.module();
        let mut fields = Vec::with_capacity(keys.len());
        let mut index_signatures = Vec::new();
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
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
                    match self.reduce_type_head(origin, value)? {
                        Answer::Ready(value) => value,
                        Answer::Pending(dependencies) => {
                            blockers.extend(dependencies);

                            continue;
                        }
                    }
                }
            };

            let remapped = match mapped.parameter.key_remap {
                Some(remap) => {
                    let remap = self.substitute_type(remap, &substitution)?;

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
                return Ok(Answer::Ready(None));
            };
            let access = match is_readonly {
                true => dir::PropertyAccess::Read(value),
                false => dir::PropertyAccess::ReadWrite {
                    read: value,
                    write: value,
                },
            };
            fields.push(dir::TypeProperty {
                key,
                access,
                is_optional,
            });
        }
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        let fields = self.intern_properties(module, &fields)?;
        let index_signatures = self.intern_index_signatures(module, &index_signatures)?;
        let shape = dir::Type::from(dir::ShapeType {
            properties: fields,
            call_signatures: dir::TypeListId::EMPTY,
            construct_signatures: dir::TypeListId::EMPTY,
            index_signatures,
        });
        let projected = self.intern_type(shape)?;

        Ok(Answer::Ready(Some(projected)))
    }
}

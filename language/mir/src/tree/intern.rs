use std::hash::Hash;

use destack_core::{StringId, stable_hash_value};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::rewrite::Substitution;
use crate::{
    Attribute, Field, GenericParameter, Lifetime, LifetimeParameter, LocalNodeId,
    SignatureParameter, Symbol, Tree, Type, TypeDeclaration, TypeHeritage, TypeId, VariantCase,
};

/// One stored MIR type.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub(crate) enum TypeEntry {
    /// A structural type interned by equality.
    Structural {
        /// The type.
        ty: Type,
        /// The definition one application denotes.
        representation: Option<TypeId>,
    },
    /// An identified type reserved for its recursive definition, or declared without one.
    Reserved {
        /// The persistent identity of the type.
        symbol: Symbol,
        /// The source declaration of the type when present.
        declaration: Option<LocalNodeId<TypeDeclaration>>,
    },
    /// A completely defined identified type.
    Identified {
        /// The MIR type.
        ty: Type,
        /// The persistent identity of the type.
        symbol: Symbol,
        /// The source declaration of the type when present.
        declaration: Option<LocalNodeId<TypeDeclaration>>,
    },
}

/// One canonical MIR type index key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub(crate) enum TypeIndexKey {
    /// The structural lookup hash of an anonymous type.
    Structural(u64),
    /// The persistent symbol of an identified type.
    Identified(Symbol),
}

impl Tree {
    /// Find one type equal by structure.
    pub fn find_type(&self, ty: &Type) -> Option<TypeId> {
        let hash = Self::intern_hash(ty);
        let key = TypeIndexKey::Structural(hash);
        let ids = self.type_index.get(&key)?;

        ids.iter().copied().find(|id| self.get(*id) == ty)
    }

    /// Intern one type by structure.
    pub fn intern_type(&mut self, ty: Type) -> TypeId {
        // reuse an equal structural type
        let hash = Self::intern_hash(&ty);
        let key = TypeIndexKey::Structural(hash);
        if let Some(ids) = self.type_index.get(&key) {
            for id in ids {
                if self.get(*id) == &ty {
                    return *id;
                }
            }
        }

        // allocate and index one new structural type
        let local_id = self.types.allocate(TypeEntry::Structural {
            ty,
            representation: None,
        });
        let id = self.insert_node(local_id);
        self.type_index.entry(key).or_default().push(id);
        self.represent(id);

        id
    }

    /// Represent one application, deferring it until its base defines.
    fn represent(&mut self, id: TypeId) {
        // represent only an application with generic arguments
        let Type::Application {
            base,
            arguments,
            lifetimes,
        } = self.get(id).clone()
        else {
            return;
        };
        if arguments.is_empty() {
            return;
        }

        // wait for the base definition
        if !self.is_defined_type(base) {
            return;
        }

        // intern the base definition at the arguments
        let representation = Substitution::new(self, &arguments).representation(base, &lifetimes);
        let local_id = self.node_local_id(id.id);
        let TypeEntry::Structural {
            representation: slot,
            ..
        } = self.types.get_mut(local_id)
        else {
            unreachable!("an application interned outside a structural entry");
        };
        *slot = Some(representation);
    }

    /// Return the definition one application denotes.
    pub fn representation(&self, ty: TypeId) -> Option<TypeId> {
        let local_id = self.node_local_id(ty.id);
        match self.types.get(local_id) {
            TypeEntry::Structural { representation, .. } => *representation,
            _ => None,
        }
    }

    /// Return the definition one type denotes, an application through its representation.
    pub fn represented(&self, ty: TypeId) -> TypeId {
        self.representation(ty).unwrap_or(ty)
    }

    /// Reserve one identified type for a recursive definition.
    pub fn reserve_type(&mut self, symbol: Symbol) -> TypeId {
        // reuse the declaration already carrying this persistent identity
        let key = TypeIndexKey::Identified(symbol);
        if let Some(id) = self.type_index.get(&key).and_then(|ids| ids.first()) {
            return *id;
        }

        // allocate and index one new declaration placeholder
        let local_id = self.types.allocate(TypeEntry::Reserved {
            symbol,
            declaration: None,
        });
        let id = self.insert_node(local_id);
        self.type_index.entry(key).or_default().push(id);

        id
    }

    /// Return the type interned under one canonical cycle key, when one exists.
    pub fn canonical_type(&self, key: &str) -> Option<TypeId> {
        self.canonical_index.get(key).copied()
    }

    /// Register one type under its canonical cycle key, keeping the first.
    pub fn register_canonical(&mut self, key: String, id: TypeId) {
        self.canonical_index.entry(key).or_insert(id);
    }

    /// Return the identified type declared under one symbol, when one exists.
    pub fn identified_type(&self, symbol: Symbol) -> Option<TypeId> {
        let key = TypeIndexKey::Identified(symbol);

        self.type_index
            .get(&key)
            .and_then(|ids| ids.first())
            .copied()
    }

    /// Return whether one identified type still awaits its definition.
    pub fn type_is_reserved(&self, id: TypeId) -> bool {
        let local_id = self.node_local_id(id.id);

        matches!(self.types.get(local_id), TypeEntry::Reserved { .. })
    }

    /// Define one reserved identified type exactly once.
    pub fn define_type(&mut self, id: TypeId, ty: Type) {
        // reject a definition naming itself as a direct child
        let mut is_direct = false;
        let mut children = ty.clone();
        children.map_child_type_ids(&mut |child| {
            is_direct |= child == id;
            child
        });
        assert!(
            !is_direct,
            "MIR type {id:?} defined as its own direct child: {ty:?}"
        );

        let local_id = self.node_local_id(id.id);
        let entry = self.types.get_mut(local_id);
        let TypeEntry::Reserved {
            symbol,
            declaration,
        } = entry
        else {
            panic!("defined MIR type {id:?} twice or without reserving it");
        };
        let structural = TypeIndexKey::Structural(Self::intern_hash(&ty));
        *entry = TypeEntry::Identified {
            ty,
            symbol: *symbol,
            declaration: *declaration,
        };

        // index the defined content structurally, so an equal unrolling reuses it
        self.type_index.entry(structural).or_default().push(id);

        // represent the applications interned over this definition
        let applications: Vec<TypeId> = self
            .iter_nodes::<Type>()
            .filter(|(applied, ty)| {
                matches!(ty, Type::Application { base, arguments, .. } if *base == id && !arguments.is_empty())
                    && self.representation(TypeId::from(*applied)).is_none()
            })
            .map(|(applied, _)| TypeId::from(applied))
            .collect();
        for application in applications {
            self.represent(application);
        }
    }

    /// Insert one declaration for an identified type.
    pub fn insert_type_declaration(
        &mut self,
        name: StringId,
        generics: Vec<GenericParameter>,
        lifetimes: Vec<LifetimeParameter>,
        ty: TypeId,
        heritage: TypeHeritage,
    ) -> LocalNodeId<TypeDeclaration> {
        // reject structural types and repeated declarations
        let type_local_id = self.node_local_id(ty.id);
        let (TypeEntry::Identified { declaration, .. } | TypeEntry::Reserved { declaration, .. }) =
            self.types.get(type_local_id)
        else {
            panic!("declared structural MIR type {ty:?}");
        };
        if declaration.is_some() {
            panic!("declared MIR type {ty:?} twice");
        }

        // allocate through the only declaration construction path
        let declaration = TypeDeclaration {
            name,
            generics,
            lifetimes,
            ty,
            heritage,
        };
        let declaration_local_id = self.type_declarations.allocate(declaration);
        let id = self.insert_node(declaration_local_id);

        // attach the declaration to its identified type
        let (TypeEntry::Identified { declaration, .. } | TypeEntry::Reserved { declaration, .. }) =
            self.types.get_mut(type_local_id)
        else {
            unreachable!("identified MIR type changed during declaration insertion");
        };
        *declaration = Some(id);

        id
    }

    /// Return the declaration of one identified type when present.
    pub fn type_declaration(&self, ty: TypeId) -> Option<LocalNodeId<TypeDeclaration>> {
        let local_id = self.node_local_id(ty.id);
        let (TypeEntry::Identified { declaration, .. } | TypeEntry::Reserved { declaration, .. }) =
            self.types.get(local_id)
        else {
            return None;
        };

        *declaration
    }

    /// Return one identified type's direct heritage when declared.
    pub fn type_heritage(&self, ty: TypeId) -> Option<&TypeHeritage> {
        let declaration = self.type_declaration(ty)?;

        Some(&self.get(declaration).heritage)
    }

    /// Return whether one identified type is defined.
    pub fn is_defined_type(&self, id: TypeId) -> bool {
        let local_id = self.node_local_id(id.id);

        matches!(self.types.get(local_id), TypeEntry::Identified { .. })
    }

    /// Return whether one type has stable nominal identity.
    pub fn is_identified_type(&self, id: TypeId) -> bool {
        let local_id = self.node_local_id(id.id);

        matches!(
            self.types.get(local_id),
            TypeEntry::Reserved { .. } | TypeEntry::Identified { .. }
        )
    }

    /// Return the persistent symbol of one identified type.
    pub fn type_symbol(&self, id: TypeId) -> Option<Symbol> {
        let local_id = self.node_local_id(id.id);

        match self.types.get(local_id) {
            TypeEntry::Reserved { symbol, .. } | TypeEntry::Identified { symbol, .. } => {
                Some(*symbol)
            }
            TypeEntry::Structural { .. } => None,
        }
    }

    /// Intern one field and its attributes.
    pub fn intern_field(&mut self, field: Field, attributes: Vec<Attribute>) -> LocalNodeId<Field> {
        // reuse an equal field declaration
        let hash = Self::intern_hash(&(&field, &attributes));
        if let Some(ids) = self.field_index.get(&hash) {
            for id in ids {
                if self.get(*id) == &field && self.attributes(*id) == attributes {
                    return *id;
                }
            }
        }

        // allocate and index one new field declaration
        let local_id = self.fields.allocate(field);
        let id = self.insert_node(local_id);
        self.field_index.entry(hash).or_default().push(id);
        if !attributes.is_empty() {
            self.attributes_by_node_id.insert(id.id, attributes);
        }

        id
    }

    /// Intern the runtime representation of one semantic MIR type.
    pub fn intern_representation(&mut self, id: TypeId) -> TypeId {
        // preserve nominal identity and terminate recursive definitions
        if self.is_identified_type(id) {
            return id;
        }

        let ty = self.get(id).clone();
        let representation = match ty {
            // leaves already carry their complete runtime representation
            Type::Error
            | Type::Never
            | Type::Void
            | Type::Boolean
            | Type::Character
            | Type::Int { .. }
            | Type::Isize
            | Type::Usize
            | Type::Float(_)
            | Type::TypeDescriptor
            | Type::TypeId
            | Type::Parameter { .. } => return id,

            // normalize transparent and storage wrappers
            Type::Atomic { value } => Type::Atomic {
                value: self.intern_representation(value),
            },
            Type::Dynamic {
                kind,
                lifetime: _,
                constraint,
                storage,
                access,
            } => Type::Dynamic {
                kind,
                lifetime: Lifetime::empty(),
                constraint: self.intern_representation(constraint),
                storage,
                access,
            },
            Type::Uninit { value } => Type::Uninit {
                value: self.intern_representation(value),
            },
            Type::ManuallyDrop { value } => Type::ManuallyDrop {
                value: self.intern_representation(value),
            },

            // erase borrowed origin while retaining reference representation
            Type::Reference {
                kind,
                lifetime: _,
                storage,
                access,
                pointee,
            } => Type::Reference {
                kind,
                lifetime: Default::default(),
                storage,
                access,
                pointee: self.intern_representation(pointee),
            },
            Type::Pointer { pointee, access } => Type::Pointer {
                pointee: self.intern_representation(pointee),
                access,
            },
            Type::Slice {
                kind,
                lifetime: _,
                element,
                storage,
                access,
            } => Type::Slice {
                kind,
                lifetime: Default::default(),
                element: self.intern_representation(element),
                storage,
                access,
            },
            // normalize aggregate children
            Type::FixedArray { element, length } => Type::FixedArray {
                element: self.intern_representation(element),
                length,
            },
            Type::Tuple { elements } => Type::Tuple {
                elements: elements
                    .into_iter()
                    .map(|element| self.intern_representation(element))
                    .collect(),
            },
            Type::Struct { fields, copy } => Type::Struct {
                fields: fields
                    .into_iter()
                    .map(|field| self.intern_field_representation(field))
                    .collect(),
                copy,
            },
            Type::Newtype { inner, copy } => Type::Newtype {
                inner: self.intern_representation(inner),
                copy,
            },
            Type::Variant {
                discriminant,
                cases,
                copy,
            } => Type::Variant {
                discriminant: self.intern_representation(discriminant),
                cases: cases
                    .into_iter()
                    .map(|case| VariantCase {
                        discriminant: case.discriminant,
                        ty: self.intern_representation(case.ty),
                    })
                    .collect(),
                copy,
            },
            Type::Vector { element, lanes } => Type::Vector {
                element: self.intern_representation(element),
                lanes,
            },
            // erase lifetime binders
            Type::FunctionSignature {
                lifetimes: _,
                parameters,
                result,
            } => Type::FunctionSignature {
                lifetimes: Vec::new(),
                parameters: parameters
                    .into_iter()
                    .map(|parameter| SignatureParameter {
                        ty: self.intern_representation(parameter.ty),
                    })
                    .collect(),
                result: self.intern_representation(result),
            },
            Type::Function {
                multiplicity,
                kind,
                lifetime: _,
                signature,
                storage,
                access,
            } => Type::Function {
                multiplicity,
                kind,
                lifetime: Lifetime::empty(),
                signature: self.intern_representation(signature),
                storage,
                access,
            },
            Type::FunctionPointer { signature } => Type::FunctionPointer {
                signature: self.intern_representation(signature),
            },

            // erase pure lifetime applications
            Type::Application { base, .. } => return self.intern_representation(base),
        };

        self.intern_type(representation)
    }

    /// Instantiate type-local lifetime slots with one complete argument list.
    pub fn instantiate_type_lifetimes(&mut self, id: TypeId, arguments: &[Lifetime]) -> TypeId {
        // identified types terminate recursive definitions and carry applications explicitly
        if arguments.is_empty() || self.is_identified_type(id) {
            return id;
        }

        let ty = self.get(id).clone();
        let instantiated = match ty {
            // leaves cannot contain lifetime slots
            Type::Error
            | Type::Never
            | Type::Void
            | Type::Boolean
            | Type::Character
            | Type::Int { .. }
            | Type::Isize
            | Type::Usize
            | Type::Float(_)
            | Type::TypeDescriptor
            | Type::TypeId
            | Type::Parameter { .. } => return id,

            // instantiate transparent and storage wrappers
            Type::Atomic { value } => Type::Atomic {
                value: self.instantiate_type_lifetimes(value, arguments),
            },
            Type::Dynamic {
                kind,
                lifetime,
                constraint,
                storage,
                access,
            } => Type::Dynamic {
                kind,
                lifetime: self.substitute_lifetime(&lifetime, arguments),
                constraint: self.instantiate_type_lifetimes(constraint, arguments),
                storage,
                access,
            },
            Type::Uninit { value } => Type::Uninit {
                value: self.instantiate_type_lifetimes(value, arguments),
            },
            Type::ManuallyDrop { value } => Type::ManuallyDrop {
                value: self.instantiate_type_lifetimes(value, arguments),
            },

            // instantiate borrowed origin and nested value types
            Type::Reference {
                kind,
                lifetime,
                storage,
                access,
                pointee,
            } => Type::Reference {
                kind,
                lifetime: self.substitute_lifetime(&lifetime, arguments),
                storage,
                access,
                pointee: self.instantiate_type_lifetimes(pointee, arguments),
            },
            Type::Pointer { pointee, access } => Type::Pointer {
                pointee: self.instantiate_type_lifetimes(pointee, arguments),
                access,
            },
            Type::Slice {
                kind,
                lifetime,
                element,
                storage,
                access,
            } => Type::Slice {
                kind,
                lifetime: self.substitute_lifetime(&lifetime, arguments),
                element: self.instantiate_type_lifetimes(element, arguments),
                storage,
                access,
            },
            // instantiate aggregate contents
            Type::FixedArray { element, length } => Type::FixedArray {
                element: self.instantiate_type_lifetimes(element, arguments),
                length,
            },
            Type::Tuple { elements } => Type::Tuple {
                elements: elements
                    .into_iter()
                    .map(|element| self.instantiate_type_lifetimes(element, arguments))
                    .collect(),
            },
            Type::Struct { fields, copy } => {
                let mut instantiated = Vec::with_capacity(fields.len());
                for field_id in fields {
                    let field = self.get(field_id).clone();
                    let attributes = self.attributes(field_id).to_vec();
                    let field = Field {
                        name: field.name,
                        ty: self.instantiate_type_lifetimes(field.ty, arguments),
                    };
                    instantiated.push(self.intern_field(field, attributes));
                }

                Type::Struct {
                    fields: instantiated,
                    copy,
                }
            }
            Type::Newtype { inner, copy } => Type::Newtype {
                inner: self.instantiate_type_lifetimes(inner, arguments),
                copy,
            },
            Type::Variant {
                discriminant,
                cases,
                copy,
            } => Type::Variant {
                discriminant: self.instantiate_type_lifetimes(discriminant, arguments),
                cases: cases
                    .into_iter()
                    .map(|case| VariantCase {
                        discriminant: case.discriminant,
                        ty: self.instantiate_type_lifetimes(case.ty, arguments),
                    })
                    .collect(),
                copy,
            },
            Type::Vector { element, lanes } => Type::Vector {
                element: self.instantiate_type_lifetimes(element, arguments),
                lanes,
            },
            // preserve signature-local binders, otherwise instantiate captured lifetimes
            Type::FunctionSignature { lifetimes, .. } if !lifetimes.is_empty() => return id,
            Type::FunctionSignature {
                lifetimes,
                parameters,
                result,
            } => Type::FunctionSignature {
                lifetimes,
                parameters: parameters
                    .into_iter()
                    .map(|parameter| SignatureParameter {
                        ty: self.instantiate_type_lifetimes(parameter.ty, arguments),
                    })
                    .collect(),
                result: self.instantiate_type_lifetimes(result, arguments),
            },
            Type::Function {
                multiplicity,
                kind,
                lifetime,
                signature,
                storage,
                access,
            } => Type::Function {
                multiplicity,
                kind,
                lifetime: self.substitute_lifetime(&lifetime, arguments),
                signature: self.instantiate_type_lifetimes(signature, arguments),
                storage,
                access,
            },
            Type::FunctionPointer { signature } => Type::FunctionPointer {
                signature: self.instantiate_type_lifetimes(signature, arguments),
            },

            // instantiate explicit applications without entering their identified base
            Type::Application {
                base,
                arguments: type_arguments,
                lifetimes,
            } => Type::Application {
                base,
                arguments: type_arguments,
                lifetimes: lifetimes
                    .iter()
                    .map(|lifetime| self.substitute_lifetime(lifetime, arguments))
                    .collect(),
            },
        };

        self.intern_type(instantiated)
    }

    /// Intern one field after normalizing its value representation.
    fn intern_field_representation(&mut self, id: LocalNodeId<Field>) -> LocalNodeId<Field> {
        let field = self.get(id).clone();
        let attributes = self.attributes(id).to_vec();
        let field = Field {
            name: field.name,
            ty: self.intern_representation(field.ty),
        };

        self.intern_field(field, attributes)
    }

    /// Compute one in-memory interning hash.
    pub(crate) fn intern_hash(value: &impl Hash) -> u64 {
        stable_hash_value(value)
    }
}

#[cfg(test)]
mod tests {
    use destack_core::StringId;

    use crate::{Access, Copy, Field, Lifetime, ReferenceKind, Space, Storage, Symbol, Tree, Type};

    /// Equal structural types and fields have one canonical identity.
    #[test]
    fn test_intern_structural_types() {
        let mut tree = Tree::new();
        let first_int = tree.intern_type(Type::INT32);
        let second_int = tree.intern_type(Type::INT32);
        let first_field = tree.intern_field(
            Field {
                name: None,
                ty: first_int,
            },
            Vec::new(),
        );
        let second_field = tree.intern_field(
            Field {
                name: None,
                ty: second_int,
            },
            Vec::new(),
        );
        let first_struct = tree.intern_type(Type::Struct {
            fields: vec![first_field],
            copy: Copy::Yes,
        });
        let second_struct = tree.intern_type(Type::Struct {
            fields: vec![second_field],
            copy: Copy::Yes,
        });

        assert_eq!(first_int, second_int);
        assert_eq!(first_field, second_field);
        assert_eq!(first_struct, second_struct);
    }

    /// Identified recursive types retain distinct nominal identities.
    #[test]
    fn test_define_recursive_identified_types() {
        let mut tree = Tree::new();
        let first = tree.reserve_type(Symbol::named(StringId::for_text("First")));
        let second = tree.reserve_type(Symbol::named(StringId::for_text("Second")));
        let first_field = tree.intern_field(
            Field {
                name: None,
                ty: first,
            },
            Vec::new(),
        );
        let second_field = tree.intern_field(
            Field {
                name: None,
                ty: second,
            },
            Vec::new(),
        );
        tree.define_type(
            first,
            Type::Struct {
                fields: vec![first_field],
                copy: Copy::No,
            },
        );
        tree.define_type(
            second,
            Type::Struct {
                fields: vec![second_field],
                copy: Copy::No,
            },
        );

        assert_ne!(first, second);
        assert!(tree.is_identified_type(first));
        assert!(tree.is_identified_type(second));
    }

    /// Identified types define exactly once.
    #[test]
    #[should_panic(expected = "defined MIR type")]
    fn test_reject_second_type_definition() {
        let mut tree = Tree::new();
        let id = tree.reserve_type(Symbol::named(StringId::for_text("Nominal")));
        tree.define_type(id, Type::Void);
        tree.define_type(id, Type::Void);
    }

    /// Lifetime variants share one canonical runtime representation.
    #[test]
    fn test_intern_lifetime_erased_representations() {
        let mut tree = Tree::new();
        let pointee = tree.intern_type(Type::INT32);
        let local = tree.intern_type(Type::Reference {
            kind: ReferenceKind::Borrowed,
            lifetime: Lifetime::slot(0),
            storage: Storage::Heap(Space::Local),
            access: Access::Readonly,
            pointee,
        });
        let static_ = tree.intern_type(Type::Reference {
            kind: ReferenceKind::Borrowed,
            lifetime: Lifetime::static_storage(),
            storage: Storage::Heap(Space::Local),
            access: Access::Readonly,
            pointee,
        });

        assert_ne!(local, static_);
        assert_eq!(
            tree.intern_representation(local),
            tree.intern_representation(static_)
        );
    }

    /// Lifetime applications preserve identified runtime representations.
    #[test]
    fn test_intern_identified_lifetime_representations() {
        let mut tree = Tree::new();
        let nominal = tree.reserve_type(Symbol::named(StringId::for_text("Nominal")));
        tree.define_type(nominal, Type::Void);
        let local = tree.intern_type(Type::Application {
            base: nominal,
            arguments: Vec::new(),
            lifetimes: vec![Lifetime::slot(0)],
        });
        let static_ = tree.intern_type(Type::Application {
            base: nominal,
            arguments: Vec::new(),
            lifetimes: vec![Lifetime::static_storage()],
        });

        assert_ne!(local, static_);
        assert_eq!(tree.intern_representation(local), nominal);
        assert_eq!(tree.intern_representation(static_), nominal);
    }

    /// Serialization preserves canonical structural and identified type ids.
    #[test]
    fn test_preserve_type_identity_through_serialization() {
        let mut tree = Tree::new();
        let int32 = tree.intern_type(Type::INT32);
        let field = tree.intern_field(
            Field {
                name: None,
                ty: int32,
            },
            Vec::new(),
        );
        let structure = tree.intern_type(Type::Struct {
            fields: vec![field],
            copy: Copy::Yes,
        });
        let symbol = Symbol::named(StringId::for_text("Nominal"));
        let nominal = tree.reserve_type(symbol);
        tree.define_type(nominal, Type::Void);
        let node_count = tree.node_count();

        let bytes = destack_serde::to_vec(&tree).unwrap();
        let mut tree: Tree = destack_serde::from_slice(&bytes).unwrap();

        assert_eq!(tree.intern_type(Type::INT32), int32);
        assert_eq!(
            tree.intern_field(
                Field {
                    name: None,
                    ty: int32,
                },
                Vec::new(),
            ),
            field
        );
        assert_eq!(
            tree.intern_type(Type::Struct {
                fields: vec![field],
                copy: Copy::Yes,
            }),
            structure
        );
        assert_eq!(tree.reserve_type(symbol), nominal);
        assert_eq!(tree.node_count(), node_count);
        assert_eq!(tree.type_symbol(nominal), Some(symbol));
    }
}

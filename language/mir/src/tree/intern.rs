use destack_core::StringId;

use crate::{
    Attribute, BorrowObligation, Field, Lifetime, LifetimeParameter, LocalNodeId,
    SignatureParameter, Symbol, Tree, Type, TypeDeclaration, TypeEntry, TypeId, TypeIndexKey,
    VariantCase,
};

use super::mir_hash;

impl Tree {
    /// Intern one structural type.
    pub fn intern_type(&mut self, ty: Type) -> TypeId {
        // reuse an equal structural type
        let hash = mir_hash(&ty);
        let key = TypeIndexKey::Structural(hash);
        if let Some(ids) = self.type_index.get(&key) {
            for id in ids {
                if self.get(*id) == &ty {
                    return *id;
                }
            }
        }

        // allocate and index one new structural type
        let local_id = self.types.allocate(TypeEntry::Structural { ty });
        let id = self.insert_node(local_id);
        self.type_index.entry(key).or_default().push(id);

        id
    }

    /// Reserve one identified type for a recursive definition.
    pub fn reserve_type(&mut self, symbol: Symbol) -> TypeId {
        // reuse the declaration already carrying this persistent identity
        let key = TypeIndexKey::Identified(symbol);
        if let Some(id) = self.type_index.get(&key).and_then(|ids| ids.first()) {
            return *id;
        }

        // allocate and index one new declaration placeholder
        let local_id = self.types.allocate(TypeEntry::Reserved { symbol });
        let id = self.insert_node(local_id);
        self.type_index.entry(key).or_default().push(id);

        id
    }

    /// Define one reserved identified type exactly once.
    pub fn define_type(&mut self, id: TypeId, ty: Type) {
        let local_id = self.node_local_id(id.id);
        let entry = self.types.get_mut(local_id);
        let TypeEntry::Reserved { symbol } = entry else {
            panic!("defined MIR type {id:?} twice or without reserving it");
        };
        *entry = TypeEntry::Identified {
            ty,
            symbol: *symbol,
        };
    }

    /// Insert one declaration for a completely defined identified type.
    pub fn insert_type_declaration(
        &mut self,
        name: StringId,
        lifetimes: Vec<LifetimeParameter>,
        ty: TypeId,
    ) -> LocalNodeId<TypeDeclaration> {
        // reject structural types and incomplete recursive placeholders
        let local_id = self.node_local_id(ty.id);
        let TypeEntry::Identified { .. } = self.types.get(local_id) else {
            panic!("declared MIR type {ty:?} before its complete identified definition");
        };

        // allocate through the only declaration construction path
        let declaration = TypeDeclaration {
            name,
            lifetimes,
            ty,
        };
        let local_id = self.type_declarations.allocate(declaration);

        self.insert_node(local_id)
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
            TypeEntry::Reserved { symbol } | TypeEntry::Identified { symbol, .. } => Some(*symbol),
            TypeEntry::Structural { .. } => None,
        }
    }

    /// Intern one field and its attributes.
    pub fn intern_field(&mut self, field: Field, attributes: Vec<Attribute>) -> LocalNodeId<Field> {
        // reuse an equal field declaration
        let hash = mir_hash(&(&field, &attributes));
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
            | Type::Int { .. }
            | Type::Isize
            | Type::Usize
            | Type::Float(_)
            | Type::TypeDescriptor
            | Type::TypeId => return id,

            // erase pure lifetime applications
            Type::WithLifetimes { base, .. } => return self.intern_representation(base),

            // normalize transparent and storage wrappers
            Type::Atomic { value } => Type::Atomic {
                value: self.intern_representation(value),
            },
            Type::Dynamic { constraint } => Type::Dynamic {
                constraint: self.intern_representation(constraint),
            },
            Type::Uninit { value } => Type::Uninit {
                value: self.intern_representation(value),
            },
            Type::ManuallyDrop { value } => Type::ManuallyDrop {
                value: self.intern_representation(value),
            },

            // erase borrowed provenance while retaining reference representation
            Type::Reference {
                kind,
                lifetime: _,
                space,
                access,
                pointee,
                nullability,
            } => Type::Reference {
                kind,
                lifetime: Default::default(),
                space,
                access,
                pointee: self.intern_representation(pointee),
                nullability,
            },
            Type::Slice {
                kind,
                lifetime: _,
                element,
                space,
                access,
                nullability,
            } => Type::Slice {
                kind,
                lifetime: Default::default(),
                element: self.intern_representation(element),
                space,
                access,
                nullability,
            },
            Type::TensorView {
                kind,
                lifetime: _,
                space,
                access,
                element,
                shape,
                format,
                sharding,
                nullability,
            } => Type::TensorView {
                kind,
                lifetime: Default::default(),
                space,
                access,
                element: self.intern_representation(element),
                shape,
                format,
                sharding,
                nullability,
            },

            // normalize aggregate children
            Type::FixedArray {
                element,
                length,
                copy,
            } => Type::FixedArray {
                element: self.intern_representation(element),
                length,
                copy,
            },
            Type::Tuple { elements, copy } => Type::Tuple {
                elements: elements
                    .into_iter()
                    .map(|element| self.intern_representation(element))
                    .collect(),
                copy,
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
                storage,
                cases,
                copy,
            } => Type::Variant {
                discriminant: self.intern_representation(discriminant),
                storage: self.intern_representation(storage),
                cases: cases
                    .into_iter()
                    .map(|case| VariantCase {
                        discriminant: case.discriminant,
                        ty: self.intern_representation(case.ty),
                    })
                    .collect(),
                copy,
            },
            Type::Vector {
                element,
                lanes,
                copy,
            } => Type::Vector {
                element: self.intern_representation(element),
                lanes,
                copy,
            },
            Type::Tensor {
                element,
                shape,
                format,
                sharding,
                copy,
            } => Type::Tensor {
                element: self.intern_representation(element),
                shape,
                format,
                sharding,
                copy,
            },

            // erase lifetime binders and caller-only borrow proofs
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
                        obligations: Vec::new(),
                    })
                    .collect(),
                result: self.intern_representation(result),
            },
            Type::Function {
                signature,
                environment,
            } => Type::Function {
                signature: self.intern_representation(signature),
                environment: self.intern_representation(environment),
            },
            Type::FunctionPointer { signature } => Type::FunctionPointer {
                signature: self.intern_representation(signature),
            },
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
            | Type::Int { .. }
            | Type::Isize
            | Type::Usize
            | Type::Float(_)
            | Type::TypeDescriptor
            | Type::TypeId => return id,

            // instantiate explicit applications without entering their identified base
            Type::WithLifetimes { base, lifetimes } => Type::WithLifetimes {
                base,
                lifetimes: lifetimes
                    .iter()
                    .map(|lifetime| self.substitute_lifetime(lifetime, arguments))
                    .collect(),
            },

            // instantiate transparent and storage wrappers
            Type::Atomic { value } => Type::Atomic {
                value: self.instantiate_type_lifetimes(value, arguments),
            },
            Type::Dynamic { constraint } => Type::Dynamic {
                constraint: self.instantiate_type_lifetimes(constraint, arguments),
            },
            Type::Uninit { value } => Type::Uninit {
                value: self.instantiate_type_lifetimes(value, arguments),
            },
            Type::ManuallyDrop { value } => Type::ManuallyDrop {
                value: self.instantiate_type_lifetimes(value, arguments),
            },

            // instantiate borrowed provenance and nested value types
            Type::Reference {
                kind,
                lifetime,
                space,
                access,
                pointee,
                nullability,
            } => Type::Reference {
                kind,
                lifetime: self.substitute_lifetime(&lifetime, arguments),
                space,
                access,
                pointee: self.instantiate_type_lifetimes(pointee, arguments),
                nullability,
            },
            Type::Slice {
                kind,
                lifetime,
                element,
                space,
                access,
                nullability,
            } => Type::Slice {
                kind,
                lifetime: self.substitute_lifetime(&lifetime, arguments),
                element: self.instantiate_type_lifetimes(element, arguments),
                space,
                access,
                nullability,
            },
            Type::TensorView {
                kind,
                lifetime,
                space,
                access,
                element,
                shape,
                format,
                sharding,
                nullability,
            } => Type::TensorView {
                kind,
                lifetime: self.substitute_lifetime(&lifetime, arguments),
                space,
                access,
                element: self.instantiate_type_lifetimes(element, arguments),
                shape,
                format,
                sharding,
                nullability,
            },

            // instantiate aggregate contents
            Type::FixedArray {
                element,
                length,
                copy,
            } => Type::FixedArray {
                element: self.instantiate_type_lifetimes(element, arguments),
                length,
                copy,
            },
            Type::Tuple { elements, copy } => Type::Tuple {
                elements: elements
                    .into_iter()
                    .map(|element| self.instantiate_type_lifetimes(element, arguments))
                    .collect(),
                copy,
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
                storage,
                cases,
                copy,
            } => Type::Variant {
                discriminant: self.instantiate_type_lifetimes(discriminant, arguments),
                storage: self.instantiate_type_lifetimes(storage, arguments),
                cases: cases
                    .into_iter()
                    .map(|case| VariantCase {
                        discriminant: case.discriminant,
                        ty: self.instantiate_type_lifetimes(case.ty, arguments),
                    })
                    .collect(),
                copy,
            },
            Type::Vector {
                element,
                lanes,
                copy,
            } => Type::Vector {
                element: self.instantiate_type_lifetimes(element, arguments),
                lanes,
                copy,
            },
            Type::Tensor {
                element,
                shape,
                format,
                sharding,
                copy,
            } => Type::Tensor {
                element: self.instantiate_type_lifetimes(element, arguments),
                shape,
                format,
                sharding,
                copy,
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
                        obligations: parameter
                            .obligations
                            .into_iter()
                            .map(|obligation| match obligation {
                                BorrowObligation::SuspensionStable { lifetime } => {
                                    BorrowObligation::SuspensionStable {
                                        lifetime: self.substitute_lifetime(&lifetime, arguments),
                                    }
                                }
                            })
                            .collect(),
                    })
                    .collect(),
                result: self.instantiate_type_lifetimes(result, arguments),
            },
            Type::Function {
                signature,
                environment,
            } => Type::Function {
                signature: self.instantiate_type_lifetimes(signature, arguments),
                environment: self.instantiate_type_lifetimes(environment, arguments),
            },
            Type::FunctionPointer { signature } => Type::FunctionPointer {
                signature: self.instantiate_type_lifetimes(signature, arguments),
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
}

#[cfg(test)]
mod tests {
    use destack_core::StringId;

    use crate::{
        Access, Copy, Field, Lifetime, Nullability, ReferenceKind, Space, Symbol, Tree, Type,
    };

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
            space: Space::Local,
            access: Access::Readonly,
            pointee,
            nullability: Nullability::None,
        });
        let static_ = tree.intern_type(Type::Reference {
            kind: ReferenceKind::Borrowed,
            lifetime: Lifetime::static_storage(),
            space: Space::Local,
            access: Access::Readonly,
            pointee,
            nullability: Nullability::None,
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
        let local = tree.intern_type(Type::WithLifetimes {
            base: nominal,
            lifetimes: vec![Lifetime::slot(0)],
        });
        let static_ = tree.intern_type(Type::WithLifetimes {
            base: nominal,
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

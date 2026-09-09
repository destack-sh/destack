use std::fmt;

use crate::{
    Access, Attribute, Field, GenericArgument, Lifetime, LifetimeTerm, LocalNodeId, Residence,
    Space, Static, StaticId, Storage, Tree, Type, TypeId,
};

/// One substitution of template parameters by generic arguments, interned into the tree.
pub struct Substitution<'a> {
    /// The tree the substitution reads and writes.
    tree: &'a mut Tree,
    /// The argument bound to each parameter, in template order.
    arguments: &'a [GenericArgument],
}

impl fmt::Debug for Substitution<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Substitution")
            .field("arguments", &self.arguments)
            .finish()
    }
}

impl<'a> Substitution<'a> {
    /// Create one substitution interning into the tree.
    pub fn new(tree: &'a mut Tree, arguments: &'a [GenericArgument]) -> Self {
        Self { tree, arguments }
    }

    /// Return the tree.
    fn tree(&self) -> &Tree {
        self.tree
    }

    /// Resolve one substituted type.
    fn resolve_type(&mut self, ty: Type) -> TypeId {
        self.tree.intern_type(ty)
    }

    /// Resolve one substituted field.
    fn resolve_field(&mut self, field: Field, attributes: Vec<Attribute>) -> LocalNodeId<Field> {
        self.tree.intern_field(field, attributes)
    }

    /// Substitute the template parameters one type mentions.
    pub fn ty(&mut self, ty: TypeId) -> TypeId {
        // keep a type without parameters as it is
        let tree = self.tree();
        if self.arguments.is_empty()
            || tree.type_symbol(ty).is_some()
            || !ty.mentions_parameter(tree)
        {
            return ty;
        }

        // read a parameter off the arguments, or substitute the type's children
        match tree.get(ty).clone() {
            Type::Parameter { index } => match &self.arguments[index as usize] {
                GenericArgument::Type(argument) => *argument,
                argument => unreachable!("type parameter {index} bound to {argument:?}"),
            },
            definition => self.definition(definition),
        }
    }

    /// Substitute the children of one type definition and resolve it.
    fn definition(&mut self, mut definition: Type) -> TypeId {
        // substitute the types behind struct field nodes
        if let Type::Struct { fields, .. } = &mut definition {
            for field in fields.iter_mut() {
                let declared = self.tree().get(*field).clone();
                let attributes = self.tree().attributes(*field).to_vec();
                let ty = self.ty(declared.ty);
                *field = self.resolve_field(Field { ty, ..declared }, attributes);
            }
        }

        // substitute the region, storage, and access parameters the type names directly
        let arguments = self.arguments;
        match &mut definition {
            Type::Dynamic {
                lifetime,
                storage,
                access,
                ..
            }
            | Type::Reference {
                lifetime,
                storage,
                access,
                ..
            }
            | Type::Slice {
                lifetime,
                storage,
                access,
                ..
            }
            | Type::Function {
                lifetime,
                storage,
                access,
                ..
            } => {
                *lifetime = Self::lifetime(lifetime, arguments);
                *storage = Self::storage(*storage, arguments);
                *access = Self::access(*access, arguments);
            }
            Type::Pointer { access, .. } => *access = Self::access(*access, arguments),
            _ => {}
        }

        // substitute a fixed array's length and a vector's lane count
        if let Type::FixedArray { length, .. } | Type::Vector { lanes: length, .. } =
            &mut definition
        {
            *length = self.value(*length);
        }

        // substitute an application's base and arguments
        if let Type::Application {
            base,
            arguments: applied,
            ..
        } = &mut definition
        {
            *base = self.ty(*base);
            for argument in applied.iter_mut() {
                *argument = self.argument(argument.clone());
            }

            return self.resolve_type(definition);
        }

        // substitute each child type
        let mut children = Vec::new();
        definition.map_child_type_ids(&mut |child| {
            children.push(child);
            child
        });
        let mut substituted = Vec::with_capacity(children.len());
        for child in children {
            substituted.push(self.ty(child));
        }

        // write the substituted children back in the order they were read
        let mut substituted = substituted.into_iter();
        definition.map_child_type_ids(&mut |_| {
            substituted
                .next()
                .unwrap_or_else(|| unreachable!("substituted child count changed"))
        });

        self.resolve_type(definition)
    }

    /// Substitute the parameters one generic argument names.
    pub fn argument(&mut self, argument: GenericArgument) -> GenericArgument {
        match argument {
            GenericArgument::Type(ty) => GenericArgument::Type(self.ty(ty)),
            GenericArgument::Region { lifetime, space } => GenericArgument::Region {
                lifetime: Self::lifetime(&lifetime, self.arguments),
                space: Self::space(space, self.arguments),
            },
            GenericArgument::Space(space) => {
                GenericArgument::Space(Self::space(space, self.arguments))
            }
            GenericArgument::Access(access) => {
                GenericArgument::Access(Self::access(access, self.arguments))
            }
            GenericArgument::Value(value) => GenericArgument::Value(self.value(value)),
        }
    }

    /// Substitute the region parameters one lifetime names.
    fn lifetime(lifetime: &Lifetime, arguments: &[GenericArgument]) -> Lifetime {
        let mut terms = Vec::new();
        for term in &lifetime.terms {
            match term {
                LifetimeTerm::Parameter(index) => match &arguments[*index as usize] {
                    GenericArgument::Region { lifetime, .. } => {
                        terms.extend(lifetime.terms.iter().copied())
                    }
                    argument => unreachable!("region parameter {index} bound to {argument:?}"),
                },
                term => terms.push(*term),
            }
        }

        Lifetime::new(terms)
    }

    /// Substitute one space parameter, a region parameter through its space.
    fn space(space: Space, arguments: &[GenericArgument]) -> Space {
        match space {
            Space::Parameter(index) => match &arguments[index as usize] {
                GenericArgument::Space(space) | GenericArgument::Region { space, .. } => *space,
                argument => unreachable!("space parameter {index} bound to {argument:?}"),
            },
            space => space,
        }
    }

    /// Substitute the space parameter one storage names, keeping its residence.
    fn storage(storage: Storage, arguments: &[GenericArgument]) -> Storage {
        let space = Self::space(storage.space(), arguments);

        match storage.residence() {
            Residence::Frame => storage,
            Residence::Heap => Storage::heap(space),
            Residence::Static => Storage::global(space),
        }
    }

    /// Substitute one access parameter.
    fn access(access: Access, arguments: &[GenericArgument]) -> Access {
        match access {
            Access::Parameter(index) => match &arguments[index as usize] {
                GenericArgument::Access(access) => *access,
                argument => unreachable!("access parameter {index} bound to {argument:?}"),
            },
            access => access,
        }
    }

    /// Substitute one value parameter.
    fn value(&self, value: StaticId) -> StaticId {
        match *self.tree().static_value(value) {
            Static::Parameter(index) => match &self.arguments[index as usize] {
                GenericArgument::Value(value) => *value,
                argument => unreachable!("value parameter {index} bound to {argument:?}"),
            },
            _ => value,
        }
    }

    /// Return the template's definition at the arguments.
    pub fn representation(&mut self, base: TypeId) -> TypeId {
        let definition = self.tree().get(base).clone();

        self.definition(definition)
    }
}

/// Intern one type with its template parameters substituted by the arguments.
pub fn substitute_type(tree: &mut Tree, ty: TypeId, arguments: &[GenericArgument]) -> TypeId {
    Substitution::new(tree, arguments).ty(ty)
}

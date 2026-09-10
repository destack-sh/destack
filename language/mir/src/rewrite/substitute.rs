use crate::{
    Access, Exclusivity, Extent, Field, GenericArgument, Lifetime, Reference, RegionBound, Space,
    Static, StaticId, Storage, Tree, Type, TypeId,
};

/// Substitute generic arguments and bound regions through MIR types.
#[derive(Debug)]
pub struct Substitution<'a> {
    /// The destination tree.
    tree: &'a mut Tree,
    /// Arguments replacing template parameters.
    arguments: &'a [GenericArgument],
    /// Arguments replacing the enclosing region binder.
    regions: &'a [(Lifetime, Storage)],
    /// The number of nested binders entered.
    depth: u32,
    /// Additional enclosing binders for a replacement argument.
    shift: u32,
}

impl<'a> Substitution<'a> {
    /// Substitute the supplied template arguments.
    pub fn new(tree: &'a mut Tree, arguments: &'a [GenericArgument]) -> Self {
        Self {
            tree,
            arguments,
            regions: &[],
            depth: 0,
            shift: 0,
        }
    }

    /// Substitute one type while preserving declaration identities.
    pub fn ty(&mut self, ty: TypeId) -> TypeId {
        if matches!(self.tree.get(ty), Type::Declaration { .. })
            || (self.arguments.is_empty() && self.regions.is_empty() && self.shift == 0)
        {
            return ty;
        }

        self.definition(self.tree.get(ty).clone())
    }

    /// Substitute one definition and its children.
    fn definition(&mut self, mut definition: Type) -> TypeId {
        // replace template types without capturing their bound regions
        if let Type::Parameter { index } = definition
            && !self.arguments.is_empty()
        {
            let GenericArgument::Type(ty) = self.rebind(self.arguments[index as usize].clone())
            else {
                unreachable!("type parameter bound to a non-type argument");
            };

            return ty;
        }

        // enter the signature's region binder
        let depth = self.depth;
        if matches!(&definition, Type::FunctionSignature { lifetimes, .. } if !lifetimes.is_empty())
        {
            self.depth += 1;
        }

        // substitute reference qualifiers and signature bounds
        match &mut definition {
            Type::Reference {
                kind,
                lifetime,
                storage,
                access,
                ..
            }
            | Type::Dynamic {
                kind,
                lifetime,
                storage,
                access,
                ..
            }
            | Type::Slice {
                kind,
                lifetime,
                storage,
                access,
                ..
            }
            | Type::Function {
                kind,
                lifetime,
                storage,
                access,
                ..
            } => {
                *lifetime = self.lifetime(lifetime);
                *storage = self.storage(*storage);
                *access = self.access(*access);
                if let Reference::Borrowed(exclusivity) = kind {
                    *exclusivity = self.exclusivity(*exclusivity);
                }
            }
            Type::Pointer { access, .. } => *access = self.access(*access),
            Type::FunctionSignature { lifetimes, .. } => {
                for parameter in lifetimes {
                    parameter.outlives = self.lifetime(&parameter.outlives);
                }
            }
            _ => {}
        }

        // substitute applications through their complete arguments
        if let Type::Application { base, arguments } = &mut definition {
            *base = self.ty(*base);
            for argument in arguments {
                *argument = self.argument(argument.clone());
            }
        } else {
            if let Type::Struct { fields, .. } = &mut definition {
                for field in fields {
                    let declared = self.tree.get(*field).clone();
                    let attributes = self.tree.attributes(*field).to_vec();
                    let ty = self.ty(declared.ty);
                    *field = self.tree.intern_field(Field { ty, ..declared }, attributes);
                }
            }
            definition.map_values(&mut |value| self.value(value));
            definition.map_child_type_ids(&mut |child| self.ty(child));
        }
        self.depth = depth;

        self.tree.intern_type(definition)
    }

    /// Substitute one generic argument.
    pub fn argument(&mut self, argument: GenericArgument) -> GenericArgument {
        match argument {
            GenericArgument::Type(ty) => GenericArgument::Type(self.ty(ty)),
            GenericArgument::Region { lifetime, storage } => GenericArgument::Region {
                lifetime: self.lifetime(&lifetime),
                storage: self.storage(storage),
            },
            GenericArgument::Space(space) => GenericArgument::Space(self.space(space)),
            GenericArgument::Access(access) => GenericArgument::Access(self.access(access)),
            GenericArgument::Exclusivity(exclusivity) => {
                GenericArgument::Exclusivity(self.exclusivity(exclusivity))
            }
            GenericArgument::Value(value) => GenericArgument::Value(self.value(value)),
        }
    }

    /// Preserve a replacement argument's free regions beneath the current binders.
    fn rebind(&mut self, argument: GenericArgument) -> GenericArgument {
        if self.depth == 0 {
            return argument;
        }
        let mut substitution = Substitution::new(self.tree, &[]);
        substitution.shift = self.depth;

        substitution.argument(argument)
    }

    /// Adjust a bound region outside the binders entered during substitution.
    fn bound(&self, mut bound: RegionBound) -> RegionBound {
        if bound.depth >= self.depth {
            if self.regions.is_empty() {
                bound.depth += self.shift;
            } else {
                bound.depth -= 1;
            }
        }

        bound
    }

    /// Substitute lifetime extents.
    fn lifetime(&mut self, lifetime: &Lifetime) -> Lifetime {
        let mut extents = Vec::new();
        for &extent in &lifetime.extents {
            let argument = match extent {
                Extent::Parameter(index) if !self.arguments.is_empty() => {
                    Some(self.arguments[index as usize].clone())
                }
                Extent::Bound(bound) if !self.regions.is_empty() && bound.depth == self.depth => {
                    let (lifetime, storage) = &self.regions[bound.index as usize];
                    Some(GenericArgument::Region {
                        lifetime: lifetime.clone(),
                        storage: *storage,
                    })
                }
                _ => None,
            };
            if let Some(argument) = argument {
                let GenericArgument::Region { lifetime, .. } = self.rebind(argument) else {
                    unreachable!("region parameter bound to a non-region argument");
                };
                extents.extend(lifetime.extents);
            } else {
                extents.push(match extent {
                    Extent::Bound(bound) => Extent::Bound(self.bound(bound)),
                    extent => extent,
                });
            }
        }

        Lifetime::new(extents)
    }

    /// Substitute a space and its joined members.
    fn space(&mut self, space: Space) -> Space {
        match space {
            Space::Parameter(index) if !self.arguments.is_empty() => {
                match self.rebind(self.arguments[index as usize].clone()) {
                    GenericArgument::Space(space) => space,
                    GenericArgument::Region { storage, .. } => storage.space(self.tree),
                    _ => unreachable!("space parameter bound to an incompatible argument"),
                }
            }
            Space::Bound(bound) => self.storage(Storage::Bound(bound)).space(self.tree),
            Space::Join(id) => {
                let members = self.tree.space_join(id).to_vec();
                let members = members
                    .into_iter()
                    .map(|space| self.space(space))
                    .collect::<Vec<_>>();

                self.tree.intern_space_join(members)
            }
            space => space,
        }
    }

    /// Substitute storage and its joined members.
    fn storage(&mut self, storage: Storage) -> Storage {
        match storage {
            Storage::Parameter(index) if !self.arguments.is_empty() => {
                let GenericArgument::Region { storage, .. } =
                    self.rebind(self.arguments[index as usize].clone())
                else {
                    unreachable!("storage parameter bound to a non-region argument");
                };

                storage
            }
            Storage::Bound(bound) if !self.regions.is_empty() && bound.depth == self.depth => {
                let (lifetime, storage) = &self.regions[bound.index as usize];
                let argument = GenericArgument::Region {
                    lifetime: lifetime.clone(),
                    storage: *storage,
                };
                let GenericArgument::Region { storage, .. } = self.rebind(argument) else {
                    unreachable!("region substitution changes its argument domain");
                };

                storage
            }
            Storage::Bound(bound) => Storage::Bound(self.bound(bound)),
            Storage::Heap(space) => Storage::Heap(self.space(space)),
            Storage::Static(space) => Storage::Static(self.space(space)),
            Storage::Join(id) => {
                let members = self.tree.storage_join(id).to_vec();
                let members = members
                    .into_iter()
                    .map(|storage| self.storage(storage))
                    .collect::<Vec<_>>();

                self.tree.intern_storage_join(members)
            }
            storage => storage,
        }
    }

    /// Substitute an access parameter.
    fn access(&self, access: Access) -> Access {
        if let Access::Parameter(index) = access
            && !self.arguments.is_empty()
        {
            let GenericArgument::Access(access) = self.arguments[index as usize] else {
                unreachable!("access parameter bound to a non-access argument");
            };

            return access;
        }

        access
    }

    /// Substitute an exclusivity parameter.
    fn exclusivity(&self, exclusivity: Exclusivity) -> Exclusivity {
        if let Exclusivity::Parameter(index) = exclusivity
            && !self.arguments.is_empty()
        {
            let GenericArgument::Exclusivity(exclusivity) = self.arguments[index as usize] else {
                unreachable!("exclusivity parameter bound to a non-exclusivity argument");
            };

            return exclusivity;
        }

        exclusivity
    }

    /// Substitute static values and the types they contain.
    fn value(&mut self, id: StaticId) -> StaticId {
        if let Static::Parameter(index) = *self.tree.static_value(id)
            && !self.arguments.is_empty()
        {
            let GenericArgument::Value(value) = self.rebind(self.arguments[index as usize].clone())
            else {
                unreachable!("value parameter bound to a non-value argument");
            };

            return value;
        }
        let mut value = self.tree.static_value(id).clone();
        value.map_values(&mut |value| self.value(value));
        value.map_types(&mut |ty| self.ty(ty));
        if let Static::Space(space) = &mut value {
            *space = self.space(*space);
        }

        self.tree.intern_static(value)
    }

    /// Substitute the definition of a template.
    pub fn representation(&mut self, base: TypeId) -> TypeId {
        self.definition(self.tree.type_definition(base).clone())
    }
}

/// Substitute template arguments in a type.
pub fn substitute_type(tree: &mut Tree, ty: TypeId, arguments: &[GenericArgument]) -> TypeId {
    Substitution::new(tree, arguments).ty(ty)
}

/// Substitute the enclosing region binder while preserving nested binders.
pub fn instantiate_regions(tree: &mut Tree, ty: TypeId, regions: &[(Lifetime, Storage)]) -> TypeId {
    let mut substitution = Substitution::new(tree, &[]);
    substitution.regions = regions;

    substitution.ty(ty)
}

use crate::{
    Access, Extent, Field, GenericArgument, Lifetime, RegionBound, Static, StaticId, Tree, Type,
    TypeId, lend, referent_of,
};

/// Substitute generic arguments and bound regions through MIR types.
#[derive(Debug)]
pub struct Substitution<'a> {
    /// The destination tree.
    tree: &'a Tree,
    /// Arguments replacing template parameters.
    arguments: &'a [GenericArgument],
    /// Arguments replacing the enclosing region binder.
    regions: &'a [Lifetime],
    /// The number of nested binders entered.
    depth: u32,
    /// Additional enclosing binders for a replacement argument.
    shift: u32,
}

impl<'a> Substitution<'a> {
    /// Resolve a declaration or application to its substituted representation.
    pub fn resolve(ty: TypeId, tree: &Tree) -> TypeId {
        match tree.get(ty) {
            Type::Declaration { declaration } => match tree.get(*declaration).definition {
                Some(definition) => definition,
                None => ty,
            },
            Type::Application { base, arguments } => {
                // preserve applications whose declarations remain opaque
                let definition = tree.type_definition(*base).clone();
                if matches!(definition, Type::Declaration { .. }) {
                    return ty;
                }

                // substitute one representation while retaining nominal child types
                let arguments = arguments.clone();

                Substitution::new(tree, &arguments).definition(definition)
            }
            _ => ty,
        }
    }

    /// Substitute the supplied template arguments.
    pub fn new(tree: &'a Tree, arguments: &'a [GenericArgument]) -> Self {
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
        // take a referent parameter's argument as its object, without capturing bound regions
        if let Type::Parameter { index, referent } = definition
            && !self.arguments.is_empty()
        {
            let ty = self.parameter_argument(index);

            return match referent {
                true => referent_of(self.tree, ty),
                false => ty,
            };
        }

        // lend the object a referent parameter's argument stands for through a reference to it
        if let Type::Reference {
            kind,
            lifetime,
            access,
            pointee,
        } = &definition
            && let Type::Parameter {
                index,
                referent: true,
            } = self.tree.get(*pointee)
            && !self.arguments.is_empty()
        {
            let lifetime = self.lifetime(lifetime);
            let access = self.access(*access);
            let target = self.parameter_argument(*index);

            return lend(self.tree, *kind, lifetime, access, target);
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
                lifetime, access, ..
            }
            | Type::Dynamic {
                lifetime, access, ..
            }
            | Type::Slice {
                lifetime, access, ..
            }
            | Type::Function {
                lifetime, access, ..
            } => {
                *lifetime = self.lifetime(lifetime);
                *access = self.access(*access);
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
                    let ty = self.ty(declared.ty);
                    *field = self.tree.intern_field(Field { ty, ..declared });
                }
            }
            definition.map_values(&mut |value| self.value(value));
            definition.map_child_type_ids(&mut |child| self.ty(child));
        }
        self.depth = depth;

        self.tree.intern_type(definition)
    }

    /// Return the type bound to one type parameter, rebound past the entered binders.
    fn parameter_argument(&mut self, index: u32) -> TypeId {
        let GenericArgument::Type(ty) = self.rebind(self.arguments[index as usize].clone()) else {
            unreachable!("type parameter bound to a non-type argument");
        };

        ty
    }

    /// Substitute one generic argument.
    pub fn argument(&mut self, argument: GenericArgument) -> GenericArgument {
        match argument {
            GenericArgument::Type(ty) => GenericArgument::Type(self.ty(ty)),
            GenericArgument::Region(lifetime) => GenericArgument::Region(self.lifetime(&lifetime)),
            GenericArgument::Access(access) => GenericArgument::Access(self.access(access)),
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
    pub fn lifetime(&mut self, lifetime: &Lifetime) -> Lifetime {
        let mut extents = Vec::new();
        for &extent in &lifetime.extents {
            let argument = match extent {
                Extent::Parameter(index) if !self.arguments.is_empty() => {
                    Some(self.arguments[index as usize].clone())
                }
                Extent::Bound(bound) if !self.regions.is_empty() && bound.depth == self.depth => {
                    Some(GenericArgument::Region(
                        self.regions[bound.index as usize].clone(),
                    ))
                }
                _ => None,
            };
            if let Some(argument) = argument {
                let GenericArgument::Region(lifetime) = self.rebind(argument) else {
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

    /// Substitute an access parameter.
    pub fn access(&self, access: Access) -> Access {
        substitute_access(self.arguments, access)
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

        self.tree.intern_static(value)
    }
}

/// Substitute template arguments in an access.
pub fn substitute_access(arguments: &[GenericArgument], access: Access) -> Access {
    if let Access::Parameter(index) = access
        && !arguments.is_empty()
    {
        let GenericArgument::Access(access) = arguments[index as usize] else {
            unreachable!("access parameter bound to a non-access argument");
        };

        return access;
    }

    access
}

/// Substitute template arguments in a type.
pub fn substitute_type(tree: &Tree, ty: TypeId, arguments: &[GenericArgument]) -> TypeId {
    Substitution::new(tree, arguments).ty(ty)
}

/// Substitute the enclosing region binder while preserving nested binders.
pub fn instantiate_regions(tree: &Tree, ty: TypeId, regions: &[Lifetime]) -> TypeId {
    let mut substitution = Substitution::new(tree, &[]);
    substitution.regions = regions;

    substitution.ty(ty)
}

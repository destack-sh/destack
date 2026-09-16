use std::sync::Arc;

use destack_core::FxIndexSet;
use destack_source::ModuleId;

use crate::{
    Copy,
    Field, Function, FunctionId, FunctionParameter, GenericArgument, GenericParameter,
    GenericParameterDomain, Global, GlobalId, Linkage, Space, Static, StaticId, Storage, Symbol,
    Tree, Type, TypeHeritage, TypeId,
};

/// The declared tree of one module, absent to leave the symbols naming that module reserved.
pub type Declared<'a> = dyn FnMut(ModuleId) -> Option<Arc<Tree>> + 'a;

/// The mover copying declarations of other trees into one tree, LLVM's IRMover.
pub struct Importer<'t, 'd> {
    /// The tree imported into.
    tree: &'t mut Tree,
    /// The declared tree of each module the imports name.
    declared_of: &'t mut Declared<'d>,
    /// The identified types being defined, their own mentions naming the reservation.
    defining: FxIndexSet<Symbol>,
}

impl std::fmt::Debug for Importer<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Importer")
            .field("defining", &self.defining)
            .finish_non_exhaustive()
    }
}

impl<'t, 'd> Importer<'t, 'd> {
    /// Create the importer into one tree over the resolver.
    pub fn new(tree: &'t mut Tree, declared_of: &'t mut Declared<'d>) -> Self {
        Self {
            tree,
            declared_of,
            defining: FxIndexSet::default(),
        }
    }

    /// Import one type of the source tree, an identified one keeping its symbol.
    pub fn import_type(&mut self, source: &Tree, ty: TypeId) -> TypeId {
        let Some(symbol) = source.type_symbol(ty) else {
            let definition = self.import_type_content(source, ty);

            return self.tree.intern_type(definition, source.copy(ty));
        };

        // reserve the identity once per import, its definition and declaration naming it again
        let id = self.tree.reserve_type(symbol);
        let reserved = self.tree.intern_type(Type::Declaration { declaration: id }, Copy::No);
        if !self.defining.insert(symbol) {
            return reserved;
        }

        // fill the definition from the source, else from the declaring module
        if self.tree.get(id).definition.is_none() {
            if source.is_defined_type(ty) {
                let definition = self.import_type_content(source, ty);
                let definition = self.tree.intern_type(definition, source.copy(ty));
                self.tree.get_mut(id).definition = Some(definition);
            } else if let Some(declared) = symbol
                .module()
                .and_then(|module| (self.declared_of)(module))
            {
                let declaring: &Tree = &declared;
                if let Some(defined) = declaring.identified_type(symbol)
                    && declaring.is_defined_type(defined)
                {
                    // import the declaring module's definition with the identity reopened
                    self.defining.swap_remove(&symbol);
                    self.import_type(declaring, defined);
                    self.defining.insert(symbol);
                }
            }
        }

        // declare it as the source does, attributes included
        if self.tree.get(id).name.is_none()
            && let Some(source_declaration) = source.type_declaration(ty)
        {
            let declaration = source.get(source_declaration).clone();
            if let Some(name) = declaration.name {
                let heritage = TypeHeritage {
                    extends: declaration
                        .heritage
                        .extends
                        .iter()
                        .map(|base| self.import_type(source, *base))
                        .collect(),
                    implements: declaration
                        .heritage
                        .implements
                        .iter()
                        .map(|base| self.import_type(source, *base))
                        .collect(),
                };
                let generics = declaration
                    .generics
                    .iter()
                    .map(|parameter| self.import_generic(source, parameter))
                    .collect();
                let declaration = self.tree.get_mut(id);
                declaration.name = Some(name);
                declaration.generics = generics;
                declaration.heritage = heritage;
                for attribute in source.attributes(source_declaration) {
                    self.tree.push_attribute(id, attribute.clone());
                }
            }
        }

        // release the identity for the next import naming it
        self.defining.swap_remove(&symbol);

        reserved
    }

    /// Import one function's header of the source tree, its body left behind.
    pub fn import_function_header(&mut self, source: &Tree, function: FunctionId) -> Function {
        let declared = source.get(function).clone();
        let parameters = declared
            .parameters
            .iter()
            .map(|parameter| {
                FunctionParameter::new(parameter.value, self.import_type(source, parameter.ty))
            })
            .collect();
        let return_type = self.import_type(source, declared.return_type);
        let environment = declared
            .environment
            .map(|environment| self.import_type(source, environment));
        let arguments = declared
            .arguments
            .iter()
            .map(|argument| self.import_argument(source, argument.clone()))
            .collect();
        let generics = declared
            .generics
            .iter()
            .map(|parameter| self.import_generic(source, parameter))
            .collect();

        Function {
            linkage: Linkage::Import,
            parameters,
            return_type,
            environment,
            arguments,
            generics,
            template: None,
            body: None,
            ..declared
        }
    }

    /// Import one global of the source tree, its initializer left behind.
    pub fn import_global(&mut self, source: &Tree, global: GlobalId) -> Global {
        let declared = source.get(global).clone();
        let ty = self.import_type(source, declared.ty);

        Global {
            ty,
            linkage: Linkage::Import,
            initializer: None,
            ..declared
        }
    }

    /// Import the content of one type with its children imported.
    fn import_type_content(&mut self, source: &Tree, ty: TypeId) -> Type {
        let ty = match source.get(ty) {
            Type::Declaration { declaration } => source
                .get(*declaration)
                .definition
                .expect("a defined declaration has a body"),
            _ => ty,
        };

        // remap a declaration alias through its persistent identity
        if matches!(source.get(ty), Type::Declaration { .. }) {
            let imported = self.import_type(source, ty);

            return self.tree.get(imported).clone();
        }
        let mut definition = source.get(ty).clone();

        // intern each field under its imported type, attributes included
        if let Type::Struct { fields, .. } = &mut definition {
            for field in fields.iter_mut() {
                let declared = source.get(*field).clone();
                let imported = self.import_type(source, declared.ty);
                *field = self.tree.intern_field(Field {
                    ty: imported,
                    ..declared
                });
            }
        }

        // import compile-time values and their nested types
        definition.map_values(&mut |value| self.import_static(source, value));

        // intern the space joins here, the type children below
        definition.map_storages(&mut |storage| self.import_storage(source, storage));
        definition.map_spaces(&mut |space| self.import_space(source, space));

        // import every type child
        definition.map_child_type_ids(&mut |child| self.import_type(source, child));

        definition
    }

    /// Intern one space of the source tree here, a join through its spaces.
    fn import_space(&mut self, source: &Tree, space: Space) -> Space {
        match space {
            Space::Of(ty) => Space::Of(self.import_type(source, ty)),
            Space::Join(id) => {
                let spaces: Vec<_> = source
                    .space_join(id)
                    .iter()
                    .map(|space| self.import_space(source, *space))
                    .collect();

                self.tree.intern_space_join(spaces)
            }
            space => space,
        }
    }

    /// Import storage and its interned joins.
    fn import_storage(&mut self, source: &Tree, storage: Storage) -> Storage {
        match storage {
            Storage::Heap(space) => Storage::Heap(self.import_space(source, space)),
            Storage::Static(space) => Storage::Static(self.import_space(source, space)),
            Storage::Join(id) => {
                let members = source
                    .storage_join(id)
                    .iter()
                    .map(|storage| self.import_storage(source, *storage))
                    .collect::<Vec<_>>();

                self.tree.intern_storage_join(members)
            }
            storage => storage,
        }
    }

    /// Import one generic argument.
    pub fn import_argument(&mut self, source: &Tree, argument: GenericArgument) -> GenericArgument {
        match argument {
            GenericArgument::Type(ty) => GenericArgument::Type(self.import_type(source, ty)),
            GenericArgument::Value(value) => {
                GenericArgument::Value(self.import_static(source, value))
            }
            GenericArgument::Space(space) => {
                GenericArgument::Space(self.import_space(source, space))
            }
            GenericArgument::Region { lifetime, storage } => GenericArgument::Region {
                lifetime,
                storage: self.import_storage(source, storage),
            },
            argument => argument,
        }
    }

    /// Import one generic parameter.
    fn import_generic(&mut self, source: &Tree, parameter: &GenericParameter) -> GenericParameter {
        let domain = match &parameter.domain {
            GenericParameterDomain::Type { bounds } => GenericParameterDomain::Type {
                bounds: bounds
                    .iter()
                    .map(|bound| self.import_type(source, *bound))
                    .collect(),
            },
            GenericParameterDomain::Value { ty } => GenericParameterDomain::Value {
                ty: self.import_type(source, *ty),
            },
            domain => domain.clone(),
        };

        GenericParameter {
            name: parameter.name,
            domain,
        }
    }

    /// Import one static.
    fn import_static(&mut self, source: &Tree, id: StaticId) -> StaticId {
        let mut value = source.static_value(id).clone();
        value.map_values(&mut |value| self.import_static(source, value));
        value.map_types(&mut |ty| self.import_type(source, ty));
        if let Static::Space(space) = &mut value {
            *space = self.import_space(source, *space);
        }

        self.tree.intern_static(value)
    }
}

use std::sync::Arc;

use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::{
    FunctionDeclaration, GenericInstanceKey, GenericScope, ModuleLowerer, NominalField, TypeLowerer,
};
use crate::{CompilerError, CompilerResult};

impl TypeLowerer<'_, '_> {
    /// Lower one class declaration to its managed reference nominal.
    pub(in crate::lower) fn lower_class(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: dir::ClassDefinition,
        declaration: mir::LocalNodeId<mir::TypeDeclaration>,
    ) -> CompilerResult<Vec<NominalField>> {
        // gather the nominal fields, inherited and own
        let fields = self.lower.nominal_fields(symbol)?;
        let mut field_nodes = Vec::with_capacity(fields.len());

        // lead with the base class's fields
        let mut base_object = None;
        if let Some(heritage) = &definition.extends {
            let base = heritage.ty;
            let base = self.lower.stored(base)?;
            let dir::Type::Application(_) = self.lower.ty(base)? else {
                return Err(CompilerError::Internal {
                    message: "a class heritage outside an application type".to_string(),
                });
            };
            let storage = self.lower_nominal(base)?.storage;
            self.lower.fill_heritage(self.tree, storage)?;
            let resolved = mir::Substitution::resolve(storage, self.tree);
            let mir::Type::Class {
                fields: base_nodes, ..
            } = self.tree.get(resolved)
            else {
                return Err(CompilerError::Internal {
                    message: "a class heritage without object storage".to_string(),
                });
            };
            field_nodes.extend(base_nodes.iter().copied());
            base_object = Some(storage);
        }

        // lower each declared field's stored type into a field node
        let declared = self.lower.instance_fields(&definition.members)?;
        for field in &declared {
            let ty = self.optional_storage_representation(field.ty, field.is_optional)?;
            let name = match field.key {
                dir::StaticKey::Name(name) => Some(name),
                _ => None,
            };

            field_nodes.push(self.tree.intern_field(mir::Field {
                name,
                ty,
                attributes: Vec::new(),
            }));
        }

        // define the class storage over its base
        let definition = self.tree.intern_type(mir::Type::Class {
            base: base_object,
            fields: field_nodes,
        });
        self.tree.get_mut(declaration).definition = Some(definition);

        Ok(fields)
    }
}

/// One slot of a class's virtual table while lowering.
#[derive(Debug, Clone)]
pub(in crate::lower) enum ClassSlot {
    /// A virtual method key and the method implementing it.
    Method {
        /// The method key.
        key: dir::StaticKey,
        /// The implementing method.
        method: dir::GlobalSymbolId,
        /// The arguments the slot calls the method at, in the class's parameters.
        arguments: Vec<mir::GenericArgument>,
    },
    /// A virtual method key the class leaves abstract.
    Abstract {
        /// The method key.
        key: dir::StaticKey,
    },
    /// An interface the class implements.
    Conformance {
        /// The interface declaration.
        interface: dir::GlobalSymbolId,
        /// The interface's constraint type in the class's own parameters.
        constraint: mir::TypeId,
    },
}

impl ClassSlot {
    /// Return the method key a method slot holds.
    fn key(&self) -> Option<dir::StaticKey> {
        match self {
            Self::Method { key, .. } | Self::Abstract { key } => Some(*key),
            Self::Conformance { .. } => None,
        }
    }
}

impl ModuleLowerer<'_> {
    /// Build the virtual table of every own class.
    pub(in crate::lower) fn declare_class_tables(
        &mut self,
        tree: &mut mir::Tree,
    ) -> CompilerResult<Vec<mir::VirtualTable>> {
        // collect the own classes
        let classes = self
            .local()
            .definitions
            .iter_definitions()
            .filter_map(|(symbol, definition)| match definition {
                dir::Definition::Class(class) if symbol.module_id == self.module => {
                    Some((symbol, class.template.is_some()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        // fill each class's slots
        let scope = GenericScope::default().erased();
        let mut tables = Vec::with_capacity(classes.len());
        for (class, is_generic) in classes {
            // key a generic class's table by its template declaration
            let source = self.symbol_type(class)?;
            let mut types = self.type_lowerer(tree, &scope);
            let nominal = match is_generic {
                true => types.lower_template_nominal(source, class)?,
                false => types.lower_nominal(source)?,
            };

            // lower each slot
            let mut slots = Vec::new();
            for slot in self.class_slots(tree, class)?.iter() {
                slots.push(match slot {
                    ClassSlot::Method {
                        method, arguments, ..
                    } => mir::VirtualSlot::Method {
                        function: self.slot_function(tree, *method, &scope)?,
                        arguments: arguments.clone(),
                    },
                    ClassSlot::Abstract { .. } => mir::VirtualSlot::Abstract,
                    ClassSlot::Conformance { constraint, .. } => mir::VirtualSlot::Conformance {
                        constraint: *constraint,
                    },
                });
            }
            tables.push(mir::VirtualTable {
                concrete: nominal.storage,
                value: mir::erase_lifetimes(tree, nominal.value),
                slots,
            });
        }

        Ok(tables)
    }

    /// Return the slot one class's table holds one method in.
    pub(in crate::lower) fn class_slot(
        &mut self,
        tree: &mut mir::Tree,
        class: dir::GlobalTypeId,
        method: dir::GlobalSymbolId,
    ) -> CompilerResult<mir::DispatchSlot> {
        let class = self.stored(class)?;
        let dir::Type::Application(class) = self.ty(class)? else {
            return Err(CompilerError::Internal {
                message: "a virtual call on a class outside an application type".to_string(),
            });
        };

        // find the slot by the method's key
        let declared = self.state(method.module_id)?.definitions.member(method);
        let Some((_, _, dir::DefinitionMember::Method(declared))) = declared else {
            return Err(CompilerError::Internal {
                message: "a virtual call to a method outside its class".to_string(),
            });
        };
        let dir::MemberSlot::Key(key) = declared.slot else {
            return Err(CompilerError::Internal {
                message: "a virtual call to a method without a key".to_string(),
            });
        };
        let slots = self.class_slots(tree, class.symbol)?;
        let Some(index) = slots.iter().position(|slot| slot.key() == Some(key)) else {
            return Err(CompilerError::Internal {
                message: "a virtual call to a method outside its class table".to_string(),
            });
        };

        Ok(mir::DispatchSlot::new(index as u32))
    }

    /// Return the slots of one class's virtual table.
    fn class_slots(
        &mut self,
        tree: &mut mir::Tree,
        class: dir::GlobalSymbolId,
    ) -> CompilerResult<Arc<[ClassSlot]>> {
        if let Some(slots) = self.class_slots.get(&class) {
            return Ok(slots.clone());
        }
        let Some(declared) = self.definition(class)?.cloned() else {
            return Err(CompilerError::Internal {
                message: "a class table without its class definition".to_string(),
            });
        };
        let dir::Definition::Class(definition) = &declared else {
            return Err(CompilerError::Internal {
                message: "a class table over a non-class definition".to_string(),
            });
        };
        let (scope, own) = match definition.template {
            // call an own method at the class's own parameters
            Some(template) => {
                let template = template.into_global(class.module_id);
                let scope = GenericScope::for_declaration(self, template)?;
                let own = self.class_parameters(tree, class, &scope)?;

                (scope, own)
            }
            None => (GenericScope::default(), Vec::new()),
        };

        // start from the base's slots
        let mut slots = match &definition.extends {
            Some(heritage) => self.base_slots(tree, heritage.ty, &scope)?,
            None => Vec::new(),
        };

        // replace overridden slots, then append new virtual methods
        for member in &definition.members {
            let dir::DefinitionMember::Method(method) = member else {
                continue;
            };
            let dir::MemberSlot::Key(key) = method.slot else {
                continue;
            };
            if method.space != dir::MemberSpace::Instance {
                continue;
            }
            let slot = slots.iter_mut().find(|slot| slot.key() == Some(key));
            match (slot, method.is_override, method.abstraction) {
                (Some(slot), true, dir::MethodAbstraction::Abstract) => {
                    *slot = ClassSlot::Abstract { key };
                }
                (Some(slot), true, _) => {
                    *slot = ClassSlot::Method {
                        key,
                        method: method.symbol,
                        arguments: own.clone(),
                    };
                }
                (None, false, dir::MethodAbstraction::Virtual) => slots.push(ClassSlot::Method {
                    key,
                    method: method.symbol,
                    arguments: own.clone(),
                }),
                (None, false, dir::MethodAbstraction::Abstract) => {
                    slots.push(ClassSlot::Abstract { key });
                }
                (None, false, dir::MethodAbstraction::Concrete) => {}
                _ => {
                    return Err(CompilerError::Internal {
                        message: "a class method whose override disagrees with its base slots"
                            .to_string(),
                    });
                }
            }
        }

        // append a slot for each interface the class adds
        for conformance in declared.implementations() {
            let interface = self.stored(conformance.interface)?;
            let dir::Type::Application(application) = self.ty(interface)? else {
                return Err(CompilerError::Internal {
                    message: "a class implementing a non-application interface".to_string(),
                });
            };
            let is_slotted = slots.iter().any(|slot| {
                matches!(slot, ClassSlot::Conformance { interface, .. } if *interface == application.symbol)
            });
            if is_slotted {
                continue;
            }
            let constraint = self
                .type_lowerer(tree, &scope)
                .lower_nominal(interface)?
                .storage;
            slots.push(ClassSlot::Conformance {
                interface: application.symbol,
                constraint: mir::erase_lifetimes(tree, constraint),
            });
        }

        // memoize the slots
        let slots = Arc::<[ClassSlot]>::from(slots);
        self.class_slots.insert(class, slots.clone());

        Ok(slots)
    }

    /// Return a generic class's own parameters as arguments.
    fn class_parameters(
        &mut self,
        tree: &mut mir::Tree,
        class: dir::GlobalSymbolId,
        scope: &GenericScope,
    ) -> CompilerResult<Vec<mir::GenericArgument>> {
        let source = self.symbol_type(class)?;
        let nominal = self
            .type_lowerer(tree, scope)
            .lower_template_nominal(source, class)?;
        let mir::Type::Declaration { declaration } = tree.get(nominal.storage) else {
            return Err(CompilerError::Internal {
                message: "a generic class without its template declaration".to_string(),
            });
        };

        // name each declared parameter
        let generics = &tree.get(*declaration).generics;

        Ok(generics
            .iter()
            .enumerate()
            .map(|(index, parameter)| parameter.argument(index as u32, tree))
            .collect())
    }

    /// Return a base class's slots at the arguments one heritage applies.
    fn base_slots(
        &mut self,
        tree: &mut mir::Tree,
        heritage: dir::GlobalTypeId,
        scope: &GenericScope,
    ) -> CompilerResult<Vec<ClassSlot>> {
        let base = self.stored(heritage)?;
        let dir::Type::Application(application) = self.ty(base)? else {
            return Err(CompilerError::Internal {
                message: "a class heritage outside an application type".to_string(),
            });
        };
        let lowered = self.type_lowerer(tree, scope).lower_nominal(base)?.storage;
        let arguments = match tree.get(lowered) {
            mir::Type::Application { arguments, .. } => arguments.clone(),
            mir::Type::Declaration { .. } => Vec::new(),
            _ => {
                return Err(CompilerError::Internal {
                    message: "a class base outside a nominal type".to_string(),
                });
            }
        };

        // substitute the base into this class's parameters
        let mut slots = self.class_slots(tree, application.symbol)?.to_vec();
        for slot in &mut slots {
            match slot {
                ClassSlot::Conformance { constraint, .. } => {
                    *constraint = mir::substitute_type(tree, *constraint, &arguments);
                }
                ClassSlot::Method {
                    arguments: inherited,
                    ..
                } => {
                    let mut substitution = mir::Substitution::new(tree, &arguments);
                    for argument in inherited.iter_mut() {
                        *argument = substitution.argument(argument.clone());
                    }
                }
                ClassSlot::Abstract { .. } => {}
            }
        }

        Ok(slots)
    }

    /// Return the function implementing one class slot.
    fn slot_function(
        &mut self,
        tree: &mut mir::Tree,
        symbol: dir::GlobalSymbolId,
        scope: &GenericScope,
    ) -> CompilerResult<mir::FunctionId> {
        let key = GenericInstanceKey::non_generic(symbol);
        if !self.functions.contains_key(&key)
            && let Some(definition) = self.declare_callable(tree, &key, scope)?
        {
            self.pending.push(definition);
        }
        let Some(FunctionDeclaration::Declared(function)) = self.functions.get(&key) else {
            return Err(CompilerError::Internal {
                message: "a class slot without its declared function".to_string(),
            });
        };

        Ok(*function)
    }
}

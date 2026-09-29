use tspp_mir as mir;

use crate::instantiate::state::InstantiateState;
use crate::{CompilerError, CompilerResult};

impl InstantiateState<'_> {
    /// Close the virtual table of every closed class.
    pub(crate) fn declare_class_tables(&mut self) -> CompilerResult<()> {
        // close the inherited methods of each own non-generic class
        let own = self
            .dispatch
            .iter_virtual_tables()
            .filter(|table| {
                !Self::is_template(&self.tree, table.concrete)
                    && table.slots.iter().any(|slot| {
                        matches!(slot, mir::VirtualSlot::Method { arguments, .. } if !arguments.is_empty())
                    })
            })
            .cloned()
            .collect::<Vec<_>>();
        for table in own {
            let slots = self.close_slots(table.slots, &[])?;
            self.dispatch
                .insert_virtual_table(mir::VirtualTable { slots, ..table });
        }

        // specialize the table of each closed class application
        let applications = self
            .tree
            .types()
            .filter_map(|(id, ty)| match ty {
                mir::Type::Application { .. } => Some(id),
                _ => None,
            })
            .filter(|id| {
                self.dispatch.virtual_table(*id).is_none() && !Self::is_open(&self.tree, *id)
            })
            .collect::<Vec<_>>();
        for application in applications {
            let storage = mir::Substitution::resolve(application, &self.tree);
            if matches!(self.tree.get(storage), mir::Type::Class { .. }) {
                self.class_table(application)?;
            }
        }

        // declare the dynamic table of each conformance slot
        let conformances = self
            .dispatch
            .iter_virtual_tables()
            .filter(|table| table.is_concrete() && !Self::is_template(&self.tree, table.concrete))
            .flat_map(|table| {
                table.slots.iter().filter_map(|slot| match slot {
                    mir::VirtualSlot::Conformance { constraint } => {
                        Some((table.value, *constraint))
                    }
                    _ => None,
                })
            })
            .collect::<Vec<_>>();
        for (value, constraint) in conformances {
            self.declare_dynamic_table(value, constraint)?;
        }

        Ok(())
    }

    /// Drop the template tables of generic classes.
    pub(crate) fn drop_template_class_tables(&mut self) {
        let tree = &self.tree;
        self.dispatch
            .retain_virtual_tables(|table| !Self::is_template(tree, table.concrete));
    }

    /// Return whether one class type is a generic class's template declaration.
    fn is_template(tree: &mir::Tree, class: mir::TypeId) -> bool {
        match tree.get(class) {
            mir::Type::Declaration { declaration } => !tree.get(*declaration).generics.is_empty(),
            _ => false,
        }
    }

    /// Return the class type one payload type stores.
    pub(crate) fn class_type(&self, ty: mir::TypeId) -> Option<mir::TypeId> {
        let nominal = match self.tree.type_definition(ty) {
            mir::Type::Reference { pointee, .. } => *pointee,
            _ => ty,
        };
        let storage = mir::Substitution::resolve(nominal, &self.tree);

        matches!(self.tree.get(storage), mir::Type::Class { .. }).then_some(nominal)
    }

    /// Return the virtual table of one class type.
    pub(crate) fn class_table(
        &mut self,
        class: mir::TypeId,
    ) -> CompilerResult<Option<mir::VirtualTable>> {
        if let Some(table) = self.dispatch.virtual_table(class) {
            return Ok(Some(table.clone()));
        }

        match self.tree.get(class).clone() {
            // specialize the template table at the instance's arguments
            mir::Type::Application { base, arguments } => {
                let Some(template) = self.class_table(base)? else {
                    return Err(CompilerError::Internal {
                        message: format!("a class instance {class:?} without its template table"),
                    });
                };
                let slots = self.close_slots(template.slots, &arguments)?;
                let value = self.instance_value(template.value, class)?;
                let table = mir::VirtualTable {
                    concrete: class,
                    value,
                    slots,
                };
                self.dispatch.insert_virtual_table(table.clone());

                Ok(Some(table))
            }
            // import a foreign class's table from its home
            mir::Type::Declaration { declaration } => self.import_class_table(class, declaration),
            _ => Ok(None),
        }
    }

    /// Close each slot of one class table at the class's arguments.
    fn close_slots(
        &mut self,
        slots: Vec<mir::VirtualSlot>,
        arguments: &[mir::GenericArgument],
    ) -> CompilerResult<Vec<mir::VirtualSlot>> {
        let mut closed = Vec::with_capacity(slots.len());
        for slot in slots {
            closed.push(match slot {
                mir::VirtualSlot::Method {
                    function,
                    arguments: inherited,
                } => {
                    let mut substitution = mir::Substitution::new(&self.tree, arguments);
                    let applied = inherited
                        .into_iter()
                        .map(|argument| substitution.argument(argument))
                        .collect::<Vec<_>>();
                    mir::VirtualSlot::Method {
                        function: self.dispatch_direct(function, &applied)?,
                        arguments: Vec::new(),
                    }
                }
                mir::VirtualSlot::Abstract => mir::VirtualSlot::Abstract,
                mir::VirtualSlot::Conformance { constraint } => {
                    let constraint = mir::substitute_type(&self.tree, constraint, arguments);
                    mir::VirtualSlot::Conformance {
                        constraint: mir::erase_lifetimes(&self.tree, constraint),
                    }
                }
            });
        }

        Ok(closed)
    }

    /// Import one foreign class's virtual table.
    fn import_class_table(
        &mut self,
        class: mir::TypeId,
        declaration: mir::LocalNodeId<mir::TypeDeclaration>,
    ) -> CompilerResult<Option<mir::VirtualTable>> {
        if let Some(imported) = self.imported_class_tables.get(&class) {
            return Ok(imported.clone());
        }
        let imported = self.read_class_table(class, declaration)?;
        self.imported_class_tables.insert(class, imported.clone());

        Ok(imported)
    }

    /// Read one foreign class's virtual table from its home into this tree.
    fn read_class_table(
        &mut self,
        class: mir::TypeId,
        declaration: mir::LocalNodeId<mir::TypeDeclaration>,
    ) -> CompilerResult<Option<mir::VirtualTable>> {
        let symbol = self.tree.get(declaration).symbol;
        let Some(module) = symbol.module().filter(|module| *module != self.module) else {
            return Ok(None);
        };
        let source = self.home_source(module)?;
        let table = source.dispatch.iter_virtual_tables().find(|table| {
            matches!(
                source.tree.get(table.concrete),
                mir::Type::Declaration { declaration } if source.tree.get(*declaration).symbol == symbol
            )
        });
        let Some(table) = table.cloned() else {
            return Ok(None);
        };

        // import each slot into this tree
        let slots = table
            .slots
            .iter()
            .map(|slot| match slot {
                mir::VirtualSlot::Method {
                    function,
                    arguments,
                } => mir::VirtualSlot::Method {
                    function: self.import_function(module, &source.tree, *function),
                    arguments: arguments
                        .iter()
                        .map(|argument| self.import_argument(module, argument))
                        .collect(),
                },
                mir::VirtualSlot::Abstract => mir::VirtualSlot::Abstract,
                mir::VirtualSlot::Conformance { constraint } => mir::VirtualSlot::Conformance {
                    constraint: self.import_type(module, *constraint, &[]),
                },
            })
            .collect();

        // close the inherited methods of a non-generic class
        let slots = match Self::is_template(&self.tree, class) {
            true => slots,
            false => self.close_slots(slots, &[])?,
        };

        Ok(Some(mir::VirtualTable {
            concrete: class,
            value: self.import_type(module, table.value, &[]),
            slots,
        }))
    }

    /// Return the value type of one class instance.
    fn instance_value(
        &mut self,
        template: mir::TypeId,
        instance: mir::TypeId,
    ) -> CompilerResult<mir::TypeId> {
        let mut value = self.tree.type_definition(template).clone();
        let mir::Type::Reference { pointee, .. } = &mut value else {
            return Err(CompilerError::Internal {
                message: format!("a class template {template:?} with a non-handle value"),
            });
        };
        *pointee = instance;
        let value = self.tree.intern_type(value);

        Ok(mir::erase_lifetimes(&self.tree, value))
    }

    /// Return whether one type mentions a generic parameter.
    fn is_open(tree: &mir::Tree, ty: mir::TypeId) -> bool {
        if matches!(tree.get(ty), mir::Type::Parameter { .. }) {
            return true;
        }

        // walk the children
        let mut is_open = false;
        tree.get(ty).clone().map_child_type_ids(&mut |child| {
            is_open |= Self::is_open(tree, child);
            child
        });

        is_open
    }
}

use tspp_core::StringId;
use tspp_mir as mir;
use tspp_mir::Substitution;

use crate::instantiate::state::InstantiateState;
use crate::{CompilerError, CompilerResult};

/// What one call in a template body dispatches to once its receiver closes.
pub(crate) enum Dispatch {
    /// The function the call names after substitution.
    Function(mir::FunctionId),
    /// A clone the compiler implements by loading through the receiver.
    Clone,
    /// A drop the compiler answers with an empty body.
    Drop,
    /// The additive identity of the receiver's representation.
    Zero,
    /// The multiplicative identity of the receiver's representation.
    One,
}

impl InstantiateState<'_> {
    /// Return the function one direct call names, an applied template specialized on demand.
    pub(crate) fn dispatch_direct(
        &mut self,
        callee: mir::FunctionId,
        applied: &[mir::GenericArgument],
    ) -> CompilerResult<mir::FunctionId> {
        if applied.is_empty() {
            return Ok(callee);
        }

        self.specialization(callee, applied.to_vec())
    }

    /// Return what one witness call dispatches to at its closed receiver.
    pub(crate) fn dispatch_witness(
        &mut self,
        receiver: mir::TypeId,
        interface: mir::TypeId,
        requirement: mir::FunctionId,
        applied: &[mir::GenericArgument],
    ) -> CompilerResult<Dispatch> {
        // normalize types introduced by generic substitution
        let receiver = mir::erase_lifetimes(&self.tree, receiver);
        let interface = mir::erase_lifetimes(&self.tree, interface);
        let Some(witness) = self.witnesses.get(receiver, interface) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "a witness call to '{}' at a closed receiver without a witness",
                    self.strings.get(self.tree.get(requirement).name)
                ),
            });
        };

        // take the function the witness selects for this requirement
        let named = witness
            .functions
            .iter()
            .find(|function| function.requirement == requirement)
            .cloned();
        if let Some(function) = named {
            if !self.tree.get(function.function).is_polymorphic() {
                return Ok(Dispatch::Function(function.function));
            }
            let Some(arguments) = function.fill(applied) else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "a witness call to '{}' with {} arguments for the implementer's places",
                        self.strings.get(self.tree.get(requirement).name),
                        applied.len()
                    ),
                });
            };

            return self
                .specialization(function.function, arguments)
                .map(Dispatch::Function);
        }

        // specialize the requirement's default body at the receiver and its own arguments
        let symbol = self.tree.get(requirement).symbol;
        if self.templates.contains_key(&symbol) {
            let mut arguments = vec![mir::GenericArgument::Type(receiver)];
            if let mir::Type::Application {
                arguments: interface_arguments,
                ..
            } = self.tree.get(interface)
            {
                arguments.extend(interface_arguments.iter().cloned());
            }
            arguments.extend(applied.iter().cloned());
            let specialization = self.specialization(requirement, arguments)?;

            return Ok(Dispatch::Function(specialization));
        }

        self.intrinsic_requirement(interface, requirement)
    }

    /// Declare the dynamic tables one function body binds.
    pub(crate) fn declare_dynamic_tables(
        &mut self,
        function: mir::FunctionId,
    ) -> CompilerResult<()> {
        let function = self.tree.get(function);
        let Some(body) = &function.body else {
            return Ok(());
        };

        // collect each erased concrete type and its constraint
        let mut erasures = Vec::new();
        for block in body.blocks() {
            for instruction in &self.tree.get(*block).instructions {
                if let mir::Instruction::DynamicBind {
                    destination,
                    concrete,
                    ..
                } = self.tree.get(*instruction)
                {
                    let dynamic = function.value_type(*destination).ok_or_else(|| {
                        CompilerError::Internal {
                            message: "an erasure without a typed destination".to_string(),
                        }
                    })?;
                    let mir::Type::Dynamic { constraint, .. } = *self.tree.type_definition(dynamic)
                    else {
                        return Err(CompilerError::Internal {
                            message: "an erasure outside a dynamic destination".to_string(),
                        });
                    };
                    erasures.push((*concrete, constraint));
                }
            }
        }

        // declare the implementations before the function worklist drains
        for (concrete, constraint) in erasures {
            self.declare_dynamic_table(concrete, constraint)?;
        }

        Ok(())
    }

    /// Record the dynamic table one closed concrete type answers a constraint with, once.
    pub(crate) fn declare_dynamic_table(
        &mut self,
        concrete: mir::TypeId,
        constraint: mir::TypeId,
    ) -> CompilerResult<()> {
        if self.dispatch.dynamic_table(concrete, constraint).is_some() {
            return Ok(());
        }
        let Some(shape) = self.dispatch.dynamic_shape(constraint).cloned() else {
            return Err(CompilerError::Internal {
                message: "an erasure without a registered constraint shape".to_string(),
            });
        };
        let witness = self.witnesses.get(concrete, constraint).cloned();

        // lay out the concrete storage before recording field offsets
        let storage = match self.tree.type_definition(concrete) {
            mir::Type::Reference { pointee, .. } => *pointee,
            _ => concrete,
        };
        let storage = self.tree.storage_type(storage);
        mir::LayoutBuilder::new(&self.tree, &mut self.layouts, self.layout)
            .witnesses(&self.witnesses)
            .layout_type(storage)
            .map_err(|error| CompilerError::Internal {
                message: format!("dynamic storage layout failed: {error:?}"),
            })?;
        let fields = self.layouts.named_field_offsets(storage);

        // fill one entry per constraint slot from the layout and the witness
        let mut entries = Vec::with_capacity(shape.slots.len());
        for slot in &shape.slots {
            let entry = match slot {
                // read a field at its laid-out offset, an absent optional field as undefined
                mir::DynamicSlot::Field { name, .. } => {
                    match fields.iter().find(|(field, _)| field == name) {
                        Some((_, offset)) => mir::DynamicEntry::Field { offset: *offset },
                        None => mir::DynamicEntry::Absent,
                    }
                }
                mir::DynamicSlot::Function {
                    name: Some(name), ..
                } => {
                    // select the witness function implementing the slot's member
                    let function = witness.as_ref().and_then(|witness| {
                        witness
                            .functions
                            .iter()
                            .find(|function| function.member == *name)
                            .cloned()
                    });
                    let Some(function) = function else {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "a dynamic slot '{}' without an implementing function",
                                self.strings.get(*name)
                            ),
                        });
                    };
                    mir::DynamicEntry::Function {
                        function: function.function,
                    }
                }
                mir::DynamicSlot::Function { name: None, .. } => {
                    return Err(CompilerError::Internal {
                        message: "a call-signature constraint member".to_string(),
                    });
                }
            };
            entries.push(entry);
        }

        // index the concrete fields by name for keyed constraints
        let mut names = Vec::new();
        if shape.is_keyed {
            names.extend(fields.iter().map(|(name, offset)| mir::DynamicNamedEntry {
                name: *name,
                entry: mir::DynamicEntry::Field { offset: *offset },
            }));
            names.sort_by(|left, right| {
                self.strings
                    .get(left.name)
                    .cmp(self.strings.get(right.name))
            });
        }
        self.dispatch.insert_dynamic_table(mir::DynamicTable {
            concrete,
            constraint,
            entries,
            names,
        });

        Ok(())
    }

    /// Return the global one closed receiver answers an interface's associated const with.
    pub(crate) fn witness_constant(
        &mut self,
        receiver: mir::TypeId,
        interface: mir::TypeId,
        member: StringId,
    ) -> CompilerResult<mir::GlobalId> {
        // normalize types introduced by generic substitution
        let receiver = mir::erase_lifetimes(&self.tree, receiver);
        let interface = mir::erase_lifetimes(&self.tree, interface);
        let Some(witness) = self.witnesses.get(receiver, interface) else {
            return Err(CompilerError::Internal {
                message: format!(
                    "a witness const '{}' read at a closed receiver without a witness",
                    self.strings.get(member)
                ),
            });
        };

        witness
            .constants
            .iter()
            .find(|constant| constant.member == member)
            .map(|constant| constant.global)
            .ok_or_else(|| CompilerError::Internal {
                message: format!(
                    "a witness without the associated const '{}'",
                    self.strings.get(member)
                ),
            })
    }

    /// Return the structural implementation of one requirement the witness leaves unnamed.
    fn intrinsic_requirement(
        &self,
        interface: mir::TypeId,
        requirement: mir::FunctionId,
    ) -> CompilerResult<Dispatch> {
        match mir::LanguageItem::of_type(&self.tree, interface) {
            Some(mir::LanguageItem::Clone) => Ok(Dispatch::Clone),
            Some(mir::LanguageItem::Drop) => Ok(Dispatch::Drop),
            Some(mir::LanguageItem::Zero) => Ok(Dispatch::Zero),
            Some(mir::LanguageItem::One) => Ok(Dispatch::One),
            Some(mir::LanguageItem::Copy) | None => Err(CompilerError::Internal {
                message: format!(
                    "an intrinsic '{}' requirement outside the structural set",
                    self.strings.get(self.tree.get(requirement).name)
                ),
            }),
        }
    }

    /// Return the specialization of one template at closed arguments, declared on first demand.
    fn specialization(
        &mut self,
        template: mir::FunctionId,
        arguments: Vec<mir::GenericArgument>,
    ) -> CompilerResult<mir::FunctionId> {
        let declared = self.tree.get(template).clone();
        let symbol = declared.symbol.instantiate(&arguments, &self.tree);
        if let Some(existing) = self.functions.get(&symbol) {
            return Ok(*existing);
        }

        // declare the header at the arguments, the body arriving from the template
        let parameters = declared
            .parameters
            .iter()
            .map(|parameter| {
                let ty = self.close_type(parameter.ty, &arguments);
                mir::FunctionParameter::new(parameter.value, ty)
            })
            .collect();
        let return_type = self.close_type(declared.return_type, &arguments);
        let specialization = mir::Function {
            generics: Vec::new(),
            arguments,
            symbol,
            linkage: mir::Linkage::Shared,
            parameters,
            return_type,
            template: Some(template),
            body: None,
            ..declared
        };
        let id = self.tree.insert(specialization);
        self.functions.insert(symbol, id);
        self.pending.push(id);

        Ok(id)
    }

    /// Substitute one type of this tree at the arguments and represent its closed applications.
    pub(crate) fn close_type(
        &mut self,
        ty: mir::TypeId,
        arguments: &[mir::GenericArgument],
    ) -> mir::TypeId {
        let closed = Substitution::new(&self.tree, arguments).ty(ty);

        mir::resolve_witness_types(&self.tree, &self.witnesses, closed)
    }
}

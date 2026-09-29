use tspp_core::StringId;
use tspp_mir as mir;
use tspp_mir::Substitution;

use crate::instantiate::state::InstantiateState;
use crate::{CompilerError, CompilerResult};

/// What one call in a template body dispatches to once its receiver closes.
pub(crate) enum Dispatch {
    /// The function the call names after substitution.
    Function(mir::FunctionId),
    /// The slot the call reaches through its receiver's virtual table.
    Virtual(mir::DispatchSlot),
    /// The slot the call reaches through its dynamic receiver's table.
    Dynamic {
        /// The dispatching constraint.
        constraint: mir::TypeId,
        /// The dynamic slot.
        slot: mir::DispatchSlot,
    },
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
        let Some(witness) = self.witness(receiver, interface)? else {
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
        match named.map(|named| named.implementation) {
            // call a virtual implementation through the receiver's table
            Some(mir::WitnessImplementation::Virtual { slot }) => {
                return Ok(Dispatch::Virtual(slot));
            }
            // call a dynamic receiver's slot through its own table
            Some(mir::WitnessImplementation::Dynamic { slot }) => {
                return Ok(Dispatch::Dynamic {
                    constraint: interface,
                    slot,
                });
            }
            Some(mir::WitnessImplementation::Function {
                function,
                arguments,
            }) => {
                if !self.tree.get(function).is_polymorphic() {
                    return Ok(Dispatch::Function(function));
                }
                let Some(arguments) = mir::WitnessImplementation::fill(&arguments, applied) else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "a witness call to '{}' with {} arguments for the implementer's places",
                            self.strings.get(self.tree.get(requirement).name),
                            applied.len()
                        ),
                    });
                };

                return self
                    .specialization(function, arguments)
                    .map(Dispatch::Function);
            }
            Some(mir::WitnessImplementation::Default) | None => {}
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
                    erasures.push((*instruction, *concrete, constraint));
                }
            }
        }

        // bind a class through its conformance slot
        for (instruction, concrete, constraint) in erasures {
            match self.class_type(concrete) {
                Some(class) => {
                    let slot = self.conformance_slot(class, constraint)?;
                    let mir::Instruction::DynamicBind { table, .. } =
                        self.tree.get_mut(instruction)
                    else {
                        return Err(CompilerError::Internal {
                            message: "an erasure outside a dynamic binding".to_string(),
                        });
                    };
                    *table = mir::BindTable::Virtual { slot };
                }
                None => self.declare_dynamic_table(concrete, constraint)?,
            }
        }

        Ok(())
    }

    /// Return the conformance slot of one constraint in one class's table.
    fn conformance_slot(
        &mut self,
        class: mir::TypeId,
        constraint: mir::TypeId,
    ) -> CompilerResult<mir::DispatchSlot> {
        let constraint = mir::erase_lifetimes(&self.tree, constraint);
        let Some(table) = self.class_table(class)? else {
            return Err(CompilerError::Internal {
                message: format!("an erased class {class:?} without its virtual table"),
            });
        };
        let slot = table.slots.iter().position(|slot| {
            matches!(slot, mir::VirtualSlot::Conformance { constraint: slotted } if *slotted == constraint)
        });
        let Some(slot) = slot else {
            return Err(CompilerError::Internal {
                message: format!("an erased class {class:?} without a conformance slot"),
            });
        };

        Ok(mir::DispatchSlot::new(slot as u32))
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
        let shape = self.shapes.shape(&self.tree, constraint);
        let Some(shape) = shape.map(|(shape, _)| shape.clone()) else {
            return Err(CompilerError::Internal {
                message: "an erasure without a registered constraint shape".to_string(),
            });
        };
        // lay out the concrete storage before recording field offsets
        let nominal = match self.tree.type_definition(concrete) {
            mir::Type::Reference { pointee, .. } => *pointee,
            _ => concrete,
        };
        let storage = nominal.storage(&self.tree);
        let mut layouts = mir::LayoutBuilder::new(&self.tree, &mut self.layouts, self.layout)
            .witnesses(&self.witnesses);
        for ty in [storage, constraint] {
            layouts
                .layout_type(ty)
                .map_err(|error| CompilerError::Internal {
                    message: format!("dynamic storage layout failed: {error:?}"),
                })?;
        }
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
                // dispatch a method slot like its witness call
                mir::DynamicSlot::Function { requirement, .. } => {
                    let function = self.witness(concrete, constraint)?.and_then(|witness| {
                        witness
                            .functions
                            .iter()
                            .map(|function| function.requirement)
                            .find(|function| self.tree.get(*function).symbol == *requirement)
                    });
                    let Some(function) = function else {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "a dynamic slot {requirement:?} without its witness requirement"
                            ),
                        });
                    };
                    self.dynamic_function(concrete, constraint, nominal, function)?
                }
            };
            entries.push(entry);
        }

        // index the concrete fields by name for keyed constraints
        let mut names = Vec::new();
        if shape.lookup == mir::ShapeLookup::Name {
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

    /// Return the dynamic entry one method slot holds.
    fn dynamic_function(
        &mut self,
        concrete: mir::TypeId,
        constraint: mir::TypeId,
        nominal: mir::TypeId,
        requirement: mir::FunctionId,
    ) -> CompilerResult<mir::DynamicEntry> {
        match self.dispatch_witness(concrete, constraint, requirement, &[])? {
            Dispatch::Function(function) => {
                let function = self.payload_shim(function, concrete)?;

                Ok(mir::DynamicEntry::Function { function })
            }
            // take the concrete class's own method for a virtual slot
            Dispatch::Virtual(slot) => {
                let table = self.class_table(nominal)?;
                let method = table.and_then(|table| table.slots.get(slot.index()).cloned());
                let Some(mir::VirtualSlot::Method {
                    function,
                    arguments,
                }) = method
                else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "a dynamic slot {requirement:?} on a class slot without a method"
                        ),
                    });
                };
                if !arguments.is_empty() {
                    return Err(CompilerError::Internal {
                        message: format!("a dynamic slot {requirement:?} on an open class method"),
                    });
                }

                Ok(mir::DynamicEntry::Function { function })
            }
            Dispatch::Dynamic { .. }
            | Dispatch::Clone
            | Dispatch::Drop
            | Dispatch::Zero
            | Dispatch::One => Err(CompilerError::Internal {
                message: format!(
                    "a dynamic slot {requirement:?} without a concrete implementation"
                ),
            }),
        }
    }

    /// Return the function a table slot calls with its payload.
    fn payload_shim(
        &mut self,
        function: mir::FunctionId,
        concrete: mir::TypeId,
    ) -> CompilerResult<mir::FunctionId> {
        // keep an implementation taking the payload address
        let declared = self.tree.get(function).clone();
        let Some(receiver) = declared.parameters.first() else {
            return Err(CompilerError::Internal {
                message: "a dynamic slot implementation without a receiver".to_string(),
            });
        };
        let is_boxed = !matches!(
            self.tree.type_definition(concrete),
            mir::Type::Reference { .. } | mir::Type::Parameter { .. }
        );
        if !is_boxed || mir::erase_lifetimes(&self.tree, receiver.ty) != concrete {
            return Ok(function);
        }

        // copy the receiver out of the box
        if !mir::is_copy(&self.tree, concrete, &[]) {
            return Err(CompilerError::Internal {
                message: format!(
                    "a dynamic slot implementation '{}' consuming a boxed receiver",
                    self.strings.get(declared.name)
                ),
            });
        }

        // declare one shared shim per implementation
        let name = format!("{}.shim", self.strings.get(declared.name));
        let name = self.strings.intern(&name);
        let symbol = declared.symbol.shim();
        if let Some(existing) = self.functions.get(&symbol) {
            return Ok(*existing);
        }
        // append the payload lifetime
        let payload_lifetime = declared.lifetimes.len() as u32;
        let mut lifetimes = declared.lifetimes.clone();
        lifetimes.push(mir::LifetimeParameter::new(Some(
            self.strings.intern("'payload"),
        )));
        let payload = self.tree.intern_type(mir::Type::Reference {
            kind: mir::Reference::Borrowed,
            lifetime: mir::Lifetime::bound(payload_lifetime),
            access: mir::Access::Readonly,
            pointee: concrete,
        });
        let mut parameters = vec![mir::FunctionParameter::new(mir::Value::new(0), payload)];
        for (index, parameter) in declared.parameters.iter().enumerate().skip(1) {
            let value = mir::Value::new(index as u32);
            parameters.push(mir::FunctionParameter::new(value, parameter.ty));
        }
        let mut shim = mir::Function::declare(
            self.module,
            name,
            lifetimes,
            parameters,
            declared.return_type,
        )
        .with_symbol(symbol);
        shim.linkage = mir::Linkage::Shared;
        let shim = self.tree.insert(shim);
        self.functions.insert(symbol, shim);

        // load the receiver and forward the arguments
        let pointer_bits = self.layout.pointer.width_bits();
        let mut builder =
            mir::FunctionBuilder::from_declared(self.module, &mut self.tree, pointer_bits, shim)
                .map_err(|error| CompilerError::Internal {
                    message: format!("a dynamic shim failed to build: {error}"),
                })?;
        let block = builder.block();
        builder.switch_to_block(block);
        let payload = builder.function_parameter(0);
        let place = mir::Place::value(payload).with_projection(mir::Projection::Deref);
        let mut values = vec![builder.load(place, concrete)];
        values
            .extend((1..declared.parameters.len()).map(|index| builder.function_parameter(index)));

        // call the implementation and return its result
        let signature = builder.tree_mut().intern_type(declared.signature());
        let callee = mir::Callee::Direct {
            function,
            arguments: Vec::new(),
        };
        let result = builder.call(callee, signature, values, declared.return_type);
        builder.return_(result);
        builder.finish().map_err(|error| CompilerError::Internal {
            message: format!("a dynamic shim failed to build: {error}"),
        })?;

        Ok(shim)
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
        let Some(witness) = self.witness(receiver, interface)? else {
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
            Some(
                mir::LanguageItem::Copy
                | mir::LanguageItem::String
                | mir::LanguageItem::BigInt
                | mir::LanguageItem::StringEqual
                | mir::LanguageItem::BigIntEqual,
            )
            | None => Err(CompilerError::Internal {
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
        let environment = declared
            .environment
            .map(|environment| self.close_type(environment, &arguments));
        let specialization = mir::Function {
            generics: Vec::new(),
            arguments,
            symbol,
            linkage: mir::Linkage::Shared,
            parameters,
            return_type,
            environment,
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

use destack_core::FxIndexMap;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::instantiate::module::Dispatch;
use crate::instantiate::state::InstantiateState;
use crate::{CompilerError, CompilerResult};

/// One specialization taking its template's body at its arguments.
pub(crate) struct Specialization<'s, 'a> {
    /// The instantiation the body is copied into.
    pub(crate) state: &'s mut InstantiateState<'a>,
    /// The module defining the template.
    pub(crate) module: ModuleId,
    /// The tree defining the template.
    pub(crate) source: &'s mir::Tree,
    /// The copied template.
    pub(crate) template: mir::FunctionId,
    /// The arguments the specialization closes the template at.
    pub(crate) arguments: Vec<mir::GenericArgument>,
    /// The copied local of each template local.
    pub(crate) locals: FxIndexMap<mir::LocalId, mir::LocalId>,
    /// The copied block of each template block.
    pub(crate) blocks: FxIndexMap<mir::BlockId, mir::BlockId>,
    /// The types of the values the copy adds past the template's, in allocation order.
    pub(crate) added_values: Vec<Option<mir::TypeId>>,
}

impl Specialization<'_, '_> {
    /// Map one source type under the arguments.
    pub(super) fn ty(&mut self, ty: mir::TypeId) -> mir::TypeId {
        let arguments = self.arguments.clone();

        self.state.import_type(self.module, ty, &arguments)
    }

    /// Copy one template body into a body over this tree's nodes.
    pub(crate) fn body(
        &mut self,
        accesses: &mir::AccessTable,
    ) -> CompilerResult<mir::FunctionBody> {
        let function = self.source.get(self.template).clone();
        let Some(body) = function.body() else {
            return Err(CompilerError::Internal {
                message: "an instantiated template without a body".to_string(),
            });
        };

        // copy the locals and reserve every block ahead of the instructions naming them
        let mut locals = Vec::with_capacity(body.locals().len());
        for local in body.locals() {
            let declared = self.source.get(*local).clone();
            let ty = self.ty(declared.ty);
            let copied = self.state.tree.insert(mir::Local { ty, ..declared });
            self.locals.insert(*local, copied);
            locals.push(copied);
        }
        let mut blocks = Vec::with_capacity(body.blocks().len());
        for block in body.blocks() {
            let declared = self.source.get(*block);
            let parameters = declared
                .parameters
                .iter()
                .map(|parameter| mir::BlockParameter {
                    value: parameter.value,
                    ty: self.ty(parameter.ty),
                })
                .collect();
            let terminator = self.state.tree.insert(mir::Terminator::Unreachable);
            let copied = self.state.tree.insert(mir::Block {
                parameters,
                instructions: Vec::new(),
                terminator,
            });
            self.blocks.insert(*block, copied);
            blocks.push(copied);
        }

        // copy the instructions and terminators
        for block in body.blocks() {
            let copied = self.blocks[block];
            let declared = self.source.get(*block).clone();
            let mut instructions = Vec::with_capacity(declared.instructions.len());
            for instruction in declared.instructions {
                for mapped in self.instruction(instruction)? {
                    let inserted = self.state.tree.insert(mapped);
                    if let Some(span) = self.source.get_span(instruction) {
                        self.state.tree.set_span(inserted, span);
                    }
                    if let Some(entries) = accesses.get(instruction) {
                        self.state.accesses.insert(inserted, entries.to_vec());
                    }
                    instructions.push(inserted);
                }
            }
            let terminator = self.terminator(declared.terminator)?;
            let terminator_id = self.state.tree.get(copied).terminator;
            self.state.tree.set(terminator_id, terminator);
            self.state.tree.get_mut(copied).instructions = instructions;
        }

        // type every value under the arguments, the added values after the template's
        let mut value_types = body
            .value_types()
            .iter()
            .map(|ty| ty.map(|ty| self.ty(ty)))
            .collect::<Vec<_>>();
        value_types.extend(self.added_values.iter().copied());
        let entry = self.blocks[&body.entry()];
        let next_value_id = value_types.len() as u32;

        Ok(mir::FunctionBody::new(
            entry,
            blocks,
            locals,
            value_types,
            next_value_id,
            &self.state.tree,
        ))
    }

    /// Allocate one value past the template's, typed in this tree.
    fn added_value(&mut self, ty: mir::TypeId) -> mir::Value {
        let template = self.source.get(self.template);
        let count =
            template.body().map_or(0, |body| body.value_types().len()) + self.added_values.len();
        self.added_values.push(Some(ty));

        mir::Value(count as u32)
    }

    /// Copy one instruction with its ids mapped and its requirements resolved.
    fn instruction(
        &mut self,
        id: mir::LocalNodeId<mir::Instruction>,
    ) -> CompilerResult<Vec<mir::Instruction>> {
        let mut instruction = self.source.get(id).clone();

        // map every id into this tree before the callees dispatch on the mapped types
        instruction.map_ids(self);

        // rewrite the ids the id map leaves to the specialization
        match &mut instruction {
            mir::Instruction::LocalGet { local, .. }
            | mir::Instruction::LocalAddr { local, .. }
            | mir::Instruction::LocalSet { local, .. } => *local = self.locals[&*local],
            mir::Instruction::Aggregate { values, .. } => {
                *values = self.values(*values);
            }
            mir::Instruction::VectorShuffle { mask, .. } => {
                let indices = self.source.get_indices(*mask).to_vec();
                *mask = self.state.tree.add_indices(&indices);
            }
            mir::Instruction::Intrinsic { arguments, .. } => *arguments = self.values(*arguments),
            mir::Instruction::VariantNew {
                destination, case, ..
            } => *case = self.case(*destination, *case)?,
            mir::Instruction::VariantPayload { variant, case, .. }
            | mir::Instruction::VariantPayloadAddr { variant, case, .. } => {
                *case = self.case(*variant, *case)?;
            }
            mir::Instruction::Const {
                destination,
                value:
                    mir::Constant::Witness {
                        receiver,
                        interface,
                        member,
                    },
            } => {
                let global = self
                    .state
                    .witness_constant(*receiver, *interface, *member)?;

                return Ok(self.global_read(*destination, global));
            }
            mir::Instruction::Const {
                destination,
                value: value @ mir::Constant::Parameter(_),
            } => {
                let index = match *value {
                    mir::Constant::Parameter(index) => index,
                    _ => unreachable!("a constant parameter matched above"),
                };
                let Some(result) = self.source.get(self.template).value_type(*destination) else {
                    return Err(CompilerError::Internal {
                        message: "a constant parameter without a typed destination".to_string(),
                    });
                };
                let ty = self.ty(result);
                *value = self.constant_argument(index, ty)?;
            }
            mir::Instruction::FunctionAddr {
                function,
                arguments: applied,
                ..
            }
            | mir::Instruction::FunctionBind {
                function,
                arguments: applied,
                ..
            } if !applied.is_empty() => {
                *function = self.state.dispatch_direct(*function, applied)?;
                applied.clear();
            }
            mir::Instruction::Call { destination, call } => {
                let arguments = self.values(call.arguments);
                match self.dispatch(&call.callee)? {
                    Some(Dispatch::Function(function)) => {
                        call.callee = mir::Callee::Direct {
                            function,
                            arguments: Vec::new(),
                        };
                    }
                    Some(dispatch) => {
                        return self.structural_call(dispatch, *destination, arguments);
                    }
                    None => {}
                }
                call.arguments = arguments;
            }
            _ => {}
        }

        Ok(vec![instruction])
    }

    /// Replace one structural requirement call by the instructions its representation answers with.
    fn structural_call(
        &mut self,
        dispatch: Dispatch,
        destination: Option<mir::Value>,
        arguments: mir::ValueSlice,
    ) -> CompilerResult<Vec<mir::Instruction>> {
        match dispatch {
            Dispatch::Function(_) => unreachable!("a function dispatch rewrites the callee"),
            // a clone loads through the receiver
            Dispatch::Clone => {
                let Some(destination) = destination else {
                    return Err(CompilerError::Internal {
                        message: "a clone call without a destination".to_string(),
                    });
                };
                let Some(&pointer) = self.state.tree.get_values(arguments).first() else {
                    return Err(CompilerError::Internal {
                        message: "a clone call without a receiver".to_string(),
                    });
                };
                let Some(result) = self.source.get(self.template).value_type(destination) else {
                    return Err(CompilerError::Internal {
                        message: "a clone call without a typed destination".to_string(),
                    });
                };
                let result_type = self.ty(result);

                Ok(vec![mir::Instruction::Load {
                    destination,
                    pointer,
                    result_type,
                }])
            }
            // answer an empty body for a drop dispatch
            Dispatch::Drop => Ok(Vec::new()),
            // an identity is the constant of the destination's representation
            Dispatch::Zero | Dispatch::One => {
                let (destination, ty) = self.structural_result(destination)?;
                let value = self.identity_constant(ty, &dispatch)?;

                Ok(vec![mir::Instruction::Const { destination, value }])
            }
        }
    }

    /// Read one global into a destination: its address projected, then loaded.
    fn global_read(
        &mut self,
        destination: mir::Value,
        global: mir::GlobalId,
    ) -> Vec<mir::Instruction> {
        let declared = self.state.tree.get(global);
        let result_type = declared.ty;
        let space = declared.space;
        let pointer_type = self.state.tree.intern_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::empty(),
            storage: mir::Storage::global(space),
            access: mir::Access::Readonly,
            pointee: result_type,
        });
        let pointer = self.added_value(pointer_type);

        vec![
            mir::Instruction::GlobalAddr {
                destination: pointer,
                global,
                result_type: pointer_type,
                kind: mir::AddressKind::Projection,
            },
            mir::Instruction::Load {
                destination,
                pointer,
                result_type,
            },
        ]
    }

    /// Copy one terminator, its targets and ids mapped.
    fn terminator(
        &mut self,
        id: mir::LocalNodeId<mir::Terminator>,
    ) -> CompilerResult<mir::Terminator> {
        let mut terminator = self.source.get(id).clone();

        // map every id into this tree before the callees dispatch on the mapped types
        terminator.map_ids(self);

        // rewrite the blocks and values the terminator names
        match &mut terminator {
            mir::Terminator::Jump { target } => self.target(target),
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                self.target(then_target);
                self.target(else_target);
            }
            mir::Terminator::Check {
                success, failure, ..
            }
            | mir::Terminator::NewZeroedTry {
                success, failure, ..
            }
            | mir::Terminator::NewUninitTry {
                success, failure, ..
            }
            | mir::Terminator::NewSliceZeroedTry {
                success, failure, ..
            }
            | mir::Terminator::NewSliceUninitTry {
                success, failure, ..
            } => {
                self.target(success);
                self.target(failure);
            }
            mir::Terminator::Switch { default, cases, .. } => {
                self.target(default);
                *cases = self.cases(*cases);
            }
            mir::Terminator::VariantSwitch {
                value,
                default,
                cases,
            } => {
                if let Some(default) = default {
                    self.target(default);
                }
                let mut mapped = self.source.get_switch_cases(*cases).to_vec();
                for case in &mut mapped {
                    self.target(&mut case.target);
                    case.value = i128::from(self.case(*value, case.value as u32)?);
                }
                *cases = self.state.tree.add_switch_cases(&mapped);
            }
            mir::Terminator::Invoke {
                call,
                target,
                unwind,
            } => {
                self.target(target);
                self.target(unwind);
                call.arguments = self.values(call.arguments);
                self.dispatch_call(call, "an unwind edge")?;
            }
            mir::Terminator::TailCall { call } => {
                call.arguments = self.values(call.arguments);
                self.dispatch_call(call, "tail position")?;
            }
            mir::Terminator::Error
            | mir::Terminator::Return { .. }
            | mir::Terminator::Panic { .. }
            | mir::Terminator::UnwindResume
            | mir::Terminator::Abort { .. }
            | mir::Terminator::Unreachable => {}
        }
        Ok(terminator)
    }

    /// Rewrite the callee of one terminator call.
    fn dispatch_call(&mut self, call: &mut mir::Call, position: &str) -> CompilerResult<()> {
        match self.dispatch(&call.callee)? {
            Some(Dispatch::Function(function)) => {
                call.callee = mir::Callee::Direct {
                    function,
                    arguments: Vec::new(),
                };
            }
            Some(Dispatch::Clone | Dispatch::Drop | Dispatch::Zero | Dispatch::One) => {
                return Err(CompilerError::Internal {
                    message: format!("a structural requirement called with {position}"),
                });
            }
            None => {}
        }

        Ok(())
    }

    /// Dispatch one callee of the template under the arguments, absent when the id maps settle it.
    fn dispatch(&mut self, callee: &mir::Callee) -> CompilerResult<Option<Dispatch>> {
        match callee {
            mir::Callee::Direct {
                function,
                arguments: applied,
            } => self
                .state
                .dispatch_direct(*function, applied)
                .map(|function| Some(Dispatch::Function(function))),
            mir::Callee::Witness {
                receiver,
                interface,
                requirement,
            } => self
                .state
                .dispatch_witness(*receiver, *interface, *requirement)
                .map(Some),
            mir::Callee::Indirect { .. }
            | mir::Callee::Virtual { .. }
            | mir::Callee::Dynamic { .. } => Ok(None),
        }
    }

    /// Copy one value slice into this tree.
    fn values(&mut self, slice: mir::ValueSlice) -> mir::ValueSlice {
        let values = self.source.get_values(slice).to_vec();
        self.state.tree.add_values(&values)
    }

    /// Map one block target's block and copy its arguments.
    fn target(&mut self, target: &mut mir::BlockTarget) {
        target.block = self.blocks[&target.block];
        target.arguments = self.values(target.arguments);
    }

    /// Map one case of the variant a template value holds to the instance's case of that payload.
    fn case(&mut self, value: mir::Value, case: u32) -> CompilerResult<u32> {
        // read the variant the template value holds
        let Some(held) = self.source.get(self.template).value_type(value) else {
            return Err(CompilerError::Internal {
                message: "a variant case on an untyped template value".to_string(),
            });
        };
        let variant = match self.source.get(held) {
            mir::Type::Reference { pointee, .. } | mir::Type::Pointer { pointee, .. } => *pointee,
            _ => held,
        };
        let variant = self.source.represented(variant);
        if self.source.type_symbol(variant).is_some() {
            return Ok(case);
        }

        // find the case's payload in the instance's variant
        let Some(payload) = self.source.case_payload(variant, case) else {
            return Err(CompilerError::Internal {
                message: "a variant case outside the template's variant".to_string(),
            });
        };
        let payload = self.ty(payload);
        let instance = self.ty(variant);
        let instance = self.state.tree.represented(instance);

        self.state
            .tree
            .payload_case(instance, payload)
            .ok_or_else(|| CompilerError::Internal {
                message: "a variant case outside the instance's variant".to_string(),
            })
    }

    /// Copy one switch case slice with its targets mapped.
    fn cases(&mut self, slice: mir::SwitchCaseSlice) -> mir::SwitchCaseSlice {
        let mut cases = self.source.get_switch_cases(slice).to_vec();
        for case in &mut cases {
            self.target(&mut case.target);
        }
        self.state.tree.add_switch_cases(&cases)
    }
}

impl mir::IdRemap for Specialization<'_, '_> {
    fn map_type(&mut self, ty: mir::TypeId) -> mir::TypeId {
        self.ty(ty)
    }

    fn map_function(&mut self, function: mir::FunctionId) -> mir::FunctionId {
        self.state
            .import_function(self.module, self.source, function)
    }

    fn map_global(&mut self, global: mir::GlobalId) -> mir::GlobalId {
        self.state.import_global(self.module, self.source, global)
    }
}

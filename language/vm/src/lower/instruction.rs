use destack_mir as mir;

use crate::{ReferenceAddressSpace, Word};
use destack_heap::{HeapReference, RawPointer, SharedHeapReference, SharedRawPointer};

use crate::program::{
    ConstValue, Instruction, Opcode, Operands, PointeeAccess, PointerClass, ValueRepr,
    pack_optional_value, value_repr_from_type,
};
use crate::{Error, Result};

use super::access::*;
use super::lower::BlockLowerer;
use super::opcode::{
    select_binary_opcode_for_repr, select_element_addr_opcode, select_element_get_opcode,
    select_element_load_opcode, select_element_store_opcode, select_field_addr_opcode,
    select_field_get_opcode, select_field_load_opcode, select_field_store_opcode,
    select_load_opcode, select_specialized_const_int_opcode, select_specialized_int_opcode,
    select_store_opcode, select_unary_opcode,
};
use super::pool::Pool;
use super::repr::{
    heap_pointee_type_for_value, heap_pointee_type_for_value_repr, pointer_class_for_value,
    raw_pointee_type_for_value, raw_pointee_type_for_value_repr, reference_meta_for_type,
    reference_meta_for_value, value_type_for_value as lookup_value_type_for_value,
};

impl<'a> BlockLowerer<'a> {
    /// Lower one MIR instruction into zero or more VM instructions.
    pub(super) fn lower_instructions(
        &self,
        inst: &mir::Instruction,
        pool: &mut Pool,
    ) -> Result<Vec<Instruction>> {
        match inst {
            mir::Instruction::Struct {
                destination,
                fields,
                ..
            } => {
                let destination =
                    destination
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "struct destination".to_string(),
                        })?;
                let values = self.tree.get_arguments(*fields);

                self.lower_frame_init(destination, values)
            }
            mir::Instruction::Tuple {
                destination,
                elements,
                ..
            } => {
                let destination =
                    destination
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tuple destination".to_string(),
                        })?;
                let values = self.tree.get_arguments(*elements);

                self.lower_frame_init(destination, values)
            }
            mir::Instruction::Array {
                destination,
                elements,
                ..
            } => {
                let destination =
                    destination
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "array destination".to_string(),
                        })?;
                let values = self.tree.get_arguments(*elements);

                self.lower_frame_init(destination, values)
            }
            mir::Instruction::FieldSet {
                destination,
                aggregate,
                index,
                value,
            } => self.lower_field_update(*destination, *aggregate, *index, *value),
            mir::Instruction::ElementSet {
                destination,
                array,
                index,
                value,
            } => self.lower_element_update(*destination, *array, *index, *value),
            _ => Ok(vec![self.lower_instruction(inst, pool)?]),
        }
    }

    /// Try to fuse addr + load/store into a single lowered instruction.
    pub(super) fn try_fuse_addr_access(
        &self,
        inst: &mir::Instruction,
        next_inst_id: Option<mir::LocalNodeId<mir::Instruction>>,
    ) -> Option<(Instruction, usize)> {
        // bail if there is no next instruction
        let next_inst_id = next_inst_id?;

        // check value usage count for the addr result
        let can_fuse = |value: mir::Value| -> bool { self.value_use_count(value) == Some(1) };

        // load next instruction for pattern matching
        let next_inst = self.tree.get(next_inst_id);

        match inst {
            mir::Instruction::FieldAddr {
                destination,
                aggregate: base,
                index,
                ..
            } => {
                let destination = destination.value()?;
                let base = base.value()?;

                if !can_fuse(destination) {
                    return None;
                }

                let field_count = self.field_count_for_value(base);

                // precompute the field access when the pointee is known
                let pointee_type = heap_pointee_type_for_value_repr(self.value_repr_map(), base)
                    .or_else(|| raw_pointee_type_for_value_repr(self.value_repr_map(), base));
                let pointer_class = pointer_class_for_value(self.value_repr_map(), base);
                let field = pointee_type.and_then(|pointee_type| {
                    field_access_for_pointee(self.layouts(), pointee_type, pointer_class, *index)
                });

                match next_inst {
                    mir::Instruction::Load {
                        destination: load_dest,
                        pointer,
                        ..
                    } if pointer.value()? == destination => Some((
                        Instruction {
                            opcode: select_field_load_opcode(self.value_repr_map(), base).ok()?,
                            operands: Operands::FieldLoad {
                                dest: load_dest.value()?,
                                base,
                                index: *index,
                                field_count,
                                field,
                            },
                        },
                        2,
                    )),
                    mir::Instruction::Store { pointer, value }
                        if pointer.value()? == destination =>
                    {
                        Some((
                            Instruction {
                                opcode: select_field_store_opcode(self.value_repr_map(), base)
                                    .ok()?,
                                operands: Operands::FieldStore {
                                    base,
                                    index: *index,
                                    value: value.value()?,
                                    reference: reference_meta_for_value(
                                        self.value_repr_map(),
                                        destination,
                                    ),
                                    field_count,
                                    field,
                                },
                            },
                            2,
                        ))
                    }
                    _ => None,
                }
            }
            mir::Instruction::ElementAddr {
                destination,
                array,
                index,
                ..
            } => {
                let destination = destination.value()?;
                let array = array.value()?;

                if !can_fuse(destination) {
                    return None;
                }

                let array_length = self.array_length_for_value(array);

                // precompute the element access when the pointee is known
                let pointee_type = heap_pointee_type_for_value_repr(self.value_repr_map(), array)
                    .or_else(|| raw_pointee_type_for_value_repr(self.value_repr_map(), array));
                let pointer_class = pointer_class_for_value(self.value_repr_map(), array);
                let element = pointee_type.and_then(|pointee_type| {
                    element_access_for_pointee(self.layouts(), pointee_type, pointer_class)
                });

                match next_inst {
                    mir::Instruction::Load {
                        destination: load_dest,
                        pointer,
                        ..
                    } if pointer.value()? == destination => Some((
                        Instruction {
                            opcode: select_element_load_opcode(self.value_repr_map(), array)
                                .ok()?,
                            operands: Operands::ElementLoad {
                                dest: load_dest.value()?,
                                array,
                                index: index.value()?,
                                array_length,
                                element,
                            },
                        },
                        2,
                    )),
                    mir::Instruction::Store { pointer, value }
                        if pointer.value()? == destination =>
                    {
                        Some((
                            Instruction {
                                opcode: select_element_store_opcode(self.value_repr_map(), array)
                                    .ok()?,
                                operands: Operands::ElementStore {
                                    array,
                                    index: index.value()?,
                                    value: value.value()?,
                                    reference: reference_meta_for_value(
                                        self.value_repr_map(),
                                        destination,
                                    ),
                                    array_length,
                                    element,
                                },
                            },
                            2,
                        ))
                    }
                    _ => None,
                }
            }
            mir::Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => {
                let destination = destination.value()?;
                let global = global.global()?;

                if !can_fuse(destination) {
                    return None;
                }

                let reference = reference_meta_for_value(self.value_repr_map(), destination);

                match next_inst {
                    mir::Instruction::Load {
                        destination: load_dest,
                        pointer,
                        ..
                    } if pointer.value()? == destination => Some((
                        Instruction {
                            opcode: Opcode::StaticLoad,
                            operands: Operands::StaticLoad {
                                dest: load_dest.value()?,
                                global: global.id,
                            },
                        },
                        2,
                    )),
                    mir::Instruction::Store { pointer, value }
                        if pointer.value()? == destination =>
                    {
                        Some((
                            Instruction {
                                opcode: Opcode::StaticStore,
                                operands: Operands::StaticStore {
                                    global: global.id,
                                    value: value.value()?,
                                    reference,
                                },
                            },
                            2,
                        ))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Try to fuse const + binary into a single instruction with embedded constant.
    pub(super) fn try_fuse_const_binary(
        &self,
        inst: &mir::Instruction,
        next_inst_id: Option<mir::LocalNodeId<mir::Instruction>>,
    ) -> Option<(Instruction, usize)> {
        // bail if there is no next instruction
        let next_inst_id = next_inst_id?;

        // check if the value is only used once
        let can_fuse = |value: mir::Value| -> bool { self.value_use_count(value) == Some(1) };

        // we need a Const instruction
        let mir::Instruction::Const { destination, value } = inst else {
            return None;
        };
        let destination = destination.value()?;

        // check if the const value is only used once
        if !can_fuse(destination) {
            return None;
        }

        // load next instruction and check if it's a binary using our constant
        let next_inst = self.tree.get(next_inst_id);
        let mir::Instruction::Binary {
            destination: bin_dest,
            operator,
            left,
            right,
        } = next_inst
        else {
            return None;
        };

        // we can fuse if the constant is the right operand
        if right.value()? != destination {
            return None;
        }

        // skip comparisons: they may be fused with branches, which expect both
        // materialize values before scalar specialization
        if operator.is_comparison() {
            return None;
        }

        // convert constant to runtime value
        let const_value = Word::from(value);

        // check if left operand is integer for specialized handler
        let left = left.value()?;
        let bin_dest = bin_dest.value()?;
        let repr = self.value_repr_map().get(left);
        if let Some(ValueRepr::Int { width, signed }) = repr {
            // try to get a specialized const handler
            if let Some(opcode) = select_specialized_const_int_opcode(*operator, signed) {
                return Some((
                    Instruction {
                        opcode,
                        operands: Operands::BinaryConstRightSpecialized {
                            dest: bin_dest,
                            left,
                            right_const: const_value,
                            width,
                        },
                    },
                    2,
                ));
            }
        }

        None
    }

    /// Convert a MIR instruction to lowered interpreter form.
    pub(super) fn lower_instruction(
        &self,
        inst: &mir::Instruction,
        pool: &mut Pool,
    ) -> Result<Instruction> {
        Ok(match inst {
            mir::Instruction::Error => {
                return Err(Error::ConcreteMirRequired {
                    context: "instruction".to_string(),
                });
            }
            mir::Instruction::Const { destination, value } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "const destination".to_string(),
                        })?;
                let const_value = if matches!(value, mir::Constant::Null) {
                    let reference =
                        reference_meta_for_type(self.tree, self.value_type_for_value(destination)?);
                    let value = match reference.kind() {
                        Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Owned)
                            if matches!(
                                reference.address_space(),
                                ReferenceAddressSpace::Shared
                            ) =>
                        {
                            Word::shared_heap_reference(SharedHeapReference::NULL)
                        }
                        Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Owned) => {
                            Word::heap_reference(HeapReference::NULL)
                        }
                        _ if matches!(reference.address_space(), ReferenceAddressSpace::Shared) => {
                            Word::shared_raw_pointer(SharedRawPointer::NULL)
                        }
                        _ => Word::raw_pointer(RawPointer::NULL),
                    };
                    ConstValue::Word(value)
                } else {
                    ConstValue::Word(Word::from(value))
                };
                Instruction {
                    opcode: Opcode::Const,
                    operands: Operands::Const {
                        dest: destination,
                        value: const_value,
                    },
                }
            }

            mir::Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "binary destination".to_string(),
                        })?;
                let left = (*left).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "binary left operand".to_string(),
                })?;
                let right = (*right).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "binary right operand".to_string(),
                })?;
                let left_type = self.value_type_for_value(left)?;
                if matches!(
                    self.tree.get(left_type),
                    mir::Type::Vector { .. } | mir::Type::Tensor { .. }
                ) {
                    let result_type = self.value_type_for_value(destination)?;
                    return Ok(Instruction {
                        opcode: Opcode::BinaryElementwise,
                        operands: Operands::BinaryElementwise {
                            dest: destination,
                            op: *operator,
                            left,
                            right,
                            result_type,
                        },
                    });
                }

                let repr = self.value_repr_map().get(left);
                if let Some(ValueRepr::Int { width, signed }) = repr
                    && let Some(opcode) = select_specialized_int_opcode(*operator, signed)
                {
                    return Ok(Instruction {
                        opcode,
                        operands: Operands::BinarySpecialized {
                            dest: destination,
                            left,
                            right,
                            width,
                        },
                    });
                }

                let repr = repr.or_else(|| Some(value_repr_from_type(self.tree, left_type)));
                Instruction {
                    opcode: select_binary_opcode_for_repr(repr, *operator),
                    operands: Operands::Binary {
                        dest: destination,
                        op: *operator,
                        left,
                        right,
                    },
                }
            }

            mir::Instruction::Unary {
                destination,
                operator,
                argument,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "unary destination".to_string(),
                        })?;
                let argument = (*argument)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "unary argument".to_string(),
                    })?;
                let argument_type = self.value_type_for_value(argument)?;
                if matches!(
                    self.tree.get(argument_type),
                    mir::Type::Vector { .. } | mir::Type::Tensor { .. }
                ) {
                    let result_type = self.value_type_for_value(destination)?;
                    return Ok(Instruction {
                        opcode: Opcode::UnaryElementwise,
                        operands: Operands::UnaryElementwise {
                            dest: destination,
                            op: *operator,
                            arg: argument,
                            result_type,
                        },
                    });
                }

                Instruction {
                    opcode: select_unary_opcode(self.value_repr_map(), argument, *operator),
                    operands: Operands::Unary {
                        dest: destination,
                        op: *operator,
                        arg: argument,
                    },
                }
            }

            mir::Instruction::Cast {
                destination,
                operator,
                argument,
                to_type,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "cast destination".to_string(),
                        })?;
                let argument = (*argument)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "cast argument".to_string(),
                    })?;
                let to_type = (*to_type).ty().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "cast destination type".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::Cast,
                    operands: Operands::Cast {
                        dest: destination,
                        op: *operator,
                        arg: argument,
                        to_type: to_type.id,
                    },
                }
            }

            mir::Instruction::Select {
                destination,
                condition,
                then_value,
                else_value,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "select destination".to_string(),
                        })?;
                let condition = (*condition)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "select condition".to_string(),
                    })?;
                let then_value =
                    (*then_value)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "select then value".to_string(),
                        })?;
                let else_value =
                    (*else_value)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "select else value".to_string(),
                        })?;

                Instruction {
                    opcode: Opcode::Select,
                    operands: Operands::Select {
                        dest: destination,
                        condition,
                        then_value,
                        else_value,
                    },
                }
            }

            mir::Instruction::Call {
                destination,
                function,
                call,
                ..
            } => {
                let function =
                    (*function)
                        .function()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "call callee".to_string(),
                        })?;
                let args = self.tree.get_arguments(call.arguments);
                let args_range = pool.argument_reference_range(args, "call argument")?;
                let arguments = args
                    .iter()
                    .map(|argument| {
                        (*argument)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "call argument".to_string(),
                            })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let callee = self.tree.get(function);
                let moves = pool.parameter_move_range(&callee.parameters, &arguments)?;
                let target = self.call_target(function)?;

                Instruction {
                    opcode: Opcode::Call,
                    operands: Operands::Call {
                        dest: pack_optional_value(
                            (*destination)
                                .map(|value| {
                                    value.value().ok_or_else(|| Error::ConcreteMirRequired {
                                        context: "call destination".to_string(),
                                    })
                                })
                                .transpose()?,
                        ),
                        function: function.id,
                        target,
                        arguments: args_range,
                        moves,
                    },
                }
            }

            mir::Instruction::CallVirtual {
                destination,
                receiver,
                slot_id: method,
                call,
                ..
            } => {
                let args = self.tree.get_arguments(call.arguments);
                let args_range = pool.argument_reference_range(args, "virtual call argument")?;
                let receiver = (*receiver)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "virtual call receiver".to_string(),
                    })?;
                Instruction {
                    opcode: Opcode::CallVirtual,
                    operands: Operands::CallVirtual {
                        dest: pack_optional_value(
                            (*destination)
                                .map(|value| {
                                    value.value().ok_or_else(|| Error::ConcreteMirRequired {
                                        context: "virtual call destination".to_string(),
                                    })
                                })
                                .transpose()?,
                        ),
                        receiver,
                        table_field: virtual_table_field_for_receiver(
                            self.layouts(),
                            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
                            pointer_class_for_value(self.value_repr_map(), receiver),
                        ),
                        method_index: method.0,
                        arguments: args_range,
                    },
                }
            }

            mir::Instruction::CallInterface {
                destination,
                receiver,
                slot_id: method,
                call,
                ..
            } => {
                let args = self.tree.get_arguments(call.arguments);
                let args_range = pool.argument_reference_range(args, "interface call argument")?;
                let receiver = (*receiver)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "interface call receiver".to_string(),
                    })?;
                Instruction {
                    opcode: Opcode::CallInterface,
                    operands: Operands::CallInterface {
                        dest: pack_optional_value(
                            (*destination)
                                .map(|value| {
                                    value.value().ok_or_else(|| Error::ConcreteMirRequired {
                                        context: "interface call destination".to_string(),
                                    })
                                })
                                .transpose()?,
                        ),
                        receiver,
                        table_field: interface_table_field_for_receiver(
                            self.layouts(),
                            heap_pointee_type_for_value(self.tree, self.value_type(), receiver),
                            pointer_class_for_value(self.value_repr_map(), receiver),
                        ),
                        method_index: method.0,
                        arguments: args_range,
                    },
                }
            }

            mir::Instruction::CallIndirect {
                destination,
                callee,
                call,
                ..
            } => {
                let args = pool.argument_reference_range(
                    self.tree.get_arguments(call.arguments),
                    "indirect call argument",
                )?;
                let callee = (*callee)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "indirect call callee".to_string(),
                    })?;
                Instruction {
                    opcode: Opcode::CallIndirect,
                    operands: Operands::CallIndirect {
                        dest: pack_optional_value(
                            (*destination)
                                .map(|value| {
                                    value.value().ok_or_else(|| Error::ConcreteMirRequired {
                                        context: "indirect call destination".to_string(),
                                    })
                                })
                                .transpose()?,
                        ),
                        callee,
                        arguments: args,
                    },
                }
            }

            mir::Instruction::LocalGet { destination, local } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "local get destination".to_string(),
                        })?;
                let local = (*local).local().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "local get source".to_string(),
                })?;
                let local_index = self.local_index(local)?;
                Instruction {
                    opcode: Opcode::LocalGet,
                    operands: Operands::LocalGet {
                        dest: destination,
                        local: local_index,
                    },
                }
            }

            mir::Instruction::LocalAddr {
                destination, local, ..
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "local address destination".to_string(),
                        })?;
                let local = (*local).local().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "local address local".to_string(),
                })?;
                let local_index = self.local_index(local)?;
                Instruction {
                    opcode: Opcode::LocalAddr,
                    operands: Operands::LocalAddr {
                        dest: destination,
                        local: local_index,
                        reference: reference_meta_for_value(self.value_repr_map(), destination),
                    },
                }
            }

            mir::Instruction::LocalSet { local, value } => {
                let local = (*local).local().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "local set destination".to_string(),
                })?;
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "local set value".to_string(),
                })?;
                let local_index = self.local_index(local)?;
                Instruction {
                    opcode: Opcode::LocalSet,
                    operands: Operands::LocalSet {
                        local: local_index,
                        value,
                    },
                }
            }

            mir::Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "global address destination".to_string(),
                        })?;
                let global = (*global)
                    .global()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "global address global".to_string(),
                    })?;

                Instruction {
                    opcode: Opcode::StaticAddr,
                    operands: Operands::StaticAddr {
                        dest: destination,
                        global: global.id,
                        reference: reference_meta_for_value(self.value_repr_map(), destination),
                    },
                }
            }

            mir::Instruction::FunctionAddr {
                destination,
                function,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "function address destination".to_string(),
                        })?;
                let function =
                    (*function)
                        .function()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "function address callee".to_string(),
                        })?;

                Instruction {
                    opcode: Opcode::FunctionAddr,
                    operands: Operands::FunctionAddr {
                        dest: destination,
                        function: function.id,
                    },
                }
            }
            mir::Instruction::CallableBind {
                destination,
                function,
                environment,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "callable bind destination".to_string(),
                        })?;
                let function =
                    (*function)
                        .function()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "callable bind callee".to_string(),
                        })?;
                let environment =
                    (*environment)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "callable bind environment".to_string(),
                        })?;

                Instruction {
                    opcode: Opcode::CallableBind,
                    operands: Operands::CallableBind {
                        dest: destination,
                        function: function.id,
                        environment,
                    },
                }
            }
            mir::Instruction::CallableEnvironment { destination } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "callable environment destination".to_string(),
                        })?;

                Instruction {
                    opcode: Opcode::CallableEnvironment,
                    operands: Operands::CallableEnvironment { dest: destination },
                }
            }
            mir::Instruction::Load {
                destination,
                pointer,
                ..
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "load destination".to_string(),
                        })?;
                let pointer = (*pointer)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "load pointer".to_string(),
                    })?;

                let pointee_type = heap_pointee_type_for_value_repr(self.value_repr_map(), pointer)
                    .or_else(|| raw_pointee_type_for_value_repr(self.value_repr_map(), pointer))
                    .or_else(|| heap_pointee_type_for_value(self.tree, self.value_type(), pointer))
                    .or_else(|| raw_pointee_type_for_value(self.tree, self.value_type(), pointer));
                let pointer_class = pointer_class_for_value(self.value_repr_map(), pointer);
                let access = pointee_type
                    .and_then(|pointee_type| {
                        pointee_access_for_type(self.layouts(), pointee_type, pointer_class)
                    })
                    .ok_or(Error::InvalidInstruction)?;

                Instruction {
                    opcode: select_load_opcode(access)?,
                    operands: Operands::Load {
                        dest: destination,
                        pointer,
                        access,
                    },
                }
            }

            mir::Instruction::Store { pointer, value } => {
                let pointer = (*pointer)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "store pointer".to_string(),
                    })?;
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "store value".to_string(),
                })?;

                let pointee_type = heap_pointee_type_for_value_repr(self.value_repr_map(), pointer)
                    .or_else(|| raw_pointee_type_for_value_repr(self.value_repr_map(), pointer))
                    .or_else(|| heap_pointee_type_for_value(self.tree, self.value_type(), pointer))
                    .or_else(|| raw_pointee_type_for_value(self.tree, self.value_type(), pointer));
                let pointer_class = pointer_class_for_value(self.value_repr_map(), pointer);
                let access = pointee_type
                    .and_then(|pointee_type| {
                        pointee_access_for_type(self.layouts(), pointee_type, pointer_class)
                    })
                    .ok_or(Error::InvalidInstruction)?;

                Instruction {
                    opcode: select_store_opcode(access)?,
                    operands: Operands::Store {
                        pointer,
                        value,
                        reference: reference_meta_for_value(self.value_repr_map(), pointer),
                        access,
                    },
                }
            }

            mir::Instruction::Dispose { .. } => Instruction {
                opcode: Opcode::Dispose,
                operands: Operands::Dispose,
            },

            mir::Instruction::AsyncDispose { .. } => Instruction {
                opcode: Opcode::AsyncDispose,
                operands: Operands::AsyncDispose,
            },

            mir::Instruction::Pin { value } => {
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "pin value".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::Pin,
                    operands: Operands::Pin { value },
                }
            }

            mir::Instruction::Unpin { value } => {
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "unpin value".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::Unpin,
                    operands: Operands::Unpin { value },
                }
            }

            mir::Instruction::Drop { value } => {
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "drop value".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::Drop,
                    operands: Operands::Drop { value },
                }
            }

            mir::Instruction::Assume { condition: _ } => Instruction {
                opcode: Opcode::Assume,
                operands: Operands::Assume,
            },

            mir::Instruction::FieldGet {
                destination,
                aggregate: base,
                index,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "field get destination".to_string(),
                        })?;
                let base = (*base).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "field get base".to_string(),
                })?;
                let field_count = self.field_count_for_value(base);
                let opcode = select_field_get_opcode(self.value_repr_map(), base)?;
                let base_type = self.value_type_for_value(base).ok();
                let pointer_class = pointer_class_for_value(self.value_repr_map(), base);
                let field = base_type.and_then(|base_type| {
                    field_access_for_pointee(self.layouts(), base_type, pointer_class, *index)
                });

                match opcode {
                    Opcode::FieldGet => Instruction {
                        opcode,
                        operands: Operands::FieldGet {
                            dest: destination,
                            base,
                            index: *index,
                            field_count,
                            field,
                        },
                    },
                    _ => Instruction {
                        opcode,
                        operands: Operands::FieldLoad {
                            dest: destination,
                            base,
                            index: *index,
                            field_count,
                            field,
                        },
                    },
                }
            }

            mir::Instruction::FieldAddr {
                destination,
                aggregate: base,
                index,
                ..
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "field address destination".to_string(),
                        })?;
                let base = (*base).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "field address base".to_string(),
                })?;

                Instruction {
                    opcode: select_field_addr_opcode(self.value_repr_map(), base)?,
                    operands: {
                        let pointee_type =
                            heap_pointee_type_for_value_repr(self.value_repr_map(), base)
                                .or_else(|| {
                                    raw_pointee_type_for_value_repr(self.value_repr_map(), base)
                                })
                                .or_else(|| {
                                    heap_pointee_type_for_value(self.tree, self.value_type(), base)
                                })
                                .or_else(|| {
                                    raw_pointee_type_for_value(self.tree, self.value_type(), base)
                                });
                        let pointer_class = pointer_class_for_value(self.value_repr_map(), base);
                        let field = pointee_type.and_then(|pointee_type| {
                            field_access_for_pointee(
                                self.layouts(),
                                pointee_type,
                                pointer_class,
                                *index,
                            )
                        });

                        Operands::FieldAddr {
                            dest: destination,
                            base,
                            index: *index,
                            reference: reference_meta_for_value(self.value_repr_map(), destination),
                            field_count: self.field_count_for_value(base),
                            field,
                        }
                    },
                }
            }

            mir::Instruction::FieldSet { .. } => return Err(Error::InvalidInstruction),

            mir::Instruction::ElementGet {
                destination,
                array,
                index,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "element get destination".to_string(),
                        })?;
                let array = (*array).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "element get array".to_string(),
                })?;
                let opcode = select_element_get_opcode(self.value_repr_map(), array)?;
                let array_length = self.array_length_for_value(array);
                let array_type = self.value_type_for_value(array).ok();
                let pointer_class = pointer_class_for_value(self.value_repr_map(), array);
                let element = array_type.and_then(|array_type| {
                    element_access_for_pointee(self.layouts(), array_type, pointer_class)
                });

                match opcode {
                    Opcode::ElementGet => Instruction {
                        opcode,
                        operands: Operands::ElementGet {
                            dest: destination,
                            array,
                            index: (*index).value().ok_or_else(|| Error::ConcreteMirRequired {
                                context: "element get index".to_string(),
                            })?,
                            array_length,
                            element,
                        },
                    },
                    _ => Instruction {
                        opcode,
                        operands: Operands::ElementLoad {
                            dest: destination,
                            array,
                            index: (*index).value().ok_or_else(|| Error::ConcreteMirRequired {
                                context: "element load index".to_string(),
                            })?,
                            array_length,
                            element,
                        },
                    },
                }
            }

            mir::Instruction::ElementAddr {
                destination,
                array,
                index,
                ..
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "element address destination".to_string(),
                        })?;
                let array = (*array).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "element address array".to_string(),
                })?;

                Instruction {
                    opcode: select_element_addr_opcode(self.value_repr_map(), array)?,
                    operands: {
                        let pointee_type =
                            heap_pointee_type_for_value_repr(self.value_repr_map(), array)
                                .or_else(|| {
                                    raw_pointee_type_for_value_repr(self.value_repr_map(), array)
                                })
                                .or_else(|| {
                                    heap_pointee_type_for_value(self.tree, self.value_type(), array)
                                })
                                .or_else(|| {
                                    raw_pointee_type_for_value(self.tree, self.value_type(), array)
                                });
                        let pointer_class = pointer_class_for_value(self.value_repr_map(), array);
                        let element = pointee_type.and_then(|pointee_type| {
                            element_access_for_pointee(self.layouts(), pointee_type, pointer_class)
                        });

                        Operands::ElementAddr {
                            dest: destination,
                            array,
                            index: (*index).value().ok_or_else(|| Error::ConcreteMirRequired {
                                context: "element address index".to_string(),
                            })?,
                            reference: reference_meta_for_value(self.value_repr_map(), destination),
                            array_length: self.array_length_for_value(array),
                            element,
                        }
                    },
                }
            }

            mir::Instruction::ElementSet { .. } => return Err(Error::InvalidInstruction),

            mir::Instruction::Struct { .. } => return Err(Error::InvalidInstruction),

            mir::Instruction::Tuple { .. } => return Err(Error::InvalidInstruction),

            mir::Instruction::Array { .. } => return Err(Error::InvalidInstruction),

            mir::Instruction::VectorSplat { destination, value } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "vector splat destination".to_string(),
                        })?;
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "vector splat value".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::VectorSplat,
                    operands: Operands::VectorSplat {
                        dest: destination,
                        value,
                    },
                }
            }

            mir::Instruction::VectorExtract {
                destination,
                vector,
                index,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "vector extract destination".to_string(),
                        })?;
                let vector = (*vector)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "vector extract input".to_string(),
                    })?;
                let index = (*index).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "vector extract index".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::VectorExtract,
                    operands: Operands::VectorExtract {
                        dest: destination,
                        vector,
                        index,
                    },
                }
            }

            mir::Instruction::VectorInsert {
                destination,
                vector,
                index,
                value,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "vector insert destination".to_string(),
                        })?;
                let vector = (*vector)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "vector insert input".to_string(),
                    })?;
                let index = (*index).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "vector insert index".to_string(),
                })?;
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "vector insert value".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::VectorInsert,
                    operands: Operands::VectorInsert {
                        dest: destination,
                        vector,
                        index,
                        value,
                    },
                }
            }

            mir::Instruction::VectorShuffle {
                destination,
                left,
                right,
                mask,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "vector shuffle destination".to_string(),
                        })?;
                let left = (*left).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "vector shuffle left".to_string(),
                })?;
                let right = (*right).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "vector shuffle right".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::VectorShuffle,
                    operands: Operands::VectorShuffle {
                        dest: destination,
                        left,
                        right,
                        mask: mask.clone(),
                    },
                }
            }
            mir::Instruction::VectorSelect {
                destination,
                mask,
                then_value,
                else_value,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "vector select destination".to_string(),
                        })?;
                let mask = (*mask).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "vector select mask".to_string(),
                })?;
                let then_value =
                    (*then_value)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "vector select then value".to_string(),
                        })?;
                let else_value =
                    (*else_value)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "vector select else value".to_string(),
                        })?;

                Instruction {
                    opcode: Opcode::VectorSelect,
                    operands: Operands::VectorSelect {
                        dest: destination,
                        mask,
                        then_value,
                        else_value,
                    },
                }
            }

            mir::Instruction::VectorReduce {
                destination,
                operator,
                vector,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "vector reduce destination".to_string(),
                        })?;
                let vector = (*vector)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "vector reduce input".to_string(),
                    })?;

                Instruction {
                    opcode: Opcode::VectorReduce,
                    operands: Operands::VectorReduce {
                        dest: destination,
                        operator: *operator,
                        vector,
                    },
                }
            }

            mir::Instruction::VectorCompare {
                destination,
                operator,
                left,
                right,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "vector compare destination".to_string(),
                        })?;
                let left = (*left).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "vector compare left".to_string(),
                })?;
                let right = (*right).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "vector compare right".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::VectorCompare,
                    operands: Operands::VectorCompare {
                        dest: destination,
                        operator: *operator,
                        left,
                        right,
                    },
                }
            }

            mir::Instruction::VectorConvert {
                destination,
                mode,
                vector,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "vector convert destination".to_string(),
                        })?;
                let vector = (*vector)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "vector convert input".to_string(),
                    })?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(vector)?;
                Instruction {
                    opcode: Opcode::VectorConvert,
                    operands: Operands::VectorConvert {
                        dest: destination,
                        mode: *mode,
                        vector,
                        source_type,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorSplat { destination, value } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor splat destination".to_string(),
                        })?;
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "tensor splat value".to_string(),
                })?;
                let tensor_type = self.value_type_for_value(destination)?;

                Instruction {
                    opcode: Opcode::TensorSplat,
                    operands: Operands::TensorSplat {
                        dest: destination,
                        value,
                        tensor_type,
                    },
                }
            }

            mir::Instruction::TensorLoad {
                destination,
                view,
                indices,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor load destination".to_string(),
                        })?;
                let view = (*view).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "tensor load view".to_string(),
                })?;
                let view_type = self.value_type_for_value(view)?;
                let args = pool.argument_reference_range(
                    self.tree.get_arguments(*indices),
                    "tensor load index",
                )?;
                let element = tensor_element_type_for_view_type(self.tree, view_type).and_then(
                    |element_type| {
                        tensor_element_access(
                            self.layouts(),
                            element_type,
                            tensor_view_pointer_class(self.tree, view_type)?,
                        )
                    },
                );
                Instruction {
                    opcode: Opcode::TensorLoad,
                    operands: Operands::TensorLoad {
                        dest: destination,
                        view,
                        indices: args,
                        view_type,
                        element,
                    },
                }
            }

            mir::Instruction::TensorExtract {
                destination,
                tensor,
                indices,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor extract destination".to_string(),
                        })?;
                let tensor = (*tensor)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor extract source".to_string(),
                    })?;
                let tensor_type = self.value_type_for_value(tensor)?;
                let args = pool.argument_reference_range(
                    self.tree.get_arguments(*indices),
                    "tensor extract index",
                )?;
                Instruction {
                    opcode: Opcode::TensorExtract,
                    operands: Operands::TensorExtract {
                        dest: destination,
                        tensor,
                        indices: args,
                        tensor_type,
                    },
                }
            }

            mir::Instruction::TensorStore {
                view,
                indices,
                value,
            } => {
                let view = (*view).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "tensor store view".to_string(),
                })?;
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "tensor store value".to_string(),
                })?;
                let view_type = self.value_type_for_value(view)?;
                let args = pool.argument_reference_range(
                    self.tree.get_arguments(*indices),
                    "tensor store index",
                )?;
                let element = tensor_element_type_for_view_type(self.tree, view_type).and_then(
                    |element_type| {
                        tensor_element_access(
                            self.layouts(),
                            element_type,
                            tensor_view_pointer_class(self.tree, view_type)?,
                        )
                    },
                );
                Instruction {
                    opcode: Opcode::TensorStore,
                    operands: Operands::TensorStore {
                        view,
                        indices: args,
                        value,
                        view_type,
                        element,
                    },
                }
            }

            mir::Instruction::TensorFill { view, value } => {
                let view = (*view).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "tensor fill view".to_string(),
                })?;
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "tensor fill value".to_string(),
                })?;
                let view_type = self.value_type_for_value(view)?;
                let element = tensor_element_type_for_view_type(self.tree, view_type).and_then(
                    |element_type| {
                        tensor_element_access(
                            self.layouts(),
                            element_type,
                            tensor_view_pointer_class(self.tree, view_type)?,
                        )
                    },
                );
                Instruction {
                    opcode: Opcode::TensorFill,
                    operands: Operands::TensorFill {
                        view,
                        value,
                        view_type,
                        element,
                    },
                }
            }

            mir::Instruction::TensorCopy { target, source } => {
                let target = (*target)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor move target".to_string(),
                    })?;
                let source = (*source)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor move source".to_string(),
                    })?;
                let target_type = self.value_type_for_value(target)?;
                let source_type = self.value_type_for_value(source)?;
                let target_element = tensor_element_type_for_view_type(self.tree, target_type)
                    .and_then(|element_type| {
                        tensor_element_access(
                            self.layouts(),
                            element_type,
                            tensor_view_pointer_class(self.tree, target_type)?,
                        )
                    });
                let source_element = tensor_element_type_for_view_type(self.tree, source_type)
                    .and_then(|element_type| {
                        tensor_element_access(
                            self.layouts(),
                            element_type,
                            tensor_view_pointer_class(self.tree, source_type)?,
                        )
                    });
                Instruction {
                    opcode: Opcode::TensorCopy,
                    operands: Operands::TensorCopy {
                        target,
                        source,
                        target_type,
                        source_type,
                        target_element,
                        source_element,
                    },
                }
            }

            mir::Instruction::TensorReshape {
                destination,
                tensor,
                shape,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor reshape destination".to_string(),
                        })?;
                let tensor = (*tensor)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor reshape source".to_string(),
                    })?;
                let dest_type = self.value_type_for_value(destination)?;
                let args = pool.argument_reference_range(
                    self.tree.get_arguments(*shape),
                    "tensor reshape shape",
                )?;
                Instruction {
                    opcode: Opcode::TensorReshape,
                    operands: Operands::TensorReshape {
                        dest: destination,
                        tensor,
                        shape: args,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorBroadcast {
                destination,
                tensor,
                dimensions,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor broadcast destination".to_string(),
                        })?;
                let tensor = (*tensor)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor broadcast source".to_string(),
                    })?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                Instruction {
                    opcode: Opcode::TensorBroadcast,
                    operands: Operands::TensorBroadcast {
                        dest: destination,
                        tensor,
                        dimensions: dimensions.clone(),
                        source_type,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorTranspose {
                destination,
                tensor,
                permutation,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor transpose destination".to_string(),
                        })?;
                let tensor = (*tensor)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor transpose source".to_string(),
                    })?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                Instruction {
                    opcode: Opcode::TensorTranspose,
                    operands: Operands::TensorTranspose {
                        dest: destination,
                        tensor,
                        permutation: permutation.clone(),
                        source_type,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorSlice {
                destination,
                tensor,
                arguments,
                offsets_count,
                sizes_count,
                strides_count,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor slice destination".to_string(),
                        })?;
                let tensor = (*tensor)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor slice source".to_string(),
                    })?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let args = pool.argument_reference_range(
                    self.tree.get_arguments(*arguments),
                    "tensor slice argument",
                )?;
                Instruction {
                    opcode: Opcode::TensorSlice,
                    operands: Operands::TensorSlice {
                        dest: destination,
                        tensor,
                        arguments: args,
                        offsets_count: *offsets_count,
                        sizes_count: *sizes_count,
                        strides_count: *strides_count,
                        source_type,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorPad {
                destination,
                tensor,
                arguments,
                low_count,
                high_count,
                interior_count,
                value,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor pad destination".to_string(),
                        })?;
                let tensor = (*tensor)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor pad source".to_string(),
                    })?;
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "tensor pad value".to_string(),
                })?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                let args = pool.argument_reference_range(
                    self.tree.get_arguments(*arguments),
                    "tensor pad argument",
                )?;
                Instruction {
                    opcode: Opcode::TensorPad,
                    operands: Operands::TensorPad {
                        dest: destination,
                        tensor,
                        arguments: args,
                        low_count: *low_count,
                        high_count: *high_count,
                        interior_count: *interior_count,
                        value,
                        source_type,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorConcat {
                destination,
                tensors,
                axis,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor concat destination".to_string(),
                        })?;
                let dest_type = self.value_type_for_value(destination)?;
                let tensor_value = self.tree.get_arguments(*tensors);
                let args = pool.argument_reference_range(tensor_value, "tensor concat operand")?;

                let mut tensor_type = Vec::with_capacity(tensor_value.len());
                for value in tensor_value {
                    let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor concat operand".to_string(),
                    })?;
                    let value_type = self.value_type_for_value(value)?;
                    tensor_type.push(value_type);
                }

                Instruction {
                    opcode: Opcode::TensorConcat,
                    operands: Operands::TensorConcat {
                        dest: destination,
                        tensors: args,
                        tensor_types: tensor_type,
                        axis: *axis,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorReduce {
                destination,
                operator,
                tensor,
                initial,
                axes,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor reduce destination".to_string(),
                        })?;
                let tensor = (*tensor)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor reduce source".to_string(),
                    })?;
                let initial = (*initial)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor reduce initial".to_string(),
                    })?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                Instruction {
                    opcode: Opcode::TensorReduce,
                    operands: Operands::TensorReduce {
                        dest: destination,
                        operator: *operator,
                        tensor,
                        initial,
                        axes: axes.clone(),
                        source_type,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorDot {
                destination,
                left,
                right,
                dimensions,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor dot destination".to_string(),
                        })?;
                let left = (*left).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "tensor dot left".to_string(),
                })?;
                let right = (*right).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "tensor dot right".to_string(),
                })?;
                let dest_type = self.value_type_for_value(destination)?;
                let left_type = self.value_type_for_value(left)?;
                let right_type = self.value_type_for_value(right)?;
                Instruction {
                    opcode: Opcode::TensorDot,
                    operands: Operands::TensorDot {
                        dest: destination,
                        left,
                        right,
                        dimensions: dimensions.clone(),
                        left_type,
                        right_type,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorConvolution {
                destination,
                input,
                kernel,
                dimensions,
                window,
                feature_group_count,
                batch_group_count,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor convolution destination".to_string(),
                        })?;
                let input = (*input).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "tensor convolution input".to_string(),
                })?;
                let kernel = (*kernel)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor convolution kernel".to_string(),
                    })?;
                let dest_type = self.value_type_for_value(destination)?;
                let input_type = self.value_type_for_value(input)?;
                let kernel_type = self.value_type_for_value(kernel)?;
                Instruction {
                    opcode: Opcode::TensorConvolution,
                    operands: Operands::TensorConvolution {
                        dest: destination,
                        input,
                        kernel,
                        dimensions: dimensions.clone(),
                        window: window.clone(),
                        feature_group_count: *feature_group_count,
                        batch_group_count: *batch_group_count,
                        input_type,
                        kernel_type,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorGather {
                destination,
                operand,
                indices,
                dimensions,
                slice_sizes,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor gather destination".to_string(),
                        })?;
                let operand = (*operand)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor gather operand".to_string(),
                    })?;
                let indices = (*indices)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor gather indices".to_string(),
                    })?;
                let dest_type = self.value_type_for_value(destination)?;
                let operand_type = self.value_type_for_value(operand)?;
                let indices_type = self.value_type_for_value(indices)?;
                Instruction {
                    opcode: Opcode::TensorGather,
                    operands: Operands::TensorGather {
                        dest: destination,
                        operand,
                        indices,
                        dimensions: dimensions.clone(),
                        slice_sizes: slice_sizes.clone(),
                        operand_type,
                        indices_type,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorScatter {
                destination,
                operand,
                indices,
                updates,
                dimensions,
                mode,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor scatter destination".to_string(),
                        })?;
                let operand = (*operand)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor scatter operand".to_string(),
                    })?;
                let indices = (*indices)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor scatter indices".to_string(),
                    })?;
                let updates = (*updates)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor scatter updates".to_string(),
                    })?;
                let dest_type = self.value_type_for_value(destination)?;
                let operand_type = self.value_type_for_value(operand)?;
                let indices_type = self.value_type_for_value(indices)?;
                let updates_type = self.value_type_for_value(updates)?;
                Instruction {
                    opcode: Opcode::TensorScatter,
                    operands: Operands::TensorScatter {
                        dest: destination,
                        operand,
                        indices,
                        updates,
                        dimensions: dimensions.clone(),
                        mode: *mode,
                        operand_type,
                        indices_type,
                        updates_type,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorCompare {
                destination,
                operator,
                left,
                right,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor compare destination".to_string(),
                        })?;
                let left = (*left).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "tensor compare left".to_string(),
                })?;
                let right = (*right).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "tensor compare right".to_string(),
                })?;
                let dest_type = self.value_type_for_value(destination)?;
                let left_type = self.value_type_for_value(left)?;
                let right_type = self.value_type_for_value(right)?;
                Instruction {
                    opcode: Opcode::TensorCompare,
                    operands: Operands::TensorCompare {
                        dest: destination,
                        operator: *operator,
                        left,
                        right,
                        left_type,
                        right_type,
                        dest_type,
                    },
                }
            }
            mir::Instruction::TensorSelect {
                destination,
                mask,
                then_value,
                else_value,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor select destination".to_string(),
                        })?;
                let mask = (*mask).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "tensor select mask".to_string(),
                })?;
                let then_value =
                    (*then_value)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor select then value".to_string(),
                        })?;
                let else_value =
                    (*else_value)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor select else value".to_string(),
                        })?;
                let dest_type = self.value_type_for_value(destination)?;
                Instruction {
                    opcode: Opcode::TensorSelect,
                    operands: Operands::TensorSelect {
                        dest: destination,
                        mask,
                        then_value,
                        else_value,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorConvert {
                destination,
                mode,
                tensor,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor convert destination".to_string(),
                        })?;
                let tensor = (*tensor)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor convert source".to_string(),
                    })?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(tensor)?;
                Instruction {
                    opcode: Opcode::TensorConvert,
                    operands: Operands::TensorConvert {
                        dest: destination,
                        mode: *mode,
                        tensor,
                        source_type,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorCast {
                destination,
                tensor,
            } => Instruction {
                opcode: Opcode::TensorCast,
                operands: Operands::TensorCast {
                    dest: (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor cast destination".to_string(),
                        })?,
                    tensor: (*tensor)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor cast source".to_string(),
                        })?,
                },
            },

            mir::Instruction::TensorView {
                destination,
                view,
                arguments,
                offsets_count,
                sizes_count,
                strides_count,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tensor view destination".to_string(),
                        })?;
                let view = (*view).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "tensor view source".to_string(),
                })?;
                let args = pool.argument_reference_range(
                    self.tree.get_arguments(*arguments),
                    "tensor view argument",
                )?;
                let dest_type = self.value_type_for_value(destination)?;
                let source_type = self.value_type_for_value(view)?;
                let element = tensor_element_type_for_view_type(self.tree, source_type).and_then(
                    |element_type| {
                        tensor_element_access(
                            self.layouts(),
                            element_type,
                            tensor_view_pointer_class(self.tree, source_type)?,
                        )
                    },
                );
                Instruction {
                    opcode: Opcode::TensorView,
                    operands: Operands::TensorView {
                        dest: destination,
                        view,
                        arguments: args,
                        offsets_count: *offsets_count,
                        sizes_count: *sizes_count,
                        strides_count: *strides_count,
                        source_type,
                        dest_type,
                        element,
                    },
                }
            }

            mir::Instruction::New {
                destination,
                layout,
                ..
            } => {
                let dest = (*destination)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "new destination".to_string(),
                    })?;
                let allocation_type = (*layout).ty().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "new layout".to_string(),
                })?;
                let layout_id = self
                    .layout_id_by_type
                    .get(&allocation_type)
                    .copied()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "new layout id".to_string(),
                    })?;

                Instruction {
                    opcode: Opcode::New,
                    operands: Operands::New {
                        dest,
                        reference: reference_meta_for_value(self.value_repr_map(), dest),
                        layout_id,
                    },
                }
            }

            mir::Instruction::NewSlice {
                destination,
                element,
                length,
                ..
            } => {
                let dest = (*destination)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "new.slice destination".to_string(),
                    })?;
                let element_type = (*element).ty().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "new.slice element type".to_string(),
                })?;
                let element_layout_id = self
                    .layout_id_by_type
                    .get(&element_type)
                    .copied()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "new.slice element layout id".to_string(),
                    })?;
                let element_alignment = self
                    .layouts()
                    .get(&element_type)
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "new.slice element layout".to_string(),
                    })?
                    .alignment();

                Instruction {
                    opcode: Opcode::NewSlice,
                    operands: Operands::NewSlice {
                        dest,
                        length: (*length)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "new.slice length".to_string(),
                            })?,
                        element_layout_id,
                        element_alignment,
                    },
                }
            }

            mir::Instruction::RawAlloc {
                destination,
                layout,
                ..
            } => Instruction {
                opcode: Opcode::RawAlloc,
                operands: Operands::RawAlloc {
                    dest: (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "raw alloc destination".to_string(),
                        })?,
                    reference: reference_meta_for_value(
                        self.value_repr_map(),
                        (*destination)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "raw alloc destination".to_string(),
                            })?,
                    ),
                    byte_len: self.layout_byte_len((*layout).ty().ok_or_else(|| {
                        Error::ConcreteMirRequired {
                            context: "raw alloc layout".to_string(),
                        }
                    })?)?,
                },
            },

            mir::Instruction::RawFree { pointer } => Instruction {
                opcode: Opcode::RawFree,
                operands: Operands::RawFree {
                    pointer: (*pointer)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "raw free pointer".to_string(),
                        })?,
                },
            },

            mir::Instruction::StackAlloc {
                destination,
                layout,
                ..
            } => Instruction {
                opcode: Opcode::StackAlloc,
                operands: Operands::StackAlloc {
                    dest: (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "stack alloc destination".to_string(),
                        })?,
                    reference: reference_meta_for_value(
                        self.value_repr_map(),
                        (*destination)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "stack alloc destination".to_string(),
                            })?,
                    ),
                    allocation_type: (*layout).ty().ok_or_else(|| Error::ConcreteMirRequired {
                        context: "stack alloc layout".to_string(),
                    })?,
                },
            },

            mir::Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
            } => {
                let args = pool.argument_reference_range(
                    self.tree.get_arguments(*arguments),
                    "intrinsic argument",
                )?;
                Instruction {
                    opcode: Opcode::Intrinsic,
                    operands: Operands::Intrinsic {
                        dest: pack_optional_value(
                            (*destination)
                                .map(|value| {
                                    value.value().ok_or_else(|| Error::ConcreteMirRequired {
                                        context: "intrinsic destination".to_string(),
                                    })
                                })
                                .transpose()?,
                        ),
                        intrinsic: *intrinsic,
                        arguments: args,
                    },
                }
            }

            mir::Instruction::AtomicLoad {
                destination,
                pointer,
                ..
            } => Instruction {
                opcode: Opcode::AtomicLoad,
                operands: Operands::AtomicLoad {
                    dest: (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "atomic load destination".to_string(),
                        })?,
                    pointer: (*pointer)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "atomic load pointer".to_string(),
                        })?,
                    raw_pointee: raw_pointee_type_for_value(
                        self.tree,
                        self.value_type(),
                        (*pointer)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "atomic load pointer".to_string(),
                            })?,
                    ),
                },
            },

            mir::Instruction::AtomicStore { pointer, value, .. } => Instruction {
                opcode: Opcode::AtomicStore,
                operands: Operands::AtomicStore {
                    pointer: (*pointer)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "atomic store pointer".to_string(),
                        })?,
                    value: (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                        context: "atomic store value".to_string(),
                    })?,
                    raw_pointee: raw_pointee_type_for_value(
                        self.tree,
                        self.value_type(),
                        (*pointer)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "atomic store pointer".to_string(),
                            })?,
                    ),
                },
            },

            mir::Instruction::AtomicCompareExchange {
                destination,
                pointer,
                expected,
                new_value,
                ..
            } => Instruction {
                opcode: Opcode::AtomicCompareExchange,
                operands: Operands::AtomicCompareExchange {
                    dest: (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "atomic compare exchange destination".to_string(),
                        })?,
                    pointer: (*pointer)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "atomic compare exchange pointer".to_string(),
                        })?,
                    expected: (*expected)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "atomic compare exchange expected".to_string(),
                        })?,
                    new_value: (*new_value)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "atomic compare exchange new value".to_string(),
                        })?,
                    raw_pointee: raw_pointee_type_for_value(
                        self.tree,
                        self.value_type(),
                        (*pointer)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "atomic compare exchange pointer".to_string(),
                            })?,
                    ),
                },
            },

            mir::Instruction::AtomicRmw {
                destination,
                operator,
                pointer,
                value,
                ..
            } => Instruction {
                opcode: Opcode::AtomicRmw,
                operands: Operands::AtomicRmw {
                    dest: (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "atomic rmw destination".to_string(),
                        })?,
                    operator: *operator,
                    pointer: (*pointer)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "atomic rmw pointer".to_string(),
                        })?,
                    value: (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                        context: "atomic rmw value".to_string(),
                    })?,
                    raw_pointee: raw_pointee_type_for_value(
                        self.tree,
                        self.value_type(),
                        (*pointer)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "atomic rmw pointer".to_string(),
                            })?,
                    ),
                },
            },

            mir::Instruction::AtomicFence { .. } => Instruction {
                opcode: Opcode::AtomicFence,
                operands: Operands::AtomicFence,
            },

            mir::Instruction::Barrier { .. } => Instruction {
                opcode: Opcode::Barrier,
                operands: Operands::Barrier,
            },
        })
    }

    /// Lower one MIR value constructor into frame stores.
    fn lower_frame_init(
        &self,
        destination: mir::Value,
        values: &[mir::ValueReference],
    ) -> Result<Vec<Instruction>> {
        let destination_type = self.value_type_for_value(destination)?;
        let ranges = self.frame_ranges(destination_type)?;
        if values.len() != ranges.len() {
            return Err(Error::InvalidFieldAccess {
                index: values.len() as u32,
                field_count: ranges.len(),
            });
        }

        let mut instructions = Vec::with_capacity(ranges.len());
        for (index, value) in values.iter().enumerate() {
            let value = value.value().ok_or_else(|| Error::ConcreteMirRequired {
                context: "frame constructor value".to_string(),
            })?;
            let range = ranges[index];
            instructions.push(self.store_frame_range(destination, value, range)?);
        }

        Ok(instructions)
    }

    /// Lower one functional field update into frame stores.
    fn lower_field_update(
        &self,
        destination: mir::ValueReference,
        base: mir::ValueReference,
        index: u32,
        value: mir::ValueReference,
    ) -> Result<Vec<Instruction>> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::ConcreteMirRequired {
                context: "field set destination".to_string(),
            })?;
        let base = base.value().ok_or_else(|| Error::ConcreteMirRequired {
            context: "field set base".to_string(),
        })?;
        let value = value.value().ok_or_else(|| Error::ConcreteMirRequired {
            context: "field set value".to_string(),
        })?;
        let destination_type = self.value_type_for_value(destination)?;
        let layout = self.layout_for_type(destination_type)?;
        let field_count = layout.field_count().ok_or(Error::InvalidInstruction)?;
        let field = layout
            .field(index)
            .ok_or(Error::InvalidFieldAccess { index, field_count })?;
        let whole = FrameRange {
            value_type: destination_type,
            byte_offset: 0,
            byte_len: layout.byte_len,
        };
        let part = FrameRange {
            value_type: field.ty,
            byte_offset: field.offset,
            byte_len: field.byte_len,
        };

        Ok(vec![
            self.store_frame_range(destination, base, whole)?,
            self.store_frame_range(destination, value, part)?,
        ])
    }

    /// Lower one functional element update into frame stores.
    fn lower_element_update(
        &self,
        destination: mir::ValueReference,
        array: mir::ValueReference,
        index: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Vec<Instruction>> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::ConcreteMirRequired {
                context: "element set destination".to_string(),
            })?;
        let array = array.value().ok_or_else(|| Error::ConcreteMirRequired {
            context: "element set array".to_string(),
        })?;
        let index = index.value().ok_or_else(|| Error::ConcreteMirRequired {
            context: "element set index".to_string(),
        })?;
        let value = value.value().ok_or_else(|| Error::ConcreteMirRequired {
            context: "element set value".to_string(),
        })?;
        let destination_type = self.value_type_for_value(destination)?;
        let layout = self.layout_for_type(destination_type)?;
        let element =
            element_access_for_pointee(self.layouts(), destination_type, PointerClass::Frame);
        let whole = FrameRange {
            value_type: destination_type,
            byte_offset: 0,
            byte_len: layout.byte_len,
        };

        Ok(vec![
            self.store_frame_range(destination, array, whole)?,
            Instruction {
                opcode: Opcode::ElementStore,
                operands: Operands::ElementStore {
                    array: destination,
                    index,
                    value,
                    reference: reference_meta_for_value(self.value_repr_map(), destination),
                    array_length: self.array_length_for_value(destination),
                    element,
                },
            },
        ])
    }

    /// Return direct byte ranges for one frame-backed value type.
    fn frame_ranges(&self, value_type: mir::LocalNodeId<mir::Type>) -> Result<Vec<FrameRange>> {
        let layout = self.layout_for_type(value_type)?;
        if let Some(field_count) = layout.field_count() {
            let mut ranges = Vec::with_capacity(field_count);
            for index in 0..field_count {
                let index = index as u32;
                let field = layout
                    .field(index)
                    .ok_or(Error::InvalidFieldAccess { index, field_count })?;
                ranges.push(FrameRange {
                    value_type: field.ty,
                    byte_offset: field.offset,
                    byte_len: field.byte_len,
                });
            }

            return Ok(ranges);
        }

        let element = layout.element().ok_or_else(|| Error::TypeMismatch {
            expected: "frame-backed layout".to_string(),
            actual: format!("{value_type:?}"),
        })?;
        let element_count = layout.element_count().ok_or(Error::InvalidInstruction)?;
        let mut ranges = Vec::with_capacity(element_count);
        for index in 0..element_count {
            let byte_offset =
                element
                    .stride
                    .checked_mul(index)
                    .ok_or(Error::InvalidArrayAccess {
                        index: index as u64,
                        length: element_count as u64,
                    })?;
            ranges.push(FrameRange {
                value_type: element.ty,
                byte_offset,
                byte_len: element.byte_len,
            });
        }

        Ok(ranges)
    }

    /// Lower one value write into destination frame bytes.
    fn store_frame_range(
        &self,
        destination: mir::Value,
        value: mir::Value,
        range: FrameRange,
    ) -> Result<Instruction> {
        let repr = self.layout_for_type(range.value_type)?;
        let access = PointeeAccess {
            pointer_class: PointerClass::Frame,
            value_type: range.value_type,
            byte_offset: range.byte_offset,
            byte_len: range.byte_len,
            is_scalar: repr.is_scalar(),
        };

        Ok(Instruction {
            opcode: Opcode::StoreFrame,
            operands: Operands::Store {
                pointer: destination,
                value,
                reference: reference_meta_for_value(self.value_repr_map(), destination),
                access,
            },
        })
    }

    /// Return one lowered layout by MIR type.
    fn layout_for_type(
        &self,
        value_type: mir::LocalNodeId<mir::Type>,
    ) -> Result<&crate::program::Layout> {
        self.layouts()
            .get(&value_type)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing lowered layout for type: {value_type:?}"),
            })
    }

    /// Return one lowered field count for one value.
    fn field_count_for_value(&self, value: mir::Value) -> Option<u32> {
        self.value_repr_map()
            .get(value)
            .and_then(|repr| field_count_from_repr(self.tree, repr))
    }

    /// Return one lowered array length for one value.
    fn array_length_for_value(&self, value: mir::Value) -> Option<u64> {
        self.value_repr_map()
            .get(value)
            .and_then(|repr| array_length_from_repr(self.tree, repr))
    }

    /// Return one lowered byte length for one layout.
    fn value_type_for_value(&self, value: mir::Value) -> Result<mir::LocalNodeId<mir::Type>> {
        lookup_value_type_for_value(value, self.value_type()).ok_or_else(|| {
            Error::InvariantViolation {
                context: format!("missing value type for {value:?}"),
            }
        })
    }

    /// Return one lowered byte length for one layout.
    fn layout_byte_len(&self, layout: mir::LocalNodeId<mir::Type>) -> Result<usize> {
        self.layouts()
            .get(&layout)
            .map(|layout| layout.byte_len)
            .ok_or_else(|| Error::InvariantViolation {
                context: format!("missing lowered layout for type: {layout:?}"),
            })
    }
}

/// One direct byte range inside a frame value.
#[derive(Clone, Copy)]
struct FrameRange {
    /// The MIR type written into this byte range.
    value_type: mir::LocalNodeId<mir::Type>,
    /// The byte offset from the frame value base.
    byte_offset: usize,
    /// The byte width of this byte range.
    byte_len: usize,
}

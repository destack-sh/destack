use destack_mir as mir;

use crate::{ReferenceAddressSpace, Value};
use destack_heap::{HeapReference, RawPointer, SharedHeapReference, SharedRawPointer};

use crate::module::{ConstValue, Immediate, Instruction, Opcode, ValueKind, pack_optional_value};
use crate::{Error, Result};

use super::access::*;
use super::kind::{
    heap_pointee_type_for_value, heap_pointee_type_for_value_kind, raw_pointee_type_for_value,
    raw_pointee_type_for_value_kind, reference_meta_for_type, reference_meta_for_value,
    value_type_for_value as lookup_value_type_for_value,
};
use super::lower::BlockLowerer;
use super::opcode::{
    select_binary_opcode, select_element_addr_opcode, select_element_get_opcode,
    select_element_load_opcode, select_element_store_opcode, select_field_addr_opcode,
    select_field_get_opcode, select_field_load_opcode, select_field_store_opcode,
    select_load_opcode, select_specialized_const_int_opcode, select_specialized_int_opcode,
    select_store_opcode, select_unary_opcode,
};
use super::pool::Pool;

impl<'a> BlockLowerer<'a> {
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
                aggregate,
                index,
                ..
            } => {
                let destination = destination.value()?;
                let aggregate = aggregate.value()?;

                if !can_fuse(destination) {
                    return None;
                }

                let field_count = self.field_count_for_value(aggregate);

                // precompute the field access when the pointee is known
                let pointee_type =
                    heap_pointee_type_for_value_kind(self.value_kind_map(), aggregate).or_else(
                        || raw_pointee_type_for_value_kind(self.value_kind_map(), aggregate),
                    );
                let field = pointee_type.and_then(|pointee_type| {
                    field_access_for_pointee(self.layouts(), pointee_type, *index)
                });

                match next_inst {
                    mir::Instruction::Load {
                        destination: load_dest,
                        pointer,
                        ..
                    } if pointer.value()? == destination => Some((
                        Instruction {
                            opcode: select_field_load_opcode(self.value_kind_map(), aggregate),
                            immediate: Immediate::FieldLoad {
                                dest: load_dest.value()?,
                                composite: aggregate,
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
                                opcode: select_field_store_opcode(self.value_kind_map(), aggregate),
                                immediate: Immediate::FieldStore {
                                    composite: aggregate,
                                    index: *index,
                                    value: value.value()?,
                                    reference: reference_meta_for_value(
                                        self.value_kind_map(),
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
                let pointee_type = heap_pointee_type_for_value_kind(self.value_kind_map(), array)
                    .or_else(|| raw_pointee_type_for_value_kind(self.value_kind_map(), array));
                let element = pointee_type.and_then(|pointee_type| {
                    element_access_for_pointee(self.layouts(), pointee_type)
                });

                match next_inst {
                    mir::Instruction::Load {
                        destination: load_dest,
                        pointer,
                        ..
                    } if pointer.value()? == destination => Some((
                        Instruction {
                            opcode: select_element_load_opcode(self.value_kind_map(), array),
                            immediate: Immediate::ElementLoad {
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
                                opcode: select_element_store_opcode(self.value_kind_map(), array),
                                immediate: Immediate::ElementStore {
                                    array,
                                    index: index.value()?,
                                    value: value.value()?,
                                    reference: reference_meta_for_value(
                                        self.value_kind_map(),
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

                let reference = reference_meta_for_value(self.value_kind_map(), destination);

                match next_inst {
                    mir::Instruction::Load {
                        destination: load_dest,
                        pointer,
                        ..
                    } if pointer.value()? == destination => Some((
                        Instruction {
                            opcode: Opcode::GlobalLoad,
                            immediate: Immediate::GlobalLoad {
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
                                opcode: Opcode::GlobalStore,
                                immediate: Immediate::GlobalStore {
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
        let const_value = Value::from(value);

        // check if left operand is integer for specialized handler
        let left = left.value()?;
        let bin_dest = bin_dest.value()?;
        let kind = self.value_kind_map().get(left);
        if let Some(ValueKind::Int { signed, .. }) = kind {
            // try to get a specialized const handler
            if let Some(opcode) = select_specialized_const_int_opcode(*operator, signed) {
                return Some((
                    Instruction {
                        opcode,
                        immediate: Immediate::BinaryConstRightSpecialized {
                            dest: bin_dest,
                            left,
                            right_const: const_value,
                        },
                    },
                    2,
                ));
            }
        }

        // fall back to generic binary with const right
        Some((
            Instruction {
                opcode: Opcode::BinaryConstRight,
                immediate: Immediate::BinaryConstRight {
                    dest: bin_dest,
                    op: *operator,
                    left,
                    right_const: const_value,
                },
            },
            2,
        ))
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
                            Value::shared_heap_reference_with_meta(
                                SharedHeapReference::NULL,
                                reference,
                            )
                        }
                        Some(mir::ReferenceKind::Managed | mir::ReferenceKind::Owned) => {
                            Value::heap_reference_with_meta(HeapReference::NULL, reference)
                        }
                        _ if matches!(reference.address_space(), ReferenceAddressSpace::Shared) => {
                            Value::shared_raw_pointer_with_meta(SharedRawPointer::NULL, reference)
                        }
                        _ => Value::raw_pointer_with_meta(RawPointer::NULL, reference),
                    };
                    ConstValue::Value(value)
                } else {
                    ConstValue::Value(Value::from(value))
                };
                Instruction {
                    opcode: Opcode::Const,
                    immediate: Immediate::Const {
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
                        immediate: Immediate::BinaryElementwise {
                            dest: destination,
                            op: *operator,
                            left,
                            right,
                            result_type,
                        },
                    });
                }

                let kind = self.value_kind_map().get(left);
                if let Some(ValueKind::Int { signed, .. }) = kind
                    && let Some(opcode) = select_specialized_int_opcode(*operator, signed)
                {
                    return Ok(Instruction {
                        opcode,
                        immediate: Immediate::BinarySpecialized {
                            dest: destination,
                            left,
                            right,
                        },
                    });
                }

                Instruction {
                    opcode: select_binary_opcode(self.value_kind_map(), left, *operator),
                    immediate: Immediate::Binary {
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
                        immediate: Immediate::UnaryElementwise {
                            dest: destination,
                            op: *operator,
                            arg: argument,
                            result_type,
                        },
                    });
                }

                Instruction {
                    opcode: select_unary_opcode(self.value_kind_map(), argument, *operator),
                    immediate: Immediate::Unary {
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
                    immediate: Immediate::Cast {
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
                    immediate: Immediate::Select {
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
                let copies = pool.parameter_copy_range(&callee.parameters, &arguments)?;
                let target = self.call_target(function)?;

                Instruction {
                    opcode: Opcode::Call,
                    immediate: Immediate::Call {
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
                        copies,
                    },
                }
            }

            mir::Instruction::CallVirtual {
                destination,
                receiver,
                slot_id,
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
                    immediate: Immediate::CallVirtual {
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
                        heap_pointee: heap_pointee_type_for_value(
                            self.tree,
                            self.value_type(),
                            receiver,
                        ),
                        slot_id: slot_id.0,
                        arguments: args_range,
                    },
                }
            }

            mir::Instruction::CallInterface {
                destination,
                receiver,
                slot_id,
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
                    immediate: Immediate::CallInterface {
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
                        heap_pointee: heap_pointee_type_for_value(
                            self.tree,
                            self.value_type(),
                            receiver,
                        ),
                        slot_id: slot_id.0,
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
                    immediate: Immediate::CallIndirect {
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
                    immediate: Immediate::LocalGet {
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
                    immediate: Immediate::LocalAddr {
                        dest: destination,
                        local: local_index,
                        reference: reference_meta_for_value(self.value_kind_map(), destination),
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
                    immediate: Immediate::LocalSet {
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
                    opcode: Opcode::GlobalAddr,
                    immediate: Immediate::GlobalAddr {
                        dest: destination,
                        global: global.id,
                        reference: reference_meta_for_value(self.value_kind_map(), destination),
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
                    immediate: Immediate::FunctionAddr {
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
                    immediate: Immediate::CallableBind {
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
                    immediate: Immediate::CallableEnvironment { dest: destination },
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

                Instruction {
                    opcode: select_load_opcode(self.value_kind_map(), pointer),
                    immediate: {
                        let pointee_type =
                            heap_pointee_type_for_value_kind(self.value_kind_map(), pointer)
                                .or_else(|| {
                                    raw_pointee_type_for_value_kind(self.value_kind_map(), pointer)
                                })
                                .or_else(|| {
                                    heap_pointee_type_for_value(
                                        self.tree,
                                        self.value_type(),
                                        pointer,
                                    )
                                })
                                .or_else(|| {
                                    raw_pointee_type_for_value(
                                        self.tree,
                                        self.value_type(),
                                        pointer,
                                    )
                                });
                        let access = pointee_type.and_then(|pointee_type| {
                            typed_access_for_pointee(self.layouts(), pointee_type)
                        });

                        Immediate::Load {
                            dest: destination,
                            pointer,
                            access,
                        }
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

                Instruction {
                    opcode: select_store_opcode(self.value_kind_map(), pointer),
                    immediate: {
                        let pointee_type =
                            heap_pointee_type_for_value_kind(self.value_kind_map(), pointer)
                                .or_else(|| {
                                    raw_pointee_type_for_value_kind(self.value_kind_map(), pointer)
                                })
                                .or_else(|| {
                                    heap_pointee_type_for_value(
                                        self.tree,
                                        self.value_type(),
                                        pointer,
                                    )
                                })
                                .or_else(|| {
                                    raw_pointee_type_for_value(
                                        self.tree,
                                        self.value_type(),
                                        pointer,
                                    )
                                });
                        let access = pointee_type.and_then(|pointee_type| {
                            typed_access_for_pointee(self.layouts(), pointee_type)
                        });

                        Immediate::Store {
                            pointer,
                            value,
                            reference: reference_meta_for_value(self.value_kind_map(), pointer),
                            access,
                        }
                    },
                }
            }

            mir::Instruction::Dispose { value } => {
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "dispose value".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::Dispose,
                    immediate: Immediate::Dispose { value },
                }
            }

            mir::Instruction::AsyncDispose { value } => {
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "async dispose value".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::AsyncDispose,
                    immediate: Immediate::AsyncDispose { value },
                }
            }

            mir::Instruction::Pin { value } => {
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "pin value".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::Pin,
                    immediate: Immediate::Pin { value },
                }
            }

            mir::Instruction::Unpin { value } => {
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "unpin value".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::Unpin,
                    immediate: Immediate::Unpin { value },
                }
            }

            mir::Instruction::Drop { value } => {
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "drop value".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::Drop,
                    immediate: Immediate::Drop { value },
                }
            }

            mir::Instruction::Assume { condition: _ } => Instruction {
                opcode: Opcode::Assume,
                immediate: Immediate::Assume,
            },

            mir::Instruction::FieldGet {
                destination,
                aggregate,
                index,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "field get destination".to_string(),
                        })?;
                let aggregate = (*aggregate)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "field get aggregate".to_string(),
                    })?;
                let field_count = self.field_count_for_value(aggregate);
                let opcode = select_field_get_opcode(self.value_kind_map(), aggregate);
                let aggregate_type = self.value_type_for_value(aggregate).ok();
                let field = aggregate_type.and_then(|aggregate_type| {
                    field_access_for_pointee(self.layouts(), aggregate_type, *index)
                });

                match opcode {
                    Opcode::FieldGet => Instruction {
                        opcode,
                        immediate: Immediate::FieldGet {
                            dest: destination,
                            composite: aggregate,
                            index: *index,
                            field_count,
                            field,
                        },
                    },
                    _ => Instruction {
                        opcode,
                        immediate: Immediate::FieldLoad {
                            dest: destination,
                            composite: aggregate,
                            index: *index,
                            field_count,
                            field,
                        },
                    },
                }
            }

            mir::Instruction::FieldAddr {
                destination,
                aggregate,
                index,
                ..
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "field address destination".to_string(),
                        })?;
                let aggregate = (*aggregate)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "field address aggregate".to_string(),
                    })?;

                Instruction {
                    opcode: select_field_addr_opcode(self.value_kind_map(), aggregate),
                    immediate: {
                        let pointee_type =
                            heap_pointee_type_for_value_kind(self.value_kind_map(), aggregate)
                                .or_else(|| {
                                    raw_pointee_type_for_value_kind(
                                        self.value_kind_map(),
                                        aggregate,
                                    )
                                })
                                .or_else(|| {
                                    heap_pointee_type_for_value(
                                        self.tree,
                                        self.value_type(),
                                        aggregate,
                                    )
                                })
                                .or_else(|| {
                                    raw_pointee_type_for_value(
                                        self.tree,
                                        self.value_type(),
                                        aggregate,
                                    )
                                });
                        let field = pointee_type.and_then(|pointee_type| {
                            field_access_for_pointee(self.layouts(), pointee_type, *index)
                        });

                        Immediate::FieldAddr {
                            dest: destination,
                            composite: aggregate,
                            index: *index,
                            reference: reference_meta_for_value(self.value_kind_map(), destination),
                            field_count: self.field_count_for_value(aggregate),
                            field,
                        }
                    },
                }
            }

            mir::Instruction::FieldSet {
                destination,
                aggregate,
                index,
                value,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "field set destination".to_string(),
                        })?;
                let aggregate = (*aggregate)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "field set aggregate".to_string(),
                    })?;
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "field set value".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::FieldSet,
                    immediate: Immediate::FieldSet {
                        dest: destination,
                        composite: aggregate,
                        index: *index,
                        value,
                        field_count: self.field_count_for_value(aggregate),
                        field: self.value_type_for_value(aggregate).ok().and_then(
                            |aggregate_type| {
                                field_access_for_pointee(self.layouts(), aggregate_type, *index)
                            },
                        ),
                    },
                }
            }

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
                let opcode = select_element_get_opcode(self.value_kind_map(), array);
                let array_length = self.array_length_for_value(array);
                let array_type = self.value_type_for_value(array).ok();
                let element = array_type
                    .and_then(|array_type| element_access_for_pointee(self.layouts(), array_type));

                match opcode {
                    Opcode::ElementGet => Instruction {
                        opcode,
                        immediate: Immediate::ElementGet {
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
                        immediate: Immediate::ElementLoad {
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
                    opcode: select_element_addr_opcode(self.value_kind_map(), array),
                    immediate: {
                        let pointee_type =
                            heap_pointee_type_for_value_kind(self.value_kind_map(), array)
                                .or_else(|| {
                                    raw_pointee_type_for_value_kind(self.value_kind_map(), array)
                                })
                                .or_else(|| {
                                    heap_pointee_type_for_value(self.tree, self.value_type(), array)
                                })
                                .or_else(|| {
                                    raw_pointee_type_for_value(self.tree, self.value_type(), array)
                                });
                        let element = pointee_type.and_then(|pointee_type| {
                            element_access_for_pointee(self.layouts(), pointee_type)
                        });

                        Immediate::ElementAddr {
                            dest: destination,
                            array,
                            index: (*index).value().ok_or_else(|| Error::ConcreteMirRequired {
                                context: "element address index".to_string(),
                            })?,
                            reference: reference_meta_for_value(self.value_kind_map(), destination),
                            array_length: self.array_length_for_value(array),
                            element,
                        }
                    },
                }
            }

            mir::Instruction::ElementSet {
                destination,
                array,
                index,
                value,
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "element set destination".to_string(),
                        })?;
                let array = (*array).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "element set array".to_string(),
                })?;
                let value = (*value).value().ok_or_else(|| Error::ConcreteMirRequired {
                    context: "element set value".to_string(),
                })?;

                Instruction {
                    opcode: Opcode::ElementSet,
                    immediate: Immediate::ElementSet {
                        dest: destination,
                        array,
                        index: (*index).value().ok_or_else(|| Error::ConcreteMirRequired {
                            context: "element set index".to_string(),
                        })?,
                        value,
                        array_length: self.array_length_for_value(array),
                        element: self
                            .value_type_for_value(array)
                            .ok()
                            .and_then(|array_type| {
                                element_access_for_pointee(self.layouts(), array_type)
                            }),
                    },
                }
            }

            mir::Instruction::Struct {
                destination,
                fields,
                ..
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "struct destination".to_string(),
                        })?;
                let args = pool.argument_reference_range(
                    self.tree.get_arguments(*fields),
                    "struct field argument",
                )?;
                Instruction {
                    opcode: Opcode::Composite,
                    immediate: Immediate::Composite {
                        dest: destination,
                        elements: args,
                    },
                }
            }

            mir::Instruction::Tuple {
                destination,
                elements,
                ..
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "tuple destination".to_string(),
                        })?;
                let args = pool.argument_reference_range(
                    self.tree.get_arguments(*elements),
                    "tuple element argument",
                )?;
                Instruction {
                    opcode: Opcode::Composite,
                    immediate: Immediate::Composite {
                        dest: destination,
                        elements: args,
                    },
                }
            }

            mir::Instruction::Array {
                destination,
                elements,
                ..
            } => {
                let destination =
                    (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "array destination".to_string(),
                        })?;
                let args = pool.argument_reference_range(
                    self.tree.get_arguments(*elements),
                    "array element argument",
                )?;
                Instruction {
                    opcode: Opcode::Composite,
                    immediate: Immediate::Composite {
                        dest: destination,
                        elements: args,
                    },
                }
            }

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
                    immediate: Immediate::VectorSplat {
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
                    immediate: Immediate::VectorExtract {
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
                    immediate: Immediate::VectorInsert {
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
                    immediate: Immediate::VectorShuffle {
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
                    immediate: Immediate::VectorSelect {
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
                    immediate: Immediate::VectorReduce {
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
                    immediate: Immediate::VectorCompare {
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
                    immediate: Immediate::VectorConvert {
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
                    immediate: Immediate::TensorSplat {
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
                let element = tensor_element_type_for_view_type(self.tree, view_type)
                    .and_then(|element_type| tensor_element_access(self.layouts(), element_type));
                Instruction {
                    opcode: Opcode::TensorLoad,
                    immediate: Immediate::TensorLoad {
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
                    immediate: Immediate::TensorExtract {
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
                let element = tensor_element_type_for_view_type(self.tree, view_type)
                    .and_then(|element_type| tensor_element_access(self.layouts(), element_type));
                Instruction {
                    opcode: Opcode::TensorStore,
                    immediate: Immediate::TensorStore {
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
                let element = tensor_element_type_for_view_type(self.tree, view_type)
                    .and_then(|element_type| tensor_element_access(self.layouts(), element_type));
                Instruction {
                    opcode: Opcode::TensorFill,
                    immediate: Immediate::TensorFill {
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
                        context: "tensor copy target".to_string(),
                    })?;
                let source = (*source)
                    .value()
                    .ok_or_else(|| Error::ConcreteMirRequired {
                        context: "tensor copy source".to_string(),
                    })?;
                let target_type = self.value_type_for_value(target)?;
                let source_type = self.value_type_for_value(source)?;
                let target_element = tensor_element_type_for_view_type(self.tree, target_type)
                    .and_then(|element_type| tensor_element_access(self.layouts(), element_type));
                let source_element = tensor_element_type_for_view_type(self.tree, source_type)
                    .and_then(|element_type| tensor_element_access(self.layouts(), element_type));
                Instruction {
                    opcode: Opcode::TensorCopy,
                    immediate: Immediate::TensorCopy {
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
                    immediate: Immediate::TensorReshape {
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
                    immediate: Immediate::TensorBroadcast {
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
                    immediate: Immediate::TensorTranspose {
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
                    immediate: Immediate::TensorSlice {
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
                    immediate: Immediate::TensorPad {
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
                    immediate: Immediate::TensorConcat {
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
                    immediate: Immediate::TensorReduce {
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
                    immediate: Immediate::TensorDot {
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
                    immediate: Immediate::TensorConvolution {
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
                    immediate: Immediate::TensorGather {
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
                    immediate: Immediate::TensorScatter {
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
                    immediate: Immediate::TensorCompare {
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
                    immediate: Immediate::TensorSelect {
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
                    immediate: Immediate::TensorConvert {
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
                immediate: Immediate::TensorCast {
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
                let element = tensor_element_type_for_view_type(self.tree, source_type)
                    .and_then(|element_type| tensor_element_access(self.layouts(), element_type));
                Instruction {
                    opcode: Opcode::TensorView,
                    immediate: Immediate::TensorView {
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
            } => Instruction {
                opcode: Opcode::New,
                immediate: Immediate::New {
                    dest: (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "new destination".to_string(),
                        })?,
                    reference: reference_meta_for_value(
                        self.value_kind_map(),
                        (*destination)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "new destination".to_string(),
                            })?,
                    ),
                    storage_type: (*layout).ty().ok_or_else(|| Error::ConcreteMirRequired {
                        context: "new layout".to_string(),
                    })?,
                    layout_id: self.tree.type_layout_id((*layout).ty().ok_or_else(|| {
                        Error::ConcreteMirRequired {
                            context: "new layout".to_string(),
                        }
                    })?),
                },
            },

            mir::Instruction::NewSlice {
                destination,
                element: _,
                length,
                result_type,
                ..
            } => Instruction {
                opcode: Opcode::NewSlice,
                immediate: Immediate::NewSlice {
                    dest: (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "new.slice destination".to_string(),
                        })?,
                    length: (*length)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "new.slice length".to_string(),
                        })?,
                    slice_type: self.slice_type_for_new_slice(*result_type)?,
                },
            },

            mir::Instruction::RawAlloc {
                destination,
                layout,
                ..
            } => Instruction {
                opcode: Opcode::RawAlloc,
                immediate: Immediate::RawAlloc {
                    dest: (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "raw alloc destination".to_string(),
                        })?,
                    reference: reference_meta_for_value(
                        self.value_kind_map(),
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
                immediate: Immediate::RawFree {
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
                immediate: Immediate::StackAlloc {
                    dest: (*destination)
                        .value()
                        .ok_or_else(|| Error::ConcreteMirRequired {
                            context: "stack alloc destination".to_string(),
                        })?,
                    reference: reference_meta_for_value(
                        self.value_kind_map(),
                        (*destination)
                            .value()
                            .ok_or_else(|| Error::ConcreteMirRequired {
                                context: "stack alloc destination".to_string(),
                            })?,
                    ),
                    storage_type: (*layout).ty().ok_or_else(|| Error::ConcreteMirRequired {
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
                    immediate: Immediate::Intrinsic {
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
                immediate: Immediate::AtomicLoad {
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
                immediate: Immediate::AtomicStore {
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
                immediate: Immediate::AtomicCompareExchange {
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
                immediate: Immediate::AtomicRmw {
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
                immediate: Immediate::AtomicFence,
            },

            mir::Instruction::Barrier { .. } => Instruction {
                opcode: Opcode::Barrier,
                immediate: Immediate::Barrier,
            },
        })
    }
    /// Return one lowered field count for one value.
    fn field_count_for_value(&self, value: mir::Value) -> Option<u32> {
        self.value_kind_map()
            .get(value)
            .and_then(|kind| field_count_from_kind(self.tree, kind))
    }

    /// Return one lowered array length for one value.
    fn array_length_for_value(&self, value: mir::Value) -> Option<u64> {
        self.value_kind_map()
            .get(value)
            .and_then(|kind| array_length_from_kind(self.tree, kind))
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

    /// Return the slice type for one new.slice result.
    fn slice_type_for_new_slice(
        &self,
        result_type: mir::TypeReference,
    ) -> Result<mir::LocalNodeId<mir::Type>> {
        let result_type = result_type.ty().ok_or_else(|| Error::ConcreteMirRequired {
            context: "new.slice result type".to_string(),
        })?;

        let mir::Type::Slice { .. } = self.tree.get(result_type) else {
            return Err(Error::InvariantViolation {
                context: "new.slice result type must be a slice".to_string(),
            });
        };

        Ok(result_type)
    }
}

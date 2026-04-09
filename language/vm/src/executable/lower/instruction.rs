use destack_mir as mir;

use destack_heap::{ManagedReference, RawPointer, SharedPointer, Value};

use crate::executable::value::ValueKind;
use crate::executable::{
    ConstValue, Instruction, InstructionData, InstructionOperation, UNKNOWN_ARRAY_LENGTH,
    UNKNOWN_FIELD_COUNT, pack_optional_value,
};

use super::access::*;
use super::kind::{
    managed_pointee_type_for_value, managed_pointee_type_for_value_kind,
    raw_pointee_type_for_value, raw_pointee_type_for_value_kind, reference_meta_for_type,
    reference_meta_for_value, value_type_for_value,
};
use super::lower::BlockLowerer;
use super::operation::{
    select_binary_operation, select_element_addr_operation, select_element_get_operation,
    select_element_load_operation, select_element_store_operation, select_field_addr_operation,
    select_field_get_operation, select_field_load_operation, select_field_store_operation,
    select_load_operation, select_specialized_const_int_operation,
    select_specialized_int_operation, select_store_operation, select_unary_operation,
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
        let can_fuse = |value: mir::Value| -> bool { self.value_use_count(value) == 1 };

        // load next instruction for pattern matching
        let next_inst = self.tree.get(next_inst_id);

        match inst {
            mir::Instruction::FieldAddr {
                destination,
                aggregate,
                index,
                ..
            } => {
                if !can_fuse(*destination) {
                    return None;
                }

                let field_count = self.field_count_for_value(*aggregate);

                // precompute the field access descriptor when the pointee is known
                let pointee_type =
                    managed_pointee_type_for_value_kind(self.value_kind_map(), *aggregate).or_else(
                        || raw_pointee_type_for_value_kind(self.value_kind_map(), *aggregate),
                    );
                let field = pointee_type.and_then(|pointee_type| {
                    field_access_for_pointee(self.layouts(), pointee_type, *index)
                });

                match next_inst {
                    mir::Instruction::Load {
                        destination: load_dest,
                        pointer,
                        ..
                    } if pointer == destination => Some((
                        Instruction {
                            operation: select_field_load_operation(
                                self.value_kind_map(),
                                *aggregate,
                            ),
                            data: InstructionData::FieldLoad {
                                dest: *load_dest,
                                composite: *aggregate,
                                index: *index,
                                field_count,
                                field,
                            },
                        },
                        2,
                    )),
                    mir::Instruction::Store { pointer, value } if pointer == destination => Some((
                        Instruction {
                            operation: select_field_store_operation(
                                self.value_kind_map(),
                                *aggregate,
                                field_count,
                                *index,
                            ),
                            data: InstructionData::FieldStore {
                                composite: *aggregate,
                                index: *index,
                                value: *value,
                                reference: reference_meta_for_value(
                                    self.value_kind_map(),
                                    *destination,
                                ),
                                field_count,
                                field,
                            },
                        },
                        2,
                    )),
                    _ => None,
                }
            }
            mir::Instruction::ElementAddr {
                destination,
                array,
                index,
                ..
            } => {
                if !can_fuse(*destination) {
                    return None;
                }

                let array_length = self.array_length_for_value(*array);

                // precompute the element access descriptor when the pointee is known
                let pointee_type =
                    managed_pointee_type_for_value_kind(self.value_kind_map(), *array)
                        .or_else(|| raw_pointee_type_for_value_kind(self.value_kind_map(), *array));
                let element = pointee_type.and_then(|pointee_type| {
                    element_access_for_pointee(self.layouts(), pointee_type)
                });

                match next_inst {
                    mir::Instruction::Load {
                        destination: load_dest,
                        pointer,
                        ..
                    } if pointer == destination => Some((
                        Instruction {
                            operation: select_element_load_operation(self.value_kind_map(), *array),
                            data: InstructionData::ElementLoad {
                                dest: *load_dest,
                                array: *array,
                                index: *index,
                                array_length,
                                element,
                            },
                        },
                        2,
                    )),
                    mir::Instruction::Store { pointer, value } if pointer == destination => Some((
                        Instruction {
                            operation: select_element_store_operation(
                                self.value_kind_map(),
                                *array,
                            ),
                            data: InstructionData::ElementStore {
                                array: *array,
                                index: *index,
                                value: *value,
                                reference: reference_meta_for_value(
                                    self.value_kind_map(),
                                    *destination,
                                ),
                                array_length,
                                element,
                            },
                        },
                        2,
                    )),
                    _ => None,
                }
            }
            mir::Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => {
                if !can_fuse(*destination) {
                    return None;
                }

                let reference = reference_meta_for_value(self.value_kind_map(), *destination);

                match next_inst {
                    mir::Instruction::Load {
                        destination: load_dest,
                        pointer,
                        ..
                    } if pointer == destination => Some((
                        Instruction {
                            operation: InstructionOperation::GlobalLoad,
                            data: InstructionData::GlobalLoad {
                                dest: *load_dest,
                                global: global.id,
                            },
                        },
                        2,
                    )),
                    mir::Instruction::Store { pointer, value } if pointer == destination => Some((
                        Instruction {
                            operation: InstructionOperation::GlobalStore,
                            data: InstructionData::GlobalStore {
                                global: global.id,
                                value: *value,
                                reference,
                            },
                        },
                        2,
                    )),
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
        let can_fuse = |value: mir::Value| -> bool { self.value_use_count(value) == 1 };

        // we need a Const instruction
        let mir::Instruction::Const { destination, value } = inst else {
            return None;
        };

        // check if the const value is only used once
        if !can_fuse(*destination) {
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
        if right != destination {
            return None;
        }

        // skip comparisons: they may be fused with branches, which expect both
        // operands to be materialized values
        if operator.is_comparison() {
            return None;
        }

        // convert constant to runtime value
        let const_value = Value::from(value);

        // check if left operand is integer for specialized handler
        let kind = self.value_kind_map().get(*left);
        if let Some(ValueKind::Int { signed, .. }) = kind {
            // try to get a specialized const handler
            if let Some(operation) = select_specialized_const_int_operation(*operator, signed) {
                return Some((
                    Instruction {
                        operation,
                        data: InstructionData::BinaryConstRightSpecialized {
                            dest: *bin_dest,
                            left: *left,
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
                operation: InstructionOperation::BinaryConstRight,
                data: InstructionData::BinaryConstRight {
                    dest: *bin_dest,
                    op: *operator,
                    left: *left,
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
    ) -> Instruction {
        match inst {
            mir::Instruction::Const { destination, value } => {
                let const_value = if matches!(value, mir::Constant::Null) {
                    let reference = reference_meta_for_type(
                        self.tree,
                        value_type_for_value(*destination, self.value_type()),
                    );
                    let value = match reference.kind() {
                        Some(mir::ReferenceKind::Managed) => {
                            Value::managed_reference_with_meta(ManagedReference::NULL, reference)
                        }
                        _ if matches!(
                            reference.address_space(),
                            destack_heap::ReferenceAddressSpace::Shared
                        ) =>
                        {
                            Value::shared_pointer_with_meta(SharedPointer::NULL, reference)
                        }
                        _ => Value::raw_pointer_with_meta(RawPointer::NULL, reference),
                    };
                    ConstValue::Value(value)
                } else {
                    ConstValue::Value(Value::from(value))
                };
                Instruction {
                    operation: InstructionOperation::Const,
                    data: InstructionData::Const {
                        dest: *destination,
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
                let left_type = value_type_for_value(*left, self.value_type());
                if matches!(
                    self.tree.get(left_type),
                    mir::Type::Vector { .. } | mir::Type::Tensor { .. }
                ) {
                    let result_type = value_type_for_value(*destination, self.value_type());
                    return Instruction {
                        operation: InstructionOperation::BinaryElementwise,
                        data: InstructionData::BinaryElementwise {
                            dest: *destination,
                            op: *operator,
                            left: *left,
                            right: *right,
                            result_type,
                        },
                    };
                }

                let kind = self.value_kind_map().get(*left);
                if let Some(ValueKind::Int { signed, .. }) = kind
                    && let Some(operation) = select_specialized_int_operation(*operator, signed)
                {
                    return Instruction {
                        operation,
                        data: InstructionData::BinarySpecialized {
                            dest: *destination,
                            left: *left,
                            right: *right,
                        },
                    };
                }

                Instruction {
                    operation: select_binary_operation(self.value_kind_map(), *left, *operator),
                    data: InstructionData::Binary {
                        dest: *destination,
                        op: *operator,
                        left: *left,
                        right: *right,
                    },
                }
            }

            mir::Instruction::Unary {
                destination,
                operator,
                argument,
            } => {
                let argument_type = value_type_for_value(*argument, self.value_type());
                if matches!(
                    self.tree.get(argument_type),
                    mir::Type::Vector { .. } | mir::Type::Tensor { .. }
                ) {
                    let result_type = value_type_for_value(*destination, self.value_type());
                    return Instruction {
                        operation: InstructionOperation::UnaryElementwise,
                        data: InstructionData::UnaryElementwise {
                            dest: *destination,
                            op: *operator,
                            arg: *argument,
                            result_type,
                        },
                    };
                }

                Instruction {
                    operation: select_unary_operation(self.value_kind_map(), *argument, *operator),
                    data: InstructionData::Unary {
                        dest: *destination,
                        op: *operator,
                        arg: *argument,
                    },
                }
            }

            mir::Instruction::Cast {
                destination,
                operator,
                argument,
                to_type,
            } => Instruction {
                operation: InstructionOperation::Cast,
                data: InstructionData::Cast {
                    dest: *destination,
                    op: *operator,
                    arg: *argument,
                    to_type: to_type.id,
                },
            },

            mir::Instruction::Select {
                destination,
                condition,
                then_value,
                else_value,
            } => Instruction {
                operation: InstructionOperation::Select,
                data: InstructionData::Select {
                    dest: *destination,
                    condition: *condition,
                    then_value: *then_value,
                    else_value: *else_value,
                },
            },

            mir::Instruction::Call {
                destination,
                function,
                arguments,
                ..
            } => {
                let args = self.tree.get_arguments(*arguments);
                let args_range = pool.argument_range(args);
                let callee = self.tree.get(*function);
                let copies = pool.parameter_copy_range(&callee.parameters, args);
                let target = self.call_target(*function);

                Instruction {
                    operation: InstructionOperation::Call,
                    data: InstructionData::Call {
                        dest: pack_optional_value(*destination),
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
                arguments,
                ..
            } => {
                let args = self.tree.get_arguments(*arguments);
                let args_range = pool.argument_range(args);
                Instruction {
                    operation: InstructionOperation::CallVirtual,
                    data: InstructionData::CallVirtual {
                        dest: pack_optional_value(*destination),
                        receiver: *receiver,
                        managed_pointee: managed_pointee_type_for_value(
                            self.tree,
                            self.value_type(),
                            *receiver,
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
                arguments,
                ..
            } => {
                let args = self.tree.get_arguments(*arguments);
                let args_range = pool.argument_range(args);
                Instruction {
                    operation: InstructionOperation::CallInterface,
                    data: InstructionData::CallInterface {
                        dest: pack_optional_value(*destination),
                        receiver: *receiver,
                        managed_pointee: managed_pointee_type_for_value(
                            self.tree,
                            self.value_type(),
                            *receiver,
                        ),
                        slot_id: slot_id.0,
                        arguments: args_range,
                    },
                }
            }

            mir::Instruction::CallIndirect {
                destination,
                callee,
                arguments,
                ..
            } => {
                let args = pool.argument_range(self.tree.get_arguments(*arguments));
                Instruction {
                    operation: InstructionOperation::CallIndirect,
                    data: InstructionData::CallIndirect {
                        dest: pack_optional_value(*destination),
                        callee: *callee,
                        arguments: args,
                    },
                }
            }

            mir::Instruction::LocalGet { destination, local } => {
                let local_index = self.local_index(*local);
                Instruction {
                    operation: InstructionOperation::LocalGet,
                    data: InstructionData::LocalGet {
                        dest: *destination,
                        local: local_index,
                    },
                }
            }

            mir::Instruction::LocalAddr {
                destination, local, ..
            } => {
                let local_index = self.local_index(*local);
                Instruction {
                    operation: InstructionOperation::LocalAddr,
                    data: InstructionData::LocalAddr {
                        dest: *destination,
                        local: local_index,
                        reference: reference_meta_for_value(self.value_kind_map(), *destination),
                    },
                }
            }

            mir::Instruction::LocalSet { local, value } => {
                let local_index = self.local_index(*local);
                Instruction {
                    operation: InstructionOperation::LocalSet,
                    data: InstructionData::LocalSet {
                        local: local_index,
                        value: *value,
                    },
                }
            }

            mir::Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => Instruction {
                operation: InstructionOperation::GlobalAddr,
                data: InstructionData::GlobalAddr {
                    dest: *destination,
                    global: global.id,
                    reference: reference_meta_for_value(self.value_kind_map(), *destination),
                },
            },

            mir::Instruction::GlobalConst {
                destination,
                global,
            } => Instruction {
                operation: InstructionOperation::GlobalConst,
                data: InstructionData::GlobalConst {
                    dest: *destination,
                    global: global.id,
                },
            },

            mir::Instruction::FunctionAddr {
                destination,
                function,
            } => Instruction {
                operation: InstructionOperation::FunctionAddr,
                data: InstructionData::FunctionAddr {
                    dest: *destination,
                    function: function.id,
                },
            },
            mir::Instruction::Closure {
                destination,
                function,
                environment,
            } => Instruction {
                operation: InstructionOperation::Closure,
                data: InstructionData::Closure {
                    dest: *destination,
                    function: function.id,
                    environment: *environment,
                },
            },
            mir::Instruction::FunctionEnvironment { destination } => Instruction {
                operation: InstructionOperation::FunctionEnvironment,
                data: InstructionData::FunctionEnvironment { dest: *destination },
            },
            mir::Instruction::Load {
                destination,
                pointer,
                ..
            } => Instruction {
                operation: select_load_operation(self.value_kind_map(), *pointer),
                data: {
                    let pointee_type =
                        managed_pointee_type_for_value_kind(self.value_kind_map(), *pointer)
                            .or_else(|| {
                                raw_pointee_type_for_value_kind(self.value_kind_map(), *pointer)
                            })
                            .or_else(|| {
                                managed_pointee_type_for_value(
                                    self.tree,
                                    self.value_type(),
                                    *pointer,
                                )
                            })
                            .or_else(|| {
                                raw_pointee_type_for_value(self.tree, self.value_type(), *pointer)
                            });
                    let access = pointee_type.and_then(|pointee_type| {
                        typed_access_for_pointee(self.layouts(), pointee_type)
                    });

                    InstructionData::Load {
                        dest: *destination,
                        pointer: *pointer,
                        access,
                    }
                },
            },

            mir::Instruction::Store { pointer, value } => Instruction {
                operation: select_store_operation(self.value_kind_map(), *pointer),
                data: {
                    let pointee_type =
                        managed_pointee_type_for_value_kind(self.value_kind_map(), *pointer)
                            .or_else(|| {
                                raw_pointee_type_for_value_kind(self.value_kind_map(), *pointer)
                            })
                            .or_else(|| {
                                managed_pointee_type_for_value(
                                    self.tree,
                                    self.value_type(),
                                    *pointer,
                                )
                            })
                            .or_else(|| {
                                raw_pointee_type_for_value(self.tree, self.value_type(), *pointer)
                            });
                    let access = pointee_type.and_then(|pointee_type| {
                        typed_access_for_pointee(self.layouts(), pointee_type)
                    });

                    InstructionData::Store {
                        pointer: *pointer,
                        value: *value,
                        reference: reference_meta_for_value(self.value_kind_map(), *pointer),
                        access,
                    }
                },
            },

            mir::Instruction::RawDrop { value } => Instruction {
                operation: InstructionOperation::RawDrop,
                data: InstructionData::RawDrop { value: *value },
            },

            mir::Instruction::StackDrop { value } => Instruction {
                operation: InstructionOperation::StackDrop,
                data: InstructionData::StackDrop { value: *value },
            },

            mir::Instruction::Assume { condition: _ } => Instruction {
                operation: InstructionOperation::Assume,
                data: InstructionData::Assume,
            },

            mir::Instruction::FieldGet {
                destination,
                aggregate,
                index,
            } => {
                let field_count = self.field_count_for_value(*aggregate);
                let operation = select_field_get_operation(
                    self.value_kind_map(),
                    *aggregate,
                    field_count,
                    *index,
                );

                let pointee_type =
                    managed_pointee_type_for_value_kind(self.value_kind_map(), *aggregate)
                        .or_else(|| {
                            raw_pointee_type_for_value_kind(self.value_kind_map(), *aggregate)
                        })
                        .or_else(|| {
                            managed_pointee_type_for_value(self.tree, self.value_type(), *aggregate)
                        })
                        .or_else(|| {
                            raw_pointee_type_for_value(self.tree, self.value_type(), *aggregate)
                        });
                let field = pointee_type.and_then(|pointee_type| {
                    field_access_for_pointee(self.layouts(), pointee_type, *index)
                });

                match operation {
                    InstructionOperation::FieldGet | InstructionOperation::FieldGetInline => {
                        Instruction {
                            operation,
                            data: InstructionData::FieldGet {
                                dest: *destination,
                                composite: *aggregate,
                                index: *index,
                            },
                        }
                    }
                    _ => Instruction {
                        operation,
                        data: InstructionData::FieldLoad {
                            dest: *destination,
                            composite: *aggregate,
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
            } => Instruction {
                operation: select_field_addr_operation(self.value_kind_map(), *aggregate),
                data: {
                    let pointee_type =
                        managed_pointee_type_for_value_kind(self.value_kind_map(), *aggregate)
                            .or_else(|| {
                                raw_pointee_type_for_value_kind(self.value_kind_map(), *aggregate)
                            })
                            .or_else(|| {
                                managed_pointee_type_for_value(
                                    self.tree,
                                    self.value_type(),
                                    *aggregate,
                                )
                            })
                            .or_else(|| {
                                raw_pointee_type_for_value(self.tree, self.value_type(), *aggregate)
                            });
                    let field = pointee_type.and_then(|pointee_type| {
                        field_access_for_pointee(self.layouts(), pointee_type, *index)
                    });

                    InstructionData::FieldAddr {
                        dest: *destination,
                        composite: *aggregate,
                        index: *index,
                        reference: reference_meta_for_value(self.value_kind_map(), *destination),
                        field_count: self.field_count_for_value(*aggregate),
                        field,
                    }
                },
            },

            mir::Instruction::FieldSet {
                destination,
                aggregate,
                index,
                value,
            } => Instruction {
                operation: InstructionOperation::FieldSet,
                data: InstructionData::FieldSet {
                    dest: *destination,
                    composite: *aggregate,
                    index: *index,
                    value: *value,
                },
            },

            mir::Instruction::ElementGet {
                destination,
                array,
                index,
            } => {
                let operation = select_element_get_operation(self.value_kind_map(), *array);
                let array_length = self.array_length_for_value(*array);

                let pointee_type =
                    managed_pointee_type_for_value_kind(self.value_kind_map(), *array)
                        .or_else(|| raw_pointee_type_for_value_kind(self.value_kind_map(), *array))
                        .or_else(|| {
                            managed_pointee_type_for_value(self.tree, self.value_type(), *array)
                        })
                        .or_else(|| {
                            raw_pointee_type_for_value(self.tree, self.value_type(), *array)
                        });
                let element = pointee_type.and_then(|pointee_type| {
                    element_access_for_pointee(self.layouts(), pointee_type)
                });

                match operation {
                    InstructionOperation::ElementGet => Instruction {
                        operation,
                        data: InstructionData::ElementGet {
                            dest: *destination,
                            array: *array,
                            index: *index,
                        },
                    },
                    _ => Instruction {
                        operation,
                        data: InstructionData::ElementLoad {
                            dest: *destination,
                            array: *array,
                            index: *index,
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
            } => Instruction {
                operation: select_element_addr_operation(self.value_kind_map(), *array),
                data: {
                    let pointee_type =
                        managed_pointee_type_for_value_kind(self.value_kind_map(), *array)
                            .or_else(|| {
                                raw_pointee_type_for_value_kind(self.value_kind_map(), *array)
                            })
                            .or_else(|| {
                                managed_pointee_type_for_value(self.tree, self.value_type(), *array)
                            })
                            .or_else(|| {
                                raw_pointee_type_for_value(self.tree, self.value_type(), *array)
                            });
                    let element = pointee_type.and_then(|pointee_type| {
                        element_access_for_pointee(self.layouts(), pointee_type)
                    });

                    InstructionData::ElementAddr {
                        dest: *destination,
                        array: *array,
                        index: *index,
                        reference: reference_meta_for_value(self.value_kind_map(), *destination),
                        array_length: self.array_length_for_value(*array),
                        element,
                    }
                },
            },

            mir::Instruction::ElementSet {
                destination,
                array,
                index,
                value,
            } => Instruction {
                operation: InstructionOperation::ElementSet,
                data: InstructionData::ElementSet {
                    dest: *destination,
                    array: *array,
                    index: *index,
                    value: *value,
                },
            },

            mir::Instruction::Struct {
                destination,
                fields,
                ..
            } => {
                let args = pool.argument_range(self.tree.get_arguments(*fields));
                Instruction {
                    operation: InstructionOperation::Composite,
                    data: InstructionData::Composite {
                        dest: *destination,
                        elements: args,
                    },
                }
            }

            mir::Instruction::Tuple {
                destination,
                elements,
                ..
            } => {
                let args = pool.argument_range(self.tree.get_arguments(*elements));
                Instruction {
                    operation: InstructionOperation::Composite,
                    data: InstructionData::Composite {
                        dest: *destination,
                        elements: args,
                    },
                }
            }

            mir::Instruction::Array {
                destination,
                elements,
                ..
            } => {
                let args = pool.argument_range(self.tree.get_arguments(*elements));
                Instruction {
                    operation: InstructionOperation::Composite,
                    data: InstructionData::Composite {
                        dest: *destination,
                        elements: args,
                    },
                }
            }

            mir::Instruction::VectorSplat { destination, value } => Instruction {
                operation: InstructionOperation::VectorSplat,
                data: InstructionData::VectorSplat {
                    dest: *destination,
                    value: *value,
                },
            },

            mir::Instruction::VectorExtract {
                destination,
                vector,
                index,
            } => Instruction {
                operation: InstructionOperation::VectorExtract,
                data: InstructionData::VectorExtract {
                    dest: *destination,
                    vector: *vector,
                    index: *index,
                },
            },

            mir::Instruction::VectorInsert {
                destination,
                vector,
                index,
                value,
            } => Instruction {
                operation: InstructionOperation::VectorInsert,
                data: InstructionData::VectorInsert {
                    dest: *destination,
                    vector: *vector,
                    index: *index,
                    value: *value,
                },
            },

            mir::Instruction::VectorShuffle {
                destination,
                left,
                right,
                mask,
            } => Instruction {
                operation: InstructionOperation::VectorShuffle,
                data: InstructionData::VectorShuffle {
                    dest: *destination,
                    left: *left,
                    right: *right,
                    mask: mask.clone(),
                },
            },
            mir::Instruction::VectorSelect {
                destination,
                mask,
                then_value,
                else_value,
            } => Instruction {
                operation: InstructionOperation::VectorSelect,
                data: InstructionData::VectorSelect {
                    dest: *destination,
                    mask: *mask,
                    then_value: *then_value,
                    else_value: *else_value,
                },
            },

            mir::Instruction::VectorReduce {
                destination,
                operator,
                vector,
            } => Instruction {
                operation: InstructionOperation::VectorReduce,
                data: InstructionData::VectorReduce {
                    dest: *destination,
                    operator: *operator,
                    vector: *vector,
                },
            },

            mir::Instruction::VectorCompare {
                destination,
                operator,
                left,
                right,
            } => Instruction {
                operation: InstructionOperation::VectorCompare,
                data: InstructionData::VectorCompare {
                    dest: *destination,
                    operator: *operator,
                    left: *left,
                    right: *right,
                },
            },

            mir::Instruction::VectorConvert {
                destination,
                mode,
                vector,
            } => {
                let dest_type = value_type_for_value(*destination, self.value_type());
                let source_type = value_type_for_value(*vector, self.value_type());
                Instruction {
                    operation: InstructionOperation::VectorConvert,
                    data: InstructionData::VectorConvert {
                        dest: *destination,
                        mode: *mode,
                        vector: *vector,
                        source_type,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorLoad {
                destination,
                view,
                indices,
            } => {
                let view_type = value_type_for_value(*view, self.value_type());
                let args = pool.argument_range(self.tree.get_arguments(*indices));
                let element = tensor_element_type_for_view_type(self.tree, view_type)
                    .and_then(|element_type| tensor_element_access(self.layouts(), element_type));
                Instruction {
                    operation: InstructionOperation::TensorLoad,
                    data: InstructionData::TensorLoad {
                        dest: *destination,
                        view: *view,
                        indices: args,
                        view_type,
                        element,
                    },
                }
            }

            mir::Instruction::TensorStore {
                view,
                indices,
                value,
            } => {
                let view_type = value_type_for_value(*view, self.value_type());
                let args = pool.argument_range(self.tree.get_arguments(*indices));
                let element = tensor_element_type_for_view_type(self.tree, view_type)
                    .and_then(|element_type| tensor_element_access(self.layouts(), element_type));
                Instruction {
                    operation: InstructionOperation::TensorStore,
                    data: InstructionData::TensorStore {
                        view: *view,
                        indices: args,
                        value: *value,
                        view_type,
                        element,
                    },
                }
            }

            mir::Instruction::TensorFill { view, value } => {
                let view_type = value_type_for_value(*view, self.value_type());
                let element = tensor_element_type_for_view_type(self.tree, view_type)
                    .and_then(|element_type| tensor_element_access(self.layouts(), element_type));
                Instruction {
                    operation: InstructionOperation::TensorFill,
                    data: InstructionData::TensorFill {
                        view: *view,
                        value: *value,
                        view_type,
                        element,
                    },
                }
            }

            mir::Instruction::TensorCopy { target, source } => {
                let target_type = value_type_for_value(*target, self.value_type());
                let source_type = value_type_for_value(*source, self.value_type());
                let target_element = tensor_element_type_for_view_type(self.tree, target_type)
                    .and_then(|element_type| tensor_element_access(self.layouts(), element_type));
                let source_element = tensor_element_type_for_view_type(self.tree, source_type)
                    .and_then(|element_type| tensor_element_access(self.layouts(), element_type));
                Instruction {
                    operation: InstructionOperation::TensorCopy,
                    data: InstructionData::TensorCopy {
                        target: *target,
                        source: *source,
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
                let dest_type = value_type_for_value(*destination, self.value_type());
                let args = pool.argument_range(self.tree.get_arguments(*shape));
                Instruction {
                    operation: InstructionOperation::TensorReshape,
                    data: InstructionData::TensorReshape {
                        dest: *destination,
                        tensor: *tensor,
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
                let dest_type = value_type_for_value(*destination, self.value_type());
                let source_type = value_type_for_value(*tensor, self.value_type());
                Instruction {
                    operation: InstructionOperation::TensorBroadcast,
                    data: InstructionData::TensorBroadcast {
                        dest: *destination,
                        tensor: *tensor,
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
                let dest_type = value_type_for_value(*destination, self.value_type());
                let source_type = value_type_for_value(*tensor, self.value_type());
                Instruction {
                    operation: InstructionOperation::TensorTranspose,
                    data: InstructionData::TensorTranspose {
                        dest: *destination,
                        tensor: *tensor,
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
                let dest_type = value_type_for_value(*destination, self.value_type());
                let source_type = value_type_for_value(*tensor, self.value_type());
                let args = pool.argument_range(self.tree.get_arguments(*arguments));
                Instruction {
                    operation: InstructionOperation::TensorSlice,
                    data: InstructionData::TensorSlice {
                        dest: *destination,
                        tensor: *tensor,
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
                let dest_type = value_type_for_value(*destination, self.value_type());
                let source_type = value_type_for_value(*tensor, self.value_type());
                let args = pool.argument_range(self.tree.get_arguments(*arguments));
                Instruction {
                    operation: InstructionOperation::TensorPad,
                    data: InstructionData::TensorPad {
                        dest: *destination,
                        tensor: *tensor,
                        arguments: args,
                        low_count: *low_count,
                        high_count: *high_count,
                        interior_count: *interior_count,
                        value: *value,
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
                let dest_type = value_type_for_value(*destination, self.value_type());
                let tensor_value = self.tree.get_arguments(*tensors);
                let args = pool.argument_range(tensor_value);

                let mut tensor_type = Vec::with_capacity(tensor_value.len());
                for value in tensor_value {
                    let value_type = value_type_for_value(*value, self.value_type());
                    tensor_type.push(value_type);
                }

                Instruction {
                    operation: InstructionOperation::TensorConcat,
                    data: InstructionData::TensorConcat {
                        dest: *destination,
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
                let dest_type = value_type_for_value(*destination, self.value_type());
                let source_type = value_type_for_value(*tensor, self.value_type());
                Instruction {
                    operation: InstructionOperation::TensorReduce,
                    data: InstructionData::TensorReduce {
                        dest: *destination,
                        operator: *operator,
                        tensor: *tensor,
                        initial: *initial,
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
                let dest_type = value_type_for_value(*destination, self.value_type());
                let left_type = value_type_for_value(*left, self.value_type());
                let right_type = value_type_for_value(*right, self.value_type());
                Instruction {
                    operation: InstructionOperation::TensorDot,
                    data: InstructionData::TensorDot {
                        dest: *destination,
                        left: *left,
                        right: *right,
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
                let dest_type = value_type_for_value(*destination, self.value_type());
                let input_type = value_type_for_value(*input, self.value_type());
                let kernel_type = value_type_for_value(*kernel, self.value_type());
                Instruction {
                    operation: InstructionOperation::TensorConvolution,
                    data: InstructionData::TensorConvolution {
                        dest: *destination,
                        input: *input,
                        kernel: *kernel,
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
                let dest_type = value_type_for_value(*destination, self.value_type());
                let operand_type = value_type_for_value(*operand, self.value_type());
                let indices_type = value_type_for_value(*indices, self.value_type());
                Instruction {
                    operation: InstructionOperation::TensorGather,
                    data: InstructionData::TensorGather {
                        dest: *destination,
                        operand: *operand,
                        indices: *indices,
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
                let dest_type = value_type_for_value(*destination, self.value_type());
                let operand_type = value_type_for_value(*operand, self.value_type());
                let indices_type = value_type_for_value(*indices, self.value_type());
                let updates_type = value_type_for_value(*updates, self.value_type());
                Instruction {
                    operation: InstructionOperation::TensorScatter,
                    data: InstructionData::TensorScatter {
                        dest: *destination,
                        operand: *operand,
                        indices: *indices,
                        updates: *updates,
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
                let dest_type = value_type_for_value(*destination, self.value_type());
                let left_type = value_type_for_value(*left, self.value_type());
                let right_type = value_type_for_value(*right, self.value_type());
                Instruction {
                    operation: InstructionOperation::TensorCompare,
                    data: InstructionData::TensorCompare {
                        dest: *destination,
                        operator: *operator,
                        left: *left,
                        right: *right,
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
                let dest_type = value_type_for_value(*destination, self.value_type());
                Instruction {
                    operation: InstructionOperation::TensorSelect,
                    data: InstructionData::TensorSelect {
                        dest: *destination,
                        mask: *mask,
                        then_value: *then_value,
                        else_value: *else_value,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorConvert {
                destination,
                mode,
                tensor,
            } => {
                let dest_type = value_type_for_value(*destination, self.value_type());
                let source_type = value_type_for_value(*tensor, self.value_type());
                Instruction {
                    operation: InstructionOperation::TensorConvert,
                    data: InstructionData::TensorConvert {
                        dest: *destination,
                        mode: *mode,
                        tensor: *tensor,
                        source_type,
                        dest_type,
                    },
                }
            }

            mir::Instruction::TensorCast {
                destination,
                tensor,
            } => Instruction {
                operation: InstructionOperation::TensorCast,
                data: InstructionData::TensorCast {
                    dest: *destination,
                    tensor: *tensor,
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
                let args = pool.argument_range(self.tree.get_arguments(*arguments));
                let dest_type = value_type_for_value(*destination, self.value_type());
                let source_type = value_type_for_value(*view, self.value_type());
                let element = tensor_element_type_for_view_type(self.tree, source_type)
                    .and_then(|element_type| tensor_element_access(self.layouts(), element_type));
                Instruction {
                    operation: InstructionOperation::TensorView,
                    data: InstructionData::TensorView {
                        dest: *destination,
                        view: *view,
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

            mir::Instruction::ManagedAlloc {
                destination,
                layout,
                ..
            } => Instruction {
                operation: InstructionOperation::ManagedAlloc,
                data: InstructionData::ManagedAlloc {
                    dest: *destination,
                    reference: reference_meta_for_value(self.value_kind_map(), *destination),
                    storage_type: *layout,
                    layout_id: self.tree.type_layout_id(*layout),
                    byte_len: self.layout_byte_len(*layout),
                },
            },

            mir::Instruction::ManagedAllocArray {
                destination,
                element,
                length,
                ..
            } => Instruction {
                operation: InstructionOperation::ManagedAllocArray,
                data: InstructionData::ManagedAllocArray {
                    dest: *destination,
                    length: *length,
                    reference: reference_meta_for_value(self.value_kind_map(), *destination),
                    element_type: *element,
                },
            },

            mir::Instruction::RawAlloc {
                destination,
                layout,
                ..
            } => Instruction {
                operation: InstructionOperation::RawAlloc,
                data: InstructionData::RawAlloc {
                    dest: *destination,
                    reference: reference_meta_for_value(self.value_kind_map(), *destination),
                    byte_len: self.layout_byte_len(*layout),
                },
            },

            mir::Instruction::RawFree { pointer } => Instruction {
                operation: InstructionOperation::RawFree,
                data: InstructionData::RawFree { pointer: *pointer },
            },

            mir::Instruction::StackAlloc {
                destination,
                layout,
                ..
            } => Instruction {
                operation: InstructionOperation::StackAlloc,
                data: InstructionData::StackAlloc {
                    dest: *destination,
                    reference: reference_meta_for_value(self.value_kind_map(), *destination),
                    storage_type: *layout,
                },
            },

            mir::Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
            } => {
                let args = pool.argument_range(self.tree.get_arguments(*arguments));
                Instruction {
                    operation: InstructionOperation::Intrinsic,
                    data: InstructionData::Intrinsic {
                        dest: pack_optional_value(*destination),
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
                operation: InstructionOperation::AtomicLoad,
                data: InstructionData::AtomicLoad {
                    dest: *destination,
                    pointer: *pointer,
                    raw_pointee: raw_pointee_type_for_value(self.tree, self.value_type(), *pointer),
                },
            },

            mir::Instruction::AtomicStore { pointer, value, .. } => Instruction {
                operation: InstructionOperation::AtomicStore,
                data: InstructionData::AtomicStore {
                    pointer: *pointer,
                    value: *value,
                    raw_pointee: raw_pointee_type_for_value(self.tree, self.value_type(), *pointer),
                },
            },

            mir::Instruction::AtomicCompareExchange {
                destination,
                pointer,
                expected,
                new_value,
                ..
            } => Instruction {
                operation: InstructionOperation::AtomicCompareExchange,
                data: InstructionData::AtomicCompareExchange {
                    dest: *destination,
                    pointer: *pointer,
                    expected: *expected,
                    new_value: *new_value,
                    raw_pointee: raw_pointee_type_for_value(self.tree, self.value_type(), *pointer),
                },
            },

            mir::Instruction::AtomicRmw {
                destination,
                operator,
                pointer,
                value,
                ..
            } => Instruction {
                operation: InstructionOperation::AtomicRmw,
                data: InstructionData::AtomicRmw {
                    dest: *destination,
                    operator: *operator,
                    pointer: *pointer,
                    value: *value,
                    raw_pointee: raw_pointee_type_for_value(self.tree, self.value_type(), *pointer),
                },
            },

            mir::Instruction::AtomicFence { .. } => Instruction {
                operation: InstructionOperation::AtomicFence,
                data: InstructionData::AtomicFence,
            },

            mir::Instruction::Barrier { .. } => Instruction {
                operation: InstructionOperation::Barrier,
                data: InstructionData::Barrier,
            },
        }
    }
    /// Return one lowered field count for one value.
    fn field_count_for_value(&self, value: mir::Value) -> u32 {
        self.value_kind_map()
            .get(value)
            .and_then(|kind| field_count_from_kind(self.tree, kind))
            .unwrap_or(UNKNOWN_FIELD_COUNT)
    }

    /// Return one lowered array length for one value.
    fn array_length_for_value(&self, value: mir::Value) -> u64 {
        self.value_kind_map()
            .get(value)
            .and_then(|kind| array_length_from_kind(self.tree, kind))
            .unwrap_or(UNKNOWN_ARRAY_LENGTH)
    }

    /// Return one lowered byte length for one layout.
    fn layout_byte_len(&self, layout: mir::LocalNodeId<mir::Type>) -> usize {
        self.layouts()
            .get(&layout)
            .map(|layout| layout.byte_len)
            .unwrap_or_else(|| panic!("missing lowered layout for type: {layout:?}"))
    }
}

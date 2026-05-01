use destack_heap::HeapOptions;
use destack_mir as mir;

use crate::program::{
    AllocationLayout, FrameAccess, INVALID_VALUE_ID, Instruction, Layout, Opcode, Operands,
    PointeeAccess, PointerClass, ValueLayout, pack_optional_value, pointer_class_from_reference,
    value_layout_from_type,
};
use crate::{Error, Result};

use super::access::{pointee_access, slice_element_access};
use super::lower::BlockLowerer;
use super::opcode::{
    select_element_addr_opcode, select_field_addr_opcode, select_load_opcode, select_store_opcode,
};
use super::pool::Pool;
use super::value::{
    heap_pointee_type_for_value, heap_pointee_type_for_value_layout, pointer_class_for_value,
    raw_pointee_type_for_value, raw_pointee_type_for_value_layout, reference_meta_for_value,
};

/// Return the heap class for one slice backing allocation.
fn slice_backing_pointer_class(
    tree: &mir::Tree,
    result_type: mir::LocalNodeId<mir::Type>,
) -> Result<PointerClass> {
    let mir::Type::Slice {
        kind,
        address_space,
        ..
    } = tree.get(result_type)
    else {
        return Err(Error::TypeMismatch {
            expected: "slice result type".to_string(),
            actual: format!("{result_type:?}"),
        });
    };

    let pointer_class = pointer_class_from_reference(address_space.clone(), *kind);
    match pointer_class {
        PointerClass::Heap | PointerClass::SharedHeap => Ok(pointer_class),
        _ => Err(Error::InvalidPointerType {
            actual: format!("{pointer_class:?}"),
        }),
    }
}

/// Build one allocation layout for a concrete MIR type.
fn allocation_layout(
    pool: &mut Pool<'_>,
    pointer_class: PointerClass,
    layout: &Layout,
    heap_options: &HeapOptions,
    shared_heap_options: &HeapOptions,
) -> Result<AllocationLayout> {
    let is_noscan = !layout.reference_map.has_reference();
    let has_shared_reference = layout.reference_map.has_shared_reference();
    let class = match pointer_class {
        PointerClass::Heap => {
            heap_options.allocation_class(layout.byte_len, layout.alignment(), is_noscan)
        }
        PointerClass::SharedHeap => {
            shared_heap_options.allocation_class(layout.byte_len, layout.alignment(), is_noscan)
        }
        _ => {
            return Err(Error::InvalidPointerType {
                actual: format!("{pointer_class:?}"),
            });
        }
    };
    let reference_map = pool.reference_map(layout.reference_map.clone());
    let class = pool.allocation_class(class);

    Ok(AllocationLayout {
        byte_len: layout.byte_len,
        alignment: layout.alignment(),
        reference_map,
        is_noscan,
        has_shared_reference,
        class,
    })
}

/// Return one frame access from a pointer access.
fn frame_access(access: PointeeAccess) -> FrameAccess {
    FrameAccess {
        value_type: access.value_type,
        byte_offset: access.byte_offset,
        byte_stride: 0,
        length: 0,
        byte_len: access.byte_len,
        word_layout: access.word_layout,
    }
}

impl<'a> BlockLowerer<'a> {
    /// Lower one local get.
    pub(super) fn lower_local_get(
        &self,
        destination: mir::ValueReference,
        local: mir::LocalReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "local get destination".to_string(),
            })?;
        let local = local.local().ok_or_else(|| Error::MissingRepresentation {
            context: "local get source".to_string(),
        })?;
        let local = self.local_index(local)?;

        Ok(Instruction {
            opcode: Opcode::LoadLocal,
            operands: Operands::LocalGet {
                dest: destination,
                local,
            },
        })
    }

    /// Lower one local address.
    pub(super) fn lower_local_addr(
        &self,
        destination: mir::ValueReference,
        local: mir::LocalReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "local address destination".to_string(),
            })?;
        let local = local.local().ok_or_else(|| Error::MissingRepresentation {
            context: "local address local".to_string(),
        })?;
        let local = self.local_index(local)?;

        Ok(Instruction {
            opcode: Opcode::AddressLocal,
            operands: Operands::LocalAddr {
                dest: destination,
                local,
                reference: reference_meta_for_value(self.value_layout_map(), destination),
            },
        })
    }

    /// Lower one local set.
    pub(super) fn lower_local_set(
        &self,
        local: mir::LocalReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        let local = local.local().ok_or_else(|| Error::MissingRepresentation {
            context: "local set destination".to_string(),
        })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "local set value".to_string(),
        })?;
        let local = self.local_index(local)?;

        Ok(Instruction {
            opcode: Opcode::StoreLocal,
            operands: Operands::LocalSet { local, value },
        })
    }

    /// Lower one static address.
    pub(super) fn lower_static_addr(
        &self,
        destination: mir::ValueReference,
        global: mir::GlobalReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "static address destination".to_string(),
            })?;
        let global = global
            .global()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "static address global".to_string(),
            })?;

        Ok(Instruction {
            opcode: Opcode::AddressStatic,
            operands: Operands::StaticAddr {
                dest: destination,
                global: global.id,
                reference: reference_meta_for_value(self.value_layout_map(), destination),
            },
        })
    }

    /// Lower one function address.
    pub(super) fn lower_function_addr(
        &self,
        destination: mir::ValueReference,
        function: mir::FunctionReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "function address destination".to_string(),
            })?;
        let function = function
            .function()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "function address callee".to_string(),
            })?;

        Ok(Instruction {
            opcode: Opcode::AddressFunction,
            operands: Operands::FunctionAddr {
                dest: destination,
                function: function.id,
            },
        })
    }

    /// Lower one load.
    pub(super) fn lower_load(
        &self,
        pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        pointer: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "load destination".to_string(),
            })?;
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "load pointer".to_string(),
            })?;
        let access = self.pointee_access_for_value(pointer)?;
        let opcode = select_load_opcode(access)?;
        let operands = match opcode {
            Opcode::LoadFrame => Operands::LoadFrame {
                dest: destination,
                base: pointer,
                index: mir::Value(INVALID_VALUE_ID),
                access: pool.frame_access(frame_access(access)),
            },
            Opcode::LoadFrameBytes => Operands::LoadFrameBytes {
                destination,
                address: pointer,
                access: pool.pointee_access(access),
            },
            _ => Operands::Load {
                dest: destination,
                pointer,
                access: pool.pointee_access(access),
            },
        };

        Ok(Instruction { opcode, operands })
    }

    /// Lower one store.
    pub(super) fn lower_store(
        &self,
        pool: &mut Pool<'_>,
        pointer: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "store pointer".to_string(),
            })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "store value".to_string(),
        })?;
        let access = self.pointee_access_for_value(pointer)?;
        let opcode = select_store_opcode(access)?;
        let operands = match opcode {
            Opcode::StoreFrame => Operands::StoreFrame {
                base: pointer,
                index: mir::Value(INVALID_VALUE_ID),
                value,
                reference: reference_meta_for_value(self.value_layout_map(), pointer),
                access: pool.frame_access(frame_access(access)),
            },
            Opcode::StoreFrameBytes => Operands::StoreFrameBytes {
                address: pointer,
                source: value,
                access: pool.pointee_access(access),
            },
            _ => Operands::Store {
                pointer,
                value,
                access: pool.pointee_access(access),
            },
        };

        Ok(Instruction { opcode, operands })
    }

    /// Lower one field address.
    pub(super) fn lower_field_addr(
        &self,
        pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        base: mir::ValueReference,
        index: u32,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "field address destination".to_string(),
            })?;
        let base = base.value().ok_or_else(|| Error::MissingRepresentation {
            context: "field address base".to_string(),
        })?;
        let opcode = select_field_addr_opcode(self.value_layout_map(), base)?;
        let field = self.field_access_for_value(base, index)?;

        if opcode == Opcode::AddressFrame {
            return Ok(Instruction {
                opcode,
                operands: Operands::AddressFrame {
                    dest: destination,
                    base,
                    index: mir::Value(INVALID_VALUE_ID),
                    reference: reference_meta_for_value(self.value_layout_map(), destination),
                    access: pool.frame_access(field.into()),
                },
            });
        }

        Ok(Instruction {
            opcode,
            operands: Operands::FieldAddr {
                dest: destination,
                base,
                index,
                reference: reference_meta_for_value(self.value_layout_map(), destination),
                field_count: self.field_count_for_value(base)?,
                field: pool.field_access(field),
            },
        })
    }

    /// Lower one element address.
    pub(super) fn lower_element_addr(
        &self,
        pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        array: mir::ValueReference,
        index: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "element address destination".to_string(),
            })?;
        let array = array.value().ok_or_else(|| Error::MissingRepresentation {
            context: "element address array".to_string(),
        })?;
        let index = index.value().ok_or_else(|| Error::MissingRepresentation {
            context: "element address index".to_string(),
        })?;
        let pointer_class = pointer_class_for_value(self.value_layout_map(), array);
        let pointee_type = self.aggregate_type_for_value(array)?;

        if let Some(access) = pointee_type.and_then(|pointee_type| {
            slice_element_access(self.tree, self.layouts(), pointee_type, pointer_class)
        }) {
            return Ok(Instruction {
                opcode: Opcode::AddressSliceElement,
                operands: Operands::SliceElementAddr {
                    dest: destination,
                    slice: array,
                    index,
                    reference: reference_meta_for_value(self.value_layout_map(), destination),
                    access: pool.slice_element_access(access),
                },
            });
        }
        let opcode = select_element_addr_opcode(self.value_layout_map(), array)?;
        let element = self.element_access_for_value(array)?;
        let array_length = self.array_length_for_value(array)?;

        if opcode == Opcode::AddressFrame {
            return Ok(Instruction {
                opcode,
                operands: Operands::AddressFrame {
                    dest: destination,
                    base: array,
                    index,
                    reference: reference_meta_for_value(self.value_layout_map(), destination),
                    access: pool.frame_access(element.into_frame_access(0, array_length)),
                },
            });
        }

        Ok(Instruction {
            opcode,
            operands: Operands::ElementAddr {
                dest: destination,
                array,
                index,
                reference: reference_meta_for_value(self.value_layout_map(), destination),
                array_length,
                element: pool.element_access(element),
            },
        })
    }

    /// Lower one managed allocation.
    pub(super) fn lower_new(
        &self,
        pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        layout: mir::TypeReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "new destination".to_string(),
            })?;
        let allocation_type = layout.ty().ok_or_else(|| Error::MissingRepresentation {
            context: "new layout".to_string(),
        })?;
        let layout = self.layout_for_type(allocation_type)?;
        let pointer_class = pointer_class_for_value(self.value_layout_map(), destination);
        let opcode = match pointer_class {
            PointerClass::Heap => Opcode::AllocateHeap,
            PointerClass::SharedHeap => Opcode::AllocateSharedHeap,
            _ => {
                return Err(Error::InvalidPointerType {
                    actual: format!("{pointer_class:?}"),
                });
            }
        };
        let allocation = allocation_layout(
            pool,
            pointer_class,
            layout,
            self.heap_options,
            self.shared_heap_options,
        )?;

        Ok(Instruction {
            opcode,
            operands: Operands::New {
                dest: destination,
                allocation: pool.allocation_layout(allocation),
            },
        })
    }

    /// Lower one slice allocation.
    pub(super) fn lower_new_slice(
        &self,
        pool: &mut Pool<'_>,
        destination: mir::ValueReference,
        element: mir::TypeReference,
        length: mir::ValueReference,
        result_type: mir::TypeReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "new.slice destination".to_string(),
            })?;
        let element_type = element.ty().ok_or_else(|| Error::MissingRepresentation {
            context: "new.slice element type".to_string(),
        })?;
        let result_type = result_type
            .ty()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "new.slice result type".to_string(),
            })?;
        let length = length.value().ok_or_else(|| Error::MissingRepresentation {
            context: "new.slice length".to_string(),
        })?;
        let element_layout = self.layout_for_type(element_type)?;
        let element_alignment = element_layout.alignment();
        let pointer_class = slice_backing_pointer_class(self.tree, result_type)?;
        let element = allocation_layout(
            pool,
            pointer_class,
            element_layout,
            self.heap_options,
            self.shared_heap_options,
        )?;

        Ok(Instruction {
            opcode: Opcode::AllocateSlice,
            operands: Operands::NewSlice {
                dest: destination,
                length,
                pointer_class,
                element: pool.allocation_layout(element),
                element_alignment,
            },
        })
    }

    /// Lower one raw allocation.
    pub(super) fn lower_raw_alloc(
        &self,
        destination: mir::ValueReference,
        layout: mir::TypeReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "raw alloc destination".to_string(),
            })?;
        let layout = layout.ty().ok_or_else(|| Error::MissingRepresentation {
            context: "raw alloc layout".to_string(),
        })?;

        Ok(Instruction {
            opcode: Opcode::AllocateRaw,
            operands: Operands::RawAlloc {
                dest: destination,
                byte_len: self.byte_len_for_type(layout)?,
            },
        })
    }

    /// Lower one raw free.
    pub(super) fn lower_raw_free(&self, pointer: mir::ValueReference) -> Result<Instruction> {
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "raw free pointer".to_string(),
            })?;

        Ok(Instruction {
            opcode: Opcode::FreeRaw,
            operands: Operands::RawFree { pointer },
        })
    }

    /// Lower one stack allocation.
    pub(super) fn lower_stack_alloc(
        &self,
        destination: mir::ValueReference,
        layout: mir::TypeReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "stack alloc destination".to_string(),
            })?;
        let allocation_type = layout.ty().ok_or_else(|| Error::MissingRepresentation {
            context: "stack alloc layout".to_string(),
        })?;

        Ok(Instruction {
            opcode: Opcode::AllocateStack,
            operands: Operands::StackAlloc {
                dest: destination,
                reference: reference_meta_for_value(self.value_layout_map(), destination),
                allocation_type,
            },
        })
    }

    /// Lower one intrinsic call.
    pub(super) fn lower_intrinsic(
        &self,
        destination: Option<mir::ValueReference>,
        intrinsic: mir::Intrinsic,
        arguments: mir::ArgumentSlice,
        pool: &mut Pool<'_>,
    ) -> Result<Instruction> {
        let arguments = pool
            .argument_reference_range(self.tree.get_arguments(arguments), "intrinsic argument")?;
        let destination = pack_optional_value(
            destination
                .map(|value| {
                    value.value().ok_or_else(|| Error::MissingRepresentation {
                        context: "intrinsic destination".to_string(),
                    })
                })
                .transpose()?,
        );

        Ok(Instruction {
            opcode: Opcode::Intrinsic,
            operands: Operands::Intrinsic {
                dest: destination,
                intrinsic,
                arguments,
            },
        })
    }

    /// Lower one atomic load.
    pub(super) fn lower_atomic_load(
        &self,
        destination: mir::ValueReference,
        pointer: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic load destination".to_string(),
            })?;
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic load pointer".to_string(),
            })?;

        Ok(Instruction {
            opcode: Opcode::AtomicLoad,
            operands: Operands::AtomicLoad {
                dest: destination,
                pointer,
                raw_pointee: raw_pointee_type_for_value(self.tree, self.value_type(), pointer),
            },
        })
    }

    /// Lower one atomic store.
    pub(super) fn lower_atomic_store(
        &self,
        pointer: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic store pointer".to_string(),
            })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "atomic store value".to_string(),
        })?;

        Ok(Instruction {
            opcode: Opcode::AtomicStore,
            operands: Operands::AtomicStore {
                pointer,
                value,
                raw_pointee: raw_pointee_type_for_value(self.tree, self.value_type(), pointer),
            },
        })
    }

    /// Lower one atomic compare exchange.
    pub(super) fn lower_atomic_compare_exchange(
        &self,
        destination: mir::ValueReference,
        pointer: mir::ValueReference,
        expected: mir::ValueReference,
        new_value: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic compare exchange destination".to_string(),
            })?;
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic compare exchange pointer".to_string(),
            })?;
        let expected = expected
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic compare exchange expected".to_string(),
            })?;
        let new_value = new_value
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic compare exchange new value".to_string(),
            })?;

        Ok(Instruction {
            opcode: Opcode::AtomicCompareExchange,
            operands: Operands::AtomicCompareExchange {
                dest: destination,
                pointer,
                expected,
                new_value,
                raw_pointee: raw_pointee_type_for_value(self.tree, self.value_type(), pointer),
            },
        })
    }

    /// Lower one atomic read-modify-write.
    pub(super) fn lower_atomic_rmw(
        &self,
        destination: mir::ValueReference,
        operator: mir::AtomicRmwOperator,
        pointer: mir::ValueReference,
        value: mir::ValueReference,
    ) -> Result<Instruction> {
        let destination = destination
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic rmw destination".to_string(),
            })?;
        let pointer = pointer
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "atomic rmw pointer".to_string(),
            })?;
        let value = value.value().ok_or_else(|| Error::MissingRepresentation {
            context: "atomic rmw value".to_string(),
        })?;

        Ok(Instruction {
            opcode: Opcode::AtomicRmw,
            operands: Operands::AtomicRmw {
                dest: destination,
                operator,
                pointer,
                value,
                raw_pointee: raw_pointee_type_for_value(self.tree, self.value_type(), pointer),
            },
        })
    }

    /// Lower one write barrier.
    pub(super) fn lower_barrier_write(
        &self,
        object: mir::ValueReference,
        offset: mir::ValueReference,
        byte_len: mir::ValueReference,
    ) -> Result<Instruction> {
        let object = object.value().ok_or_else(|| Error::MissingRepresentation {
            context: "barrier.write object".to_string(),
        })?;
        let offset = offset.value().ok_or_else(|| Error::MissingRepresentation {
            context: "barrier.write offset".to_string(),
        })?;
        let byte_len = byte_len
            .value()
            .ok_or_else(|| Error::MissingRepresentation {
                context: "barrier.write byte length".to_string(),
            })?;
        let object_type = self.value_type_for_value(object)?;
        let object_layout = value_layout_from_type(self.tree, object_type);
        let ValueLayout::Pointer { pointer_class, .. } = object_layout else {
            return Err(Error::TypeMismatch {
                expected: "managed barrier reference".to_string(),
                actual: format!("{object_layout:?}"),
            });
        };

        Ok(Instruction {
            opcode: Opcode::BarrierWrite,
            operands: Operands::BarrierWrite {
                object,
                offset,
                byte_len,
                pointer_class,
            },
        })
    }

    /// Return the lowered access for a pointer value.
    fn pointee_access_for_value(&self, pointer: mir::Value) -> Result<PointeeAccess> {
        let pointee_type = heap_pointee_type_for_value_layout(self.value_layout_map(), pointer)
            .or_else(|| raw_pointee_type_for_value_layout(self.value_layout_map(), pointer))
            .or_else(|| heap_pointee_type_for_value(self.tree, self.value_type(), pointer))
            .or_else(|| raw_pointee_type_for_value(self.tree, self.value_type(), pointer));
        let pointer_class = pointer_class_for_value(self.value_layout_map(), pointer);

        pointee_type
            .and_then(|pointee_type| {
                pointee_access(self.tree, self.layouts(), pointee_type, pointer_class)
            })
            .ok_or(Error::InvalidInstruction)
    }
}

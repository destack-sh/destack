use std::mem::offset_of;

use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use tspp_mir as mir;
use tspp_native as native;

use crate::EmitError;

use super::super::r#type::ValueType;
use super::{FunctionEmitter, Value};

/// One selected place's address.
#[derive(Debug, Clone, Copy)]
pub(super) struct PlaceAddress {
    /// The address of the selected storage.
    pub(super) address: cir::Value,
    /// The memory the address points into: a world offset or a native pointer.
    pub(super) kind: mir::AddressKind,
    /// The second descriptor word, like a slice length, when the place selects a referent.
    pub(super) metadata: Option<cir::Value>,
}

impl<'a> FunctionEmitter<'a> {
    /// Select the storage of one place.
    pub(super) fn emit_place(
        &mut self,
        place: &mir::Place,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<PlaceAddress, EmitError> {
        let steps = self
            .optimized
            .layouts
            .address_steps(place, self.function_id, &self.optimized.tree)
            .map_err(|error| self.invalid(&error.to_string()))?;

        // read a root descriptor from its value when the place follows it
        let (mut selected, steps) = match (place.origin, steps.split_first()) {
            (
                mir::PlaceOrigin::Value(value),
                Some((mir::AddressStep::Follow(descriptor), rest)),
            ) => (
                self.split_descriptor(self.value(value)?, *descriptor)?,
                rest,
            ),
            // address a value's canonical bytes in native stack storage
            (mir::PlaceOrigin::Value(value), _) => {
                let value_type = self.types.value(self.value_type(value)?)?;
                let address = self.materialize(self.value(value)?, value_type, builder)?;

                (PlaceAddress::pointer(address), steps.as_slice())
            }
            // address a local's stack slot
            (mir::PlaceOrigin::Local(local), _) => {
                let slot = self.locals[&local].slot;
                let address = builder.ins().stack_addr(self.types.pointer(), slot, 0);

                (PlaceAddress::pointer(address), steps.as_slice())
            }
            // offset a global inside its static space
            (mir::PlaceOrigin::Global(global), _) => {
                let address = PlaceAddress {
                    address: self.global_reference(global, builder)?,
                    kind: mir::AddressKind::Reference,
                    metadata: None,
                };

                (address, steps.as_slice())
            }
        };

        // apply each address computation to the selected storage
        for step in steps {
            selected = match *step {
                // load the descriptor stored at the address
                mir::AddressStep::Follow(descriptor) => {
                    let pointer = self.pointer(selected, builder)?;
                    let value = self.load(pointer, self.types.value(descriptor.ty)?, builder)?;

                    self.split_descriptor(value, descriptor)?
                }
                // advance by a constant offset
                mir::AddressStep::Offset(offset) => {
                    let offset = i64::try_from(offset)
                        .map_err(|_| self.invalid("native place offset exceeds i64"))?;
                    let address = match offset {
                        0 => selected.address,
                        offset => builder.ins().iadd_imm_u(selected.address, offset),
                    };

                    PlaceAddress {
                        address,
                        kind: selected.kind,
                        metadata: None,
                    }
                }
                // advance by a scaled runtime index
                mir::AddressStep::Index {
                    index,
                    stride,
                    length,
                } => {
                    let index = self.pointer_integer(self.scalar(index)?, builder)?;
                    let offset = builder.ins().imul_imm_u(index, i64::from(stride));

                    PlaceAddress {
                        address: builder.ins().iadd(selected.address, offset),
                        kind: selected.kind,
                        metadata: length.map(|length| self.scalar(length)).transpose()?,
                    }
                }
            };
        }

        Ok(selected)
    }

    /// Select one place as a native pointer.
    pub(super) fn place_pointer(
        &mut self,
        place: &mir::Place,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let selected = self.emit_place(place, builder)?;

        self.pointer(selected, builder)
    }

    /// Load one MIR value from a place, volatile when requested.
    pub(super) fn emit_load(
        &mut self,
        destination: mir::Value,
        place: &mir::Place,
        is_volatile: bool,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let pointer = self.place_pointer(place, builder)?;
        let value_type = self.types.value(self.value_type(destination)?)?;
        let value = if is_volatile {
            self.load_volatile(pointer, value_type, builder)?
        } else {
            self.load(pointer, value_type, builder)?
        };

        self.set(destination, value)
    }

    /// Store one MIR value into a place, volatile when requested.
    pub(super) fn emit_store(
        &mut self,
        place: &mir::Place,
        value: mir::Value,
        is_volatile: bool,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let pointer = self.place_pointer(place, builder)?;
        let value_type = self.types.value(self.value_type(value)?)?;
        let value = self.value(value)?;

        if is_volatile {
            self.store_volatile(pointer, value, value_type, builder)
        } else {
            self.store(pointer, value, value_type, builder)
        }
    }

    /// Define one reference-like descriptor addressing a place.
    pub(super) fn emit_address(
        &mut self,
        destination: mir::Value,
        place: &mir::Place,
        result_type: mir::TypeId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let selected = self.emit_place(place, builder)?;
        let descriptor = self.descriptor(result_type)?;

        // convert the address into the memory the result type points into
        let address = match (selected.kind, descriptor.kind) {
            (mir::AddressKind::Reference, mir::AddressKind::Pointer) => {
                self.rebase(selected.address, builder)?
            }
            (mir::AddressKind::Pointer, mir::AddressKind::Reference) => {
                self.world_offset(selected.address, builder)?
            }
            _ => selected.address,
        };

        // assemble the descriptor words in the result representation
        let value = match (
            self.types.value(result_type)?,
            selected.metadata,
            descriptor.metadata,
        ) {
            (ValueType::Direct { .. }, None, None) => Value::Direct(address),
            (ValueType::ScalarPair { .. }, Some(metadata), Some(metadata_offset)) => {
                let index = usize::from(descriptor.address > metadata_offset);
                let mut fields = [metadata; 2];
                fields[index] = address;

                Value::ScalarPair(fields)
            }
            _ => return Err(self.invalid("native address disagrees with its result descriptor")),
        };

        self.set(destination, value)
    }

    /// Return the descriptor words of one reference-like type.
    pub(super) fn descriptor(&self, ty: mir::TypeId) -> Result<mir::Descriptor, EmitError> {
        mir::Descriptor::new(ty, &self.optimized.tree, &self.optimized.layouts)
            .map_err(|error| self.invalid(&error.to_string()))
    }

    /// Split one reference-like value into the address and metadata words its descriptor names.
    pub(super) fn split_descriptor(
        &self,
        value: Value,
        descriptor: mir::Descriptor,
    ) -> Result<PlaceAddress, EmitError> {
        let (address, metadata) = match (value, descriptor.metadata) {
            (Value::Direct(address), None) => (address, None),
            (Value::ScalarPair(fields), Some(metadata)) => {
                let index = usize::from(descriptor.address > metadata);

                (fields[index], Some(fields[1 - index]))
            }
            _ => return Err(self.invalid("native descriptor disagrees with its layout")),
        };

        Ok(PlaceAddress {
            address,
            kind: descriptor.kind,
            metadata,
        })
    }

    /// Return one selected address as a native pointer.
    pub(super) fn pointer(
        &self,
        selected: PlaceAddress,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        match selected.kind {
            mir::AddressKind::Reference => self.rebase(selected.address, builder),
            mir::AddressKind::Pointer => Ok(selected.address),
        }
    }

    /// Return the world offset of one global.
    fn global_reference(
        &mut self,
        global: mir::GlobalId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let definition = self.optimized.tree.get(global);
        let offset = self.index_pointer(native::Index::Global { global: global.id }, builder)?;

        // offset the global from the base of the static space it lives in
        let base = match definition.space {
            mir::Space::Constant => offset_of!(native::abi::Activation, constants),
            mir::Space::Local => offset_of!(native::abi::Activation, local_statics),
            mir::Space::Shared => offset_of!(native::abi::Activation, shared_statics),
        };
        let base = self.static_offset(base, builder)?;

        Ok(builder.ins().iadd(base, offset))
    }

    /// Convert one unsigned index into the native pointer width.
    fn pointer_integer(
        &self,
        value: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let source = builder.func.dfg.value_type(value);
        let target = self.types.pointer();
        if !source.is_int() {
            return Err(self.invalid("a native index outside an integer type"));
        }

        let value = if source.bits() < target.bits() {
            builder.ins().uextend(target, value)
        } else if source.bits() > target.bits() {
            builder.ins().ireduce(target, value)
        } else {
            value
        };

        Ok(value)
    }
}

impl PlaceAddress {
    /// Create one address of native storage outside any descriptor.
    fn pointer(address: cir::Value) -> Self {
        Self {
            address,
            kind: mir::AddressKind::Pointer,
            metadata: None,
        }
    }
}

use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::ir::condcodes::IntCC;
use destack_mir as mir;
use destack_native as native;

use crate::EmitError;

use super::{FunctionEmitter, Value};

impl<'a> FunctionEmitter<'a> {
    /// Emit one machine memory intrinsic.
    pub(super) fn emit_memory_intrinsic(
        &mut self,
        destination: Option<mir::Value>,
        intrinsic: mir::Intrinsic,
        arguments: &[mir::Value],
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        match intrinsic {
            // transfer a byte range between two pointers
            mir::Intrinsic::Memcpy | mir::Intrinsic::Memmove => {
                let [destination, source, byte_len] = arguments else {
                    return Err(self.invalid("native memory transfer requires three arguments"));
                };

                let destination = self.materialize_pointer(*destination, builder)?;
                let source = self.materialize_pointer(*source, builder)?;
                let byte_len = self.scalar(*byte_len)?;

                // copy a disjoint range
                if intrinsic == mir::Intrinsic::Memcpy {
                    builder.call_memcpy(
                        self.types.frontend_config(),
                        destination,
                        source,
                        byte_len,
                    );
                }
                // move a range that may overlap
                else {
                    builder.call_memmove(
                        self.types.frontend_config(),
                        destination,
                        source,
                        byte_len,
                    );
                }
            }
            // fill a byte range with one byte
            mir::Intrinsic::Memset => {
                let [destination, byte, byte_len] = arguments else {
                    return Err(self.invalid("native memory fill requires three arguments"));
                };

                let destination = self.materialize_pointer(*destination, builder)?;
                let byte = self.scalar(*byte)?;
                let byte_len = self.scalar(*byte_len)?;

                builder.call_memset(self.types.frontend_config(), destination, byte, byte_len);
            }
            // order two byte ranges
            mir::Intrinsic::Memcmp => {
                let [left, right, byte_len] = arguments else {
                    return Err(self.invalid("native memory comparison requires three arguments"));
                };
                let destination = destination
                    .ok_or_else(|| self.invalid("native memory comparison has no destination"))?;

                let left = self.materialize_pointer(*left, builder)?;
                let right = self.materialize_pointer(*right, builder)?;
                let byte_len = self.scalar(*byte_len)?;

                let value =
                    builder.call_memcmp(self.types.frontend_config(), left, right, byte_len);
                self.set(destination, Value::Direct(value))?;
            }
            // check the argument count, the machine prefetch hint carries no effect
            mir::Intrinsic::PrefetchRead | mir::Intrinsic::PrefetchWrite => {
                let [_pointer] = arguments else {
                    return Err(self.invalid("native prefetch requires one argument"));
                };
            }
            // subtract two pointers into a byte distance
            mir::Intrinsic::PointerByteOffsetFrom => {
                let [pointer, origin] = arguments else {
                    return Err(self.invalid("native pointer difference requires two arguments"));
                };
                let destination = destination
                    .ok_or_else(|| self.invalid("native pointer difference has no destination"))?;

                let pointer = self.materialize_pointer(*pointer, builder)?;
                let origin = self.materialize_pointer(*origin, builder)?;

                let offset = builder.ins().isub(pointer, origin);
                self.set(destination, Value::Direct(offset))?;
            }
            // read through a volatile pointer
            mir::Intrinsic::VolatileLoad => {
                let [pointer] = arguments else {
                    return Err(self.invalid("native volatile load requires one argument"));
                };
                let destination = destination
                    .ok_or_else(|| self.invalid("native volatile load has no destination"))?;

                let pointer = self.materialize_pointer(*pointer, builder)?;
                let value_type = self.types.value(self.value_type(destination)?)?;

                let value = self.load_volatile(pointer, value_type, builder)?;
                self.set(destination, value)?;
            }
            // write through a volatile pointer
            mir::Intrinsic::VolatileStore => {
                let [pointer, source] = arguments else {
                    return Err(self.invalid("native volatile store requires two arguments"));
                };

                let pointer = self.materialize_pointer(*pointer, builder)?;
                let value_type = self.types.value(self.value_type(*source)?)?;
                let source = self.value(*source)?;

                self.store_volatile(pointer, source, value_type, builder)?;
            }
            // compare two values byte for byte
            mir::Intrinsic::RawEq => {
                let [left, right] = arguments else {
                    return Err(self.invalid("native raw equality requires two arguments"));
                };
                let destination = destination
                    .ok_or_else(|| self.invalid("native raw equality has no destination"))?;

                let value_type = self.types.value(self.value_type(*left)?)?;
                let left = self.value(*left)?;
                let left = self.materialize(left, value_type, builder)?;
                let right = self.value(*right)?;
                let right = self.materialize(right, value_type, builder)?;

                // clamp the alignment into the byte width the comparison takes
                let alignment = value_type.alignment().min(u32::from(u8::MAX)) as u8;
                let alignment = std::num::NonZeroU8::new(alignment)
                    .ok_or_else(|| self.invalid("native raw equality has zero alignment"))?;

                let value = builder.emit_small_memory_compare(
                    self.types.frontend_config(),
                    IntCC::Equal,
                    left,
                    right,
                    u64::from(value_type.byte_len()),
                    alignment,
                    alignment,
                    cir::MemFlagsData::trusted(),
                );
                self.set(destination, Value::Direct(value))?;
            }
            // reject every intrinsic outside memory
            _ => return Err(self.invalid("native intrinsic is not a memory operation")),
        }

        Ok(())
    }

    /// Emit one stable global reference.
    pub(super) fn emit_global_address(
        &mut self,
        destination: mir::Value,
        global: mir::GlobalId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        // read the global definition
        let definition = self.optimized.tree.get(global);
        let offset = self.index_pointer(native::Index::Global { global: global.id }, builder)?;

        // offset the global from the base of the static space it lives in
        let reference = match definition.space {
            mir::Space::Parameter(_) => {
                return Err(self.invalid("native globals close every space"));
            }
            mir::Space::Constant => {
                let base = self.static_offset(
                    std::mem::offset_of!(native::abi::Activation, constants),
                    builder,
                )?;

                builder.ins().iadd(base, offset)
            }
            mir::Space::Local => {
                let base = self.static_offset(
                    std::mem::offset_of!(native::abi::Activation, local_statics),
                    builder,
                )?;

                builder.ins().iadd(base, offset)
            }
            mir::Space::Shared => {
                let base = self.static_offset(
                    std::mem::offset_of!(native::abi::Activation, shared_statics),
                    builder,
                )?;

                builder.ins().iadd(base, offset)
            }
        };
        self.set(destination, Value::Direct(reference))?;

        Ok(())
    }

    /// Load one MIR value through a stable reference.
    pub(super) fn emit_load(
        &mut self,
        destination: mir::Value,
        reference: mir::Value,
        result_type: mir::TypeId,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let pointer = self.materialize_pointer(reference, builder)?;
        let value_type = self.types.value(result_type)?;
        let value = self.load(pointer, value_type, builder)?;
        self.set(destination, value)?;

        Ok(())
    }

    /// Store one MIR value through a stable reference.
    pub(super) fn emit_store(
        &mut self,
        reference: mir::Value,
        value: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let pointer = self.materialize_pointer(reference, builder)?;
        let value_type = self.types.value(self.value_type(value)?)?;
        let value = self.value(value)?;

        self.store(pointer, value, value_type, builder)
    }

    /// Materialize one stable reference as a process-local native pointer.
    pub(super) fn materialize_pointer(
        &self,
        reference: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        // resolve the address storage type
        let ty = self
            .optimized
            .tree
            .storage_type(self.value_type(reference)?);
        let reference = self.reference(reference, builder)?;

        // take a pointer as it stands
        match self.optimized.tree.get(ty) {
            mir::Type::Pointer { .. } => Ok(reference),
            // rebase reference bits on the storage they name
            ty => {
                let storage = ty.reference_storage().ok_or_else(|| {
                    self.invalid("native memory access requires a reference or pointer")
                })?;

                self.materialize_reference(reference, storage, builder)
            }
        }
    }

    /// Materialize stable reference bits as one process-local native pointer.
    pub(super) fn materialize_reference(
        &self,
        reference: cir::Value,
        storage: mir::Storage,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        // pass frame stack addresses through, rebase every other storage on the memory base
        let base = match storage {
            mir::Storage::Frame => return Ok(reference),
            _ => self.activation_pointer(
                std::mem::offset_of!(native::abi::Activation, memory_base),
                builder,
            )?,
        };

        Ok(builder.ins().iadd(base, reference))
    }

    /// Load one pointer field from the native activation.
    pub(super) fn activation_pointer(
        &self,
        byte_offset: usize,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let flags = cir::MemFlagsData::trusted();
        let activation = self.activation()?;

        Ok(builder
            .ins()
            .load(self.types.pointer(), flags, activation, byte_offset as i32))
    }

    /// Read one value through a volatile native pointer.
    pub(super) fn load_volatile(
        &mut self,
        source: cir::Value,
        value_type: super::super::r#type::ValueType,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<Value, EmitError> {
        let destination = self.allocate(value_type, builder);
        let byte_len = builder
            .ins()
            .iconst(self.types.pointer(), i64::from(value_type.byte_len()));
        self.emit_runtime(
            native::abi::Operation::VolatileRead,
            &[source, destination, byte_len],
            builder,
        )?;

        self.load(destination, value_type, builder)
    }

    /// Write one value through a volatile native pointer.
    pub(super) fn store_volatile(
        &mut self,
        destination: cir::Value,
        value: Value,
        value_type: super::super::r#type::ValueType,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<(), EmitError> {
        let source = self.materialize(value, value_type, builder)?;
        let byte_len = builder
            .ins()
            .iconst(self.types.pointer(), i64::from(value_type.byte_len()));
        self.emit_runtime(
            native::abi::Operation::VolatileWrite,
            &[destination, source, byte_len],
            builder,
        )?;

        Ok(())
    }

    /// Load one static-space offset from the native activation.
    fn static_offset(
        &self,
        byte_offset: usize,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let byte_offset = byte_offset + std::mem::offset_of!(native::abi::StaticSpace, offset);

        self.activation_pointer(byte_offset, builder)
    }
}

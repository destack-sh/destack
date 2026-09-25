use cranelift_codegen::ir as cir;
use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::ir::condcodes::IntCC;
use tspp_mir as mir;
use tspp_native as native;

use crate::EmitError;

use super::super::r#type::ValueType;
use super::{FunctionEmitter, Value};

/// One disjoint alias region native accesses name for alias analysis.
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub(super) enum AliasRegion {
    /// The fields of the native activation, stable between calls.
    Activation = 0,
    /// World memory: the heap, the statics, and the frames.
    World = 1,
}

impl AliasRegion {
    /// The regions in alias identity order.
    pub(super) const ALL: [Self; 2] = [Self::Activation, Self::World];

    /// Declare this region in one function and return the flags of trusted accesses in it.
    pub(super) fn declare(self, function: &mut cir::Function) -> cir::MemFlagsData {
        let description = match self {
            Self::Activation => "activation",
            Self::World => "world",
        };
        let region = function.dfg.alias_regions.insert(cir::AliasRegionData {
            user_id: self as u32,
            description: description.into(),
        });

        cir::MemFlagsData::trusted().with_alias_region(Some(region))
    }
}

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
                let place = mir::Place::value(*pointer).with_projection(mir::Projection::Deref);

                self.emit_load(destination, &place, true, builder)?;
            }
            // write through a volatile pointer
            mir::Intrinsic::VolatileStore => {
                let [pointer, source] = arguments else {
                    return Err(self.invalid("native volatile store requires two arguments"));
                };
                let place = mir::Place::value(*pointer).with_projection(mir::Projection::Deref);

                self.emit_store(&place, *source, true, builder)?;
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

                // compare the bytes as world memory loads
                let flags = self.memory_flags(AliasRegion::World);
                let value = builder.emit_small_memory_compare(
                    self.types.frontend_config(),
                    IntCC::Equal,
                    left,
                    right,
                    u64::from(value_type.byte_len()),
                    alignment,
                    alignment,
                    flags,
                );
                self.set(destination, Value::Direct(value))?;
            }
            // reject every intrinsic outside memory
            _ => return Err(self.invalid("native intrinsic is not a memory operation")),
        }

        Ok(())
    }

    /// Return the native pointer one reference or pointer value addresses.
    pub(super) fn materialize_pointer(
        &self,
        value: mir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let descriptor = self.descriptor(self.value_type(value)?)?;
        let selected = self.split_descriptor(self.value(value)?, descriptor)?;

        self.pointer(selected, builder)
    }

    /// Rebase one world offset on the memory base as a native pointer, the base loaded at the use.
    pub(super) fn rebase(
        &self,
        offset: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let base = self.activation_pointer(
            std::mem::offset_of!(native::abi::Activation, memory_base),
            builder,
        )?;

        Ok(builder.ins().iadd(base, offset))
    }

    /// Return the world offset of one native address inside world memory.
    pub(super) fn world_offset(
        &self,
        address: cir::Value,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let base = self.activation_pointer(
            std::mem::offset_of!(native::abi::Activation, memory_base),
            builder,
        )?;

        Ok(builder.ins().isub(address, base))
    }

    /// Return the flags of one trusted access in one alias region.
    pub(super) fn memory_flags(&self, region: AliasRegion) -> cir::MemFlagsData {
        self.memory_flags[region as usize]
    }

    /// Load one pointer field from the native activation.
    pub(super) fn activation_pointer(
        &self,
        byte_offset: usize,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let flags = self.memory_flags(AliasRegion::Activation);
        let activation = self.activation()?;

        Ok(builder
            .ins()
            .load(self.types.pointer(), flags, activation, byte_offset as i32))
    }

    /// Read one value through a volatile native pointer.
    pub(super) fn load_volatile(
        &mut self,
        source: cir::Value,
        value_type: ValueType,
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
        value_type: ValueType,
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
    pub(super) fn static_offset(
        &self,
        byte_offset: usize,
        builder: &mut cranelift_frontend::FunctionBuilder<'_>,
    ) -> Result<cir::Value, EmitError> {
        let byte_offset = byte_offset + std::mem::offset_of!(native::abi::StaticSpace, offset);

        self.activation_pointer(byte_offset, builder)
    }
}

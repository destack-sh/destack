use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;
use std::ptr::NonNull;

use crate::diagnostic::Error;
use crate::executable::{Executable, StorageComponentLayout, StorageLayout, repr_type};
use crate::interpreter::machine::storage::{
    decode_raw_value, encode_raw_value, raw_type_alignment, raw_type_size,
};
use destack_heap::{MemoryContext, RawPointer, ReferenceMap, SharedPointer, Value};
use destack_mir as mir;

use super::{SchemaRegistry, StorageSchema, StringHandle, StringInterner, StringRef};

/// Align one byte offset up to the requested alignment.
#[inline(always)]
fn align_offset(offset: usize, alignment: usize) -> usize {
    if alignment <= 1 {
        return offset;
    }

    let remainder = offset % alignment;
    if remainder == 0 {
        offset
    } else {
        offset + (alignment - remainder)
    }
}

/// Handler invoked by the VM when calling an external function.
pub trait ExternalHandler:
    for<'ctx> Fn(&mut ExternalCallContext<'ctx>, &[Value]) -> Result<Value, Error> + Send + Sync
{
}

impl<T> ExternalHandler for T where
    T: for<'ctx> Fn(&mut ExternalCallContext<'ctx>, &[Value]) -> Result<Value, Error> + Send + Sync
{
}

/// Boxed external handler type.
pub type ExternalFn = Box<dyn ExternalHandler>;

/// Cached external handler pointer.
pub(crate) type ExternalFnPtr = NonNull<dyn ExternalHandler>;

/// Runtime call context with restricted access to isolate state.
pub struct ExternalCallContext<'ctx> {
    /// The immutable executable metadata for this isolate.
    executable: &'ctx Executable,
    /// The fallback runtime storage schemas for this isolate.
    schema: &'ctx SchemaRegistry,
    /// The string interner backing this external call.
    string_interner: &'ctx mut StringInterner,
    /// The execution memory backing this external call.
    memory: MemoryContext<'ctx>,
}

/// One VM aggregate storage shape.
#[derive(Clone, Debug)]
enum VmValueStorage {
    /// One isolate registered runtime storage payload.
    Runtime {
        /// The synthetic runtime storage schema.
        schema: StorageSchema,
    },
    /// One boxed callable payload.
    Function {
        /// The MIR function value type.
        ty: mir::LocalNodeId<mir::Type>,
    },
    /// One typed executable storage payload.
    Typed {
        /// The MIR storage type.
        ty: mir::LocalNodeId<mir::Type>,
        /// The semantic component layouts.
        components: Vec<StorageComponentLayout>,
    },
}

/// One cached borrowed VM value view.
#[derive(Debug)]
pub struct VmValueRef<'ctx> {
    /// The active external call context.
    context: *mut ExternalCallContext<'ctx>,
    /// The resolved storage shape.
    storage: VmValueStorage,
    /// The cached payload bytes.
    bytes: Vec<u8>,
    /// The lifetime marker for the call context.
    _marker: PhantomData<&'ctx ExternalCallContext<'ctx>>,
}

impl VmValueRef<'_> {
    /// Return the semantic component count for this value.
    pub fn component_count(&self) -> usize {
        match &self.storage {
            VmValueStorage::Runtime { schema } => schema.component_count(),
            VmValueStorage::Function { .. } => 2,
            VmValueStorage::Typed { components, .. } => components.len(),
        }
    }

    /// Decode one semantic component value from this cached payload.
    pub fn component_value(&self, index: u32) -> Result<Value, Error> {
        let context = unsafe { &mut *self.context };

        match &self.storage {
            VmValueStorage::Runtime { schema } => {
                let component_count = schema.component_count();
                if index as usize >= component_count {
                    return Err(Error::InvalidManagedReference);
                }

                let start = (index as usize)
                    .checked_mul(Value::BYTE_LEN)
                    .ok_or(Error::InvalidManagedReference)?;
                let end = start
                    .checked_add(Value::BYTE_LEN)
                    .ok_or(Error::InvalidManagedReference)?;
                let window = self
                    .bytes
                    .get(start..end)
                    .ok_or(Error::InvalidManagedReference)?;

                Value::from_byte_slice(window).ok_or(Error::InvalidManagedReference)
            }
            VmValueStorage::Function { ty } => {
                let values =
                    context.decode_function_value_components_from_storage(*ty, &self.bytes)?;
                values
                    .get(index as usize)
                    .copied()
                    .ok_or(Error::InvalidManagedReference)
            }
            VmValueStorage::Typed { components, .. } => {
                let component = components
                    .get(index as usize)
                    .copied()
                    .ok_or(Error::InvalidManagedReference)?;
                let byte_end = component
                    .offset
                    .checked_add(component.byte_len)
                    .ok_or(Error::InvalidManagedReference)?;
                let component_bytes = self
                    .bytes
                    .get(component.offset..byte_end)
                    .ok_or(Error::InvalidManagedReference)?;

                context.materialize_value_from_storage(component.ty, component_bytes)
            }
        }
    }

    /// Decode all semantic component values from this cached payload.
    pub fn component_values(&self) -> Result<Vec<Value>, Error> {
        let component_count = self.component_count();
        let mut values = Vec::with_capacity(component_count);

        // decode each semantic component from the cached payload
        for index in 0..component_count {
            values.push(self.component_value(index as u32)?);
        }

        Ok(values)
    }
}

/// One deferred VM value builder.
#[derive(Debug)]
pub struct VmValueBuilder<'ctx> {
    /// The active external call context.
    context: *mut ExternalCallContext<'ctx>,
    /// The target storage shape.
    storage: VmValueStorage,
    /// The managed allocation type id to stamp.
    type_id: u32,
    /// The optional managed layout id to stamp.
    layout_id: Option<destack_heap::LayoutId>,
    /// The staged semantic component values.
    components: Vec<Option<Value>>,
    /// The lifetime marker for the call context.
    _marker: PhantomData<&'ctx ExternalCallContext<'ctx>>,
}

impl VmValueBuilder<'_> {
    /// Return the semantic component count for this builder.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Write one semantic component value into this builder.
    pub fn write_component(&mut self, index: u32, value: Value) -> Result<(), Error> {
        let Some(slot) = self.components.get_mut(index as usize) else {
            return Err(Error::InvalidManagedReference);
        };

        *slot = Some(value);
        Ok(())
    }

    /// Finish this builder and return the final managed VM value.
    pub fn finish(self) -> Result<Value, Error> {
        let context = unsafe { &mut *self.context };

        let mut component_values = Vec::with_capacity(self.components.len());

        // validate that every semantic component was written
        for component in self.components {
            let Some(component) = component else {
                return Err(Error::TypeMismatch {
                    expected: "initialized composite component".to_string(),
                    actual: "missing composite component".to_string(),
                });
            };

            component_values.push(component);
        }

        let handle =
            context.allocate_zeroed_value_storage(&self.storage, self.type_id, self.layout_id)?;
        context.write_value_storage_components(handle, &self.storage, &component_values)?;

        Ok(Value::managed_reference(handle))
    }
}

impl<'ctx> ExternalCallContext<'ctx> {
    /// Return the semantic component count for one VM storage shape.
    fn value_storage_component_count(storage: &VmValueStorage) -> usize {
        match storage {
            VmValueStorage::Runtime { schema } => schema.component_count(),
            VmValueStorage::Function { .. } => 2,
            VmValueStorage::Typed { components, .. } => components.len(),
        }
    }

    /// Resolve one builtin collection storage type by short display name.
    fn builtin_collection_type(
        &self,
        short_name: &str,
        component_count: usize,
    ) -> Result<(u32, StorageSchema), Error> {
        if let Some(type_id) = self
            .schema
            .builtin_collection_type(short_name, component_count)
        {
            let schema = self
                .schema
                .schema(type_id)
                .ok_or(Error::InvalidInstruction)?;

            return Ok((type_id, schema));
        }

        Err(Error::TypeMismatch {
            expected: format!("builtin {short_name} storage type"),
            actual: "missing isolate schema".to_string(),
        })
    }

    /// Return the callable box payload layout for one function value type.
    fn function_value_payload_layout(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<(mir::LocalNodeId<mir::Type>, usize, usize, usize), Error> {
        let mir::Type::FunctionValue { signature } = self.executable.tree.get(ty) else {
            return Err(Error::TypeMismatch {
                expected: "function value type".to_string(),
                actual: format!("{ty:?}"),
            });
        };

        let signature_size = raw_type_size(&self.executable.tree, *signature)?;
        let signature_alignment = raw_type_alignment(&self.executable.tree, *signature)?;
        let environment_size = Value::BYTE_LEN;
        let environment_alignment = std::mem::align_of::<Value>();

        let function_offset = 0usize;
        let environment_offset = align_offset(signature_size, environment_alignment);
        let alignment = signature_alignment.max(environment_alignment).max(1);
        let byte_len = align_offset(environment_offset + environment_size, alignment);

        Ok((*signature, function_offset, environment_offset, byte_len))
    }

    /// Decode one boxed callable payload from one storage byte slice.
    fn decode_function_value_components_from_storage(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        bytes: &[u8],
    ) -> Result<Vec<Value>, Error> {
        let (signature_type, function_offset, environment_offset, byte_len) =
            self.function_value_payload_layout(ty)?;

        if bytes.len() != byte_len {
            return Err(Error::InvalidManagedReference);
        }

        let function_byte_len = raw_type_size(&self.executable.tree, signature_type)?;
        let function_end = function_offset
            .checked_add(function_byte_len)
            .ok_or(Error::InvalidManagedReference)?;
        let environment_end = environment_offset
            .checked_add(Value::BYTE_LEN)
            .ok_or(Error::InvalidManagedReference)?;
        let function_bytes = bytes
            .get(function_offset..function_end)
            .ok_or(Error::InvalidManagedReference)?;
        let environment_bytes = bytes
            .get(environment_offset..environment_end)
            .ok_or(Error::InvalidManagedReference)?;

        let function_value =
            decode_raw_value(&self.executable.tree, signature_type, function_bytes)?;
        let environment_value =
            Value::from_byte_slice(environment_bytes).ok_or(Error::InvalidManagedReference)?;

        Ok(vec![function_value, environment_value])
    }

    /// Wrap the isolate string interner for external calls.
    pub(crate) fn new(
        executable: &'ctx Executable,
        schema: &'ctx SchemaRegistry,
        string_interner: &'ctx mut StringInterner,
        memory: MemoryContext<'ctx>,
    ) -> Self {
        Self {
            executable,
            schema,
            string_interner,
            memory,
        }
    }

    /// Return one compiled storage layout by type.
    fn storage_layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Result<&StorageLayout, Error> {
        self.executable
            .storage_layout(ty)
            .ok_or(Error::InvalidInstruction)
    }

    /// Return one managed allocation type id.
    fn managed_storage_type_id(
        &self,
        handle: destack_heap::ManagedReference,
    ) -> Result<u32, Error> {
        self.heap_ref()
            .managed_type_id(handle)
            .ok_or(Error::InvalidManagedReference)
    }

    /// Materialize one VM value from one typed storage payload.
    fn materialize_value_from_storage(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        bytes: &[u8],
    ) -> Result<Value, Error> {
        let component_specs = {
            let layout = self.storage_layout(ty)?;

            // decode scalar storage directly
            if layout.is_scalar() {
                return decode_raw_value(&self.executable.tree, ty, bytes);
            }

            let component_count = layout.component_count().ok_or(Error::InvalidInstruction)?;
            let mut component_specs = Vec::with_capacity(component_count);

            for index in 0..component_count {
                let component = layout
                    .component(index as u32)
                    .ok_or(Error::InvalidInstruction)?;
                component_specs.push(component);
            }

            component_specs
        };

        // decode semantic composite components recursively
        let mut values = Vec::with_capacity(component_specs.len());

        for component in component_specs {
            let byte_end = component
                .offset
                .checked_add(component.byte_len)
                .ok_or(Error::InvalidManagedReference)?;
            let component_bytes = bytes
                .get(component.offset..byte_end)
                .ok_or(Error::InvalidManagedReference)?;
            let value = self.materialize_value_from_storage(component.ty, component_bytes)?;
            values.push(value);
        }

        self.materialize_storage_value(ty, values)
    }

    /// Build one cached VM value view for a managed composite.
    fn build_value_ref(&self, value: Value) -> Result<VmValueRef<'ctx>, Error> {
        let handle = value
            .as_managed_reference()
            .ok_or_else(|| Error::TypeMismatch {
                expected: "managed composite".to_string(),
                actual: format!("{:?}", value.tag()),
            })?;
        let composite_type_id = self.managed_storage_type_id(handle)?;
        let bytes = self
            .heap_ref()
            .managed_bytes_to_vec(handle)
            .ok_or(Error::InvalidManagedReference)?;
        let storage = if let Some(schema) = self.schema.schema(composite_type_id) {
            VmValueStorage::Runtime { schema }
        } else {
            let composite_type = mir::LocalNodeId::new(composite_type_id);
            if matches!(
                self.executable
                    .tree
                    .get(repr_type(&self.executable.tree, composite_type)),
                mir::Type::FunctionValue { .. }
            ) {
                VmValueStorage::Function { ty: composite_type }
            } else {
                let components = self.composite_component_specs(composite_type)?;
                VmValueStorage::Typed {
                    ty: composite_type,
                    components,
                }
            }
        };

        Ok(VmValueRef {
            context: self as *const Self as *mut Self,
            storage,
            bytes,
            _marker: PhantomData,
        })
    }

    /// Allocate one zeroed managed payload for one VM storage shape.
    fn allocate_zeroed_value_storage(
        &mut self,
        storage: &VmValueStorage,
        type_id: u32,
        layout_id: Option<destack_heap::LayoutId>,
    ) -> Result<destack_heap::ManagedReference, Error> {
        match storage {
            VmValueStorage::Runtime { schema } => {
                let byte_len = schema
                    .component_count()
                    .checked_mul(Value::BYTE_LEN)
                    .ok_or(Error::InvalidManagedReference)?;
                let reference_map = self.runtime_storage_reference_map(schema.component_count());
                self.heap()
                    .allocate_managed_zeroed(byte_len, reference_map, layout_id)
                    .map_err(Error::from)
                    .and_then(|handle| {
                        if self.heap().set_managed_type_id(handle, type_id) {
                            Ok(handle)
                        } else {
                            Err(Error::InvalidManagedReference)
                        }
                    })
            }
            VmValueStorage::Function { ty } => {
                let (_, _, environment_offset, byte_len) =
                    self.function_value_payload_layout(*ty)?;
                let reference_map = ReferenceMap::ValueOffsets {
                    offsets: vec![environment_offset as u32],
                };
                self.heap()
                    .allocate_managed_zeroed_borrowed_typed(
                        byte_len,
                        &reference_map,
                        layout_id,
                        type_id,
                    )
                    .map_err(Error::from)
            }
            VmValueStorage::Typed { ty, .. } => {
                let layout = self.storage_layout(*ty)?;
                let byte_len = layout.byte_len;
                let reference_map = layout.reference_map.clone();
                self.heap()
                    .allocate_managed_zeroed_borrowed_typed(
                        byte_len,
                        &reference_map,
                        layout_id,
                        type_id,
                    )
                    .map_err(Error::from)
            }
        }
    }

    /// Write one typed value directly into one managed payload window.
    fn write_storage_value_into_handle(
        &mut self,
        handle: destack_heap::ManagedReference,
        start: usize,
        ty: mir::LocalNodeId<mir::Type>,
        value: Value,
    ) -> Result<(), Error> {
        let repr_ty = repr_type(&self.executable.tree, ty);

        // write scalars directly into the target storage
        if self.storage_layout(ty)?.is_scalar() {
            let bytes = encode_raw_value(&self.executable.tree, ty, value)?;
            if !self.heap().set_managed_bytes(handle, start, &bytes) {
                return Err(Error::InvalidManagedReference);
            }

            return Ok(());
        }

        // write boxed callable payloads directly
        if matches!(
            self.executable.tree.get(repr_ty),
            mir::Type::FunctionValue { .. }
        ) {
            let values = self.decode_component_values(value)?;
            if values.len() != 2 {
                return Err(Error::TypeMismatch {
                    expected: "2 composite components".to_string(),
                    actual: format!("{} composite components", values.len()),
                });
            }

            let (signature_type, function_offset, environment_offset, _) =
                self.function_value_payload_layout(ty)?;
            let function_bytes =
                encode_raw_value(&self.executable.tree, signature_type, values[0])?;
            let environment_bytes = values[1].to_byte_array();
            let function_start = start
                .checked_add(function_offset)
                .ok_or(Error::InvalidManagedReference)?;
            let environment_start = start
                .checked_add(environment_offset)
                .ok_or(Error::InvalidManagedReference)?;

            if !self
                .heap()
                .set_managed_bytes(handle, function_start, &function_bytes)
            {
                return Err(Error::InvalidManagedReference);
            }

            if !self
                .heap()
                .set_managed_bytes(handle, environment_start, &environment_bytes)
            {
                return Err(Error::InvalidManagedReference);
            }

            return Ok(());
        }

        let component_specs = self.composite_component_specs(ty)?;
        let component_values = self.decode_component_values(value)?;
        if component_values.len() != component_specs.len() {
            return Err(Error::TypeMismatch {
                expected: format!("{} composite components", component_specs.len()),
                actual: format!("{} composite components", component_values.len()),
            });
        }

        // recursively write each semantic component into the final payload
        for (component, component_value) in component_specs.into_iter().zip(component_values) {
            let component_start = start
                .checked_add(component.offset)
                .ok_or(Error::InvalidManagedReference)?;
            self.write_storage_value_into_handle(
                handle,
                component_start,
                component.ty,
                component_value,
            )?;
        }

        Ok(())
    }

    /// Write semantic component values into one managed payload.
    fn write_value_storage_components(
        &mut self,
        handle: destack_heap::ManagedReference,
        storage: &VmValueStorage,
        values: &[Value],
    ) -> Result<(), Error> {
        match storage {
            VmValueStorage::Runtime { schema } => {
                if values.len() != schema.component_count() {
                    return Err(Error::TypeMismatch {
                        expected: format!("{} composite components", schema.component_count()),
                        actual: format!("{} composite components", values.len()),
                    });
                }

                // write each runtime value lane directly
                for (index, value) in values.iter().copied().enumerate() {
                    let start = index
                        .checked_mul(Value::BYTE_LEN)
                        .ok_or(Error::InvalidManagedReference)?;
                    let bytes = value.to_byte_array();
                    if !self.heap().set_managed_bytes(handle, start, &bytes) {
                        return Err(Error::InvalidManagedReference);
                    }
                }
            }
            VmValueStorage::Function { ty } => {
                if values.len() != 2 {
                    return Err(Error::TypeMismatch {
                        expected: "2 composite components".to_string(),
                        actual: format!("{} composite components", values.len()),
                    });
                }

                let (signature_type, function_offset, environment_offset, _) =
                    self.function_value_payload_layout(*ty)?;
                let function_bytes =
                    encode_raw_value(&self.executable.tree, signature_type, values[0])?;
                let environment_bytes = values[1].to_byte_array();

                if !self
                    .heap()
                    .set_managed_bytes(handle, function_offset, &function_bytes)
                {
                    return Err(Error::InvalidManagedReference);
                }

                if !self
                    .heap()
                    .set_managed_bytes(handle, environment_offset, &environment_bytes)
                {
                    return Err(Error::InvalidManagedReference);
                }
            }
            VmValueStorage::Typed { components, .. } => {
                if values.len() != components.len() {
                    return Err(Error::TypeMismatch {
                        expected: format!("{} composite components", components.len()),
                        actual: format!("{} composite components", values.len()),
                    });
                }

                // write each semantic component directly into the final payload
                for (component, value) in components.iter().copied().zip(values.iter().copied()) {
                    self.write_storage_value_into_handle(
                        handle,
                        component.offset,
                        component.ty,
                        value,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Materialize one typed composite value in heap storage.
    fn materialize_storage_value(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        values: Vec<Value>,
    ) -> Result<Value, Error> {
        let storage = if matches!(
            self.executable
                .tree
                .get(repr_type(&self.executable.tree, ty)),
            mir::Type::FunctionValue { .. }
        ) {
            VmValueStorage::Function { ty }
        } else {
            VmValueStorage::Typed {
                ty,
                components: self.composite_component_specs(ty)?,
            }
        };
        let component_count = Self::value_storage_component_count(&storage);
        let mut builder = VmValueBuilder {
            context: self as *mut Self,
            storage,
            type_id: ty.id,
            layout_id: self.executable.tree.type_layout_id(ty),
            components: vec![None; component_count],
            _marker: PhantomData,
        };

        // stage each semantic component before one final direct write
        for (index, value) in values.into_iter().enumerate() {
            builder.write_component(index as u32, value)?;
        }

        builder.finish()
    }

    /// Materialize one runtime ABI storage value in heap storage.
    fn materialize_runtime_storage_value(
        &mut self,
        type_id: u32,
        schema: StorageSchema,
        values: &[Value],
    ) -> Result<Value, Error> {
        let storage = VmValueStorage::Runtime { schema };
        let component_count = Self::value_storage_component_count(&storage);
        let mut builder = VmValueBuilder {
            context: self as *mut Self,
            storage,
            type_id,
            layout_id: None,
            components: vec![None; component_count],
            _marker: PhantomData,
        };

        // stage each runtime storage lane before one final direct write
        for (index, value) in values.iter().copied().enumerate() {
            builder.write_component(index as u32, value)?;
        }

        builder.finish()
    }

    /// Return one reference map for one runtime ABI storage payload.
    fn runtime_storage_reference_map(&self, component_count: usize) -> ReferenceMap {
        // empty aggregates do not need any reference metadata
        if component_count == 0 {
            return ReferenceMap::empty();
        }

        // otherwise track every full value lane as a reference-capable slot
        let mut offsets = Vec::with_capacity(component_count);
        for index in 0..component_count {
            offsets.push((index * Value::BYTE_LEN) as u32);
        }

        ReferenceMap::ValueOffsets { offsets }
    }

    /// Write runtime ABI component values back into one managed handle.
    fn encode_runtime_storage_components(
        &mut self,
        handle: destack_heap::ManagedReference,
        schema: StorageSchema,
        values: &[Value],
    ) -> Result<(), Error> {
        // validate the semantic component count first
        if values.len() != schema.component_count() {
            return Err(Error::TypeMismatch {
                expected: format!("{} composite components", schema.component_count()),
                actual: format!("{} composite components", values.len()),
            });
        }

        // validate the backing allocation size before writing any bytes
        let expected_byte_len = schema
            .component_count()
            .checked_mul(Value::BYTE_LEN)
            .ok_or(Error::InvalidManagedReference)?;
        let byte_len = self
            .heap_ref()
            .managed_byte_len(handle)
            .ok_or(Error::InvalidManagedReference)?;
        if byte_len != expected_byte_len {
            return Err(Error::InvalidManagedReference);
        }

        // encode the full runtime storage payload once
        let mut bytes = Vec::with_capacity(expected_byte_len);

        for value in values.iter().copied() {
            bytes.extend_from_slice(&value.to_byte_array());
        }

        // commit the full payload in one heap write
        if !self.heap().set_managed_bytes(handle, 0, &bytes) {
            return Err(Error::InvalidManagedReference);
        }

        Ok(())
    }

    /// Materialize one typed storage value for the provided MIR type.
    pub(crate) fn materialize_storage_value_for_type(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        values: Vec<Value>,
    ) -> Result<Value, Error> {
        self.materialize_storage_value(ty, values)
    }

    /// Build one cached VM value view for one managed composite value.
    pub fn value_ref(&self, value: Value) -> Result<VmValueRef<'ctx>, Error> {
        self.build_value_ref(value)
    }

    /// Begin one named runtime storage value builder.
    pub fn begin_named_storage_value_builder(
        &mut self,
        type_name: &str,
    ) -> Result<VmValueBuilder<'ctx>, Error> {
        let type_id = self
            .schema
            .type_id(type_name)
            .ok_or_else(|| Error::TypeMismatch {
                expected: format!("registered named storage type '{type_name}'"),
                actual: "missing isolate schema".to_string(),
            })?;
        let schema = self
            .schema
            .schema(type_id)
            .ok_or(Error::InvalidInstruction)?;
        let storage = VmValueStorage::Runtime { schema };
        let component_count = Self::value_storage_component_count(&storage);

        Ok(VmValueBuilder {
            context: self as *mut Self,
            storage,
            type_id,
            layout_id: None,
            components: vec![None; component_count],
            _marker: PhantomData,
        })
    }

    /// Materialize one builtin Slice value from raw pointer and length.
    pub fn materialize_builtin_slice_value(
        &mut self,
        data: RawPointer,
        len: u32,
    ) -> Result<Value, Error> {
        let (type_id, schema) = self.builtin_collection_type("Slice", 2)?;
        let values = [Value::raw_pointer(data), Value::uint(len as u64, 32)];

        self.materialize_runtime_storage_value(type_id, schema, &values)
    }

    /// Materialize one builtin Array value from length, capacity, and raw pointer.
    pub fn materialize_builtin_array_value(
        &mut self,
        len: u32,
        capacity: u32,
        data: RawPointer,
    ) -> Result<Value, Error> {
        let (type_id, schema) = self.builtin_collection_type("Array", 3)?;
        let values = [
            Value::uint(len as u64, 32),
            Value::uint(capacity as u64, 32),
            Value::raw_pointer(data),
        ];

        self.materialize_runtime_storage_value(type_id, schema, &values)
    }

    /// Materialize one registered named storage value from the isolate schema.
    pub fn materialize_named_storage_value(
        &mut self,
        type_name: &str,
        values: Vec<Value>,
    ) -> Result<Value, Error> {
        // resolve the registered runtime ABI schema directly
        let type_id = self
            .schema
            .type_id(type_name)
            .ok_or_else(|| Error::TypeMismatch {
                expected: format!("registered named storage type '{type_name}'"),
                actual: "missing isolate schema".to_string(),
            })?;
        let schema = self
            .schema
            .schema(type_id)
            .ok_or(Error::InvalidInstruction)?;

        self.materialize_runtime_storage_value(type_id, schema, &values)
    }

    /// Materialize one registered named storage value from one fixed component array.
    pub fn materialize_named_storage_value_array<const N: usize>(
        &mut self,
        type_name: &str,
        values: [Value; N],
    ) -> Result<Value, Error> {
        // resolve the registered runtime ABI schema directly
        let type_id = self
            .schema
            .type_id(type_name)
            .ok_or_else(|| Error::TypeMismatch {
                expected: format!("registered named storage type '{type_name}'"),
                actual: "missing isolate schema".to_string(),
            })?;
        let schema = self
            .schema
            .schema(type_id)
            .ok_or(Error::InvalidInstruction)?;

        self.materialize_runtime_storage_value(type_id, schema, &values)
    }

    /// Return semantic component layouts for one type.
    fn composite_component_specs(
        &self,
        aggregate_type: mir::LocalNodeId<mir::Type>,
    ) -> Result<Vec<crate::executable::StorageComponentLayout>, Error> {
        let layout = self.storage_layout(aggregate_type)?;
        let component_count = layout
            .component_count()
            .ok_or(Error::InvalidManagedReference)?;
        let mut component_specs = Vec::with_capacity(component_count);

        for index in 0..component_count {
            let component = layout
                .component(index as u32)
                .ok_or(Error::InvalidInstruction)?;
            component_specs.push(component);
        }

        Ok(component_specs)
    }

    /// Borrow the local heap.
    fn heap(&mut self) -> &mut destack_heap::Heap {
        self.memory.heap()
    }

    /// Borrow the local heap immutably.
    fn heap_ref(&self) -> &destack_heap::Heap {
        self.memory.heap_ref()
    }

    /// Borrow the world shared memory immutably.
    fn shared_ref(&self) -> &destack_heap::SharedSpace {
        self.memory.shared_ref()
    }

    /// Intern a UTF-8 string and return the managed string value.
    pub fn intern_string(&mut self, value: &str) -> Result<Value, Error> {
        let heap = self.memory.heap();
        self.string_interner.try_intern_string_literal(heap, value)
    }

    /// Intern a UTF-8 string and return the managed string handle.
    pub fn string_handle(&mut self, value: &str) -> Result<StringHandle, Error> {
        let value = self.intern_string(value)?;

        Ok(StringHandle::new(value))
    }

    /// Validate one VM value as a string handle.
    pub fn string_handle_from_value(&self, value: Value) -> Result<StringHandle, Error> {
        self.string_interner.string_handle(self.heap_ref(), value)
    }

    /// Read a UTF-8 string value from the heap.
    pub fn string_value(&self, value: Value) -> Result<String, Error> {
        self.string_interner.string_value(self.heap_ref(), value)
    }

    /// Read a UTF-8 string view from the heap.
    pub fn string_value_ref(&self, value: Value) -> Result<StringRef<'_>, Error> {
        self.string_interner
            .string_value_ref(self.heap_ref(), value)
    }

    /// Read a UTF-8 string view from the heap using a string handle.
    pub fn string_ref(&self, value: StringHandle) -> Result<StringRef<'_>, Error> {
        self.string_interner
            .string_value_ref(self.heap_ref(), value.value())
    }

    /// Allocate a raw heap byte buffer and return its pointer.
    pub fn allocate_raw_bytes(&mut self, bytes: &[u8]) -> Result<RawPointer, Error> {
        let heap = self.memory.heap();
        heap.allocate_raw_bytes(bytes).map_err(Error::from)
    }

    /// Allocate one raw packed-value buffer and return its pointer.
    pub fn allocate_raw_values(&mut self, values: Vec<Value>) -> Result<RawPointer, Error> {
        self.heap().allocate_raw_values(values).map_err(Error::from)
    }

    /// Allocate a shared heap byte region and return its pointer.
    pub fn allocate_shared_bytes(&mut self, bytes: &[u8]) -> Result<SharedPointer, Error> {
        self.memory
            .allocate_shared_bytes(bytes)
            .map_err(Error::from)
    }

    /// Read semantic component values from one typed composite value.
    pub fn decode_component_values(&mut self, value: Value) -> Result<Vec<Value>, Error> {
        self.build_value_ref(value)?.component_values()
    }

    /// Return the semantic component count for one typed composite value.
    pub fn storage_component_count(&self, value: Value) -> Result<usize, Error> {
        Ok(self.build_value_ref(value)?.component_count())
    }

    /// Read one semantic component value from one typed composite value.
    pub fn decode_storage_component_value(
        &mut self,
        value: Value,
        index: u32,
    ) -> Result<Value, Error> {
        self.build_value_ref(value)?.component_value(index)
    }

    /// Write semantic component values back into one typed heap composite.
    pub fn encode_component_values(&mut self, value: Value, values: &[Value]) -> Result<(), Error> {
        let handle = value
            .as_managed_reference()
            .ok_or_else(|| Error::TypeMismatch {
                expected: "managed composite".to_string(),
                actual: format!("{:?}", value.tag()),
            })?;

        let composite_type_id = self.managed_storage_type_id(handle)?;
        if let Some(schema) = self.schema.schema(composite_type_id) {
            return self.encode_runtime_storage_components(handle, schema, values);
        }

        let composite_type = mir::LocalNodeId::new(composite_type_id);
        let storage = if matches!(
            self.executable
                .tree
                .get(repr_type(&self.executable.tree, composite_type)),
            mir::Type::FunctionValue { .. }
        ) {
            VmValueStorage::Function { ty: composite_type }
        } else {
            VmValueStorage::Typed {
                ty: composite_type,
                components: self.composite_component_specs(composite_type)?,
            }
        };

        self.write_value_storage_components(handle, &storage, values)
    }

    /// Read raw bytes from a pointer to a bytes cell.
    pub fn raw_bytes_ref(&self, pointer: RawPointer) -> Result<Cow<'_, [u8]>, Error> {
        self.heap_ref()
            .raw_bytes(pointer)
            .ok_or(Error::InvalidManagedReference)
    }

    /// Read raw bytes from a pointer to a bytes cell.
    pub fn raw_bytes(&self, pointer: RawPointer) -> Result<Vec<u8>, Error> {
        Ok(self.raw_bytes_ref(pointer)?.into_owned())
    }

    /// Read the raw byte length for one pointer.
    pub fn raw_byte_len(&self, pointer: RawPointer) -> Result<usize, Error> {
        self.heap_ref()
            .raw_byte_len(pointer)
            .ok_or(Error::InvalidManagedReference)
    }

    /// Read raw packed values from one pointer.
    pub fn raw_values(&self, pointer: RawPointer) -> Result<Vec<Value>, Error> {
        self.heap_ref()
            .raw_values(pointer)
            .ok_or(Error::InvalidManagedReference)
    }

    /// Read shared bytes from a pointer to one shared region.
    pub fn shared_bytes(&self, pointer: SharedPointer) -> Result<Vec<u8>, Error> {
        self.shared_ref()
            .bytes_to_vec(pointer)
            .ok_or(Error::InvalidManagedReference)
    }

    /// Write raw bytes into a pointer to a bytes cell.
    pub fn write_raw_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> Result<(), Error> {
        if self
            .heap()
            .replace_raw_bytes(pointer, bytes)
            .map_err(Error::from)?
        {
            return Ok(());
        }

        Err(Error::InvalidManagedReference)
    }

    /// Write raw packed values into one pointer.
    pub fn write_raw_values(&mut self, pointer: RawPointer, values: &[Value]) -> Result<(), Error> {
        if self
            .heap()
            .replace_raw_values(pointer, values)
            .map_err(Error::from)?
        {
            return Ok(());
        }

        Err(Error::InvalidManagedReference)
    }

    /// Write shared bytes into a pointer to one shared region.
    pub fn write_shared_bytes(
        &mut self,
        pointer: SharedPointer,
        bytes: &[u8],
    ) -> Result<(), Error> {
        if self
            .memory
            .replace_shared_bytes(pointer, bytes)
            .map_err(Error::from)?
        {
            return Ok(());
        }

        Err(Error::InvalidManagedReference)
    }
}

impl fmt::Debug for ExternalCallContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExternalCallContext")
            .field("string_interner", &"<isolate>")
            .finish()
    }
}

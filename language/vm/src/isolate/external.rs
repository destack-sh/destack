use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use crate::diagnostic::Error;
use crate::executable::{ComponentLayout, Executable, Layout, repr_type};
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
pub type ExternalFn = Arc<dyn ExternalHandler>;

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

/// Read-only external VM call capability.
#[derive(Debug, Clone, Copy)]
pub struct ExternalReadContext<'call, 'ctx> {
    /// The owning external call context.
    context: &'call ExternalCallContext<'ctx>,
}

/// Mutable external VM call capability.
#[derive(Debug)]
pub struct ExternalWriteContext<'call, 'ctx> {
    /// The owning external call context.
    context: &'call mut ExternalCallContext<'ctx>,
}

/// One VM aggregate storage shape.
#[derive(Clone, Debug)]
enum VmValueStorage {
    /// One isolate registered runtime storage payload.
    Runtime {
        /// The registered runtime type id.
        type_id: u32,
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
        /// The MIR type.
        ty: mir::LocalNodeId<mir::Type>,
        /// The semantic component layouts.
        components: Vec<ComponentLayout>,
    },
}

impl VmValueStorage {
    /// Return the semantic component count for this storage shape.
    fn component_count(&self) -> usize {
        match self {
            Self::Runtime { schema, .. } => schema.component_count(),
            Self::Function { .. } => 2,
            Self::Typed { components, .. } => components.len(),
        }
    }
}

/// One cached borrowed VM value view.
#[derive(Debug)]
pub struct VmValueRef<'call, 'ctx> {
    /// The active external call context.
    context: *mut ExternalCallContext<'ctx>,
    /// The resolved storage shape.
    storage: VmValueStorage,
    /// The shared payload bytes.
    bytes: VmValueBytes<'call>,
    /// The first byte of this value inside the shared payload.
    start: usize,
    /// The byte length of this value inside the shared payload.
    byte_len: usize,
    /// The lifetime marker for the call context.
    _marker: PhantomData<&'call ExternalCallContext<'ctx>>,
}

/// One VM value payload borrow or shared owned buffer.
#[derive(Debug, Clone)]
enum VmValueBytes<'call> {
    /// Borrowed heap-managed bytes.
    Borrowed(&'call [u8]),
    /// Shared owned bytes when the heap cannot lend one stable contiguous slice.
    Owned(Rc<[u8]>),
}

impl VmValueBytes<'_> {
    /// Return the full backing bytes for this cached payload.
    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Borrowed(bytes) => bytes,
            Self::Owned(bytes) => bytes,
        }
    }
}

impl<'call, 'ctx> VmValueRef<'call, 'ctx> {
    /// Return the payload bytes for this value.
    fn bytes(&self) -> &[u8] {
        &self.bytes.as_slice()[self.start..self.start + self.byte_len]
    }

    /// Return the total payload byte length.
    fn total_len(&self) -> usize {
        self.bytes.as_slice().len()
    }

    /// Return the active external call context.
    fn context_ref(&self) -> &'ctx ExternalCallContext<'ctx> {
        // the owning call context outlives every borrowed VM value view
        unsafe { &*self.context }
    }

    /// Return the active external call context mutably.
    fn context_mut(&self) -> &'ctx mut ExternalCallContext<'ctx> {
        // mutable decode paths must not outlive the owning call context
        unsafe { &mut *self.context }
    }

    /// Return one checked byte window inside this cached payload.
    fn byte_window(&self, offset: usize, byte_len: usize) -> Result<&[u8], Error> {
        let end = offset
            .checked_add(byte_len)
            .ok_or(Error::InvalidManagedReference)?;

        self.bytes()
            .get(offset..end)
            .ok_or(Error::InvalidManagedReference)
    }

    /// Return one checked runtime lane value.
    fn runtime_lane_value(&self, index: usize) -> Result<Value, Error> {
        let start = index
            .checked_mul(Value::BYTE_LEN)
            .ok_or(Error::InvalidManagedReference)?;
        let bytes = self.byte_window(start, Value::BYTE_LEN)?;

        Value::from_byte_slice(bytes).ok_or(Error::InvalidManagedReference)
    }

    /// Return the semantic component count for this value.
    pub fn component_count(&self) -> usize {
        self.storage.component_count()
    }

    /// Return one nested VM value view for one semantic component.
    pub fn component_ref(&self, index: u32) -> Result<Self, Error> {
        let context = self.context_ref();

        match &self.storage {
            VmValueStorage::Runtime { .. } | VmValueStorage::Function { .. } => {
                Err(Error::InvalidManagedReference)
            }
            VmValueStorage::Typed { components, .. } => {
                let component = components
                    .get(index as usize)
                    .copied()
                    .ok_or(Error::InvalidManagedReference)?;
                let layout = context.layout(component.ty)?;

                // scalar components do not have nested storage views
                if layout.is_scalar() {
                    return Err(Error::InvalidManagedReference);
                }

                let storage = context.storage_for_type(component.ty)?;
                let start = self
                    .start
                    .checked_add(component.offset)
                    .ok_or(Error::InvalidManagedReference)?;
                let byte_len = component.byte_len;

                // reject out of bounds nested component windows before building the child view
                if start
                    .checked_add(byte_len)
                    .ok_or(Error::InvalidManagedReference)?
                    > self.total_len()
                {
                    return Err(Error::InvalidManagedReference);
                }

                Ok(Self {
                    context: self.context,
                    storage,
                    bytes: self.bytes.clone(),
                    start,
                    byte_len,
                    _marker: PhantomData,
                })
            }
        }
    }

    /// Materialize this value view as one VM value.
    pub fn materialize_value(&self) -> Result<Value, Error> {
        let context = self.context_mut();

        match &self.storage {
            VmValueStorage::Runtime { type_id, schema } => {
                let component_count = schema.component_count();
                let mut values = Vec::with_capacity(component_count);

                // decode each runtime lane directly from the cached payload
                for index in 0..component_count {
                    values.push(self.runtime_lane_value(index)?);
                }

                context.materialize_runtime_storage_value(*type_id, *schema, &values)
            }
            VmValueStorage::Function { ty } | VmValueStorage::Typed { ty, .. } => {
                context.materialize_value_from_storage(*ty, self.bytes())
            }
        }
    }

    /// Decode one semantic component value from this cached payload.
    pub fn component_value(&self, index: u32) -> Result<Value, Error> {
        let context = self.context_mut();

        match &self.storage {
            VmValueStorage::Runtime { schema, .. } => {
                let component_count = schema.component_count();
                if index as usize >= component_count {
                    return Err(Error::InvalidManagedReference);
                }

                self.runtime_lane_value(index as usize)
            }
            VmValueStorage::Function { ty } => {
                let values =
                    context.decode_function_value_components_from_storage(*ty, self.bytes())?;
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
                let component_bytes = self.byte_window(component.offset, component.byte_len)?;

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
    /// Return the read-only external call capability.
    pub fn read(&self) -> ExternalReadContext<'_, 'ctx> {
        ExternalReadContext { context: self }
    }

    /// Return the mutable external call capability.
    pub fn write(&mut self) -> ExternalWriteContext<'_, 'ctx> {
        ExternalWriteContext { context: self }
    }

    /// Resolve one builtin collection type by short display name.
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
            expected: format!("builtin {short_name} type"),
            actual: "missing isolate schema".to_string(),
        })
    }

    /// Resolve one registered runtime schema by type name.
    fn named_storage_schema(&self, type_name: &str) -> Result<(u32, StorageSchema), Error> {
        let type_id = self
            .schema
            .type_id(type_name)
            .ok_or_else(|| Error::TypeMismatch {
                expected: format!("registered named type '{type_name}'"),
                actual: "missing isolate schema".to_string(),
            })?;
        let schema = self
            .schema
            .schema(type_id)
            .ok_or(Error::InvalidInstruction)?;

        Ok((type_id, schema))
    }

    /// Build one deferred VM value builder for the given storage shape.
    fn value_builder(
        &mut self,
        storage: VmValueStorage,
        type_id: u32,
        layout_id: Option<destack_heap::LayoutId>,
    ) -> VmValueBuilder<'ctx> {
        let component_count = storage.component_count();

        VmValueBuilder {
            context: self as *mut Self,
            storage,
            type_id,
            layout_id,
            components: vec![None; component_count],
            _marker: PhantomData,
        }
    }

    /// Return the callable box payload layout for one function value type.
    fn function_value_payload_layout(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> Result<(mir::LocalNodeId<mir::Type>, usize, usize, usize), Error> {
        let mir::Type::Closure { signature } = self.executable.tree.get(ty) else {
            return Err(Error::TypeMismatch {
                expected: "function value type".to_string(),
                actual: format!("{ty:?}"),
            });
        };
        let signature = (*signature)
            .ty()
            .ok_or_else(|| Error::ConcreteMirRequired {
                context: "closure signature".to_string(),
            })?;

        let signature_size = raw_type_size(&self.executable.tree, signature)?;
        let signature_alignment = raw_type_alignment(&self.executable.tree, signature)?;
        let environment_size = Value::BYTE_LEN;
        let environment_alignment = std::mem::align_of::<Value>();

        let function_offset = 0usize;
        let environment_offset = align_offset(signature_size, environment_alignment);
        let alignment = signature_alignment.max(environment_alignment).max(1);
        let byte_len = align_offset(environment_offset + environment_size, alignment);

        Ok((signature, function_offset, environment_offset, byte_len))
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

    /// Return one compiled layout by type.
    fn layout(&self, ty: mir::LocalNodeId<mir::Type>) -> Result<&Layout, Error> {
        self.executable.layout(ty).ok_or(Error::InvalidInstruction)
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

    /// Resolve one VM storage shape from one MIR type.
    fn storage_for_type(&self, ty: mir::LocalNodeId<mir::Type>) -> Result<VmValueStorage, Error> {
        if let Some(schema) = self.schema.schema(ty.id) {
            return Ok(VmValueStorage::Runtime {
                type_id: ty.id,
                schema,
            });
        }

        if matches!(
            self.executable
                .tree
                .get(repr_type(&self.executable.tree, ty)),
            mir::Type::Closure { .. }
        ) {
            return Ok(VmValueStorage::Function { ty });
        }

        Ok(VmValueStorage::Typed {
            ty,
            components: self.composite_component_specs(ty)?,
        })
    }

    /// Materialize one VM value from one typed storage payload.
    fn materialize_value_from_storage(
        &mut self,
        ty: mir::LocalNodeId<mir::Type>,
        bytes: &[u8],
    ) -> Result<Value, Error> {
        let component_specs = {
            let layout = self.layout(ty)?;

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
    fn build_value_ref_snapshot<'call>(
        &'call self,
        value: Value,
    ) -> Result<VmValueRef<'call, 'ctx>, Error> {
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
        let composite_type = mir::LocalNodeId::new(composite_type_id);
        let storage = self.storage_for_type(composite_type)?;
        let bytes = VmValueBytes::Owned(Rc::<[u8]>::from(bytes));
        let byte_len = bytes.as_slice().len();

        Ok(VmValueRef {
            context: self as *const Self as *mut Self,
            storage,
            bytes,
            start: 0,
            byte_len,
            _marker: PhantomData,
        })
    }

    /// Build one cached VM value view that borrows heap bytes when possible.
    fn build_read_value_ref<'call>(
        &'call self,
        value: Value,
    ) -> Result<VmValueRef<'call, 'ctx>, Error> {
        let handle = value
            .as_managed_reference()
            .ok_or_else(|| Error::TypeMismatch {
                expected: "managed composite".to_string(),
                actual: format!("{:?}", value.tag()),
            })?;
        let composite_type_id = self.managed_storage_type_id(handle)?;
        let bytes = self
            .heap_ref()
            .managed_bytes(handle)
            .ok_or(Error::InvalidManagedReference)?;
        let composite_type = mir::LocalNodeId::new(composite_type_id);
        let storage = self.storage_for_type(composite_type)?;
        let bytes = match bytes {
            Cow::Borrowed(bytes) => VmValueBytes::Borrowed(bytes),
            Cow::Owned(bytes) => VmValueBytes::Owned(Rc::<[u8]>::from(bytes)),
        };
        let byte_len = bytes.as_slice().len();

        Ok(VmValueRef {
            context: self as *const Self as *mut Self,
            storage,
            bytes,
            start: 0,
            byte_len,
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
            VmValueStorage::Runtime { schema, .. } => {
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
                    offsets: vec![environment_offset as u32].into_boxed_slice(),
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
                let layout = self.layout(*ty)?;
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
        if self.layout(ty)?.is_scalar() {
            let bytes = encode_raw_value(&self.executable.tree, ty, value)?;
            if !self.heap().set_managed_bytes(handle, start, &bytes) {
                return Err(Error::InvalidManagedReference);
            }

            return Ok(());
        }

        // write boxed callable payloads directly
        if matches!(self.executable.tree.get(repr_ty), mir::Type::Closure { .. }) {
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
            VmValueStorage::Runtime { schema, .. } => {
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
            mir::Type::Closure { .. }
        ) {
            VmValueStorage::Function { ty }
        } else {
            VmValueStorage::Typed {
                ty,
                components: self.composite_component_specs(ty)?,
            }
        };
        let mut builder =
            self.value_builder(storage, ty.id, self.executable.tree.type_layout_id(ty));

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
        let storage = VmValueStorage::Runtime { type_id, schema };
        let mut builder = self.value_builder(storage, type_id, None);

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

        ReferenceMap::ValueOffsets {
            offsets: offsets.into_boxed_slice(),
        }
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

        // write each runtime storage lane directly
        for (index, value) in values.iter().copied().enumerate() {
            let start = index
                .checked_mul(Value::BYTE_LEN)
                .ok_or(Error::InvalidManagedReference)?;
            let bytes = value.to_byte_array();
            if !self.heap().set_managed_bytes(handle, start, &bytes) {
                return Err(Error::InvalidManagedReference);
            }
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

    /// Begin one named runtime storage value builder.
    pub fn begin_named_storage_value_builder(
        &mut self,
        type_name: &str,
    ) -> Result<VmValueBuilder<'ctx>, Error> {
        let (type_id, schema) = self.named_storage_schema(type_name)?;
        let storage = VmValueStorage::Runtime { type_id, schema };

        Ok(self.value_builder(storage, type_id, None))
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
        let (type_id, schema) = self.named_storage_schema(type_name)?;

        self.materialize_runtime_storage_value(type_id, schema, &values)
    }

    /// Materialize one registered named storage value from one fixed component array.
    pub fn materialize_named_storage_value_array<const N: usize>(
        &mut self,
        type_name: &str,
        values: [Value; N],
    ) -> Result<Value, Error> {
        // resolve the registered runtime ABI schema directly
        let (type_id, schema) = self.named_storage_schema(type_name)?;

        self.materialize_runtime_storage_value(type_id, schema, &values)
    }

    /// Return semantic component layouts for one type.
    fn composite_component_specs(
        &self,
        aggregate_type: mir::LocalNodeId<mir::Type>,
    ) -> Result<Vec<crate::executable::ComponentLayout>, Error> {
        let layout = self.layout(aggregate_type)?;
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

    /// Allocate one zeroed raw heap byte buffer and return its pointer.
    pub fn allocate_zeroed_raw_bytes(&mut self, byte_len: usize) -> Result<RawPointer, Error> {
        let heap = self.memory.heap();
        heap.allocate_raw_zeroed(byte_len).map_err(Error::from)
    }

    /// Allocate one raw packed-value buffer and return its pointer.
    pub fn allocate_raw_values(&mut self, values: Vec<Value>) -> Result<RawPointer, Error> {
        self.heap().allocate_raw_values(values).map_err(Error::from)
    }

    /// Allocate one zeroed raw packed-value buffer and return its pointer.
    pub fn allocate_raw_value_slots(&mut self, slot_count: usize) -> Result<RawPointer, Error> {
        self.heap()
            .allocate_raw_slots(slot_count)
            .map_err(Error::from)
    }

    /// Allocate a shared heap byte region and return its pointer.
    pub fn allocate_shared_bytes(&mut self, bytes: &[u8]) -> Result<SharedPointer, Error> {
        self.memory
            .allocate_shared_bytes(bytes)
            .map_err(Error::from)
    }

    /// Read semantic component values from one typed composite value.
    pub fn decode_component_values(&mut self, value: Value) -> Result<Vec<Value>, Error> {
        self.build_value_ref_snapshot(value)?.component_values()
    }

    /// Return the semantic component count for one typed composite value.
    pub fn storage_component_count(&self, value: Value) -> Result<usize, Error> {
        Ok(self.build_value_ref_snapshot(value)?.component_count())
    }

    /// Read one semantic component value from one typed composite value.
    pub fn decode_storage_component_value(
        &mut self,
        value: Value,
        index: u32,
    ) -> Result<Value, Error> {
        self.build_value_ref_snapshot(value)?.component_value(index)
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
            mir::Type::Closure { .. }
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

    /// Read one raw packed value by slot index.
    pub fn raw_value_at(&self, pointer: RawPointer, index: usize) -> Result<Value, Error> {
        let start = index
            .checked_mul(Value::BYTE_LEN)
            .ok_or(Error::InvalidManagedReference)?;
        let mut bytes = [0u8; Value::BYTE_LEN];

        // read the packed value lane without materializing the whole raw payload
        for (offset, byte) in bytes.iter_mut().enumerate() {
            *byte = self
                .heap_ref()
                .raw_byte_at(pointer, start + offset)
                .ok_or(Error::InvalidManagedReference)?;
        }

        Value::from_byte_slice(&bytes).ok_or(Error::InvalidManagedReference)
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

    /// Write one raw packed value into one pointer slot.
    pub fn write_raw_value(
        &mut self,
        pointer: RawPointer,
        index: usize,
        value: Value,
    ) -> Result<(), Error> {
        if self.heap().set_raw_value(pointer, index, value) {
            return Ok(());
        }

        Err(Error::InvalidManagedReference)
    }

    /// Write one raw byte into one pointer slot.
    pub fn write_raw_byte(
        &mut self,
        pointer: RawPointer,
        index: usize,
        byte: u8,
    ) -> Result<(), Error> {
        if self.heap().set_raw_byte(pointer, index, byte) {
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

impl<'call, 'ctx> ExternalReadContext<'call, 'ctx> {
    /// Return the semantic component count for one composite value.
    pub fn storage_component_count(&self, value: Value) -> Result<usize, Error> {
        self.context.storage_component_count(value)
    }

    /// Return one cached VM value view.
    pub fn value_ref(&self, value: Value) -> Result<VmValueRef<'call, 'ctx>, Error> {
        self.context.build_read_value_ref(value)
    }

    /// Return one VM string handle from one value.
    pub fn string_handle_from_value(&self, value: Value) -> Result<StringHandle, Error> {
        self.context.string_handle_from_value(value)
    }

    /// Return one borrowed VM string value.
    pub fn string_value_ref(&self, value: Value) -> Result<StringRef<'call>, Error> {
        self.context.string_value_ref(value)
    }

    /// Return one borrowed VM string by handle.
    pub fn string_ref(&self, value: StringHandle) -> Result<StringRef<'call>, Error> {
        self.context.string_ref(value)
    }

    /// Return one raw byte payload borrow or copy.
    pub fn raw_bytes_ref(&self, pointer: RawPointer) -> Result<Cow<'call, [u8]>, Error> {
        self.context.raw_bytes_ref(pointer)
    }

    /// Return one owned raw byte payload.
    pub fn raw_bytes(&self, pointer: RawPointer) -> Result<Vec<u8>, Error> {
        self.context.raw_bytes(pointer)
    }

    /// Return the raw byte length for one pointer.
    pub fn raw_byte_len(&self, pointer: RawPointer) -> Result<usize, Error> {
        self.context.raw_byte_len(pointer)
    }

    /// Return one raw packed value by slot index.
    pub fn raw_value_at(&self, pointer: RawPointer, index: usize) -> Result<Value, Error> {
        self.context.raw_value_at(pointer, index)
    }

    /// Return one raw packed-value payload copy.
    pub fn raw_values(&self, pointer: RawPointer) -> Result<Vec<Value>, Error> {
        self.context.raw_values(pointer)
    }

    /// Return one shared byte payload copy.
    pub fn shared_bytes(&self, pointer: SharedPointer) -> Result<Vec<u8>, Error> {
        self.context.shared_bytes(pointer)
    }
}

impl<'call, 'ctx> ExternalWriteContext<'call, 'ctx> {
    /// Return the raw byte length for one pointer.
    pub fn raw_byte_len(&self, pointer: RawPointer) -> Result<usize, Error> {
        self.context.raw_byte_len(pointer)
    }

    /// Return one managed value builder by runtime storage name.
    pub fn begin_named_storage_value_builder(
        &mut self,
        storage_type: &str,
    ) -> Result<VmValueBuilder<'ctx>, Error> {
        let context = self.context as *mut ExternalCallContext<'ctx>;

        unsafe { (&mut *context).begin_named_storage_value_builder(storage_type) }
    }

    /// Materialize one builtin slice value.
    pub fn materialize_builtin_slice_value(
        &mut self,
        data: RawPointer,
        len: u32,
    ) -> Result<Value, Error> {
        self.context.materialize_builtin_slice_value(data, len)
    }

    /// Materialize one builtin array value.
    pub fn materialize_builtin_array_value(
        &mut self,
        len: u32,
        capacity: u32,
        data: RawPointer,
    ) -> Result<Value, Error> {
        self.context
            .materialize_builtin_array_value(len, capacity, data)
    }

    /// Materialize one named runtime storage value.
    pub fn materialize_named_storage_value(
        &mut self,
        storage_type: &str,
        values: Vec<Value>,
    ) -> Result<Value, Error> {
        self.context
            .materialize_named_storage_value(storage_type, values)
    }

    /// Materialize one named runtime storage value from one fixed-size array.
    pub fn materialize_named_storage_value_array<const N: usize>(
        &mut self,
        storage_type: &str,
        values: [Value; N],
    ) -> Result<Value, Error> {
        self.context
            .materialize_named_storage_value_array(storage_type, values)
    }

    /// Intern one string into VM heap storage.
    pub fn intern_string(&mut self, value: &str) -> Result<Value, Error> {
        self.context.intern_string(value)
    }

    /// Return one VM string handle for one string.
    pub fn string_handle(&mut self, value: &str) -> Result<StringHandle, Error> {
        self.context.string_handle(value)
    }

    /// Allocate one raw byte buffer.
    pub fn allocate_raw_bytes(&mut self, bytes: &[u8]) -> Result<RawPointer, Error> {
        self.context.allocate_raw_bytes(bytes)
    }

    /// Allocate one zeroed raw byte buffer.
    pub fn allocate_zeroed_raw_bytes(&mut self, byte_len: usize) -> Result<RawPointer, Error> {
        self.context.allocate_zeroed_raw_bytes(byte_len)
    }

    /// Allocate one raw packed-value buffer.
    pub fn allocate_raw_values(&mut self, values: Vec<Value>) -> Result<RawPointer, Error> {
        self.context.allocate_raw_values(values)
    }

    /// Allocate one zeroed raw packed-value buffer.
    pub fn allocate_raw_value_slots(&mut self, slot_count: usize) -> Result<RawPointer, Error> {
        self.context.allocate_raw_value_slots(slot_count)
    }

    /// Allocate one shared byte region.
    pub fn allocate_shared_bytes(&mut self, bytes: &[u8]) -> Result<SharedPointer, Error> {
        self.context.allocate_shared_bytes(bytes)
    }

    /// Write one raw byte payload.
    pub fn write_raw_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> Result<(), Error> {
        self.context.write_raw_bytes(pointer, bytes)
    }

    /// Write one raw packed-value payload.
    pub fn write_raw_values(&mut self, pointer: RawPointer, values: &[Value]) -> Result<(), Error> {
        self.context.write_raw_values(pointer, values)
    }

    /// Write one raw packed value.
    pub fn write_raw_value(
        &mut self,
        pointer: RawPointer,
        index: usize,
        value: Value,
    ) -> Result<(), Error> {
        self.context.write_raw_value(pointer, index, value)
    }

    /// Write one raw byte into one pointer slot.
    pub fn write_raw_byte(
        &mut self,
        pointer: RawPointer,
        index: usize,
        byte: u8,
    ) -> Result<(), Error> {
        self.context.write_raw_byte(pointer, index, byte)
    }

    /// Write one shared byte payload.
    pub fn write_shared_bytes(
        &mut self,
        pointer: SharedPointer,
        bytes: &[u8],
    ) -> Result<(), Error> {
        self.context.write_shared_bytes(pointer, bytes)
    }
}

impl fmt::Debug for ExternalCallContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExternalCallContext")
            .field("string_interner", &"<isolate>")
            .finish()
    }
}

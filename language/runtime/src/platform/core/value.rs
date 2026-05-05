use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::abi::{NativeSlice, NativeStringRef, NativeStringSlice};
use crate::platform::{NativeArray, VmAggregateCodec, VmArray, VmCollectionElement, VmSlice};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Mechanical native ABI codec for one materialized Rust value.
pub(crate) trait NativeAbiCodec: Sized {
    /// The materialized Rust value corresponding to this native binding type.
    type Value;

    /// Decode this native binding value into one materialized Rust value.
    unsafe fn into_value(self) -> RuntimeResult<Self::Value>;

    /// Encode one materialized Rust value into this native binding type.
    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self;
}

/// Mechanical VM ABI codec for one materialized Rust value.
pub(crate) trait VmAbiCodec: Sized {
    /// The materialized Rust value corresponding to this VM binding type.
    type Value;

    /// Decode this VM binding value into one materialized Rust value.
    fn into_value(self, context: &vm::BindingRead<'_, '_>) -> RuntimeResult<Self::Value>;

    /// Encode one materialized Rust value into this VM binding type.
    fn from_value(
        context: &mut vm::BindingWrite<'_, '_>,
        value: Self::Value,
    ) -> RuntimeResult<Self>;
}

macro_rules! identity_value_codec {
    ($($type:ty),+ $(,)?) => {
        $(
            impl NativeAbiCodec for $type {
                type Value = Self;

                unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
                    Ok(self)
                }

                fn from_value(_binding: &BindingCallContext, value: Self::Value) -> Self {
                    value
                }
            }

            impl VmAbiCodec for $type {
                type Value = Self;

                fn into_value(
                    self,
                    _context: &vm::BindingRead<'_, '_>,
                ) -> RuntimeResult<Self::Value> {
                    Ok(self)
                }

                fn from_value(
                    _context: &mut vm::BindingWrite<'_, '_>,
                    value: Self::Value,
                ) -> RuntimeResult<Self> {
                    Ok(value)
                }
            }
        )+
    };
}

identity_value_codec!(bool, i8, i16, i32, i64, u8, u16, u32, u64, f32, f64,);

impl<T: NativeAbiCodec> NativeAbiCodec for Option<T> {
    type Value = Option<T::Value>;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        self.map(|value| unsafe { value.into_value() }).transpose()
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        value.map(|value| T::from_value(binding, value))
    }
}

impl VmAbiCodec for vm::StringHandle {
    type Value = String;

    fn into_value(self, context: &vm::BindingRead<'_, '_>) -> RuntimeResult<Self::Value> {
        Ok(context
            .string_ref(self)
            .map_err(Box::<RuntimeError>::from)?
            .to_string())
    }

    fn from_value(
        context: &mut vm::BindingWrite<'_, '_>,
        value: Self::Value,
    ) -> RuntimeResult<Self> {
        context
            .string_handle(&value)
            .map_err(Box::<RuntimeError>::from)
    }
}

impl<T: VmAbiCodec> VmAbiCodec for Option<T> {
    type Value = Option<T::Value>;

    fn into_value(self, context: &vm::BindingRead<'_, '_>) -> RuntimeResult<Self::Value> {
        self.map(|value| value.into_value(context)).transpose()
    }

    fn from_value(
        context: &mut vm::BindingWrite<'_, '_>,
        value: Self::Value,
    ) -> RuntimeResult<Self> {
        value.map(|value| T::from_value(context, value)).transpose()
    }
}

impl NativeAbiCodec for NativeStringRef {
    type Value = String;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        Ok(unsafe { self.as_str()? }.to_string())
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        binding.store_string_owned(value)
    }
}

impl NativeAbiCodec for NativeStringSlice {
    type Value = Vec<String>;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        let values = unsafe { self.as_slice()? };
        let mut value_strings = Vec::with_capacity(values.len());

        // decode each string
        for value in values {
            value_strings.push(unsafe { value.into_value()? });
        }

        Ok(value_strings)
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        let mut values = Vec::with_capacity(value.len());

        // encode each string
        for value in value {
            values.push(binding.store_string_owned(value));
        }

        binding.store_string_slice(values)
    }
}

impl<T> NativeAbiCodec for NativeSlice<T>
where
    T: Copy + NativeAbiCodec + 'static,
{
    type Value = Vec<T::Value>;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        let values = unsafe { self.as_slice()? };
        let mut decoded_values = Vec::with_capacity(values.len());

        // decode each value
        for value in values {
            decoded_values.push(unsafe { value.into_value()? });
        }

        Ok(decoded_values)
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        let mut values = Vec::with_capacity(value.len());

        // encode each value
        for value in value {
            values.push(T::from_value(binding, value));
        }

        binding.store_slice(values)
    }
}

impl<T> NativeAbiCodec for NativeArray<T>
where
    T: Copy + NativeAbiCodec + 'static,
{
    type Value = Vec<T::Value>;

    unsafe fn into_value(self) -> RuntimeResult<Self::Value> {
        let values = unsafe { self.as_slice()? };
        let mut decoded_values = Vec::with_capacity(values.len());

        // decode each value
        for value in values {
            decoded_values.push(unsafe { value.into_value()? });
        }

        Ok(decoded_values)
    }

    fn from_value(binding: &BindingCallContext, value: Self::Value) -> Self {
        let mut values = Vec::with_capacity(value.len());

        // encode each value
        for value in value {
            values.push(T::from_value(binding, value));
        }

        binding.store_array(values)
    }
}

impl<T> VmAbiCodec for VmSlice<T>
where
    T: Copy + VmAggregateCodec + VmAbiCodec + VmCollectionElement,
{
    type Value = Vec<T::Value>;

    fn into_value(self, context: &vm::BindingRead<'_, '_>) -> RuntimeResult<Self::Value> {
        let values = self.read_values(context)?;
        let mut decoded_values = Vec::with_capacity(values.len());

        // decode each value
        for value in values {
            decoded_values.push(value.into_value(context)?);
        }

        Ok(decoded_values)
    }

    fn from_value(
        context: &mut vm::BindingWrite<'_, '_>,
        value: Self::Value,
    ) -> RuntimeResult<Self> {
        let mut builder = VmSlice::<T>::builder(context, value.len())?;

        // decode each value directly into final VM storage
        for value in value {
            let value = T::from_value(context, value)?;
            builder.push(context, value)?;
        }

        builder.finish()
    }
}

impl<T> VmAbiCodec for VmArray<T>
where
    T: Copy + VmAggregateCodec + VmAbiCodec + VmCollectionElement,
{
    type Value = Vec<T::Value>;

    fn into_value(self, context: &vm::BindingRead<'_, '_>) -> RuntimeResult<Self::Value> {
        let values = self.read_values(context)?;
        let mut decoded_values = Vec::with_capacity(values.len());

        // decode each value
        for value in values {
            decoded_values.push(value.into_value(context)?);
        }

        Ok(decoded_values)
    }

    fn from_value(
        context: &mut vm::BindingWrite<'_, '_>,
        value: Self::Value,
    ) -> RuntimeResult<Self> {
        let slice = <VmSlice<T> as VmAbiCodec>::from_value(context, value)?;

        Ok(VmArray {
            data: slice.data,
            len: slice.len,
            capacity: slice.len,
            _marker: std::marker::PhantomData,
        })
    }
}

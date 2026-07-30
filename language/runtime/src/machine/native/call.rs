use std::fmt;

use destack_native::abi;
use destack_program as program;
use destack_program::Runtime;

use crate::diagnostic::RuntimeError;
use crate::worker::Activation;

use super::Error;

/// Runtime owner for one active native call.
pub struct Call<'program, 'runtime, 'memory, 'state> {
    /// The executing Program.
    program: &'program program::Program,
    /// Runtime and memory operations available to generated code.
    activation: &'program mut program::Activation<'runtime, 'memory, Activation<'state>>,
    /// The panic payload copied before native frames return.
    panic: Option<program::Value>,
    /// A failure raised by one native runtime operation.
    error: Option<Box<RuntimeError>>,
}

impl<'program, 'runtime, 'memory, 'state> Call<'program, 'runtime, 'memory, 'state> {
    /// Create one active native call.
    pub fn new(
        program: &'program program::Program,
        activation: &'program mut program::Activation<'runtime, 'memory, Activation<'state>>,
    ) -> Self {
        Self {
            program,
            activation,
            panic: None,
            error: None,
        }
    }

    /// Build the ABI activation borrowing this call.
    pub fn activation(&mut self, exit: &mut abi::Exit) -> abi::Activation {
        let call = (self as *mut Self).cast::<abi::Call>();
        let memory = &mut self.activation.memory;

        abi::Activation::new(
            call,
            memory.constant_space.native(self.program.sections()),
            memory.shared_static.native(),
            memory.local_static.native(),
            exit,
        )
    }

    /// Take one panic payload copied by generated native code.
    pub fn take_panic(&mut self) -> Option<program::Value> {
        self.panic.take()
    }

    /// Take one native runtime operation failure.
    pub fn take_error(&mut self) -> Option<Box<RuntimeError>> {
        self.error.take()
    }

    /// Call one linked runtime binding.
    unsafe extern "C" fn binding(
        activation: *mut abi::Activation,
        function: u32,
        arguments: *const u64,
        argument_count: usize,
        result: *mut u64,
        result_count: usize,
    ) -> abi::RuntimeStatusCode {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        let function = program::FunctionId(function);
        let Some(binding) = call.program.function_binding(function) else {
            return call.fail(RuntimeError::Internal {
                message: format!("function {function:?} has no runtime binding"),
            });
        };
        if (argument_count != 0 && arguments.is_null()) || (result_count != 0 && result.is_null()) {
            return call.fail(RuntimeError::Internal {
                message: format!("native binding call for {function:?} has null value storage"),
            });
        }

        // SAFETY: generated code supplies the exact linked signature ranges
        let arguments = if argument_count == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(arguments.cast(), argument_count) }
        };
        let result = if result_count == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(result.cast(), result_count) }
        };
        let memory = call.activation.memory.reborrow();
        match call
            .activation
            .runtime
            .call_binding(memory, binding, arguments, result)
        {
            Ok(()) => abi::RuntimeStatus::Continue.code(),
            Err(error) => call.fail_boxed(error),
        }
    }

    /// Record one payloadless language panic.
    unsafe extern "C" fn panic(activation: *mut abi::Activation) -> abi::ExitCode {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        call.panic = None;

        // SAFETY: the active activation owns one live exit record
        let exit = unsafe { &mut *(*activation).exit };

        exit.panic()
    }

    /// Copy one typed language panic payload into this call.
    unsafe extern "C" fn panic_value(
        activation: *mut abi::Activation,
        ty: u32,
        words: *const u64,
    ) -> abi::ExitCode {
        // SAFETY: generated code passes the active activation supplied to abi::Entry
        let call = unsafe { Self::from_activation(activation) };
        let ty = program::TypeId(ty);
        let Some(byte_len) = call.program.type_byte_len(ty) else {
            call.error = Some(Error::TypeMissing { ty }.into());

            // SAFETY: the active activation owns one live exit record
            let exit = unsafe { &mut *(*activation).exit };

            return exit.panic();
        };

        // SAFETY: generated code keeps the exact typed payload words live for this callback
        let word_count = byte_len.div_ceil(program::Word::BYTE_LEN);
        let words = unsafe { std::slice::from_raw_parts(words.cast(), word_count) };

        match call.program.value(ty, words.iter().copied()) {
            Ok(payload) => call.panic = Some(payload),
            Err(error) => call.error = Some(Error::from(error).into()),
        }

        // SAFETY: the active activation owns one live exit record
        let exit = unsafe { &mut *(*activation).exit };

        exit.panic()
    }

    /// Recover the runtime call owning one ABI activation.
    unsafe fn from_activation<'call>(activation: *mut abi::Activation) -> &'call mut Self {
        // SAFETY: abi::Activation.call was built from this exact Call type
        unsafe { &mut *(*activation).call.cast::<Self>() }
    }

    /// Retain one runtime operation failure and stop native execution.
    fn fail(&mut self, error: impl Into<Box<RuntimeError>>) -> abi::RuntimeStatusCode {
        self.fail_boxed(error.into())
    }

    /// Retain one boxed runtime failure and stop native execution.
    fn fail_boxed(&mut self, error: Box<RuntimeError>) -> abi::RuntimeStatusCode {
        self.error = Some(error);

        abi::RuntimeStatus::Exit.code()
    }

    /// Return the runtime binding operation.
    pub const fn binding_entry() -> abi::BindingCall {
        Self::binding
    }

    /// Return the payloadless panic runtime operation.
    pub const fn panic_entry() -> abi::Panic {
        Self::panic
    }

    /// Return the typed panic runtime operation.
    pub const fn panic_value_entry() -> abi::PanicValue {
        Self::panic_value
    }
}

impl fmt::Debug for Call<'_, '_, '_, '_> {
    /// Format this native call without exposing borrowed runtime state.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Call")
            .field("program", &self.program)
            .field("panic", &self.panic)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

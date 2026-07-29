use std::fmt;

use destack_program as program;
use destack_program::Runtime;
use destack_program::native::{
    NativeCall, NativeContext, NativeExit, NativeExitCode, NativeRuntimeStatus,
    NativeRuntimeStatusCode,
};

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

    /// Build the ABI context borrowing this call.
    pub fn context(&mut self, exit: &mut NativeExit) -> NativeContext {
        let call = (self as *mut Self).cast::<NativeCall>();
        let memory = &mut self.activation.memory;

        NativeContext::new(
            call,
            memory
                .constant_space
                .as_native_constants(self.program.sections()),
            memory.shared_static.as_native_statics(),
            memory.local_static.as_native_statics(),
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
        context: *mut NativeContext,
        function: program::FunctionId,
        arguments: *const program::Word,
        argument_count: usize,
        result: *mut program::Word,
        result_count: usize,
    ) -> NativeRuntimeStatusCode {
        // SAFETY: generated code passes the active context supplied to NativeEntry
        let call = unsafe { Self::from_context(context) };
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
            unsafe { std::slice::from_raw_parts(arguments, argument_count) }
        };
        let result = if result_count == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(result, result_count) }
        };
        let memory = call.activation.memory.reborrow();
        match call
            .activation
            .runtime
            .call_binding(memory, binding, arguments, result)
        {
            Ok(()) => NativeRuntimeStatus::Continue.code(),
            Err(error) => call.fail_boxed(error),
        }
    }

    /// Record one payloadless language panic.
    unsafe extern "C" fn panic(context: *mut NativeContext) -> NativeExitCode {
        // SAFETY: generated code passes the active context supplied to NativeEntry
        let call = unsafe { Self::from_context(context) };
        call.panic = None;

        // SAFETY: the active context owns one live exit record
        let exit = unsafe { &mut *(*context).exit };

        exit.panic()
    }

    /// Copy one typed language panic payload into this call.
    unsafe extern "C" fn panic_value(
        context: *mut NativeContext,
        ty: program::TypeId,
        words: *const program::Word,
    ) -> NativeExitCode {
        // SAFETY: generated code passes the active context supplied to NativeEntry
        let call = unsafe { Self::from_context(context) };
        let Some(byte_len) = call.program.type_byte_len(ty) else {
            call.error = Some(Error::TypeMissing { ty }.into());

            // SAFETY: the active context owns one live exit record
            let exit = unsafe { &mut *(*context).exit };

            return exit.panic();
        };

        // SAFETY: generated code keeps the exact typed payload words live for this callback
        let word_count = byte_len.div_ceil(program::Word::BYTE_LEN);
        let words = unsafe { std::slice::from_raw_parts(words, word_count) };

        match call.program.value(ty, words.iter().copied()) {
            Ok(payload) => call.panic = Some(payload),
            Err(error) => call.error = Some(Error::from(error).into()),
        }

        // SAFETY: the active context owns one live exit record
        let exit = unsafe { &mut *(*context).exit };

        exit.panic()
    }

    /// Recover the runtime call owning one ABI context.
    unsafe fn from_context<'call>(context: *mut NativeContext) -> &'call mut Self {
        // SAFETY: NativeContext.call was built from this exact Call type
        unsafe { &mut *(*context).call.cast::<Self>() }
    }

    /// Retain one runtime operation failure and stop native execution.
    fn fail(&mut self, error: impl Into<Box<RuntimeError>>) -> NativeRuntimeStatusCode {
        self.fail_boxed(error.into())
    }

    /// Retain one boxed runtime failure and stop native execution.
    fn fail_boxed(&mut self, error: Box<RuntimeError>) -> NativeRuntimeStatusCode {
        self.error = Some(error);

        NativeRuntimeStatus::Exit.code()
    }

    /// Return the runtime binding operation.
    pub const fn binding_entry() -> program::native::NativeBindingCall {
        Self::binding
    }

    /// Return the payloadless panic runtime operation.
    pub const fn panic_entry() -> program::native::NativePanic {
        Self::panic
    }

    /// Return the typed panic runtime operation.
    pub const fn panic_value_entry() -> program::native::NativePanicValue {
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

use std::cell::Cell;
use std::ptr::null_mut;

use tspp_native::abi;
use tspp_signal::Fault;

use super::Call;

thread_local! {
    /// The innermost running native activation on this thread.
    static ACTIVATION: Cell<*mut abi::Activation> = const { Cell::new(null_mut()) };
}

/// Run a function with one native activation visible to the trap handler.
pub(super) fn enter<T>(activation: *mut abi::Activation, function: impl FnOnce() -> T) -> T {
    let previous = ACTIVATION.replace(activation);
    let result = function();
    ACTIVATION.set(previous);

    result
}

/// Redirect one fault at a linked trap site into a trap unwind.
pub(super) fn handle_trap(fault: &mut Fault) -> bool {
    let activation = ACTIVATION.get();
    if activation.is_null() {
        return false;
    }

    // SAFETY: the published activation belongs to the live native call on this thread
    let call = unsafe { Call::from_activation(activation) };
    let Some(trap) = call.trap_at(fault.program_counter()) else {
        return false;
    };

    // call raise_trap from the faulting instruction, or from its caller on overflow
    let raise = raise_trap as *const () as usize;
    let code = trap.code() as usize;
    match trap {
        // SAFETY: the stack limit check runs after the frame record and before any register save
        abi::Trap::StackOverflow => unsafe { fault.call_from_caller(raise, code) },
        // SAFETY: generated frames save their return address and registers before any body trap site
        _ => unsafe { fault.call(raise, code) },
    }

    true
}

/// Record one trap on the running call and unwind to its entry.
extern "C-unwind" fn raise_trap(code: usize) -> ! {
    let trap = abi::Trap::try_from(code as abi::TrapCode)
        .unwrap_or_else(|error| unreachable!("the trap handler passes linked trap codes: {error}"));

    // SAFETY: the trap handler only redirects threads with a published activation
    let call = unsafe { Call::from_activation(ACTIVATION.get()) };

    call.trap(trap)
}

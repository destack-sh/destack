use std::fmt;
use std::io::Error as IoError;
use std::mem::{transmute, zeroed};
use std::ptr::null_mut;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::Fault;

/// The synchronous fault signals routed through the registered handlers.
const SIGNALS: [libc::c_int; 4] = [libc::SIGSEGV, libc::SIGBUS, libc::SIGILL, libc::SIGFPE];
/// The number of fault handlers the process registers at most.
const HANDLER_CAPACITY: usize = 4;

/// The registered fault handlers in registration order, unused entries holding zero.
static HANDLERS: [AtomicUsize; HANDLER_CAPACITY] =
    [const { AtomicUsize::new(0) }; HANDLER_CAPACITY];
/// The serialized handler registrations.
static REGISTRATION: Mutex<()> = Mutex::new(());
/// The signal actions replaced by the dispatcher, in `SIGNALS` order.
static REPLACED: OnceLock<Result<[ReplacedAction; SIGNALS.len()], SignalError>> = OnceLock::new();

/// One fault handler that returns true when it resolved the fault.
pub type Handler = fn(&mut Fault) -> bool;

/// One signal action the platform rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignalError {
    /// The rejected signal.
    pub signal: libc::c_int,
    /// The platform error code.
    pub code: Option<i32>,
}

/// One signal action replaced by the dispatcher.
struct ReplacedAction(libc::sigaction);

// SAFETY: signal actions are immutable after installation
unsafe impl Send for ReplacedAction {}

// SAFETY: signal actions are immutable after installation
unsafe impl Sync for ReplacedAction {}

impl fmt::Display for SignalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { signal, code } = self;

        write!(
            formatter,
            "install handler for signal {signal} (code {code:?})"
        )
    }
}

impl std::error::Error for SignalError {}

/// Register one fault handler once, installing the dispatcher at first use.
pub fn register(handler: Handler) -> Result<(), SignalError> {
    let address = handler as usize;

    // return the installation of a handler registered before
    if is_registered(address)
        && let Some(installed) = REPLACED.get()
    {
        return installed.as_ref().map(|_| ()).map_err(|error| *error);
    }

    let _registration = REGISTRATION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    // append the handler at the first unused entry unless it is already registered
    let appended = if is_registered(address) {
        None
    } else {
        let index = HANDLERS
            .iter()
            .position(|entry| entry.load(Ordering::Acquire) == 0)
            .unwrap_or_else(|| panic!("fault handler table holds {HANDLER_CAPACITY} handlers"));
        HANDLERS[index].store(address, Ordering::Release);

        Some(index)
    };

    // install the dispatcher once, dropping an appended handler when installation fails
    let installed = REPLACED.get_or_init(install);
    if installed.is_err()
        && let Some(index) = appended
    {
        HANDLERS[index].store(0, Ordering::Release);
    }

    installed.as_ref().map(|_| ()).map_err(|error| *error)
}

/// Return whether the handler table holds one handler address.
fn is_registered(address: usize) -> bool {
    HANDLERS
        .iter()
        .any(|entry| entry.load(Ordering::Acquire) == address)
}

/// Install the dispatcher for every fault signal, restoring every replaced action on failure.
fn install() -> Result<[ReplacedAction; SIGNALS.len()], SignalError> {
    // SAFETY: zeroed sigaction is filled before installation
    let mut action = unsafe { zeroed::<libc::sigaction>() };
    action.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK;
    action.sa_sigaction = dispatch as *const () as usize;

    // SAFETY: action contains storage for one signal mask
    unsafe { libc::sigemptyset(&mut action.sa_mask) };

    // replace every fault signal action, keeping the replaced one
    // SAFETY: zeroed sigaction is a valid out parameter
    let mut replaced = SIGNALS.map(|_| ReplacedAction(unsafe { zeroed() }));
    for (index, signal) in SIGNALS.into_iter().enumerate() {
        // SAFETY: action contains a valid SA_SIGINFO handler
        if unsafe { libc::sigaction(signal, &action, &mut replaced[index].0) } == 0 {
            continue;
        }
        let code = IoError::last_os_error().raw_os_error();

        // put back the actions this installation already replaced
        for (restored, signal) in SIGNALS.into_iter().enumerate().take(index) {
            // SAFETY: the action was captured from sigaction above
            unsafe { libc::sigaction(signal, &replaced[restored].0, null_mut()) };
        }

        return Err(SignalError { signal, code });
    }

    Ok(replaced)
}

/// Dispatch one fault signal to the registered handlers.
unsafe extern "C" fn dispatch(
    signal: libc::c_int,
    siginfo: *mut libc::siginfo_t,
    context: *mut libc::c_void,
) {
    // SAFETY: SA_SIGINFO delivers valid siginfo and context pointers
    let mut fault = unsafe { Fault::new(siginfo, context) };

    // offer the fault to each handler in registration order
    for entry in &HANDLERS {
        let address = entry.load(Ordering::Acquire);
        if address == 0 {
            break;
        }

        // SAFETY: the table only stores registered handler addresses
        let handler = unsafe { transmute::<usize, Handler>(address) };
        if handler(&mut fault) {
            return;
        }
    }

    // restore the replaced action and retry the faulting instruction under it
    restore(signal);
}

/// Restore the action the dispatcher replaced for one signal, the default one before installation completes.
fn restore(signal: libc::c_int) {
    let Some(Ok(replaced)) = REPLACED.get() else {
        // SAFETY: restoring the default action delegates the fault to the platform
        unsafe { libc::signal(signal, libc::SIG_DFL) };

        return;
    };

    // select the replaced action, defaulting ignored faults
    let index = SIGNALS
        .iter()
        .position(|candidate| *candidate == signal)
        .unwrap_or_else(|| unreachable!("dispatch only receives installed signals"));
    let action = &replaced[index].0;
    if action.sa_sigaction == libc::SIG_IGN {
        // SAFETY: restoring the default action delegates the fault to the platform
        unsafe { libc::signal(signal, libc::SIG_DFL) };

        return;
    }

    // SAFETY: the action was captured from sigaction at installation
    unsafe { libc::sigaction(signal, action, null_mut()) };
}

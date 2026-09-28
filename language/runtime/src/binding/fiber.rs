use tspp_program as program;
use tspp_program::{FunctionId, Memory, Word};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::scheduler::Call;
use crate::worker::Activation;

use super::{Binding, BindingTable, ReplayPayload};

/// Stable name of the fiber identity binding.
pub const FIBER_CURRENT: &str = "tspp.fiber.current";
/// Stable name of the fiber creation binding.
pub const FIBER_CREATE: &str = "tspp.fiber.create";
/// Stable name of the fiber park binding.
pub const FIBER_PARK: &str = "tspp.fiber.park";
/// Stable name of the fiber wake binding.
pub const FIBER_WAKE: &str = "tspp.fiber.wake";
/// Stable name of the fiber resume binding.
pub const FIBER_RESUME: &str = "tspp.fiber.resume";
/// Stable name of the fiber cancellation binding.
pub const FIBER_CANCEL: &str = "tspp.fiber.cancel";
/// Stable name of the microtask queue binding.
pub const MICROTASK_QUEUE: &str = "tspp.async.microtask.queue";

impl BindingTable {
    /// Register the runtime-provided fiber scheduling bindings.
    pub fn with_fiber_bindings(mut self) -> Self {
        self.upsert(Binding::new(
            program::BindingId::from_static_name(FIBER_CURRENT),
            ReplayPayload::Results,
            current,
        ));
        self.upsert(Binding::new(
            program::BindingId::from_static_name(FIBER_CREATE),
            ReplayPayload::ArgumentsAndResults,
            create,
        ));
        self.upsert(Binding::new(
            program::BindingId::from_static_name(FIBER_PARK),
            ReplayPayload::Results,
            park,
        ));
        self.upsert(Binding::new(
            program::BindingId::from_static_name(FIBER_WAKE),
            ReplayPayload::ArgumentsAndResults,
            wake,
        ));
        self.upsert(Binding::new(
            program::BindingId::from_static_name(FIBER_RESUME),
            ReplayPayload::ArgumentsAndResults,
            resume,
        ));
        self.upsert(Binding::new(
            program::BindingId::from_static_name(FIBER_CANCEL),
            ReplayPayload::ArgumentsAndResults,
            cancel,
        ));
        self.upsert(Binding::new(
            program::BindingId::from_static_name(MICROTASK_QUEUE),
            ReplayPayload::ArgumentsAndResults,
            queue_microtask,
        ));

        self
    }
}

/// Return the executing logical fiber handle.
fn current(
    _activation: &mut Activation<'_>,
    _memory: Memory<'_>,
    _context: program::Context,
    fiber_id: Option<program::FiberId>,
    _declaration: &program::Binding,
    _arguments: &[Word],
    result: &mut [Word],
) -> RuntimeResult<program::BindingExit> {
    let [slot] = result else {
        return Err(RuntimeError::Internal {
            message: "fiber.current returns one handle word".to_string(),
        }
        .boxed());
    };
    let Some(fiber_id) = fiber_id else {
        return Err(RuntimeError::Internal {
            message: "fiber.current requires a logical fiber".to_string(),
        }
        .boxed());
    };
    *slot = Word::from_bits(fiber_id.bits());

    Ok(program::BindingExit::Returned)
}

/// Create one fiber that runs a body once first woken.
fn create(
    activation: &mut Activation<'_>,
    _memory: Memory<'_>,
    context: program::Context,
    _fiber_id: Option<program::FiberId>,
    _declaration: &program::Binding,
    arguments: &[Word],
    result: &mut [Word],
) -> RuntimeResult<program::BindingExit> {
    let [slot] = result else {
        return Err(RuntimeError::Internal {
            message: "fiber.create returns one handle word".to_string(),
        }
        .boxed());
    };
    let (function, environment) = decode_thunk(activation, arguments)?;
    let fiber_id = activation.create_fiber(Call {
        function,
        environment,
        arguments: Vec::new(),
        context,
    });
    *slot = Word::from_bits(fiber_id.bits());

    Ok(program::BindingExit::Returned)
}

/// Hand the worker to one fiber, parking the caller until that fiber parks or finishes.
fn resume(
    activation: &mut Activation<'_>,
    _memory: Memory<'_>,
    _context: program::Context,
    fiber_id: Option<program::FiberId>,
    declaration: &program::Binding,
    arguments: &[Word],
    _result: &mut [Word],
) -> RuntimeResult<program::BindingExit> {
    let [fiber] = arguments else {
        return Err(RuntimeError::Internal {
            message: "fiber.resume requires one fiber handle".to_string(),
        }
        .boxed());
    };
    let Some(resumer) = fiber_id else {
        return Err(RuntimeError::Internal {
            message: "fiber.resume requires a logical fiber".to_string(),
        }
        .boxed());
    };
    let target = program::FiberId::from_bits(fiber.bits());

    // deliver void to the target's park and to this call
    let value = void_value(activation, declaration)?;
    let resumer_value = void_value(activation, declaration)?;
    activation.resume_fiber(target, resumer, value, resumer_value)?;

    Ok(program::BindingExit::Parked)
}

/// Park the calling fiber until a wake delivers its results.
fn park(
    activation: &mut Activation<'_>,
    _memory: Memory<'_>,
    _context: program::Context,
    fiber_id: Option<program::FiberId>,
    _declaration: &program::Binding,
    _arguments: &[Word],
    result: &mut [Word],
) -> RuntimeResult<program::BindingExit> {
    let Some(fiber_id) = fiber_id else {
        return Err(RuntimeError::Internal {
            message: "fiber.park requires a logical fiber".to_string(),
        }
        .boxed());
    };

    activation.park_fiber(fiber_id, result)
}

/// Return the void result value of one fiber binding.
fn void_value(
    activation: &Activation<'_>,
    declaration: &program::Binding,
) -> RuntimeResult<program::Value> {
    let ty = activation
        .program()
        .function_result(declaration.function)
        .ok_or_else(|| {
            RuntimeError::Internal {
                message: "a fiber binding has no result type".to_string(),
            }
            .boxed()
        })?;

    activation
        .program()
        .value(ty, [])
        .map_err(Box::<RuntimeError>::from)
}

/// Request cancellation of one fiber, unwinding it at its next park.
fn cancel(
    activation: &mut Activation<'_>,
    _memory: Memory<'_>,
    _context: program::Context,
    _fiber_id: Option<program::FiberId>,
    declaration: &program::Binding,
    arguments: &[Word],
    _result: &mut [Word],
) -> RuntimeResult<program::BindingExit> {
    let [fiber] = arguments else {
        return Err(RuntimeError::Internal {
            message: "fiber.cancel requires one fiber handle".to_string(),
        }
        .boxed());
    };
    let fiber_id = program::FiberId::from_bits(fiber.bits());
    let value = void_value(activation, declaration)?;

    activation.cancel_fiber(fiber_id, value)?;

    Ok(program::BindingExit::Returned)
}

/// Deliver one wake value, buffering it until the target fiber parks.
fn wake(
    activation: &mut Activation<'_>,
    _memory: Memory<'_>,
    _context: program::Context,
    _fiber_id: Option<program::FiberId>,
    declaration: &program::Binding,
    arguments: &[Word],
    _result: &mut [Word],
) -> RuntimeResult<program::BindingExit> {
    let [fiber] = arguments else {
        return Err(RuntimeError::Internal {
            message: "fiber.wake requires one fiber handle".to_string(),
        }
        .boxed());
    };
    let fiber_id = program::FiberId::from_bits(fiber.bits());

    // deliver the void value returned by the matching fiber park
    let value = void_value(activation, declaration)?;

    activation.wake_fiber(fiber_id, value)?;

    Ok(program::BindingExit::Returned)
}

/// Queue one callback after the active task finishes.
fn queue_microtask(
    activation: &mut Activation<'_>,
    _memory: Memory<'_>,
    context: program::Context,
    _fiber_id: Option<program::FiberId>,
    _declaration: &program::Binding,
    arguments: &[Word],
    _result: &mut [Word],
) -> RuntimeResult<program::BindingExit> {
    let (function, environment) = decode_thunk(activation, arguments)?;
    activation.queue_microtask(function, environment, Vec::new(), context);

    Ok(program::BindingExit::Returned)
}

/// Decode one thunk function value: an id word and an optional environment.
fn decode_thunk(
    activation: &Activation<'_>,
    arguments: &[Word],
) -> RuntimeResult<(FunctionId, Option<program::Value>)> {
    let (function, environment) = match arguments {
        [function] => (*function, None),
        [function, environment] => (*function, Some(*environment)),
        _ => {
            return Err(RuntimeError::Internal {
                message: "a thunk binding requires one function value".to_string(),
            }
            .boxed());
        }
    };
    let function = FunctionId::from_word(function).ok_or_else(|| {
        RuntimeError::Internal {
            message: "a thunk binding value is not a linked function".to_string(),
        }
        .boxed()
    })?;

    // retain the typed closure environment for scheduler root visiting
    let environment = environment
        .map(|word| {
            let ty = activation
                .program()
                .function(function)
                .and_then(program::Function::environment)
                .ok_or_else(|| {
                    RuntimeError::Internal {
                        message: "a thunk binding value has no environment type".to_string(),
                    }
                    .boxed()
                })?;

            activation
                .program()
                .value(ty, [word])
                .map_err(Box::<RuntimeError>::from)
        })
        .transpose()?;

    Ok((function, environment))
}

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tspp_program as program;
use tspp_program::{BindingId, Memory, Program, Word};
use tspp_repository::ReplayPayloadMode;

use crate::binding::{CodecId, DEFAULT_BINDING_CODEC};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::worker::Activation;

/// Runtime binding dispatch table.
#[derive(Debug)]
pub struct BindingTable {
    /// Registered bindings in stable insertion order.
    bindings: Vec<Binding>,
    /// Binding index keyed by stable binding id.
    indices: HashMap<BindingId, usize>,
}

/// One registered runtime binding.
#[derive(Debug, Clone, Copy)]
pub struct Binding {
    /// Stable Program binding identity.
    id: BindingId,
    /// Trace payload codec.
    codec: CodecId,
    /// Trace payload capability.
    replay_payload: ReplayPayload,
    /// Generated typed binding thunk.
    invoke: BindingFn,
}

/// Replay payload capability for one generated binding thunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplayPayload {
    /// Record only the result value.
    Results,
    /// Record arguments and results for verification.
    ArgumentsAndResults,
}

/// Generated runtime binding thunk.
pub type BindingFn = fn(
    activation: &mut Activation<'_>,
    memory: Memory<'_>,
    context: program::Context,
    fiber_id: Option<program::FiberId>,
    declaration: &program::Binding,
    arguments: &[Word],
    result: &mut [Word],
) -> RuntimeResult<program::BindingExit>;

impl BindingTable {
    /// Create one empty binding table.
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            indices: HashMap::new(),
        }
    }

    /// Call one registered binding.
    #[allow(clippy::too_many_arguments)]
    pub fn call(
        &self,
        declaration: &program::Binding,
        activation: &mut Activation<'_>,
        memory: Memory<'_>,
        context: program::Context,
        fiber_id: Option<program::FiberId>,
        arguments: &[Word],
        result: &mut [Word],
    ) -> RuntimeResult<program::BindingExit> {
        let id = declaration.id;
        let Some(index) = self.indices.get(&id).copied() else {
            return Err(RuntimeError::binding_not_found(format!("{id:?}")).boxed());
        };
        let binding = self.bindings[index];

        binding.call(
            declaration,
            activation,
            memory,
            context,
            fiber_id,
            arguments,
            result,
        )
    }

    /// Ensure every binding required by one program is registered.
    pub fn require(&self, program: &Program) -> RuntimeResult<()> {
        for binding in program.bindings() {
            // skip program-defined bindings
            if !binding.is_imported() {
                continue;
            }

            // external bindings require one registered runtime implementation
            if !self.indices.contains_key(&binding.id) {
                let name = program.string(binding.name).ok_or_else(|| {
                    RuntimeError::Internal {
                        message: format!("binding {:?} has no name", binding.id),
                    }
                    .boxed()
                })?;

                return Err(RuntimeError::binding_not_found(name).boxed());
            }
        }

        Ok(())
    }

    /// Insert or replace one binding implementation.
    pub fn upsert(&mut self, binding: Binding) {
        let id = binding.id;
        if let Some(index) = self.indices.get(&id).copied() {
            self.bindings[index] = binding;

            return;
        }

        let index = self.bindings.len();
        self.bindings.push(binding);
        self.indices.insert(id, index);
    }
}

impl Binding {
    /// Create one registered binding implementation.
    pub const fn new(id: BindingId, replay_payload: ReplayPayload, invoke: BindingFn) -> Self {
        Self {
            id,
            codec: DEFAULT_BINDING_CODEC,
            replay_payload,
            invoke,
        }
    }

    /// Return this binding with one explicit trace codec.
    pub const fn with_codec(mut self, codec: CodecId) -> Self {
        self.codec = codec;

        self
    }

    /// Return the stable Program identity.
    pub const fn id(self) -> BindingId {
        self.id
    }

    /// Return the trace payload codec.
    pub const fn codec(self) -> CodecId {
        self.codec
    }

    /// Return the supported replay payload.
    pub const fn replay_payload(self) -> ReplayPayload {
        self.replay_payload
    }

    /// Invoke this binding through one worker activation.
    #[allow(clippy::too_many_arguments)]
    fn call(
        self,
        declaration: &program::Binding,
        activation: &mut Activation<'_>,
        memory: Memory<'_>,
        context: program::Context,
        fiber_id: Option<program::FiberId>,
        arguments: &[Word],
        result: &mut [Word],
    ) -> RuntimeResult<program::BindingExit> {
        activation.on_before_binding(declaration)?;

        (self.invoke)(
            activation,
            memory,
            context,
            fiber_id,
            declaration,
            arguments,
            result,
        )
    }
}

impl From<ReplayPayloadMode> for ReplayPayload {
    /// Convert World configuration into a runtime replay payload.
    fn from(mode: ReplayPayloadMode) -> Self {
        match mode {
            ReplayPayloadMode::ResultsOnly => Self::Results,
            ReplayPayloadMode::ArgumentsAndResults => Self::ArgumentsAndResults,
        }
    }
}

impl From<ReplayPayload> for ReplayPayloadMode {
    /// Convert a runtime replay payload into World configuration.
    fn from(payload: ReplayPayload) -> Self {
        match payload {
            ReplayPayload::Results => Self::ResultsOnly,
            ReplayPayload::ArgumentsAndResults => Self::ArgumentsAndResults,
        }
    }
}

impl Default for BindingTable {
    fn default() -> Self {
        Self::new()
    }
}

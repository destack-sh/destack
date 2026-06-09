use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{
    Block, Lifetime, LifetimeParameter, Linkage, Local, LocalNodeId, Node, NodeType, Parameter,
    Place, PlaceEffect, Projection, Tree, Type, TypeReference, Value, ValueReference,
};

/// Memory allocation restrictions for a function.
///
/// This allows marking functions as managed-allocation-free or heap-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AllocationMode {
    /// No restrictions on allocation.
    #[default]
    Any,
    /// Managed allocation forbidden.
    /// Unique, raw, and frame allocation are still allowed.
    NoManaged,
    /// No heap allocation.
    /// Only `FrameAlloc` is allowed.
    NoHeap,
}

impl AllocationMode {
    /// Text representation for formatting/parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            AllocationMode::Any => "any",
            AllocationMode::NoManaged => "noManaged",
            AllocationMode::NoHeap => "noHeap",
        }
    }
}

/// The suspension kind for one function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SuspensionKind {
    /// Generator function (`function*`).
    /// Yields values to the caller, who controls resumption via `.next()`.
    /// Returns an `Iterator<T>`.
    Generator,
    /// Async function (`async function`).
    /// Awaits promises, with the runtime controlling resumption.
    /// Returns a `Promise<T>`.
    Async,
    /// Async generator function (`async function*`).
    /// Both yields values and awaits promises.
    /// Returns an `AsyncIterator<T>`.
    AsyncGenerator,
}

impl SuspensionKind {
    /// Text representation for formatting/parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            SuspensionKind::Generator => "generator",
            SuspensionKind::Async => "async",
            SuspensionKind::AsyncGenerator => "asyncGenerator",
        }
    }

    /// Whether this coroutine yields values (generator or async generator).
    pub fn is_generator(self) -> bool {
        matches!(
            self,
            SuspensionKind::Generator | SuspensionKind::AsyncGenerator
        )
    }

    /// Whether this coroutine awaits promises (async or async generator).
    pub fn is_async(self) -> bool {
        matches!(self, SuspensionKind::Async | SuspensionKind::AsyncGenerator)
    }
}

/// A function in MIR.
///
/// Functions are the top-level compilation unit, containing:
/// - Parameters as SSA values
/// - Local variables as stack slots
/// - Basic blocks forming a control flow graph
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Function {
    /// The function's name (for linking and debugging).
    pub name: StringId,
    /// Linkage (local, export, or import).
    pub linkage: Linkage,

    /// Function parameters as typed SSA slots.
    pub parameters: Vec<Parameter>,
    /// Lifetime parameters in function-local slot order.
    pub lifetimes: Vec<LifetimeParameter>,
    /// Optional parameter names for diagnostics.
    pub parameter_names: Vec<Option<StringId>>,

    /// Optional explicit SSA value names keyed by value id.
    pub value_names: Vec<Option<StringId>>,
    /// SSA value types keyed by value id.
    pub value_types: Vec<Option<LocalNodeId<Type>>>,
    /// Memory places keyed by SSA value.
    pub value_places: Vec<Option<Place>>,
    /// Counter for allocating unique SSA value IDs.
    pub(crate) next_value_id: u32,

    /// The return type.
    pub return_type: TypeReference,
    /// Borrow obligations required by this function body.
    pub borrow_obligations: Vec<BorrowObligation>,
    /// The hidden environment type for this function when present.
    pub environment: Option<TypeReference>,
    /// Local variables (stack-allocated slots for mutable bindings).
    pub locals: Vec<LocalNodeId<Local>>,
    /// All basic blocks in this function.
    pub blocks: Vec<LocalNodeId<Block>>,
    /// The entry block (execution starts here).
    pub entry: Option<LocalNodeId<Block>>,

    /// Memory allocation restrictions for this function.
    pub allocation: AllocationMode,
    /// The suspension kind when this function can suspend.
    pub suspension: Option<SuspensionKind>,
}

/// Borrow source proof required by a function body.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BorrowObligation {
    /// Borrow source must be stable across suspension.
    SuspensionStable {
        /// The lifetime whose source must be stable.
        lifetime: Lifetime,
    },
}

impl Node for Function {
    const TYPE: NodeType = NodeType::Function;
}

impl Function {
    /// Build parameter-derived SSA tables.
    pub(crate) fn parameter_state(
        parameters: &[Parameter],
    ) -> (u32, Vec<Option<LocalNodeId<Type>>>) {
        // derive the next value id from concrete parameters
        let next_value_id = parameters
            .iter()
            .filter_map(|parameter| match parameter.value {
                ValueReference::Value(value) => Some(value.0 + 1),
                ValueReference::Missing | ValueReference::Error => None,
            })
            .max()
            .unwrap_or(0);

        // initialize the type table with the next value id
        let mut value_types = vec![None; next_value_id as usize];

        // record concrete parameter types by value id
        for parameter in parameters {
            let ValueReference::Value(value) = parameter.value else {
                continue;
            };
            let Some(ty) = parameter.ty.ty() else {
                continue;
            };

            let index = value.0 as usize;
            let slot = &mut value_types[index];

            // idempotent duplicates are okay
            if slot.is_some_and(|existing| existing != ty) {
                unreachable!("value {value:?} has mismatched parameter types");
            }

            *slot = Some(ty);
        }

        (next_value_id, value_types)
    }

    /// Create one function from its signature facts.
    fn with_signature(
        name: StringId,
        parameters: Vec<Parameter>,
        return_type: TypeReference,
        linkage: Linkage,
        entry: Option<LocalNodeId<Block>>,
    ) -> Self {
        // seed parameter-derived state
        let parameter_names = vec![None; parameters.len()];
        let (next_value_id, value_types) = Self::parameter_state(&parameters);

        // function body and signature
        Self {
            name,
            parameters,
            lifetimes: Vec::new(),
            parameter_names,
            value_names: vec![None; next_value_id as usize],
            value_types,
            value_places: vec![None; next_value_id as usize],
            return_type,
            borrow_obligations: Vec::new(),
            linkage,
            allocation: AllocationMode::Any,
            suspension: None,
            environment: None,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry,
            next_value_id,
        }
    }

    /// Create a local function declaration without a body.
    pub fn declare(name: StringId, parameters: Vec<Parameter>, return_type: TypeReference) -> Self {
        Self::with_signature(name, parameters, return_type, Linkage::Local, None)
    }

    /// Create a new local (private) function with the given signature.
    pub fn local(
        name: StringId,
        parameters: Vec<Parameter>,
        return_type: TypeReference,
        entry: LocalNodeId<Block>,
    ) -> Self {
        Self::with_signature(name, parameters, return_type, Linkage::Local, Some(entry))
    }

    /// Create an imported function declaration (no body).
    pub fn import(name: StringId, parameters: Vec<Parameter>, return_type: TypeReference) -> Self {
        Self::with_signature(name, parameters, return_type, Linkage::Import, None)
    }

    /// Get the type for an SSA value.
    pub fn value_type(&self, value: Value) -> Option<LocalNodeId<Type>> {
        self.value_types.get(value.0 as usize).copied().flatten()
    }

    /// Get the explicit name for an SSA value when one exists.
    pub fn value_name(&self, value: Value) -> Option<StringId> {
        self.value_names.get(value.0 as usize).copied().flatten()
    }

    /// Get the place for an SSA value.
    pub fn value_place(&self, value: Value) -> Option<&Place> {
        self.value_places
            .get(value.0 as usize)
            .and_then(Option::as_ref)
    }

    /// Get the type for an SSA value or panic if missing.
    pub fn require_value_type(&self, value: Value) -> LocalNodeId<Type> {
        // ensure value types are always recorded for SSA values
        match self.value_type(value) {
            Some(ty) => ty,
            None => unreachable!("missing type for value {value:?}"),
        }
    }

    /// Record the type for an SSA value.
    pub fn set_value_type(&mut self, value: Value, ty: LocalNodeId<Type>) {
        let index = self.resize_value_slots(value);

        if let Some(existing) = self.value_types[index] {
            if existing != ty {
                unreachable!("value {value:?} has mismatched types {existing:?} and {ty:?}");
            }

            return;
        }

        self.value_types[index] = Some(ty);
    }

    /// Record the place for an SSA value.
    pub fn set_value_place(&mut self, value: Value, place: Place) {
        self.place_slot_mut(value).replace(place);
    }

    /// Record a projected place for an SSA value.
    pub fn set_projected_place(&mut self, value: Value, base: Value, projection: Projection) {
        let place = self
            .value_place(base)
            .cloned()
            .unwrap_or_else(|| Place::value(base.into()))
            .with_projection(projection);

        self.set_value_place(value, place);
    }

    /// Copy a place from one SSA value to another.
    pub fn copy_value_place(&mut self, value: Value, source: Value) {
        let Some(place) = self.value_place(source).cloned() else {
            return;
        };

        self.set_value_place(value, place);
    }

    /// Record one place table effect.
    pub(crate) fn record_place_effect(&mut self, effect: PlaceEffect) {
        match effect {
            PlaceEffect::Root { value, place } => {
                self.set_value_place(value, place);
            }
            PlaceEffect::Projection {
                value,
                base,
                projection,
            } => {
                self.set_projected_place(value, base, projection);
            }
            PlaceEffect::Copy { value, source } => {
                self.copy_value_place(value, source);
            }
        }
    }

    /// Replace value references inside stored places.
    pub fn replace_place_values(&mut self, from: Value, to: Value) {
        if from == to {
            return;
        }

        // rewrite projected operands
        for place in self.value_places.iter_mut().flatten() {
            place.replace_value(from, to);
        }

        // move place ownership
        let from_index = from.0 as usize;
        let from_place = self.value_places.get(from_index).cloned().flatten();

        let to_index = to.0 as usize;
        if to_index >= self.value_places.len() {
            self.value_places.resize(to_index + 1, None);
        }

        if self.value_places[to_index].is_none() {
            self.value_places[to_index] = from_place;
        }

        if let Some(slot) = self.value_places.get_mut(from_index) {
            *slot = None;
        }
    }

    /// Return the mutable place slot for one value.
    fn place_slot_mut(&mut self, value: Value) -> &mut Option<Place> {
        let index = self.resize_value_slots(value);

        &mut self.value_places[index]
    }

    /// Resize SSA side tables for one value.
    fn resize_value_slots(&mut self, value: Value) -> usize {
        let index = value.0 as usize;
        let value_count = index + 1;

        if self.value_types.len() < value_count {
            self.value_types.resize(value_count, None);
        }

        if self.value_names.len() < value_count {
            self.value_names.resize(value_count, None);
        }

        if self.value_places.len() < value_count {
            self.value_places.resize(value_count, None);
        }

        index
    }

    /// Set the linkage and return self (builder pattern).
    pub fn with_linkage(mut self, linkage: Linkage) -> Self {
        self.linkage = linkage;
        self
    }

    /// Set the suspension kind and return self (builder pattern).
    pub fn with_suspension(mut self, kind: SuspensionKind) -> Self {
        self.suspension = Some(kind);
        self
    }

    /// Check if this function can suspend.
    pub fn is_suspendable(&self) -> bool {
        self.suspension.is_some()
    }

    /// Check if this function is imported (defined elsewhere).
    pub fn is_import(&self) -> bool {
        self.linkage.is_import()
    }

    /// Allocate a new SSA value.
    pub fn next_value(&mut self) -> Value {
        let id = self.next_value_id;
        self.next_value_id += 1;
        Value::new(id)
    }

    /// Allocate a new SSA value and record its type.
    pub fn next_typed_value(&mut self, ty: LocalNodeId<Type>) -> Value {
        let value = self.next_value();
        self.set_value_type(value, ty);
        value
    }

    /// Allocate a new SSA value with the same type as an existing value.
    pub fn next_typed_value_like(&mut self, source: Value) -> Value {
        let ty = self.require_value_type(source);
        self.next_typed_value(ty)
    }

    /// Recompute `next_value_id` by scanning all values in the function.
    ///
    /// Call this before allocating new values if the function was parsed
    /// or modified externally and `next_value_id` may be stale.
    pub fn recompute_next_value_id(&mut self, tree: &Tree) {
        let mut max_id: u32 = 0;

        // function parameters
        for param in &self.parameters {
            if let ValueReference::Value(value) = param.value {
                max_id = max_id.max(value.0);
            }
        }

        // block parameters and instruction destinations
        for &block_id in &self.blocks {
            let block = tree.get(block_id);
            for param in &block.parameters {
                if let ValueReference::Value(value) = param.value {
                    max_id = max_id.max(value.0);
                }
            }
            for &instr_id in &block.instructions {
                let Some(ValueReference::Value(value)) = tree.get(instr_id).destination() else {
                    continue;
                };

                max_id = max_id.max(value.0);
            }
        }

        // avoid reusing value ids when blocks were removed
        let computed_next = max_id + 1;
        let min_next = self.value_types.len() as u32;
        self.next_value_id = self.next_value_id.max(computed_next).max(min_next);
    }

    /// Add a local variable and return its id.
    pub fn add_local(&mut self, local: LocalNodeId<Local>) {
        self.locals.push(local);
    }

    /// Add a basic block and return its id.
    pub fn add_block(&mut self, block: LocalNodeId<Block>) {
        self.blocks.push(block);
    }
}

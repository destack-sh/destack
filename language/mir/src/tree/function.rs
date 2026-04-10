use destack_core::StringId;
use serde::{Deserialize, Serialize};

use crate::{
    AllocationSize, Block, CallBehavior, Lifetime, Linkage, Local, LocalNodeId, MemoryEffect, Node,
    NodeTree, NodeType, PointerAttribute, Type, TypedValue, Value,
};

/// Memory allocation restrictions for a function.
///
/// This allows marking functions as realtime-safe (no managed allocations)
/// or embedded-safe (stack only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AllocationMode {
    /// No restrictions on allocation.
    #[default]
    Any,
    /// Managed allocation forbidden (realtime-safe).
    /// Only `RawAlloc`, `RawFree`, and `StackAlloc` are allowed.
    NoManaged,
    /// No heap allocation at all (stack only, embedded-safe).
    /// Only `StackAlloc` is allowed.
    StackOnly,
}

impl AllocationMode {
    /// Text representation for formatting/parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            AllocationMode::Any => "any",
            AllocationMode::NoManaged => "noManaged",
            AllocationMode::StackOnly => "stackOnly",
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

/// Execution model for GPU and accelerator kernels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionModel {
    /// General-purpose kernel entry point.
    Kernel,
    /// Graphics pipeline entry point.
    Graphics,
    /// Ray tracing pipeline entry point.
    RayTracing,
}

impl ExecutionModel {
    /// Text representation for formatting and parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            ExecutionModel::Kernel => "kernel",
            ExecutionModel::Graphics => "graphics",
            ExecutionModel::RayTracing => "rayTracing",
        }
    }
}

/// Parse execution models from their text identifiers.
impl TryFrom<&str> for ExecutionModel {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "kernel" => Ok(ExecutionModel::Kernel),
            "compute" => Ok(ExecutionModel::Kernel),
            "graphics" => Ok(ExecutionModel::Graphics),
            "rayTracing" => Ok(ExecutionModel::RayTracing),
            _ => Err(()),
        }
    }
}

/// Execution stage within a pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExecutionStage {
    /// Vertex stage.
    Vertex,
    /// Tessellation control stage.
    TessellationControl,
    /// Tessellation evaluation stage.
    TessellationEvaluation,
    /// Geometry stage.
    Geometry,
    /// Fragment stage.
    Fragment,
    /// Task stage.
    Task,
    /// Mesh stage.
    Mesh,
    /// Ray generation stage.
    RayGen,
    /// Any hit stage.
    AnyHit,
    /// Closest hit stage.
    ClosestHit,
    /// Miss stage.
    Miss,
    /// Intersection stage.
    Intersection,
    /// Callable stage.
    Callable,
}

impl ExecutionStage {
    /// Text representation for formatting and parsing.
    pub fn to_str(self) -> &'static str {
        match self {
            ExecutionStage::Vertex => "vertex",
            ExecutionStage::TessellationControl => "tessellationControl",
            ExecutionStage::TessellationEvaluation => "tessellationEvaluation",
            ExecutionStage::Geometry => "geometry",
            ExecutionStage::Fragment => "fragment",
            ExecutionStage::Task => "task",
            ExecutionStage::Mesh => "mesh",
            ExecutionStage::RayGen => "raygen",
            ExecutionStage::AnyHit => "anyHit",
            ExecutionStage::ClosestHit => "closestHit",
            ExecutionStage::Miss => "miss",
            ExecutionStage::Intersection => "intersection",
            ExecutionStage::Callable => "callable",
        }
    }
}

/// Parse execution stages from their text identifiers.
impl TryFrom<&str> for ExecutionStage {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "vertex" => Ok(ExecutionStage::Vertex),
            "tessellationControl" => Ok(ExecutionStage::TessellationControl),
            "tessellationEvaluation" => Ok(ExecutionStage::TessellationEvaluation),
            "geometry" => Ok(ExecutionStage::Geometry),
            "fragment" => Ok(ExecutionStage::Fragment),
            "task" => Ok(ExecutionStage::Task),
            "mesh" => Ok(ExecutionStage::Mesh),
            "raygen" => Ok(ExecutionStage::RayGen),
            "anyHit" => Ok(ExecutionStage::AnyHit),
            "closestHit" => Ok(ExecutionStage::ClosestHit),
            "miss" => Ok(ExecutionStage::Miss),
            "intersection" => Ok(ExecutionStage::Intersection),
            "callable" => Ok(ExecutionStage::Callable),
            _ => Err(()),
        }
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

    /// Function parameters as typed SSA values.
    pub parameters: Vec<TypedValue>,
    /// Optional parameter names for diagnostics.
    pub parameter_names: Vec<Option<StringId>>,
    /// Pointer attributes for parameters, indexed by parameter position.
    pub parameter_attributes: Vec<PointerAttribute>,

    /// Optional explicit SSA value names keyed by value id.
    pub value_names: Vec<Option<StringId>>,
    /// SSA value types by value id.
    pub value_types: Vec<LocalNodeId<Type>>,
    /// Counter for allocating unique SSA value IDs.
    pub(crate) next_value_id: u32,

    /// The return type.
    pub return_type: LocalNodeId<Type>,
    /// Lifetime bounds for the return value.
    pub return_lifetime: Lifetime,
    /// Pointer attribute for the return value.
    pub return_attribute: PointerAttribute,

    /// The hidden environment type for this function when present.
    pub environment: Option<LocalNodeId<Type>>,
    /// Local variables (stack-allocated slots for mutable bindings).
    pub locals: Vec<LocalNodeId<Local>>,
    /// All basic blocks in this function.
    pub blocks: Vec<LocalNodeId<Block>>,
    /// The entry block (execution starts here).
    pub entry: Option<LocalNodeId<Block>>,

    /// Memory effect for this function.
    pub memory_effect: MemoryEffect,
    /// Behavioral effects for this function.
    pub call_behavior: CallBehavior,
    /// Allocation size metadata for allocator-like functions.
    pub allocation_size: Option<AllocationSize>,
    /// Memory allocation restrictions for this function.
    pub allocation: AllocationMode,
    /// The suspension kind when this function can suspend.
    pub suspension: Option<SuspensionKind>,

    /// The execution model for GPU kernels.
    pub execution_model: Option<ExecutionModel>,
    /// The execution stage within the pipeline.
    pub execution_stage: Option<ExecutionStage>,
    /// The workgroup size for compute kernels.
    pub workgroup_size: Option<[u32; 3]>,
}

impl Node for Function {
    const TYPE: NodeType = NodeType::Function;
}

impl Function {
    /// Create a local function declaration without a body.
    pub fn declare(
        name: StringId,
        parameters: Vec<TypedValue>,
        return_type: LocalNodeId<Type>,
    ) -> Self {
        // seed parameter attributes
        let parameter_attributes = vec![PointerAttribute::default(); parameters.len()];
        let parameter_names = vec![None; parameters.len()];
        let next_value_id = parameters.iter().map(|p| p.value.0 + 1).max().unwrap_or(0);
        let value_types = Self::seed_value_types(&parameters, next_value_id);

        Self {
            name,
            parameters,
            parameter_names,
            value_names: vec![None; next_value_id as usize],
            value_types,
            return_type,
            return_lifetime: Lifetime::Inferred,
            memory_effect: MemoryEffect::unknown(),
            call_behavior: CallBehavior::unknown(),
            allocation_size: None,
            parameter_attributes,
            return_attribute: PointerAttribute::default(),
            linkage: Linkage::Local,
            allocation: AllocationMode::Any,
            suspension: None,
            execution_model: None,
            execution_stage: None,
            workgroup_size: None,
            environment: None,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: None,
            next_value_id,
        }
    }

    /// Create a new local (private) function with the given signature.
    pub fn local(
        name: StringId,
        parameters: Vec<TypedValue>,
        return_type: LocalNodeId<Type>,
        entry: LocalNodeId<Block>,
    ) -> Self {
        // seed parameter attributes
        let parameter_attributes = vec![PointerAttribute::default(); parameters.len()];
        let parameter_names = vec![None; parameters.len()];

        // compute the next value id from parameters
        let next_value_id = parameters.iter().map(|p| p.value.0 + 1).max().unwrap_or(0);
        let value_types = Self::seed_value_types(&parameters, next_value_id);

        // construct the function
        Self {
            name,
            parameters,
            parameter_names,
            value_names: vec![None; next_value_id as usize],
            value_types,
            return_type,
            return_lifetime: Lifetime::Inferred,
            memory_effect: MemoryEffect::unknown(),
            call_behavior: CallBehavior::unknown(),
            allocation_size: None,
            parameter_attributes,
            return_attribute: PointerAttribute::default(),
            linkage: Linkage::Local,
            allocation: AllocationMode::Any,
            suspension: None,
            execution_model: None,
            execution_stage: None,
            workgroup_size: None,
            environment: None,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: Some(entry),
            next_value_id,
        }
    }

    /// Create an imported function declaration (no body).
    pub fn import(
        name: StringId,
        parameters: Vec<TypedValue>,
        return_type: LocalNodeId<Type>,
    ) -> Self {
        // seed parameter attributes
        let parameter_attributes = vec![PointerAttribute::default(); parameters.len()];
        let parameter_names = vec![None; parameters.len()];
        let next_value_id = parameters.iter().map(|p| p.value.0 + 1).max().unwrap_or(0);
        let value_types = Self::seed_value_types(&parameters, next_value_id);

        // construct the imported function
        Self {
            name,
            parameters,
            parameter_names,
            value_names: vec![None; next_value_id as usize],
            value_types,
            return_type,
            return_lifetime: Lifetime::Inferred,
            memory_effect: MemoryEffect::unknown(),
            call_behavior: CallBehavior::unknown(),
            allocation_size: None,
            parameter_attributes,
            return_attribute: PointerAttribute::default(),
            linkage: Linkage::Import,
            allocation: AllocationMode::Any,
            suspension: None,
            execution_model: None,
            execution_stage: None,
            workgroup_size: None,
            environment: None,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: None,
            next_value_id,
        }
    }

    /// Get the type for an SSA value.
    pub fn value_type(&self, value: Value) -> Option<LocalNodeId<Type>> {
        self.value_types.get(value.0 as usize).copied()
    }

    /// Get the explicit name for an SSA value when one exists.
    pub fn value_name(&self, value: Value) -> Option<StringId> {
        self.value_names.get(value.0 as usize).copied().flatten()
    }

    /// Get the type for an SSA value or panic if missing.
    pub fn require_value_type(&self, value: Value) -> LocalNodeId<Type> {
        // ensure value types are always recorded for SSA values
        match self.value_type(value) {
            Some(ty) => ty,
            None => panic!("missing type for value {value:?}"),
        }
    }

    /// Record the type for an SSA value.
    pub fn set_value_type(&mut self, value: Value, ty: LocalNodeId<Type>) {
        // append at the end is okay
        let index = value.0 as usize;
        if index == self.value_types.len() {
            self.value_types.push(ty);
            if index == self.value_names.len() {
                self.value_names.push(None);
            }
            return;
        }

        // idempotent set is okay
        if index < self.value_types.len() {
            let existing = self.value_types[index];
            if existing == ty {
                return;
            }

            panic!("value {value:?} has mismatched types {existing:?} and {ty:?}");
        }

        panic!("value type index {index} exceeds next value id");
    }

    /// Seed the value type table from typed parameters.
    fn seed_value_types(parameters: &[TypedValue], next_value_id: u32) -> Vec<LocalNodeId<Type>> {
        // initialize the type table with the next value id
        let mut value_types = Vec::with_capacity(next_value_id as usize);
        let mut ordered: Vec<_> = parameters.iter().collect();
        ordered.sort_by_key(|param| param.value.0);

        // populate parameter types by their value ids
        for (expected, param) in ordered.into_iter().enumerate() {
            if param.value.0 as usize != expected {
                panic!("missing value type for v{expected}");
            }

            value_types.push(param.ty);
        }

        value_types
    }

    /// Set the return lifetime and return self (builder pattern).
    pub fn with_return_lifetime(mut self, lifetime: Lifetime) -> Self {
        self.return_lifetime = lifetime;
        self
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
    pub fn recompute_next_value_id(&mut self, tree: &NodeTree) {
        let mut max_id: u32 = 0;

        // function parameters
        for param in &self.parameters {
            max_id = max_id.max(param.value.0);
        }

        // block parameters and instruction destinations
        for &block_id in &self.blocks {
            let block = tree.get(block_id);
            for param in &block.parameters {
                max_id = max_id.max(param.value.0);
            }
            for &instr_id in &block.instructions {
                if let Some(dest) = tree.get(instr_id).destination() {
                    max_id = max_id.max(dest.0);
                }
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

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use destack_mir as mir;
use destack_source::ModuleId;

use crate::common::mir::{
    Analysis, AnalysisId, ModuleAnalyses, ModuleAnalysis, PackageAnalyses, PackageAnalysis,
    ProgramAnalyses, ProgramAnalysis,
};
use crate::optimize::{ModuleWorkItem, PackageWorkset, ProgramWorkset};

/// Directed edge in the call graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallEdge {
    /// The caller function id.
    pub caller: mir::LocalNodeId<mir::Function>,
    /// The callee function id.
    pub callee: mir::LocalNodeId<mir::Function>,
    /// The callsite that performs the call.
    pub callsite: CallSiteRef,
    /// The dispatch kind for this callsite.
    pub dispatch: mir::CallDispatchKind,
}

impl CallEdge {
    /// Return true when this edge is a direct call.
    pub fn is_direct(&self) -> bool {
        matches!(self.dispatch, mir::CallDispatchKind::Direct)
    }
}

/// Callsite that does not have a resolved target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnknownCallSite {
    /// The caller function id.
    pub caller: mir::LocalNodeId<mir::Function>,
    /// The callsite that performs the call.
    pub callsite: CallSiteRef,
    /// The dispatch kind for this callsite.
    pub dispatch: mir::CallDispatchKind,
    /// The declared callee when known.
    pub callee: Option<mir::LocalNodeId<mir::Function>>,
}

/// Callsite reference for module call graphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallSiteRef {
    /// Callsite is an instruction.
    Instruction(mir::LocalNodeId<mir::Instruction>),
    /// Callsite is a terminator in the block.
    Terminator(mir::LocalNodeId<mir::Block>),
}

/// Callsite identifier for cross module graphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallSiteId {
    /// The module containing the callsite.
    pub module: ModuleId,
    /// The block containing the callsite.
    pub block: mir::LocalNodeId<mir::Block>,
    /// The instruction id within the block, if any.
    pub instruction: Option<mir::LocalNodeId<mir::Instruction>>,
}

/// Symbol name used for cross module call graphs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolName {
    /// The interned symbol text.
    value: Arc<str>,
}

impl SymbolName {
    /// Create a symbol name from a string.
    pub fn new(name: &str) -> Self {
        Self {
            value: Arc::from(name),
        }
    }

    /// Create the sentinel symbol for unknown or external calls.
    pub fn external() -> Self {
        Self {
            value: Arc::from("<external>"),
        }
    }

    /// Return the symbol name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

/// Signature for resolving callsites across modules.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SignatureKey {
    /// The parameter type signatures.
    parameters: Vec<SignatureType>,
    /// The return type signature.
    result: SignatureType,
    /// Borrow obligations callers must satisfy.
    borrow_obligations: Vec<mir::BorrowObligation>,
}

impl SignatureKey {
    /// Build a signature key from a MIR function.
    fn from_function(tree: &mir::Tree, function: &mir::Function) -> Option<Self> {
        // capture parameter signatures
        let parameters = function
            .parameters
            .iter()
            .map(|param| SignatureType::from_type(tree, param.ty.ty()?))
            .collect::<Option<Vec<_>>>()?;

        // capture result signature
        let result = SignatureType::from_type(tree, function.return_type.ty()?)?;

        Some(Self {
            parameters,
            result,
            borrow_obligations: function.borrow_obligations.clone(),
        })
    }

    /// Build a signature key from a function pointer type.
    fn from_function_type(
        tree: &mir::Tree,
        signature: impl Into<mir::TypeReference>,
    ) -> Option<Self> {
        let signature = signature.into().ty()?;

        // load the function pointer signature
        let mir::Type::FunctionSignature {
            parameters,
            result,
            borrow_obligations,
        } = tree.get(signature)
        else {
            return None;
        };

        // capture parameter signatures
        let parameters = parameters
            .iter()
            .map(|param| SignatureType::from_type(tree, param.ty()?))
            .collect::<Option<Vec<_>>>()?;

        // capture result signature
        let result = SignatureType::from_type(tree, result.ty()?)?;

        Some(Self {
            parameters,
            result,
            borrow_obligations: borrow_obligations.clone(),
        })
    }
}

/// Structural signature for a MIR type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SignatureType {
    /// Void type signature.
    Void,
    /// Boolean type signature.
    Boolean,
    /// Integer type signature.
    Int {
        /// Bit width for the integer type.
        width: u16,
        /// Signedness of the integer type.
        signed: bool,
    },
    /// Pointer-sized signed integer type signature.
    Isize,
    /// Pointer-sized unsigned integer type signature.
    Usize,
    /// Floating point type signature.
    Float {
        /// Bit width for the floating point type.
        width: u16,
    },
    /// Runtime type descriptor handle signature.
    TypeDescriptor,
    /// Compact runtime type id signature.
    TypeId,
    /// Atomic storage type signature.
    Atomic {
        /// Stored value type signature.
        value: Box<SignatureType>,
    },
    /// Erased Any value signature.
    Any {
        /// Interface type signature.
        interface: Box<SignatureType>,
    },
    /// Linear uninitialized allocation token signature.
    Uninit {
        /// Value type under construction.
        value: Box<SignatureType>,
    },
    /// Reference type signature.
    Reference {
        /// Reference kind for the pointer.
        kind: mir::ReferenceKind,
        /// Escaping lifetime root.
        lifetime: mir::Lifetime,
        /// Space for the reference.
        space: mir::Space,
        /// Access for the reference.
        access: mir::Access,
        /// Pointee type signature.
        pointee: Box<SignatureType>,
        /// Nullability for the reference.
        nullability: mir::Nullability,
    },
    /// Array type signature.
    Array {
        /// Element type signature.
        element: Box<SignatureType>,
        /// Array length.
        length: u64,
        /// Copy of the array.
        copy: mir::Copy,
    },
    /// Slice type signature.
    Slice {
        /// Reference kind for the slice base.
        kind: mir::ReferenceKind,
        /// Escaping lifetime root.
        lifetime: mir::Lifetime,
        /// Element type signature.
        element: Box<SignatureType>,
        /// Space for the slice base.
        space: mir::Space,
        /// Access exposed by the slice.
        access: mir::Access,
        /// Nullability for the slice.
        nullability: mir::Nullability,
    },
    /// Tuple type signature.
    Tuple {
        /// Element type signatures.
        elements: Vec<SignatureType>,
        /// Copy of the tuple.
        copy: mir::Copy,
    },
    /// Struct type signature.
    Struct {
        /// Field type signatures in declaration order.
        fields: Vec<SignatureType>,
        /// Copy of the struct.
        copy: mir::Copy,
    },
    /// Nominal newtype signature.
    Newtype {
        /// Inner type signature.
        inner: Box<SignatureType>,
        /// Copy of the newtype.
        copy: mir::Copy,
    },
    /// Variant type signature.
    Variant {
        /// Tag type signature.
        tag: Box<SignatureType>,
        /// Storage type signature.
        storage: Box<SignatureType>,
        /// Variant type signatures in tag order.
        cases: Vec<(mir::Constant, SignatureType)>,
        /// Copy of the variant.
        copy: mir::Copy,
    },
    /// Vector type signature.
    Vector {
        /// Element type signature.
        element: Box<SignatureType>,
        /// Lane count.
        lanes: u32,
        /// Copy of the vector.
        copy: mir::Copy,
    },
    /// Tensor value signature.
    Tensor {
        /// Element type signature.
        element: Box<SignatureType>,
        /// Tensor shape.
        shape: Vec<mir::TensorDimension>,
        /// Tensor layout.
        layout: mir::TensorLayout,
        /// Copy of the tensor.
        copy: mir::Copy,
    },
    /// Tensor view signature.
    TensorView {
        /// Reference kind for the view.
        kind: mir::ReferenceKind,
        /// Escaping lifetime root.
        lifetime: mir::Lifetime,
        /// Space for the view.
        space: mir::Space,
        /// Access for the view.
        access: mir::Access,
        /// Element type signature.
        element: Box<SignatureType>,
        /// Tensor shape.
        shape: Vec<mir::TensorDimension>,
        /// Tensor view layout.
        layout: mir::TensorViewLayout,
        /// Nullability for the view.
        nullability: mir::Nullability,
    },
    /// Bare function signature.
    FunctionSignature {
        /// Parameter type signatures.
        parameters: Vec<SignatureType>,
        /// Result type signature.
        result: Box<SignatureType>,
        /// Borrow obligations callers must satisfy.
        borrow_obligations: Vec<mir::BorrowObligation>,
    },
    /// Function pointer type signature.
    FunctionPointer {
        /// The bare function signature.
        signature: Box<SignatureType>,
    },
    /// Closure value signature.
    Closure {
        /// The bare function signature.
        signature: Box<SignatureType>,
        /// The captured environment representation.
        environment: Box<SignatureType>,
    },
}

impl SignatureType {
    /// Build a signature type from a MIR type.
    fn from_type(tree: &mir::Tree, ty_id: mir::LocalNodeId<mir::Type>) -> Option<Self> {
        // load the MIR type
        let ty = tree.get(ty_id);

        // map the MIR type to a structural signature
        Some(match ty {
            mir::Type::Void => SignatureType::Void,
            mir::Type::Boolean => SignatureType::Boolean,
            mir::Type::Int {
                width,
                is_signed: signed,
            } => SignatureType::Int {
                width: *width,
                signed: *signed,
            },
            mir::Type::Isize => SignatureType::Isize,
            mir::Type::Usize => SignatureType::Usize,
            mir::Type::Float(float_type) => SignatureType::Float {
                width: float_type.width(),
            },
            mir::Type::TypeDescriptor => SignatureType::TypeDescriptor,
            mir::Type::TypeId => SignatureType::TypeId,
            mir::Type::Atomic { value } => SignatureType::Atomic {
                value: Box::new(SignatureType::from_type(tree, value.ty()?)?),
            },
            mir::Type::Any { interface } => SignatureType::Any {
                interface: Box::new(SignatureType::from_type(tree, interface.ty()?)?),
            },
            mir::Type::Uninit { value } => SignatureType::Uninit {
                value: Box::new(SignatureType::from_type(tree, value.ty()?)?),
            },
            mir::Type::Reference {
                kind,
                lifetime,
                space,
                access,
                pointee,
                nullability,
            } => SignatureType::Reference {
                kind: *kind,
                lifetime: lifetime.clone(),
                space: space.clone(),
                access: *access,
                pointee: Box::new(SignatureType::from_type(tree, pointee.ty()?)?),
                nullability: *nullability,
            },
            mir::Type::Array {
                element,
                length,
                copy,
            } => SignatureType::Array {
                element: Box::new(SignatureType::from_type(tree, element.ty()?)?),
                length: *length,
                copy: *copy,
            },
            mir::Type::Slice {
                kind,
                lifetime,
                element,
                space,
                access,
                nullability,
            } => SignatureType::Slice {
                kind: *kind,
                lifetime: lifetime.clone(),
                element: Box::new(SignatureType::from_type(tree, element.ty()?)?),
                space: space.clone(),
                access: *access,
                nullability: *nullability,
            },
            mir::Type::Tuple { elements, copy } => {
                // convert tuple elements to signature types
                let elements = elements
                    .iter()
                    .map(|element| SignatureType::from_type(tree, element.ty()?))
                    .collect::<Option<Vec<_>>>()?;

                SignatureType::Tuple {
                    elements,
                    copy: *copy,
                }
            }
            mir::Type::Struct { fields, copy } => {
                // convert struct fields to signature types
                let fields = fields
                    .iter()
                    .map(|field_id| SignatureType::from_type(tree, tree.get(*field_id).ty.ty()?))
                    .collect::<Option<Vec<_>>>()?;

                SignatureType::Struct {
                    fields,
                    copy: *copy,
                }
            }
            mir::Type::Newtype { inner, copy } => SignatureType::Newtype {
                inner: Box::new(SignatureType::from_type(tree, inner.ty()?)?),
                copy: *copy,
            },
            mir::Type::Variant {
                tag,
                storage,
                cases,
                copy,
            } => {
                let tag = Box::new(SignatureType::from_type(tree, tag.ty()?)?);
                let storage = Box::new(SignatureType::from_type(tree, storage.ty()?)?);
                let cases = cases
                    .iter()
                    .map(|case| {
                        let ty = SignatureType::from_type(tree, case.ty.ty()?)?;
                        Some((case.tag.clone(), ty))
                    })
                    .collect::<Option<Vec<_>>>()?;

                SignatureType::Variant {
                    tag,
                    storage,
                    cases,
                    copy: *copy,
                }
            }
            mir::Type::Vector {
                element,
                lanes,
                copy,
            } => SignatureType::Vector {
                element: Box::new(SignatureType::from_type(tree, element.ty()?)?),
                lanes: *lanes,
                copy: *copy,
            },
            mir::Type::Tensor {
                element,
                shape,
                layout,
                copy,
            } => SignatureType::Tensor {
                element: Box::new(SignatureType::from_type(tree, element.ty()?)?),
                shape: shape.clone(),
                layout: layout.clone(),
                copy: *copy,
            },
            mir::Type::TensorView {
                kind,
                lifetime,
                space,
                access,
                element,
                shape,
                layout,
                nullability,
            } => SignatureType::TensorView {
                kind: *kind,
                lifetime: lifetime.clone(),
                space: space.clone(),
                access: *access,
                element: Box::new(SignatureType::from_type(tree, element.ty()?)?),
                shape: shape.clone(),
                layout: layout.clone(),
                nullability: *nullability,
            },
            mir::Type::FunctionSignature {
                parameters,
                result,
                borrow_obligations,
            } => {
                // convert function signatures recursively
                let parameters = parameters
                    .iter()
                    .map(|param| SignatureType::from_type(tree, param.ty()?))
                    .collect::<Option<Vec<_>>>()?;

                // capture the result type signature
                let result = Box::new(SignatureType::from_type(tree, result.ty()?)?);

                SignatureType::FunctionSignature {
                    parameters,
                    result,
                    borrow_obligations: borrow_obligations.clone(),
                }
            }
            mir::Type::FunctionPointer { signature } => SignatureType::FunctionPointer {
                signature: Box::new(SignatureType::from_type(tree, signature.ty()?)?),
            },
            mir::Type::Closure {
                signature,
                environment,
            } => SignatureType::Closure {
                signature: Box::new(SignatureType::from_type(tree, signature.ty()?)?),
                environment: Box::new(SignatureType::from_type(tree, environment.ty()?)?),
            },
        })
    }
}

/// Directed edge in a symbol call graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolCallEdge {
    /// The caller symbol.
    pub caller: SymbolName,
    /// The callee symbol.
    pub callee: SymbolName,
    /// The callsite identifier.
    pub callsite: CallSiteId,
    /// The dispatch kind for this callsite.
    pub dispatch: mir::CallDispatchKind,
}

/// Callsite that does not have a resolved target symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolUnknownCallSite {
    /// The caller symbol.
    pub caller: SymbolName,
    /// The callsite identifier.
    pub callsite: CallSiteId,
    /// The dispatch kind for this callsite.
    pub dispatch: mir::CallDispatchKind,
    /// The unresolved symbol name, when known.
    pub callee: Option<SymbolName>,
}

/// Cross-module call graph keyed by symbol names.
#[derive(Debug)]
pub struct SymbolCallGraph {
    /// Outgoing edges by caller.
    outgoing: HashMap<SymbolName, Vec<SymbolCallEdge>>,
    /// Incoming edges by callee.
    incoming: HashMap<SymbolName, Vec<SymbolCallEdge>>,
    /// Unresolved callsites by caller.
    unknown: HashMap<SymbolName, Vec<SymbolUnknownCallSite>>,
    /// Definition details for symbols in scope.
    definitions: HashMap<SymbolName, Vec<SymbolDefinition>>,
    /// Symbols with multiple definitions in scope.
    ambiguous: HashSet<SymbolName>,
    /// Sentinel symbol for external or unknown targets.
    external_symbol: SymbolName,
}

/// Definition information for a symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SymbolDefinition {
    /// mir::Linkage for this definition.
    linkage: mir::Linkage,
    /// Signature for this definition.
    signature: SignatureKey,
}

impl SymbolCallGraph {
    /// Get outgoing call edges for a symbol.
    pub fn outgoing(&self, caller: &SymbolName) -> &[SymbolCallEdge] {
        const EMPTY: [SymbolCallEdge; 0] = [];
        self.outgoing
            .get(caller)
            .map(Vec::as_slice)
            .unwrap_or(&EMPTY)
    }

    /// Get incoming call edges for a symbol.
    pub fn incoming(&self, callee: &SymbolName) -> &[SymbolCallEdge] {
        const EMPTY: [SymbolCallEdge; 0] = [];
        self.incoming
            .get(callee)
            .map(Vec::as_slice)
            .unwrap_or(&EMPTY)
    }

    /// Get unresolved callsites for a symbol.
    pub fn unknown_calls(&self, caller: &SymbolName) -> &[SymbolUnknownCallSite] {
        const EMPTY: [SymbolUnknownCallSite; 0] = [];
        self.unknown
            .get(caller)
            .map(Vec::as_slice)
            .unwrap_or(&EMPTY)
    }

    /// Return the sentinel symbol for unknown targets.
    pub fn external_symbol(&self) -> &SymbolName {
        &self.external_symbol
    }

    /// Check whether a symbol is defined in this graph.
    pub fn is_defined(&self, symbol: &SymbolName) -> bool {
        self.definitions.contains_key(symbol)
    }

    /// Check whether a symbol is ambiguous in this graph.
    pub fn is_ambiguous(&self, symbol: &SymbolName) -> bool {
        self.ambiguous.contains(symbol)
    }
}

/// Module scoped call graph.
#[derive(Debug)]
pub struct CallGraph {
    /// Outgoing edges by caller.
    outgoing: HashMap<mir::LocalNodeId<mir::Function>, Vec<CallEdge>>,
    /// Incoming edges by callee.
    incoming: HashMap<mir::LocalNodeId<mir::Function>, Vec<CallEdge>>,
    /// Unresolved callsites by caller.
    unknown: HashMap<mir::LocalNodeId<mir::Function>, Vec<UnknownCallSite>>,
}

impl CallGraph {
    /// Get outgoing call edges for a function.
    pub fn outgoing(&self, caller: mir::LocalNodeId<mir::Function>) -> &[CallEdge] {
        const EMPTY: [CallEdge; 0] = [];
        self.outgoing
            .get(&caller)
            .map(Vec::as_slice)
            .unwrap_or(&EMPTY)
    }

    /// Get incoming call edges for a function.
    pub fn incoming(&self, callee: mir::LocalNodeId<mir::Function>) -> &[CallEdge] {
        const EMPTY: [CallEdge; 0] = [];
        self.incoming
            .get(&callee)
            .map(Vec::as_slice)
            .unwrap_or(&EMPTY)
    }

    /// Get unresolved callsites for a function.
    pub fn unknown_calls(&self, caller: mir::LocalNodeId<mir::Function>) -> &[UnknownCallSite] {
        const EMPTY: [UnknownCallSite; 0] = [];
        self.unknown
            .get(&caller)
            .map(Vec::as_slice)
            .unwrap_or(&EMPTY)
    }

    /// Build a call graph for the given module.
    fn build(tree: &mir::Tree) -> Self {
        let mut graph = Self {
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
            unknown: HashMap::new(),
        };

        // scan each function for call instructions
        for (function_id, function) in tree.iter_nodes::<mir::Function>() {
            // skip functions without bodies
            if function.entry.is_none() {
                continue;
            }

            for block_id in &function.blocks {
                let block = tree.get(*block_id);
                let terminator = tree.get(block.terminator);

                for &instruction_id in &block.instructions {
                    let instruction = tree.get(instruction_id);

                    let Some(callsite) =
                        CallSite::from_instruction(function_id, instruction_id, instruction, tree)
                    else {
                        continue;
                    };

                    graph.insert_callsite(callsite);
                }

                if let Some(callsite) =
                    CallSite::from_terminator(function_id, *block_id, terminator, tree)
                {
                    graph.insert_callsite(callsite);
                }
            }
        }

        graph
    }

    /// Insert a callsite into the graph.
    fn insert_callsite(&mut self, callsite: CallSite) {
        // record resolved edges when a target is known
        if let Some(callee) = callsite.callee {
            let edge = CallEdge {
                caller: callsite.caller,
                callee,
                callsite: callsite.callsite,
                dispatch: callsite.dispatch,
            };

            self.outgoing.entry(callsite.caller).or_default().push(edge);
            self.incoming.entry(callee).or_default().push(edge);

            if callsite.is_precise {
                return;
            }
        }

        // record unresolved or partially resolved callsites
        let unknown = UnknownCallSite {
            caller: callsite.caller,
            callsite: callsite.callsite,
            dispatch: callsite.dispatch,
            callee: callsite.callee,
        };

        self.unknown
            .entry(callsite.caller)
            .or_default()
            .push(unknown);
    }
}

impl Analysis for CallGraph {
    const ID: AnalysisId = AnalysisId("callgraph");
}

impl ModuleAnalysis for CallGraph {
    /// Compute the module call graph.
    fn compute(tree: &mir::Tree, _analyses: &ModuleAnalyses<'_>) -> Self {
        Self::build(tree)
    }
}

/// Strongly connected components for a module call graph.
#[derive(Debug, Default)]
pub struct CallGraphScc {
    /// Maps each function to its SCC id.
    function_scc: HashMap<mir::LocalNodeId<mir::Function>, usize>,
    /// SCCs that are recursive.
    recursive_sccs: HashSet<usize>,
}

impl CallGraphScc {
    /// Return the SCC id for a function.
    pub fn scc_id(&self, function_id: mir::LocalNodeId<mir::Function>) -> Option<usize> {
        self.function_scc.get(&function_id).copied()
    }

    /// Return true when the SCC is recursive.
    pub fn is_recursive_scc(&self, scc_id: usize) -> bool {
        self.recursive_sccs.contains(&scc_id)
    }

    /// Return true when the function is part of a recursive SCC.
    pub fn is_recursive_function(&self, function_id: mir::LocalNodeId<mir::Function>) -> bool {
        let Some(scc_id) = self.scc_id(function_id) else {
            return false;
        };

        self.is_recursive_scc(scc_id)
    }
}

impl Analysis for CallGraphScc {
    const ID: AnalysisId = AnalysisId("callgraph-scc");
}

impl ModuleAnalysis for CallGraphScc {
    /// Compute the SCCs for the module call graph.
    fn compute(tree: &mir::Tree, analyses: &ModuleAnalyses<'_>) -> Self {
        let callgraph = analyses.get::<CallGraph>();
        compute_callgraph_scc(tree, &callgraph)
    }
}

/// Compute SCCs for the module call graph.
fn compute_callgraph_scc(tree: &mir::Tree, callgraph: &CallGraph) -> CallGraphScc {
    // prepare tarjan state
    let mut index = 0usize;
    let mut next_scc_id = 0usize;
    let mut stack = Vec::new();
    let mut on_stack = HashSet::new();
    let mut indices = HashMap::new();
    let mut lowlinks = HashMap::new();
    let mut scc_map = CallGraphScc::default();

    // collect function ids for traversal
    let function_ids: Vec<_> = tree
        .iter_nodes::<mir::Function>()
        .map(|(id, _)| id)
        .collect();

    // run tarjan across all functions
    for function_id in function_ids {
        if !indices.contains_key(&function_id) {
            tarjan_visit(
                function_id,
                callgraph,
                &mut index,
                &mut next_scc_id,
                &mut stack,
                &mut on_stack,
                &mut indices,
                &mut lowlinks,
                &mut scc_map,
            );
        }
    }

    scc_map
}

/// Tarjan recursion for SCC discovery.
#[allow(clippy::too_many_arguments)]
fn tarjan_visit(
    function_id: mir::LocalNodeId<mir::Function>,
    callgraph: &CallGraph,
    index: &mut usize,
    next_scc_id: &mut usize,
    stack: &mut Vec<mir::LocalNodeId<mir::Function>>,
    on_stack: &mut HashSet<mir::LocalNodeId<mir::Function>>,
    indices: &mut HashMap<mir::LocalNodeId<mir::Function>, usize>,
    lowlinks: &mut HashMap<mir::LocalNodeId<mir::Function>, usize>,
    scc_map: &mut CallGraphScc,
) {
    // initialize tarjan state for this node
    indices.insert(function_id, *index);
    lowlinks.insert(function_id, *index);
    *index += 1;
    stack.push(function_id);
    on_stack.insert(function_id);

    // walk direct call edges to discover SCCs
    for edge in callgraph.outgoing(function_id) {
        // skip non direct edges for recursion detection
        if edge.dispatch != mir::CallDispatchKind::Direct {
            continue;
        }

        // visit the callee for scc discovery
        let callee = edge.callee;
        if !indices.contains_key(&callee) {
            tarjan_visit(
                callee,
                callgraph,
                index,
                next_scc_id,
                stack,
                on_stack,
                indices,
                lowlinks,
                scc_map,
            );

            // update the lowlink with the child lowlink
            let lowlink = lowlinks[&function_id].min(lowlinks[&callee]);
            lowlinks.insert(function_id, lowlink);
        } else if on_stack.contains(&callee) {
            let lowlink = lowlinks[&function_id].min(indices[&callee]);
            lowlinks.insert(function_id, lowlink);
        }
    }

    // finalize SCC if this node is a root
    if lowlinks[&function_id] == indices[&function_id] {
        // assign a new scc id
        let scc_id = *next_scc_id;
        *next_scc_id += 1;
        let mut scc_members = Vec::new();

        // pop the scc nodes from the stack
        loop {
            let Some(node) = stack.pop() else {
                break;
            };
            on_stack.remove(&node);
            scc_map.function_scc.insert(node, scc_id);
            scc_members.push(node);
            if node == function_id {
                break;
            }
        }

        // mark the scc as recursive when it has a cycle
        if scc_members.len() > 1 {
            scc_map.recursive_sccs.insert(scc_id);
        } else {
            let node = scc_members[0];

            // detect a self edge to mark recursion
            let self_edge = callgraph
                .outgoing(node)
                .iter()
                .any(|edge| edge.callee == node && edge.dispatch == mir::CallDispatchKind::Direct);
            if self_edge {
                scc_map.recursive_sccs.insert(scc_id);
            }
        }
    }
}

/// Package scoped call graph keyed by symbol names.
#[derive(Debug)]
pub struct PackageCallGraph {
    /// The underlying symbol call graph.
    graph: SymbolCallGraph,
}

impl PackageCallGraph {
    /// Get outgoing call edges for a symbol.
    pub fn outgoing(&self, caller: &SymbolName) -> &[SymbolCallEdge] {
        self.graph.outgoing(caller)
    }

    /// Get incoming call edges for a symbol.
    pub fn incoming(&self, callee: &SymbolName) -> &[SymbolCallEdge] {
        self.graph.incoming(callee)
    }

    /// Get unresolved callsites for a symbol.
    pub fn unknown_calls(&self, caller: &SymbolName) -> &[SymbolUnknownCallSite] {
        self.graph.unknown_calls(caller)
    }

    /// Return the sentinel symbol for unknown targets.
    pub fn external_symbol(&self) -> &SymbolName {
        self.graph.external_symbol()
    }

    /// Check whether a symbol is defined in this graph.
    pub fn is_defined(&self, symbol: &SymbolName) -> bool {
        self.graph.is_defined(symbol)
    }

    /// Check whether a symbol is ambiguous in this graph.
    pub fn is_ambiguous(&self, symbol: &SymbolName) -> bool {
        self.graph.is_ambiguous(symbol)
    }
}

impl Analysis for PackageCallGraph {
    const ID: AnalysisId = AnalysisId("package-callgraph");
}

impl PackageAnalysis for PackageCallGraph {
    /// Compute the package call graph from the workset modules.
    fn compute(workset: &PackageWorkset, _analyses: &PackageAnalyses<'_>) -> Self {
        let graph = build_symbol_call_graph(workset.modules());

        Self { graph }
    }
}

/// Program scoped call graph keyed by symbol names.
#[derive(Debug)]
pub struct ProgramCallGraph {
    /// The underlying symbol call graph.
    graph: SymbolCallGraph,
}

impl ProgramCallGraph {
    /// Get outgoing call edges for a symbol.
    pub fn outgoing(&self, caller: &SymbolName) -> &[SymbolCallEdge] {
        self.graph.outgoing(caller)
    }

    /// Get incoming call edges for a symbol.
    pub fn incoming(&self, callee: &SymbolName) -> &[SymbolCallEdge] {
        self.graph.incoming(callee)
    }

    /// Get unresolved callsites for a symbol.
    pub fn unknown_calls(&self, caller: &SymbolName) -> &[SymbolUnknownCallSite] {
        self.graph.unknown_calls(caller)
    }

    /// Return the sentinel symbol for unknown targets.
    pub fn external_symbol(&self) -> &SymbolName {
        self.graph.external_symbol()
    }

    /// Check whether a symbol is defined in this graph.
    pub fn is_defined(&self, symbol: &SymbolName) -> bool {
        self.graph.is_defined(symbol)
    }

    /// Check whether a symbol is ambiguous in this graph.
    pub fn is_ambiguous(&self, symbol: &SymbolName) -> bool {
        self.graph.is_ambiguous(symbol)
    }
}

impl Analysis for ProgramCallGraph {
    const ID: AnalysisId = AnalysisId("program-callgraph");
}

impl ProgramAnalysis for ProgramCallGraph {
    /// Compute the program call graph from the workset modules.
    fn compute(workset: &ProgramWorkset, _analyses: &ProgramAnalyses<'_>) -> Self {
        // collect modules across packages
        let modules: Vec<ModuleWorkItem> = workset
            .packages()
            .iter()
            .flat_map(|package| package.modules())
            .cloned()
            .collect();

        // build the shared call graph
        let graph = build_symbol_call_graph(&modules);

        Self { graph }
    }
}

/// Resolved callsite data for call graph construction.
struct CallSite {
    /// The caller function id.
    caller: mir::LocalNodeId<mir::Function>,
    /// The callsite reference.
    callsite: CallSiteRef,
    /// Dispatch kind for the callsite.
    dispatch: mir::CallDispatchKind,
    /// Resolved callee when known.
    callee: Option<mir::LocalNodeId<mir::Function>>,
    /// True when the dispatch is fully resolved.
    is_precise: bool,
}

/// Interns symbol names to reduce repeated allocation.
#[derive(Default)]
struct SymbolInterner {
    /// Map of interned symbols by string.
    names: HashMap<String, SymbolName>,
}

impl SymbolInterner {
    /// Intern a symbol name.
    fn intern(&mut self, name: &str) -> SymbolName {
        // reuse existing symbol entry
        if let Some(symbol) = self.names.get(name) {
            return symbol.clone();
        }

        // insert a new symbol entry
        let symbol = SymbolName::new(name);
        self.names.insert(name.to_string(), symbol.clone());
        symbol
    }
}

/// Build a symbol call graph over a set of module work items.
fn build_symbol_call_graph(modules: &[ModuleWorkItem]) -> SymbolCallGraph {
    // seed shared symbol state
    let mut interner = SymbolInterner::default();
    let mut definitions: HashMap<SymbolName, Vec<SymbolDefinition>> = HashMap::new();
    let mut export_counts: HashMap<SymbolName, usize> = HashMap::new();

    // collect definition metadata for all modules
    for module in modules {
        module.with_strings(|strings| {
            module.with_mir(|mir| {
                // scan the published mir artifact
                let tree = &mir.tree;

                // scan for defined functions
                for (_, function) in tree.iter_nodes::<mir::Function>() {
                    if function.entry.is_none() {
                        continue;
                    }

                    if !function.linkage.is_defined() {
                        continue;
                    }

                    let name = strings.get(function.name);
                    let symbol = interner.intern(name.as_ref());
                    let Some(signature) = SignatureKey::from_function(tree, function) else {
                        continue;
                    };

                    definitions
                        .entry(symbol.clone())
                        .or_default()
                        .push(SymbolDefinition {
                            linkage: function.linkage,
                            signature,
                        });

                    if function.linkage.is_exported() {
                        *export_counts.entry(symbol).or_insert(0) += 1;
                    }
                }
            });
        });
    }

    // compute ambiguous symbol set
    let ambiguous: HashSet<SymbolName> = export_counts
        .iter()
        .filter_map(|(symbol, count)| (*count > 1).then_some(symbol.clone()))
        .collect();

    let mut graph = SymbolCallGraph {
        outgoing: HashMap::new(),
        incoming: HashMap::new(),
        unknown: HashMap::new(),
        definitions,
        ambiguous,
        external_symbol: SymbolName::external(),
    };

    // collect call edges for each module
    for module in modules {
        // capture module id for callsite keys
        let module_id = module.module_id();

        module.with_strings(|strings| {
            module.with_mir(|mir| {
                // scan the published mir artifact
                let tree = &mir.tree;

                // build symbol names for all functions
                let mut symbols_by_function: HashMap<mir::LocalNodeId<mir::Function>, SymbolName> =
                    HashMap::new();

                for (function_id, function) in tree.iter_nodes::<mir::Function>() {
                    let name = strings.get(function.name);
                    let symbol = interner.intern(name.as_ref());
                    symbols_by_function.insert(function_id, symbol);
                }

                // walk call instructions for each function with a body
                for (function_id, function) in tree.iter_nodes::<mir::Function>() {
                    if function.entry.is_none() {
                        continue;
                    }

                    let Some(caller_symbol) = symbols_by_function.get(&function_id).cloned() else {
                        continue;
                    };

                    for block_id in &function.blocks {
                        let block = tree.get(*block_id);

                        for &instruction_id in &block.instructions {
                            let instruction = tree.get(instruction_id);
                            let Some(callsite) = SymbolCallSite::from_instruction(
                                module_id,
                                *block_id,
                                instruction_id,
                                instruction,
                                &symbols_by_function,
                                tree,
                            ) else {
                                continue;
                            };

                            insert_symbol_callsite(&mut graph, callsite, &caller_symbol);
                        }

                        let terminator = tree.get(block.terminator);
                        if let Some(callsite) = SymbolCallSite::from_terminator(
                            module_id,
                            *block_id,
                            terminator,
                            &symbols_by_function,
                            tree,
                        ) {
                            insert_symbol_callsite(&mut graph, callsite, &caller_symbol);
                        }
                    }
                }
            });
        });
    }

    graph
}

/// Resolved callsite data for symbol call graphs.
struct SymbolCallSite {
    /// The callsite id.
    callsite: CallSiteId,
    /// The dispatch kind.
    dispatch: mir::CallDispatchKind,
    /// The callee symbol name, when known.
    callee: Option<SymbolName>,
    /// The callee linkage, when known.
    callee_linkage: Option<mir::Linkage>,
    /// The callsite signature, when known.
    signature: Option<SignatureKey>,
    /// True when the dispatch is fully resolved.
    is_precise: bool,
}

impl SymbolCallSite {
    /// Build a symbol callsite from an instruction when it represents a call.
    fn from_instruction(
        module_id: ModuleId,
        block_id: mir::LocalNodeId<mir::Block>,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
        symbols_by_function: &HashMap<mir::LocalNodeId<mir::Function>, SymbolName>,
        tree: &mir::Tree,
    ) -> Option<Self> {
        // capture the callsite identity
        let callsite = CallSiteId {
            module: module_id,
            block: block_id,
            instruction: Some(instruction_id),
        };
        // resolve the dispatch kind
        let dispatch = instruction.call_dispatch_kind()?;

        // resolve the direct or analyzed target when present
        let resolved_target = instruction_resolved_target(instruction_id, instruction, tree);
        let callee = resolved_target.and_then(|target| symbols_by_function.get(&target).cloned());
        let callee_linkage = resolved_target.map(|target| tree.get(target).linkage);

        // resolve the signature type
        let signature = instruction
            .call_signature()
            .and_then(|sig| sig.ty())
            .and_then(|sig| SignatureKey::from_function_type(tree, sig))
            .or_else(|| {
                resolved_target
                    .and_then(|target| SignatureKey::from_function(tree, tree.get(target)))
            });

        let is_precise = matches!(dispatch, mir::CallDispatchKind::Direct);

        Some(Self {
            callsite,
            dispatch,
            callee,
            callee_linkage,
            signature,
            is_precise,
        })
    }

    /// Build a symbol callsite from a terminator when it represents a call.
    fn from_terminator(
        module_id: ModuleId,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
        symbols_by_function: &HashMap<mir::LocalNodeId<mir::Function>, SymbolName>,
        tree: &mir::Tree,
    ) -> Option<Self> {
        // capture the callsite identity
        let callsite = CallSiteId {
            module: module_id,
            block: block_id,
            instruction: None,
        };
        match terminator {
            mir::Terminator::Call { function, .. } => {
                let callee = function
                    .function()
                    .and_then(|function| symbols_by_function.get(&function).cloned());
                let callee_linkage = function
                    .function()
                    .map(|function| tree.get(function).linkage);
                let signature = function
                    .function()
                    .and_then(|function| SignatureKey::from_function(tree, tree.get(function)));

                Some(Self {
                    callsite,
                    dispatch: mir::CallDispatchKind::Direct,
                    callee,
                    callee_linkage,
                    signature,
                    is_precise: true,
                })
            }
            mir::Terminator::CallIndirect { call, .. } => Some(Self {
                callsite,
                dispatch: mir::CallDispatchKind::Indirect,
                callee: None,
                callee_linkage: None,
                signature: SignatureKey::from_function_type(tree, call.signature),
                is_precise: false,
            }),
            mir::Terminator::CallClass { slot, call, .. } => {
                let resolved_target = terminator_resolved_target(block_id, terminator, tree);
                let callee =
                    resolved_target.and_then(|target| symbols_by_function.get(&target).cloned());
                let callee_linkage = resolved_target.map(|target| tree.get(target).linkage);
                let signature =
                    SignatureKey::from_function_type(tree, call.signature).or_else(|| {
                        resolved_target
                            .and_then(|target| SignatureKey::from_function(tree, tree.get(target)))
                    });

                Some(Self {
                    callsite,
                    dispatch: mir::CallDispatchKind::Class { slot: *slot },
                    callee,
                    callee_linkage,
                    signature,
                    is_precise: false,
                })
            }
            mir::Terminator::CallInterface { slot, call, .. } => {
                let resolved_target = terminator_resolved_target(block_id, terminator, tree);
                let callee =
                    resolved_target.and_then(|target| symbols_by_function.get(&target).cloned());
                let callee_linkage = resolved_target.map(|target| tree.get(target).linkage);
                let signature =
                    SignatureKey::from_function_type(tree, call.signature).or_else(|| {
                        resolved_target
                            .and_then(|target| SignatureKey::from_function(tree, tree.get(target)))
                    });

                Some(Self {
                    callsite,
                    dispatch: mir::CallDispatchKind::Interface { slot: *slot },
                    callee,
                    callee_linkage,
                    signature,
                    is_precise: false,
                })
            }
            mir::Terminator::TailCall { function, .. } => {
                let callee = function
                    .function()
                    .and_then(|function| symbols_by_function.get(&function).cloned());
                let callee_linkage = function
                    .function()
                    .map(|function| tree.get(function).linkage);
                let signature = function
                    .function()
                    .and_then(|function| SignatureKey::from_function(tree, tree.get(function)));

                Some(Self {
                    callsite,
                    dispatch: mir::CallDispatchKind::Direct,
                    callee,
                    callee_linkage,
                    signature,
                    is_precise: true,
                })
            }
            mir::Terminator::TailCallIndirect { call, .. } => Some(Self {
                callsite,
                dispatch: mir::CallDispatchKind::Indirect,
                callee: None,
                callee_linkage: None,
                signature: SignatureKey::from_function_type(tree, call.signature),
                is_precise: false,
            }),
            mir::Terminator::TailCallClass { slot, call, .. } => {
                let resolved_target = terminator_resolved_target(block_id, terminator, tree);
                let callee =
                    resolved_target.and_then(|target| symbols_by_function.get(&target).cloned());
                let callee_linkage = resolved_target.map(|target| tree.get(target).linkage);
                let signature =
                    SignatureKey::from_function_type(tree, call.signature).or_else(|| {
                        resolved_target
                            .and_then(|target| SignatureKey::from_function(tree, tree.get(target)))
                    });

                Some(Self {
                    callsite,
                    dispatch: mir::CallDispatchKind::Class { slot: *slot },
                    callee,
                    callee_linkage,
                    signature,
                    is_precise: false,
                })
            }
            mir::Terminator::TailCallInterface { slot, call, .. } => {
                let resolved_target = terminator_resolved_target(block_id, terminator, tree);
                let callee =
                    resolved_target.and_then(|target| symbols_by_function.get(&target).cloned());
                let callee_linkage = resolved_target.map(|target| tree.get(target).linkage);
                let signature =
                    SignatureKey::from_function_type(tree, call.signature).or_else(|| {
                        resolved_target
                            .and_then(|target| SignatureKey::from_function(tree, tree.get(target)))
                    });

                Some(Self {
                    callsite,
                    dispatch: mir::CallDispatchKind::Interface { slot: *slot },
                    callee,
                    callee_linkage,
                    signature,
                    is_precise: false,
                })
            }
            _ => None,
        }
    }
}

/// Return the resolved function target for an instruction callsite.
fn instruction_resolved_target(
    instruction_id: mir::LocalNodeId<mir::Instruction>,
    instruction: &mir::Instruction,
    tree: &mir::Tree,
) -> Option<mir::LocalNodeId<mir::Function>> {
    instruction
        .call_direct_target()
        .and_then(|target| target.function())
        .or_else(|| {
            tree.metadata
                .functions
                .call(mir::CallSite::Instruction(instruction_id))
                .and_then(|metadata| metadata.target)
        })
}

/// Return the resolved function target for a terminator callsite.
fn terminator_resolved_target(
    block_id: mir::LocalNodeId<mir::Block>,
    terminator: &mir::Terminator,
    tree: &mir::Tree,
) -> Option<mir::LocalNodeId<mir::Function>> {
    terminator
        .call_direct_target()
        .and_then(|target| target.function())
        .or_else(|| {
            tree.metadata
                .functions
                .call(mir::CallSite::Terminator(block_id))
                .and_then(|metadata| metadata.target)
        })
}

/// Insert a symbol callsite into a graph.
fn insert_symbol_callsite(
    graph: &mut SymbolCallGraph,
    callsite: SymbolCallSite,
    caller_symbol: &SymbolName,
) {
    // resolve the callee symbol
    let Some(callee) = callsite.callee.clone() else {
        insert_unknown_symbol_call(graph, callsite, caller_symbol, None);
        return;
    };

    // direct calls to local or exported definitions are always resolved
    if matches!(
        callsite.callee_linkage,
        Some(mir::Linkage::Local | mir::Linkage::Export)
    ) {
        insert_symbol_edge(graph, caller_symbol, &callee, &callsite);

        if !callsite.is_precise {
            insert_unknown_symbol_call(graph, callsite, caller_symbol, Some(callee));
        }

        return;
    }

    // resolve imported symbols against exported definitions
    if matches!(callsite.callee_linkage, Some(mir::Linkage::Import)) {
        let resolution = resolve_exported_target(graph, &callee, callsite.signature.as_ref());

        if resolution {
            insert_symbol_edge(graph, caller_symbol, &callee, &callsite);

            if !callsite.is_precise {
                insert_unknown_symbol_call(graph, callsite, caller_symbol, Some(callee));
            }
        } else {
            insert_unknown_symbol_call(graph, callsite, caller_symbol, Some(callee));
        }

        return;
    }

    // fall back to unresolved when linkage is missing
    insert_unknown_symbol_call(graph, callsite, caller_symbol, Some(callee));
}

/// Insert a resolved symbol edge into the graph.
fn insert_symbol_edge(
    graph: &mut SymbolCallGraph,
    caller_symbol: &SymbolName,
    callee: &SymbolName,
    callsite: &SymbolCallSite,
) {
    // build the resolved edge
    let edge = SymbolCallEdge {
        caller: caller_symbol.clone(),
        callee: callee.clone(),
        callsite: callsite.callsite,
        dispatch: callsite.dispatch,
    };

    // store the edge in adjacency maps
    graph
        .outgoing
        .entry(caller_symbol.clone())
        .or_default()
        .push(edge.clone());
    graph.incoming.entry(callee.clone()).or_default().push(edge);
}

/// Insert an unresolved callsite into the graph.
fn insert_unknown_symbol_call(
    graph: &mut SymbolCallGraph,
    callsite: SymbolCallSite,
    caller_symbol: &SymbolName,
    callee: Option<SymbolName>,
) {
    // record the unresolved callsite
    let unknown = SymbolUnknownCallSite {
        caller: caller_symbol.clone(),
        callsite: callsite.callsite,
        dispatch: callsite.dispatch,
        callee: callee.clone(),
    };

    graph
        .unknown
        .entry(caller_symbol.clone())
        .or_default()
        .push(unknown);

    // attach the callsite to the external sentinel
    let edge = SymbolCallEdge {
        caller: caller_symbol.clone(),
        callee: graph.external_symbol.clone(),
        callsite: callsite.callsite,
        dispatch: callsite.dispatch,
    };

    graph
        .outgoing
        .entry(caller_symbol.clone())
        .or_default()
        .push(edge.clone());
    graph
        .incoming
        .entry(graph.external_symbol.clone())
        .or_default()
        .push(edge);
}

/// Resolve whether an imported symbol can bind to a unique export.
fn resolve_exported_target(
    graph: &SymbolCallGraph,
    symbol: &SymbolName,
    signature: Option<&SignatureKey>,
) -> bool {
    // reject ambiguous exports
    if graph.is_ambiguous(symbol) {
        return false;
    }

    // fetch known definitions
    let Some(definitions) = graph.definitions.get(symbol) else {
        return false;
    };

    // filter to exported definitions
    let exported: Vec<&SymbolDefinition> = definitions
        .iter()
        .filter(|definition| definition.linkage.is_exported())
        .collect();

    if exported.is_empty() {
        return false;
    }

    // allow a single export without signature evidence
    let Some(signature) = signature else {
        return exported.len() == 1;
    };

    // require a single matching signature
    exported
        .iter()
        .filter(|definition| definition.signature == *signature)
        .count()
        == 1
}

impl CallSite {
    /// Build a callsite from an instruction when it represents a call.
    fn from_instruction(
        caller: mir::LocalNodeId<mir::Function>,
        instruction_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
        tree: &mir::Tree,
    ) -> Option<Self> {
        let dispatch = instruction.call_dispatch_kind()?;
        let callee = instruction_resolved_target(instruction_id, instruction, tree);
        let is_precise = matches!(dispatch, mir::CallDispatchKind::Direct);

        Some(Self {
            caller,
            callsite: CallSiteRef::Instruction(instruction_id),
            dispatch,
            callee,
            is_precise,
        })
    }

    /// Build a callsite from a terminator when it represents a call.
    fn from_terminator(
        caller: mir::LocalNodeId<mir::Function>,
        block_id: mir::LocalNodeId<mir::Block>,
        terminator: &mir::Terminator,
        tree: &mir::Tree,
    ) -> Option<Self> {
        let callsite = CallSiteRef::Terminator(block_id);

        match terminator {
            mir::Terminator::Call { function, .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Direct,
                callee: function.function(),
                is_precise: true,
            }),
            mir::Terminator::CallIndirect { .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Indirect,
                callee: None,
                is_precise: false,
            }),
            mir::Terminator::CallClass { slot, .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Class { slot: *slot },
                callee: terminator_resolved_target(block_id, terminator, tree),
                is_precise: false,
            }),
            mir::Terminator::CallInterface { slot, .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Interface { slot: *slot },
                callee: terminator_resolved_target(block_id, terminator, tree),
                is_precise: false,
            }),
            mir::Terminator::TailCall { function, .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Direct,
                callee: function.function(),
                is_precise: true,
            }),
            mir::Terminator::TailCallIndirect { .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Indirect,
                callee: None,
                is_precise: false,
            }),
            mir::Terminator::TailCallClass { slot, .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Class { slot: *slot },
                callee: terminator_resolved_target(block_id, terminator, tree),
                is_precise: false,
            }),
            mir::Terminator::TailCallInterface { slot, .. } => Some(Self {
                caller,
                callsite,
                dispatch: mir::CallDispatchKind::Interface { slot: *slot },
                callee: terminator_resolved_target(block_id, terminator, tree),
                is_precise: false,
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_artifact::MirLowered;
    use destack_source::{FileId, ModuleId, PackageId, ProfileId, TargetId};

    use crate::common::mir::{ModuleAnalyses, PackageAnalyses, ProgramAnalyses};
    use crate::optimize::common::tests::TestProgram;
    use crate::optimize::{
        ModuleWorkItem, OptimizationLevel, PackageWorkset, PipelineOptions, ProgramWorkset,
    };

    use super::*;

    /// Build one stable test target id for one package.
    fn test_target_id(package_id: PackageId, name: &str) -> TargetId {
        TargetId::new(package_id, name)
    }

    /// Build one stable test profile id for one package.
    fn test_profile_id() -> ProfileId {
        ProfileId::new(0)
    }

    /// Build a module work item from MIR text.
    fn module_work_item(package_id: PackageId, module_index: u32, source: &str) -> ModuleWorkItem {
        let module_id = ModuleId::new(package_id, u128::from(module_index));
        let target_id = test_target_id(package_id, "test");
        let (tree, strings) =
            mir::parse::Parser::parse(FileId::new(0), source, mir::parse::ParseOptions::default())
                .finish()
                .expect("failed to parse MIR");
        let pool = Arc::new(destack_core::StringPool::new());
        pool.ensure_all_from(&strings);

        let mut module_mir = MirLowered::new();
        module_mir.tree = tree;

        ModuleWorkItem::new(
            module_id,
            test_profile_id(),
            target_id,
            module_mir,
            pool,
            PipelineOptions::default(),
        )
    }

    /// Direct calls create edges in the call graph.
    #[test]
    fn test_call_graph_direct_call() {
        let test = TestProgram::new(
            r#"
function callee(): int32 {
b0:
    v0: int32 = 7int32
    return v0
}
function test(): int32 {
b0:
    v0: int32 = call callee(): () -> int32
    return v0
}"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");

        let analyses = ModuleAnalyses::new(&test.tree);
        let callgraph = analyses.get::<CallGraph>();

        let outgoing = callgraph.outgoing(test_id);
        let incoming = callgraph.incoming(callee_id);

        assert_eq!(outgoing.len(), 1);
        assert_eq!(incoming.len(), 1);
        assert!(outgoing[0].is_direct());
        assert_eq!(outgoing[0].callee, callee_id);
        assert_eq!(incoming[0].caller, test_id);
        assert!(callgraph.unknown_calls(test_id).is_empty());
    }

    /// Call graph SCCs detect recursive functions.
    #[test]
    fn test_call_graph_scc_recursion() {
        let test = TestProgram::new(
            r#"
function alpha(): void {
b0:
    call beta(): () -> void
    return
}
function beta(): void {
b0:
    call alpha(): () -> void
    return
}
function gamma(): void {
b0:
    call gamma(): () -> void
    return
}
function delta(): void {
b0:
    return
}"#,
        );

        let a_id = test.function_id_by_name("alpha");
        let b_id = test.function_id_by_name("beta");
        let c_id = test.function_id_by_name("gamma");
        let d_id = test.function_id_by_name("delta");

        let analyses = ModuleAnalyses::new(&test.tree);
        let scc = analyses.get::<CallGraphScc>();

        assert!(scc.is_recursive_function(a_id));
        assert!(scc.is_recursive_function(b_id));
        assert!(scc.is_recursive_function(c_id));
        assert!(!scc.is_recursive_function(d_id));
    }

    /// Indirect calls without metadata remain unresolved.
    #[test]
    fn test_call_graph_indirect_unknown() {
        let test = TestProgram::new(
            r#"
function test(v0: (int32) -> int32, v1: int32): int32  {
b0(v0: (int32) -> int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) -> int32
    return v2
}"#,
        );

        let test_id = test.function_id_by_name("test");

        let analyses = ModuleAnalyses::new(&test.tree);
        let callgraph = analyses.get::<CallGraph>();

        assert!(callgraph.outgoing(test_id).is_empty());
        assert_eq!(callgraph.unknown_calls(test_id).len(), 1);
        assert_eq!(
            callgraph.unknown_calls(test_id)[0].dispatch,
            mir::CallDispatchKind::Indirect
        );
    }

    /// Tail calls are tracked as call edges.
    #[test]
    fn test_call_graph_tailcall_direct() {
        let test = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function test(v0: int32): int32 {
b0(v0: int32):
    tailCall callee(v0): (int32) -> int32
}"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");

        let analyses = ModuleAnalyses::new(&test.tree);
        let callgraph = analyses.get::<CallGraph>();

        let outgoing = callgraph.outgoing(test_id);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callee_id);
        assert!(matches!(outgoing[0].callsite, CallSiteRef::Terminator(_)));
    }

    /// Tailcall.indirect remains unresolved without metadata.
    #[test]
    fn test_call_graph_tailcall_indirect_unknown() {
        let test = TestProgram::new(
            r#"
function test(v0: (int32) -> int32, v1: int32): int32 {
b0(v0: (int32) -> int32, v1: int32):
    tailCall.indirect v0(v1): (int32) -> int32
}"#,
        );

        let test_id = test.function_id_by_name("test");

        let analyses = ModuleAnalyses::new(&test.tree);
        let callgraph = analyses.get::<CallGraph>();

        let unknown = callgraph.unknown_calls(test_id);
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].dispatch, mir::CallDispatchKind::Indirect);
        assert!(matches!(unknown[0].callsite, CallSiteRef::Terminator(_)));
    }

    /// Call.indirect remains unresolved without a declared target.
    #[test]
    fn test_call_graph_call_indirect_unknown() {
        let test = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}

function test(v0: (int32) -> int32, v1: int32): int32  {
b0(v0: (int32) -> int32, v1: int32):
    v2: int32 = call.indirect v0(v1): (int32) -> int32
    return v2
}"#,
        );

        let test_id = test.function_id_by_name("test");

        let analyses = ModuleAnalyses::new(&test.tree);
        let callgraph = analyses.get::<CallGraph>();

        let unknown = callgraph.unknown_calls(test_id);
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].dispatch, mir::CallDispatchKind::Indirect);
    }

    /// Direct call terminators produce precise call edges.
    #[test]
    fn test_call_graph_call_terminator_direct() {
        let test = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function test(v0: int32): int32 {
b0(v0: int32):
    call callee(v0): (int32) -> int32 -> b1
b1(v1: int32):
    return v1
b2(v2: ref<int32, managed, readonly>):
    panic v2
}"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");

        let analyses = ModuleAnalyses::new(&test.tree);
        let callgraph = analyses.get::<CallGraph>();

        let outgoing = callgraph.outgoing(test_id);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callee_id);
        assert_eq!(outgoing[0].dispatch, mir::CallDispatchKind::Direct);
        assert!(matches!(outgoing[0].callsite, CallSiteRef::Terminator(_)));
        assert!(callgraph.unknown_calls(test_id).is_empty());
    }

    /// Class dispatch keeps a call edge and records an unknown target.
    #[test]
    fn test_call_graph_virtual_dispatch_is_partial() {
        let mut test = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function test(v0: int32): int32 {
b0(v0: int32):
    v1: int32 = call.class v0, int32, 1(v0): (int32) -> int32
    return v1
}"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");

        let function = test.tree.get(test_id);
        let mut call_id = None;
        for &block_id in &function.blocks {
            let block = test.tree.get(block_id);
            for &instruction_id in &block.instructions {
                if matches!(
                    test.tree.get(instruction_id),
                    mir::Instruction::CallClass { .. }
                ) {
                    call_id = Some(instruction_id);
                    break;
                }
            }
            if call_id.is_some() {
                break;
            }
        }
        let call_id = call_id.expect("missing class call instruction");

        test.tree
            .metadata
            .functions
            .call_mut(mir::CallSite::Instruction(call_id))
            .target = Some(callee_id);

        let analyses = ModuleAnalyses::new(&test.tree);
        let callgraph = analyses.get::<CallGraph>();

        assert_eq!(callgraph.outgoing(test_id).len(), 1);
        assert_eq!(callgraph.outgoing(test_id)[0].callee, callee_id);
        assert_eq!(
            callgraph.outgoing(test_id)[0].dispatch,
            mir::CallDispatchKind::Class {
                slot: mir::DispatchSlot::new(1),
            }
        );
        assert_eq!(callgraph.unknown_calls(test_id).len(), 1);
        assert_eq!(callgraph.unknown_calls(test_id)[0].callee, Some(callee_id));
    }

    /// Class call terminators keep the declared target and unknown edge.
    #[test]
    fn test_call_graph_call_class_terminator_is_partial() {
        let mut test = TestProgram::new(
            r#"
function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}
function test(v0: int32): int32 {
b0(v0: int32):
    call.class v0, int32, 1(v0): (int32) -> int32 -> b1
b1(v1: int32):
    return v1
b2(v2: ref<int32, managed, readonly>):
    panic v2
}"#,
        );

        let callee_id = test.function_id_by_name("callee");
        let test_id = test.function_id_by_name("test");
        let function = test.tree.get(test_id);
        let block_id = *function.blocks.first().expect("missing entry block");

        test.tree
            .metadata
            .functions
            .call_mut(mir::CallSite::Terminator(block_id))
            .target = Some(callee_id);

        let analyses = ModuleAnalyses::new(&test.tree);
        let callgraph = analyses.get::<CallGraph>();

        let outgoing = callgraph.outgoing(test_id);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callee_id);
        assert_eq!(
            outgoing[0].dispatch,
            mir::CallDispatchKind::Class {
                slot: mir::DispatchSlot::new(1),
            }
        );
        assert!(matches!(outgoing[0].callsite, CallSiteRef::Terminator(_)));
        assert_eq!(callgraph.unknown_calls(test_id).len(), 1);
        assert_eq!(callgraph.unknown_calls(test_id)[0].callee, Some(callee_id));
    }

    /// Package call graphs resolve imported functions.
    #[test]
    fn test_package_call_graph_resolves_import() {
        let package_id = PackageId::new(1);
        let target_id = test_target_id(package_id, "test");

        let module_a = module_work_item(
            package_id,
            0,
            r#"
export function callee(): int32 {
b0:
    v0: int32 = 3int32
    return v0
}"#,
        );

        let module_b = module_work_item(
            package_id,
            1,
            r#"
external function callee(): int32
function test(): int32 {
b0:
    v0: int32 = call callee(): () -> int32
    return v0
}"#,
        );

        let mut workset = PackageWorkset::new(package_id, target_id, OptimizationLevel::O2);
        workset.add_module(module_a);
        workset.add_module(module_b);

        let analyses = PackageAnalyses::new(&workset);
        let callgraph = analyses.get::<PackageCallGraph>();

        let caller = SymbolName::new("test");
        let callee = SymbolName::new("callee");

        let outgoing = callgraph.outgoing(&caller);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callee);
        assert!(callgraph.is_defined(&callee));
        assert!(callgraph.unknown_calls(&caller).is_empty());
    }

    /// Imported calls with missing exports remain unresolved.
    #[test]
    fn test_package_call_graph_import_missing_definition() {
        let package_id = PackageId::new(7);
        let target_id = test_target_id(package_id, "test");

        let module = module_work_item(
            package_id,
            0,
            r#"
external function callee(): int32
function test(): int32 {
b0:
    v0: int32 = call callee(): () -> int32
    return v0
}"#,
        );

        let mut workset = PackageWorkset::new(package_id, target_id, OptimizationLevel::O2);
        workset.add_module(module);

        let analyses = PackageAnalyses::new(&workset);
        let callgraph = analyses.get::<PackageCallGraph>();

        let caller = SymbolName::new("test");
        let callee = SymbolName::new("callee");
        let external = callgraph.external_symbol();

        let outgoing = callgraph.outgoing(&caller);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, external.clone());
        assert_eq!(callgraph.unknown_calls(&caller).len(), 1);
        assert_eq!(callgraph.unknown_calls(&caller)[0].callee, Some(callee));
        assert_eq!(callgraph.incoming(external).len(), 1);
    }

    /// Imports do not resolve against local-only definitions.
    #[test]
    fn test_package_call_graph_skips_local_definition() {
        let package_id = PackageId::new(8);
        let target_id = test_target_id(package_id, "test");

        let module_a = module_work_item(
            package_id,
            0,
            r#"
function callee(): int32 {
b0:
    v0: int32 = 1int32
    return v0
}"#,
        );

        let module_b = module_work_item(
            package_id,
            1,
            r#"
external function callee(): int32
function test(): int32 {
b0:
    v0: int32 = call callee(): () -> int32
    return v0
}"#,
        );

        let mut workset = PackageWorkset::new(package_id, target_id, OptimizationLevel::O2);
        workset.add_module(module_a);
        workset.add_module(module_b);

        let analyses = PackageAnalyses::new(&workset);
        let callgraph = analyses.get::<PackageCallGraph>();

        let caller = SymbolName::new("test");
        let callee = SymbolName::new("callee");

        let outgoing = callgraph.outgoing(&caller);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callgraph.external_symbol().clone());
        assert_eq!(callgraph.unknown_calls(&caller).len(), 1);
        assert_eq!(callgraph.unknown_calls(&caller)[0].callee, Some(callee));
    }

    /// Imports with mismatched signatures remain unresolved.
    #[test]
    fn test_package_call_graph_signature_mismatch() {
        let package_id = PackageId::new(9);
        let target_id = test_target_id(package_id, "test");

        let module_a = module_work_item(
            package_id,
            0,
            r#"
export function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}"#,
        );

        let module_b = module_work_item(
            package_id,
            1,
            r#"
external function callee(int64): int64
function test(v0: int64): int64 {
b0(v0: int64):
    v1: int64 = call callee(v0): (int64) -> int64
    return v1
}"#,
        );

        let mut workset = PackageWorkset::new(package_id, target_id, OptimizationLevel::O2);
        workset.add_module(module_a);
        workset.add_module(module_b);

        let analyses = PackageAnalyses::new(&workset);
        let callgraph = analyses.get::<PackageCallGraph>();

        let caller = SymbolName::new("test");
        let callee = SymbolName::new("callee");

        let outgoing = callgraph.outgoing(&caller);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callgraph.external_symbol().clone());
        assert_eq!(callgraph.unknown_calls(&caller).len(), 1);
        assert_eq!(callgraph.unknown_calls(&caller)[0].callee, Some(callee));
    }

    /// Program call graphs resolve across packages when symbols are unique.
    #[test]
    fn test_program_call_graph_resolves_cross_package() {
        let caller_pkg = PackageId::new(2);
        let callee_pkg = PackageId::new(3);
        let caller_target = test_target_id(caller_pkg, "test");
        let callee_target = test_target_id(callee_pkg, "test");

        let caller_module = module_work_item(
            caller_pkg,
            0,
            r#"
external function callee(): int32
function test(): int32 {
b0:
    v0: int32 = call callee(): () -> int32
    return v0
}"#,
        );

        let callee_module = module_work_item(
            callee_pkg,
            0,
            r#"
export function callee(): int32 {
b0:
    v0: int32 = 9int32
    return v0
}"#,
        );

        let mut caller_workset =
            PackageWorkset::new(caller_pkg, caller_target, OptimizationLevel::O2);
        caller_workset.add_module(caller_module);

        let mut callee_workset =
            PackageWorkset::new(callee_pkg, callee_target, OptimizationLevel::O2);
        callee_workset.add_module(callee_module);

        let mut program = ProgramWorkset::new();
        program.add_package(caller_workset);
        program.add_package(callee_workset);

        let analyses = ProgramAnalyses::new(&program);
        let callgraph = analyses.get::<ProgramCallGraph>();

        let caller = SymbolName::new("test");
        let callee = SymbolName::new("callee");

        let outgoing = callgraph.outgoing(&caller);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callee);
        assert!(callgraph.is_defined(&callee));
    }

    /// Ambiguous symbols are left unresolved in the program call graph.
    #[test]
    fn test_program_call_graph_ambiguous_symbol_is_unknown() {
        let caller_pkg = PackageId::new(4);
        let callee_pkg_a = PackageId::new(5);
        let callee_pkg_b = PackageId::new(6);
        let caller_target = test_target_id(caller_pkg, "test");
        let target_a = test_target_id(callee_pkg_a, "test");
        let target_b = test_target_id(callee_pkg_b, "test");

        let caller_module = module_work_item(
            caller_pkg,
            0,
            r#"
external function callee(): int32
function test(): int32 {
b0:
    v0: int32 = call callee(): () -> int32
    return v0
}"#,
        );

        let callee_module_a = module_work_item(
            callee_pkg_a,
            0,
            r#"
export function callee(): int32 {
b0:
    v0: int32 = 1int32
    return v0
}"#,
        );

        let callee_module_b = module_work_item(
            callee_pkg_b,
            0,
            r#"
export function callee(): int32 {
b0:
    v0: int32 = 2int32
    return v0
}"#,
        );

        let mut caller_workset =
            PackageWorkset::new(caller_pkg, caller_target, OptimizationLevel::O2);
        caller_workset.add_module(caller_module);

        let mut callee_workset_a =
            PackageWorkset::new(callee_pkg_a, target_a, OptimizationLevel::O2);
        callee_workset_a.add_module(callee_module_a);

        let mut callee_workset_b =
            PackageWorkset::new(callee_pkg_b, target_b, OptimizationLevel::O2);
        callee_workset_b.add_module(callee_module_b);

        let mut program = ProgramWorkset::new();
        program.add_package(caller_workset);
        program.add_package(callee_workset_a);
        program.add_package(callee_workset_b);

        let analyses = ProgramAnalyses::new(&program);
        let callgraph = analyses.get::<ProgramCallGraph>();

        let caller = SymbolName::new("test");
        let callee = SymbolName::new("callee");

        let outgoing = callgraph.outgoing(&caller);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callgraph.external_symbol().clone());
        assert!(callgraph.is_ambiguous(&callee));
        assert_eq!(callgraph.unknown_calls(&caller).len(), 1);
    }
}

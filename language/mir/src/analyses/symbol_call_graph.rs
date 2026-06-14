use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use destack_core::StringPool;
use destack_source::ModuleId;

use crate as mir;
use crate::{instruction_resolved_target, terminator_resolved_target};

/// One module's MIR and strings, the per-module input to whole-program analyses.
pub struct ModuleRef<'a> {
    /// The module's identifier.
    pub module_id: ModuleId,
    /// The module's MIR tree.
    pub tree: &'a mir::Tree,
    /// The module's string pool, for resolving symbol names.
    pub strings: &'a StringPool,
}

impl std::fmt::Debug for ModuleRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleRef")
            .field("module_id", &self.module_id)
            .finish_non_exhaustive()
    }
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
    fn from_function_type(tree: &mir::Tree, signature: &mir::TypeReference) -> Option<Self> {
        let signature = signature.ty()?;

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
    /// Erased dynamic value signature.
    Dynamic {
        /// Dynamic constraint type signature.
        constraint: Box<SignatureType>,
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
            mir::Type::Dynamic { constraint } => SignatureType::Dynamic {
                constraint: Box::new(SignatureType::from_type(tree, constraint.ty()?)?),
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
impl SymbolCallGraph {
    /// Build the cross-module call graph over a set of modules.
    pub fn build(modules: &[ModuleRef<'_>]) -> Self {
        // seed shared symbol state
        let mut interner = SymbolInterner::default();
        let mut definitions: HashMap<SymbolName, Vec<SymbolDefinition>> = HashMap::new();
        let mut export_counts: HashMap<SymbolName, usize> = HashMap::new();

        // collect definition metadata for all modules
        for module in modules {
            let strings = module.strings;
            let tree = module.tree;

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
            let module_id = module.module_id;

            let strings = module.strings;
            let tree = module.tree;

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
        }

        graph
    }
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
            .and_then(|sig| SignatureKey::from_function_type(tree, &sig))
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
                signature: SignatureKey::from_function_type(tree, &call.signature),
                is_precise: false,
            }),
            mir::Terminator::CallVirtual { slot, call, .. } => {
                let resolved_target = terminator_resolved_target(block_id, terminator, tree);
                let callee =
                    resolved_target.and_then(|target| symbols_by_function.get(&target).cloned());
                let callee_linkage = resolved_target.map(|target| tree.get(target).linkage);
                let signature =
                    SignatureKey::from_function_type(tree, &call.signature).or_else(|| {
                        resolved_target
                            .and_then(|target| SignatureKey::from_function(tree, tree.get(target)))
                    });

                Some(Self {
                    callsite,
                    dispatch: mir::CallDispatchKind::Virtual { slot: *slot },
                    callee,
                    callee_linkage,
                    signature,
                    is_precise: false,
                })
            }
            mir::Terminator::CallDynamic { slot, call, .. } => {
                let resolved_target = terminator_resolved_target(block_id, terminator, tree);
                let callee =
                    resolved_target.and_then(|target| symbols_by_function.get(&target).cloned());
                let callee_linkage = resolved_target.map(|target| tree.get(target).linkage);
                let signature =
                    SignatureKey::from_function_type(tree, &call.signature).or_else(|| {
                        resolved_target
                            .and_then(|target| SignatureKey::from_function(tree, tree.get(target)))
                    });

                Some(Self {
                    callsite,
                    dispatch: mir::CallDispatchKind::Dynamic { slot: *slot },
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
                signature: SignatureKey::from_function_type(tree, &call.signature),
                is_precise: false,
            }),
            mir::Terminator::TailCallVirtual { slot, call, .. } => {
                let resolved_target = terminator_resolved_target(block_id, terminator, tree);
                let callee =
                    resolved_target.and_then(|target| symbols_by_function.get(&target).cloned());
                let callee_linkage = resolved_target.map(|target| tree.get(target).linkage);
                let signature =
                    SignatureKey::from_function_type(tree, &call.signature).or_else(|| {
                        resolved_target
                            .and_then(|target| SignatureKey::from_function(tree, tree.get(target)))
                    });

                Some(Self {
                    callsite,
                    dispatch: mir::CallDispatchKind::Virtual { slot: *slot },
                    callee,
                    callee_linkage,
                    signature,
                    is_precise: false,
                })
            }
            mir::Terminator::TailCallDynamic { slot, call, .. } => {
                let resolved_target = terminator_resolved_target(block_id, terminator, tree);
                let callee =
                    resolved_target.and_then(|target| symbols_by_function.get(&target).cloned());
                let callee_linkage = resolved_target.map(|target| tree.get(target).linkage);
                let signature =
                    SignatureKey::from_function_type(tree, &call.signature).or_else(|| {
                        resolved_target
                            .and_then(|target| SignatureKey::from_function(tree, tree.get(target)))
                    });

                Some(Self {
                    callsite,
                    dispatch: mir::CallDispatchKind::Dynamic { slot: *slot },
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

#[cfg(test)]
mod tests {
    use destack_source::{FileId, ModuleId, PackageId};

    use super::*;
    use crate::parse::{ParseOptions, Parser};

    /// One parsed module owning its tree and strings, viewable as a `ModuleRef`.
    struct ParsedModule {
        module_id: ModuleId,
        tree: mir::Tree,
        strings: StringPool,
    }

    impl ParsedModule {
        fn module_ref(&self) -> ModuleRef<'_> {
            ModuleRef {
                module_id: self.module_id,
                tree: &self.tree,
                strings: &self.strings,
            }
        }
    }

    /// Parse one module from MIR text.
    fn parse_module(package_id: PackageId, module_index: u32, source: &str) -> ParsedModule {
        let module_id = ModuleId::new(package_id, u128::from(module_index));
        let (tree, strings) = Parser::parse(FileId::new(0), source, ParseOptions::default())
            .finish()
            .expect("failed to parse MIR");

        ParsedModule {
            module_id,
            tree,
            strings,
        }
    }

    /// Build the cross-module call graph over a set of parsed modules.
    fn call_graph(modules: &[ParsedModule]) -> SymbolCallGraph {
        let refs: Vec<ModuleRef<'_>> = modules.iter().map(ParsedModule::module_ref).collect();
        SymbolCallGraph::build(&refs)
    }

    /// Imported calls resolve to a unique exported definition.
    #[test]
    fn test_call_graph_resolves_import() {
        let package_id = PackageId::new(1);
        let module_a = parse_module(
            package_id,
            0,
            r#"
export function callee(): int32 {
b0:
    v0: int32 = 3int32
    return v0
}"#,
        );
        let module_b = parse_module(
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

        let callgraph = call_graph(&[module_a, module_b]);

        let caller = SymbolName::new("test");
        let callee = SymbolName::new("callee");

        // the call resolves to the exported definition
        let outgoing = callgraph.outgoing(&caller);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callee);
        assert!(callgraph.is_defined(&callee));
        assert!(callgraph.unknown_calls(&caller).is_empty());
    }

    /// Imported calls with no matching export remain unresolved.
    #[test]
    fn test_call_graph_unresolved_missing_export() {
        let package_id = PackageId::new(7);
        let module = parse_module(
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

        let callgraph = call_graph(&[module]);

        let caller = SymbolName::new("test");
        let callee = SymbolName::new("callee");
        let external = callgraph.external_symbol();

        // the call points at the external sentinel and is recorded as unknown
        let outgoing = callgraph.outgoing(&caller);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, external.clone());
        assert_eq!(callgraph.unknown_calls(&caller).len(), 1);
        assert_eq!(callgraph.unknown_calls(&caller)[0].callee, Some(callee));
        assert_eq!(callgraph.incoming(external).len(), 1);
    }

    /// Imports do not resolve against local-only definitions.
    #[test]
    fn test_call_graph_skips_local_definition() {
        let package_id = PackageId::new(8);
        let module_a = parse_module(
            package_id,
            0,
            r#"
function callee(): int32 {
b0:
    v0: int32 = 1int32
    return v0
}"#,
        );
        let module_b = parse_module(
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

        let callgraph = call_graph(&[module_a, module_b]);

        let caller = SymbolName::new("test");
        let callee = SymbolName::new("callee");

        // local (non-exported) callee does not satisfy the import
        let outgoing = callgraph.outgoing(&caller);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callgraph.external_symbol().clone());
        assert_eq!(callgraph.unknown_calls(&caller).len(), 1);
        assert_eq!(callgraph.unknown_calls(&caller)[0].callee, Some(callee));
    }

    /// Imports with mismatched signatures remain unresolved.
    #[test]
    fn test_call_graph_signature_mismatch() {
        let package_id = PackageId::new(9);
        let module_a = parse_module(
            package_id,
            0,
            r#"
export function callee(v0: int32): int32 {
b0(v0: int32):
    return v0
}"#,
        );
        let module_b = parse_module(
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

        let callgraph = call_graph(&[module_a, module_b]);

        let caller = SymbolName::new("test");
        let callee = SymbolName::new("callee");

        // signature mismatch leaves the call unresolved
        let outgoing = callgraph.outgoing(&caller);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callgraph.external_symbol().clone());
        assert_eq!(callgraph.unknown_calls(&caller).len(), 1);
        assert_eq!(callgraph.unknown_calls(&caller)[0].callee, Some(callee));
    }

    /// Calls resolve across modules when the exported symbol is unique.
    #[test]
    fn test_call_graph_resolves_cross_module() {
        let caller_module = parse_module(
            PackageId::new(2),
            0,
            r#"
external function callee(): int32
function test(): int32 {
b0:
    v0: int32 = call callee(): () -> int32
    return v0
}"#,
        );
        let callee_module = parse_module(
            PackageId::new(3),
            0,
            r#"
export function callee(): int32 {
b0:
    v0: int32 = 9int32
    return v0
}"#,
        );

        let callgraph = call_graph(&[caller_module, callee_module]);

        let caller = SymbolName::new("test");
        let callee = SymbolName::new("callee");

        // the cross-module export resolves the import
        let outgoing = callgraph.outgoing(&caller);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callee);
        assert!(callgraph.is_defined(&callee));
    }

    /// Ambiguous symbols (multiple exports) are left unresolved.
    #[test]
    fn test_call_graph_ambiguous_symbol_is_unknown() {
        let caller_module = parse_module(
            PackageId::new(4),
            0,
            r#"
external function callee(): int32
function test(): int32 {
b0:
    v0: int32 = call callee(): () -> int32
    return v0
}"#,
        );
        let callee_module_a = parse_module(
            PackageId::new(5),
            0,
            r#"
export function callee(): int32 {
b0:
    v0: int32 = 1int32
    return v0
}"#,
        );
        let callee_module_b = parse_module(
            PackageId::new(6),
            0,
            r#"
export function callee(): int32 {
b0:
    v0: int32 = 2int32
    return v0
}"#,
        );

        let callgraph = call_graph(&[caller_module, callee_module_a, callee_module_b]);

        let caller = SymbolName::new("test");
        let callee = SymbolName::new("callee");

        // two competing exports make the symbol ambiguous, so the call is unresolved
        let outgoing = callgraph.outgoing(&caller);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].callee, callgraph.external_symbol().clone());
        assert!(callgraph.is_ambiguous(&callee));
        assert_eq!(callgraph.unknown_calls(&caller).len(), 1);
    }
}

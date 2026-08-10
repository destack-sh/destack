use destack_core::FxIndexMap as IndexMap;
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::{ExportTarget, GlobalNodeIdAny, GlobalSymbolId};

/// Name resolutions for one module, keyed by the reference node.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ReferenceTable {
    /// The module id of the reference table.
    pub module_id: ModuleId,
    /// Final targets keyed by their source node.
    pub target_by_node: IndexMap<GlobalNodeIdAny, Reference>,
    /// Authored declarations that differ from their final target.
    pub declaration_by_node: IndexMap<GlobalNodeIdAny, Reference>,
}

/// One scalar target selected while resolving a source reference.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub enum ReferenceTarget {
    /// One declaration symbol.
    Symbol(GlobalSymbolId),
    /// One module namespace object.
    Namespace(ModuleId),
}

impl ReferenceTarget {
    /// Return the module that owns this target.
    pub fn module(self) -> ModuleId {
        match self {
            Self::Symbol(symbol) => symbol.module_id,
            Self::Namespace(module) => module,
        }
    }
}

impl ReferenceTable {
    /// Create an empty reference table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            target_by_node: IndexMap::default(),
            declaration_by_node: IndexMap::default(),
        }
    }

    /// Insert one final target and its authored declaration.
    pub fn insert_resolution(
        &mut self,
        node: GlobalNodeIdAny,
        declaration: Reference,
        target: Reference,
    ) {
        // retain only declarations that differ from their final target
        if declaration == target {
            self.declaration_by_node.shift_remove(&node);
        } else {
            self.declaration_by_node.insert(node, declaration);
        }

        self.target_by_node.insert(node, target);
    }

    /// Return one final target.
    pub fn get(&self, node: GlobalNodeIdAny) -> Option<&Reference> {
        self.target_by_node.get(&node)
    }

    /// Return the authored declaration, equal to the final target unless overridden.
    pub fn declaration(&self, node: GlobalNodeIdAny) -> Option<&Reference> {
        self.declaration_by_node
            .get(&node)
            .or_else(|| self.target_by_node.get(&node))
    }

    /// Return true when no references were resolved.
    pub fn is_empty(&self) -> bool {
        self.target_by_node.is_empty()
    }

    /// Return modules that own resolved reference targets.
    pub fn target_modules(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.target_by_node.values().flat_map(|reference| {
            let mut modules: SmallVec<[ModuleId; 2]> = SmallVec::new();
            match reference {
                Reference::Bound(symbols) => {
                    modules.extend(symbols.iter().map(|symbol| symbol.module_id));
                }
                Reference::Ambiguous(targets) => {
                    modules.extend(targets.iter().map(|target| target.module()));
                }
                Reference::Namespace { module, .. } => modules.push(*module),
                Reference::Projected { base, .. } => modules.push(base.module()),
                Reference::Missing => {}
            }

            modules
        })
    }
}

/// How one source reference resolves by name, before types and conditions apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Reference {
    /// Resolved to declarations by name: lexical scope or a full namespace path.
    /// A set carries overloads, narrowed by availability and dispatch in check.
    Bound(SmallVec<[GlobalSymbolId; 2]>),
    /// Resolved to a namespace: a prefix awaiting a further segment, or a bare namespace value.
    Namespace {
        /// The named module.
        module: ModuleId,
        /// The alias or clause declaring the namespace name, when one names it.
        declaration: Option<GlobalNodeIdAny>,
    },
    /// A flat path named through its first segments; `segments[from..]` project from `base`.
    Projected {
        /// The exact target named by the leading segments.
        base: ReferenceTarget,
        /// The segment index where member projection begins.
        from: u32,
    },
    /// No single binding wins; the payload retains every resolved candidate target.
    Ambiguous(SmallVec<[ReferenceTarget; 2]>),
    /// No binding by name.
    Missing,
}

impl Reference {
    /// Return the bound declaration symbols.
    pub fn symbols(&self) -> Option<&[GlobalSymbolId]> {
        match self {
            Self::Bound(symbols) => Some(symbols),
            Self::Namespace { .. }
            | Self::Projected { .. }
            | Self::Ambiguous(_)
            | Self::Missing => None,
        }
    }

    /// Return the selected module namespace.
    pub fn namespace(&self) -> Option<ModuleId> {
        match self {
            Self::Namespace { module, .. } => Some(*module),
            Self::Bound(_) | Self::Projected { .. } | Self::Ambiguous(_) | Self::Missing => None,
        }
    }

    /// Create a bound or missing reference from declaration symbols.
    pub fn from_symbols(symbols: impl IntoIterator<Item = GlobalSymbolId>) -> Self {
        let symbols = symbols.into_iter().collect::<SmallVec<_>>();
        if symbols.is_empty() {
            Self::Missing
        } else {
            Self::Bound(symbols)
        }
    }

    /// Create a name reference from final targets.
    pub fn from_targets(resolved_targets: impl IntoIterator<Item = ReferenceTarget>) -> Self {
        let mut targets = SmallVec::<[ReferenceTarget; 2]>::new();

        // retain each exact target once in source order
        for target in resolved_targets {
            if !targets.contains(&target) {
                targets.push(target);
            }
        }

        // classify the complete target set without discarding conflicts
        if targets.is_empty() {
            Self::Missing
        } else if let Some(symbols) = targets
            .iter()
            .map(|target| match target {
                ReferenceTarget::Symbol(symbol) => Some(*symbol),
                ReferenceTarget::Namespace(_) => None,
            })
            .collect::<Option<SmallVec<_>>>()
        {
            Self::Bound(symbols)
        }
        // retain one namespace target
        else if let [ReferenceTarget::Namespace(module)] = targets.as_slice() {
            Self::Namespace {
                module: *module,
                declaration: None,
            }
        }
        // retain every conflicting target
        else {
            Self::Ambiguous(targets)
        }
    }
}

impl From<&ExportTarget> for Reference {
    /// Convert one export target into a name reference.
    fn from(target: &ExportTarget) -> Self {
        match target {
            ExportTarget::Symbols(symbols) => Self::Bound(symbols.clone()),
            ExportTarget::Namespace(module) => Self::Namespace {
                module: *module,
                declaration: None,
            },
        }
    }
}

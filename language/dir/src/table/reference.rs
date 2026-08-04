use destack_core::FxIndexMap as IndexMap;
use destack_serde::Reflect;
use destack_source::ModuleId;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::{GlobalNodeIdAny, GlobalSymbolId, ImportTarget};

/// Name resolutions for one module, keyed by the reference node.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ReferenceTable {
    /// The module id of the reference table.
    pub module_id: ModuleId,
    /// Semantic targets keyed by their source node.
    pub target_by_node: IndexMap<GlobalNodeIdAny, Reference>,
    /// Declaration targets keyed by source nodes whose semantic targets differ.
    pub declarations_by_node: IndexMap<GlobalNodeIdAny, SmallVec<[GlobalSymbolId; 2]>>,
}

impl ReferenceTable {
    /// Create an empty reference table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            target_by_node: IndexMap::default(),
            declarations_by_node: IndexMap::default(),
        }
    }

    /// Insert one semantic target.
    pub fn insert(&mut self, node: GlobalNodeIdAny, target: Reference) {
        self.target_by_node.insert(node, target);
    }

    /// Insert declaration targets for one source node whose semantic targets differ.
    pub fn insert_declarations(
        &mut self,
        node: GlobalNodeIdAny,
        symbols: impl IntoIterator<Item = GlobalSymbolId>,
    ) {
        let symbols = symbols.into_iter().collect::<SmallVec<_>>();
        if !symbols.is_empty() {
            self.declarations_by_node.insert(node, symbols);
        }
    }

    /// Return one semantic target.
    pub fn get(&self, node: GlobalNodeIdAny) -> Option<&Reference> {
        self.target_by_node.get(&node)
    }

    /// Return the declaration targets recorded for one source node.
    pub fn declarations(&self, node: GlobalNodeIdAny) -> Option<&[GlobalSymbolId]> {
        self.declarations_by_node.get(&node).map(SmallVec::as_slice)
    }

    /// Return true when no references were resolved.
    pub fn is_empty(&self) -> bool {
        self.target_by_node.is_empty() && self.declarations_by_node.is_empty()
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
                Reference::Namespace(module) => modules.push(*module),
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
    Namespace(ModuleId),
    /// A flat path named through its first segments; `segments[from..]` project from `base`.
    Projected {
        /// The exact target named by the leading segments.
        base: ImportTarget,
        /// The segment index where member projection begins.
        from: u32,
    },
    /// No single binding wins; the payload retains every resolved candidate target.
    Ambiguous(SmallVec<[ImportTarget; 2]>),
    /// No binding by name.
    Missing,
}

impl Reference {
    /// Create a bound or missing reference from declaration symbols.
    pub fn from_symbols(symbols: impl IntoIterator<Item = GlobalSymbolId>) -> Self {
        let symbols = symbols.into_iter().collect::<SmallVec<_>>();
        if symbols.is_empty() {
            Self::Missing
        } else {
            Self::Bound(symbols)
        }
    }

    /// Create a symbol, namespace, or missing reference from semantic targets.
    pub fn from_targets(resolved_targets: impl IntoIterator<Item = ImportTarget>) -> Self {
        let mut targets = SmallVec::<[ImportTarget; 2]>::new();

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
                ImportTarget::Symbol(symbol) => Some(*symbol),
                ImportTarget::Namespace(_) => None,
            })
            .collect::<Option<SmallVec<_>>>()
        {
            Self::Bound(symbols)
        }
        // retain one namespace target
        else if let [ImportTarget::Namespace(module)] = targets.as_slice() {
            Self::Namespace(*module)
        }
        // retain every conflicting target
        else {
            Self::Ambiguous(targets)
        }
    }
}

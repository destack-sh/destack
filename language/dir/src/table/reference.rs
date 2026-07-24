use destack_serde::Reflect;
use destack_source::ModuleId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::{GlobalNodeIdAny, GlobalSymbolId, ImportTarget};

/// Name resolutions for one module, keyed by the reference node.
#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct ReferenceTable {
    /// The module id of the reference table.
    pub module_id: ModuleId,
    /// Resolved references keyed by their source node.
    pub entries: IndexMap<GlobalNodeIdAny, Reference>,
}

impl ReferenceTable {
    /// Create an empty reference table.
    pub fn new(module_id: ModuleId) -> Self {
        Self {
            module_id,
            entries: IndexMap::new(),
        }
    }

    /// Insert one resolved reference.
    pub fn insert(&mut self, node: GlobalNodeIdAny, reference: Reference) {
        self.entries.insert(node, reference);
    }

    /// Return one resolved reference.
    pub fn get(&self, node: GlobalNodeIdAny) -> Option<&Reference> {
        self.entries.get(&node)
    }

    /// Return true when no references were resolved.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return modules that own resolved reference targets.
    pub fn target_modules(&self) -> impl Iterator<Item = ModuleId> + '_ {
        self.entries.values().flat_map(|reference| {
            let mut modules: SmallVec<[ModuleId; 2]> = SmallVec::new();
            match reference {
                Reference::Bound(symbols) | Reference::Ambiguous(symbols) => {
                    modules.extend(symbols.iter().map(|symbol| symbol.module_id));
                }
                Reference::Namespace(module) => modules.push(*module),
                Reference::Projected { base, .. } => modules.push(base.module_id),
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
    /// A flat path named through its first segments; `segments[from..]` project as members off `base`.
    Projected {
        /// The declaration the leading segments name.
        base: GlobalSymbolId,
        /// The segment index where member projection begins.
        from: u32,
    },
    /// Conflicting bindings with no single winner.
    Ambiguous(SmallVec<[GlobalSymbolId; 2]>),
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
    pub fn from_targets(targets: impl IntoIterator<Item = ImportTarget>) -> Self {
        let mut symbols = SmallVec::new();
        let mut namespace = None;
        let mut is_namespace_ambiguous = false;

        // collect unique symbols and at most one namespace
        for target in targets {
            match target {
                ImportTarget::Symbol(symbol) if !symbols.contains(&symbol) => {
                    symbols.push(symbol);
                }
                ImportTarget::Symbol(_) => {}
                ImportTarget::Namespace(module) => {
                    is_namespace_ambiguous |= namespace.replace(module).is_some();
                }
            }
        }

        // prefer concrete symbols over namespace objects
        if !symbols.is_empty() {
            Self::Bound(symbols)
        }
        // retain one unambiguous namespace
        else if let Some(module) = namespace
            && !is_namespace_ambiguous
        {
            Self::Namespace(module)
        }
        // no semantic target remains
        else {
            Self::Missing
        }
    }
}

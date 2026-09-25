use tspp_core::FxIndexSet;
use tspp_dir as dir;
use tspp_source::ModuleId;

/// Source location that produced one check operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum Origin {
    /// Work came from one source node under one assuming template.
    Node(dir::GlobalNodeIdAny, Option<dir::GlobalGenericTemplateId>),
    /// Work came from one source symbol.
    Symbol(dir::GlobalSymbolId),
}

impl Origin {
    /// Return the source module for this origin.
    pub(in crate::sema) fn module(self) -> ModuleId {
        match self {
            Self::Node(node, _) => node.module_id,
            Self::Symbol(symbol) => symbol.module_id,
        }
    }
}

/// Component-global id of one interned check origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct OriginId(u32);

impl OriginId {
    /// Return the origin id at one arena index.
    pub(in crate::sema) fn at(index: usize) -> Self {
        Self(index as u32)
    }

    /// Return the arena index.
    pub(in crate::sema) fn index(self) -> usize {
        self.0 as usize
    }
}

/// Interned check origins, deduplicated per module.
#[derive(Debug, Default)]
pub(in crate::sema) struct OriginArena {
    /// The interned origins in first-seen order.
    origins: FxIndexSet<Origin>,
}

impl OriginArena {
    /// Intern one origin and return its id.
    pub(in crate::sema) fn intern(&mut self, origin: Origin) -> OriginId {
        let (index, _) = self.origins.insert_full(origin);

        OriginId::at(index)
    }

    /// Return one interned origin.
    pub(in crate::sema) fn get(&self, id: OriginId) -> Origin {
        match self.origins.get_index(id.index()) {
            Some(origin) => *origin,
            None => unreachable!("check origin {id:?} is not interned"),
        }
    }
}

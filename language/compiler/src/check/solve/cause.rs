use destack_core::FxIndexSet;
use destack_dir as dir;

use crate::check::{Origin, Variance};

/// One interned reason a judgment exists.
///
/// Causes form a tree from each judgment back to the written syntax that
/// demanded it, so any failure explains itself by walking its chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct Cause {
    /// The source this judgment anchors to.
    pub(in crate::check) origin: Origin,
    /// Why this judgment exists.
    pub(in crate::check) kind: CauseKind,
    /// The judgment that spawned this one.
    pub(in crate::check) parent: Option<CauseId>,
}

impl Cause {
    /// Create one root cause at its written syntax.
    pub(in crate::check) fn root(origin: Origin, kind: CauseKind) -> Self {
        Self {
            origin,
            kind,
            parent: None,
        }
    }

    /// Create one child cause descending from a parent judgment.
    pub(in crate::check) fn slot(origin: Origin, kind: CauseKind, parent: CauseId) -> Self {
        Self {
            origin,
            kind,
            parent: Some(parent),
        }
    }
}

/// Why one judgment exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) enum CauseKind {
    /// A value flows into an annotated binding.
    Initializer {
        /// The written type annotation.
        annotation: Option<dir::GlobalNodeIdAny>,
    },
    /// An argument flows into a declared parameter.
    Argument {
        /// The call expression.
        call: dir::GlobalNodeIdAny,
        /// The zero-based argument position.
        index: u32,
    },
    /// A completion value flows into a declared return type.
    Return {
        /// The written return annotation.
        annotation: Option<dir::GlobalNodeIdAny>,
    },
    /// A generic argument must satisfy its declared bound.
    Bound {
        /// The bounded generic parameter.
        parameter: dir::GlobalGenericParameterId,
    },
    /// A declaration must satisfy one heritage clause.
    Heritage {
        /// The written heritage clause.
        clause: dir::GlobalNodeIdAny,
    },
    /// A scrutinee flows into one pattern.
    Pattern {
        /// The written pattern.
        pattern: dir::GlobalNodeIdAny,
    },
    /// A value writes into one place.
    Write {
        /// The written place expression.
        place: dir::GlobalNodeIdAny,
    },
    /// A value satisfies one written relation, like `satisfies` or a cast.
    Expression,
    /// A judgment descends into one structural field.
    Field {
        /// The field key.
        key: dir::StaticKey,
    },
    /// A judgment descends into one positional element.
    Element {
        /// The zero-based element position.
        index: u32,
    },
    /// A judgment descends into one signature parameter, contravariantly.
    Parameter {
        /// The zero-based parameter position.
        index: u32,
    },
    /// A judgment descends into the signature return slot.
    ReturnSlot,
    /// A judgment descends into one type argument under its variance.
    TypeArgument {
        /// The applied symbol.
        symbol: dir::GlobalSymbolId,
        /// The zero-based argument position.
        index: u32,
        /// The variance the argument relates under.
        variance: Variance,
    },
    /// A judgment descends into one memory form payload.
    Payload,
}

impl CauseKind {
    /// Return whether this kind descends inside a parent judgment.
    pub(in crate::check) fn is_slot(self) -> bool {
        matches!(
            self,
            Self::Field { .. }
                | Self::Element { .. }
                | Self::Parameter { .. }
                | Self::ReturnSlot
                | Self::TypeArgument { .. }
                | Self::Payload
        )
    }
}

/// Component-global id of one interned cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct CauseId(u32);

impl CauseId {
    /// Return the cause id at one arena index.
    pub(in crate::check) fn at(index: usize) -> Self {
        Self(index as u32)
    }

    /// Return the arena index.
    pub(in crate::check) fn index(self) -> usize {
        self.0 as usize
    }
}

/// Interned causes, deduplicated per component.
#[derive(Debug, Default)]
pub(in crate::check) struct CauseArena {
    /// The interned causes in first-seen order.
    causes: FxIndexSet<Cause>,
}

impl CauseArena {
    /// Intern one cause and return its id.
    pub(in crate::check) fn intern(&mut self, cause: Cause) -> CauseId {
        let (index, _) = self.causes.insert_full(cause);

        CauseId::at(index)
    }

    /// Return one interned cause.
    pub(in crate::check) fn get(&self, id: CauseId) -> Cause {
        self.causes[id.index()]
    }
}

use tspp_core::FxIndexSet;
use tspp_dir as dir;

use crate::sema::{Origin, Variance};

/// One interned reason a constraint exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) struct Cause {
    /// The source this constraint anchors to.
    pub(in crate::sema) origin: Origin,
    /// Why this constraint exists.
    pub(in crate::sema) kind: CauseKind,
    /// The constraint that spawned this one.
    pub(in crate::sema) parent: Option<CauseId>,
}

impl Cause {
    /// Create one root cause at its written syntax.
    pub(in crate::sema) fn root(origin: Origin, kind: CauseKind) -> Self {
        Self {
            origin,
            kind,
            parent: None,
        }
    }

    /// Create one cause descending from a parent constraint.
    pub(in crate::sema) fn child(origin: Origin, kind: CauseKind, parent: CauseId) -> Self {
        Self {
            origin,
            kind,
            parent: Some(parent),
        }
    }

    /// Create one cause descending from a parent constraint, or a root cause without one.
    pub(in crate::sema) fn child_maybe(
        origin: Origin,
        kind: CauseKind,
        parent: Option<CauseId>,
    ) -> Self {
        Self {
            origin,
            kind,
            parent,
        }
    }
}

/// Why one constraint exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum CauseKind {
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
    /// A callable value's body requires access from its receiver.
    Receiver,
    /// A constraint descends into one structural field.
    Field {
        /// The field key.
        key: dir::StaticKey,
    },
    /// A constraint descends into one positional element.
    Element {
        /// The zero-based element position.
        index: u32,
    },
    /// A constraint descends into one signature parameter, contravariantly.
    Parameter {
        /// The zero-based parameter position.
        index: u32,
    },
    /// A constraint descends into the signature return slot.
    ReturnSlot,
    /// A constraint descends into one type argument under its variance.
    TypeArgument {
        /// The applied symbol.
        symbol: dir::GlobalSymbolId,
        /// The zero-based argument position.
        index: u32,
        /// The variance the argument relates under.
        variance: Variance,
    },
    /// A constraint descends into one memory form payload.
    Payload,
}

impl CauseKind {
    /// Return whether this kind descends inside a parent constraint.
    pub(in crate::sema) fn is_slot(self) -> bool {
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
pub(in crate::sema) struct CauseId(u32);

impl CauseId {
    /// Return the cause id at one arena index.
    pub(in crate::sema) fn at(index: usize) -> Self {
        Self(index as u32)
    }

    /// Return the arena index.
    pub(in crate::sema) fn index(self) -> usize {
        self.0 as usize
    }
}

/// Interned causes, deduplicated per module.
#[derive(Debug, Default)]
pub(in crate::sema) struct CauseArena {
    /// The interned causes in first-seen order.
    causes: FxIndexSet<Cause>,
}

impl CauseArena {
    /// Intern one cause and return its id.
    pub(in crate::sema) fn intern(&mut self, cause: Cause) -> CauseId {
        let (index, _) = self.causes.insert_full(cause);

        CauseId::at(index)
    }

    /// Return one interned cause.
    pub(in crate::sema) fn get(&self, id: CauseId) -> Cause {
        self.causes[id.index()]
    }
}

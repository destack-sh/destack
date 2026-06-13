use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::{CheckError, CompilerError, CompilerResult, DiagnosticAnchor};

use crate::check::{Answer, CheckState, Condition, Mutation, Place, Task};

/// Stable index of one collected obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct ObligationId(u32);

impl ObligationId {
    /// Return the obligation index.
    fn index(self) -> usize {
        self.0 as usize
    }
}

/// Selector for one active match case.
///
/// Examples:
/// ```ds
/// match (value) { _ => value }
/// match (value) { Some(item) => item }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum MatchCase {
    /// Default selector.
    Default,
    /// Pattern selector with an optional guard type.
    Pattern {
        /// The pattern checked for this case.
        pattern: dir::GlobalNodeId<dir::Pattern>,
        /// The optional guard type.
        guard: Option<dir::GlobalTypeId>,
    },
}

/// User-facing check that runs once its inputs solve.
///
/// Examples:
/// ```ds
/// match (value) { _ => value }
/// const { name } = user
/// sizeOf<T>()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum Obligation {
    /// Match cases must cover every possible selector value.
    Match(MatchObligation),
    /// Binding patterns in non-matching positions must always succeed.
    Pattern(PatternObligation),
    /// Try propagation must fit the enclosing return type.
    Try(TryObligation),
    /// A place assignment must target writable storage.
    Place(PlaceObligation),
    /// A type at a representation slot must have a computed layout.
    Layout(LayoutObligation),
    /// A class declaration must satisfy its heritage rules.
    Heritage(HeritageObligation),
}

impl Obligation {
    /// Return the source node anchoring diagnostics.
    pub(in crate::check) fn source(&self) -> dir::GlobalNodeIdAny {
        match self {
            Self::Match(obligation) => obligation.source,
            Self::Pattern(obligation) => obligation.source,
            Self::Try(obligation) => obligation.source,
            Self::Place(obligation) => obligation.place.source,
            Self::Layout(obligation) => obligation.source,
            Self::Heritage(obligation) => obligation.source,
        }
    }

    /// Return the static condition under which this obligation exists.
    pub(in crate::check) fn condition(&self) -> &Condition {
        match self {
            Self::Match(obligation) => &obligation.condition,
            Self::Pattern(obligation) => &obligation.condition,
            Self::Try(obligation) => &obligation.condition,
            Self::Place(obligation) => &obligation.condition,
            Self::Layout(obligation) => &obligation.condition,
            Self::Heritage(obligation) => &obligation.condition,
        }
    }
}

/// One match coverage obligation.
///
/// ```ds
/// match (value) { true => 1, false => 0 }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct MatchObligation {
    /// The match expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The matched value type.
    pub(in crate::check) value: dir::GlobalTypeId,
    /// The active match cases in source order.
    pub(in crate::check) cases: Vec<MatchCase>,
}

/// One irrefutable binding pattern obligation.
///
/// ```ds
/// let { name } = user;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct PatternObligation {
    /// The checked pattern source.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The checked pattern node.
    pub(in crate::check) pattern: dir::GlobalNodeId<dir::Pattern>,
    /// The matched value type.
    pub(in crate::check) value: dir::GlobalTypeId,
}

/// One try propagation obligation.
///
/// ```ds
/// value?
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TryObligation {
    /// The try expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The tried value type.
    pub(in crate::check) value: dir::GlobalTypeId,
    /// The enclosing function return type.
    pub(in crate::check) return_type: Option<dir::GlobalTypeId>,
}

/// One writable place obligation.
///
/// ```ds
/// value = 2;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct PlaceObligation {
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The place being written.
    pub(in crate::check) place: Place,
}

/// One class heritage obligation.
///
/// ```ds
/// class Admin extends User {}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct HeritageObligation {
    /// The class declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The checked class symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
}

/// One layout obligation.
///
/// ```ds
/// sizeOf<T>()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct LayoutObligation {
    /// The source expression requiring one fixed representation.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The type that must have one fixed representation.
    pub(in crate::check) ty: dir::GlobalTypeId,
}

/// Collected obligations with completion tracking.
#[derive(Debug)]
pub(in crate::check) struct ObligationTable {
    /// The collected obligations in allocation order.
    obligations: Vec<Obligation>,
    /// Whether each obligation finished checking.
    completed: Vec<bool>,
}

impl ObligationTable {
    /// Create an empty obligation table.
    pub(in crate::check) fn new() -> Self {
        Self {
            obligations: Vec::new(),
            completed: Vec::new(),
        }
    }

    /// Collect one obligation.
    pub(in crate::check) fn allocate(&mut self, obligation: Obligation) -> ObligationId {
        let id = ObligationId(self.obligations.len() as u32);
        self.obligations.push(obligation);
        self.completed.push(false);

        id
    }

    /// Return one collected obligation.
    pub(in crate::check) fn get(&self, id: ObligationId) -> CompilerResult<&Obligation> {
        self.obligations
            .get(id.index())
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check obligation {id:?} does not exist"),
            })
    }

    /// Mark one obligation complete.
    pub(in crate::check) fn complete(&mut self, id: ObligationId) {
        self.completed[id.index()] = true;
    }

    /// Return whether one obligation finished checking.
    pub(in crate::check) fn is_complete(&self, id: ObligationId) -> bool {
        self.completed[id.index()]
    }

    /// Return the number of collected obligations.
    pub(in crate::check) fn count(&self) -> usize {
        self.obligations.len()
    }

    /// Drop the youngest obligations down to one count.
    pub(in crate::check) fn truncate(&mut self, count: usize) {
        self.obligations.truncate(count);
        self.completed.truncate(count);
    }

    /// Iterate obligations that never finished checking.
    pub(in crate::check) fn unfinished(&self) -> impl Iterator<Item = &Obligation> + '_ {
        self.obligations
            .iter()
            .zip(self.completed.iter())
            .filter_map(|(obligation, complete)| (!complete).then_some(obligation))
    }
}

impl CheckState<'_> {
    /// Collect one journaled obligation and schedule it.
    pub(in crate::check) fn push_obligation(&mut self, obligation: Obligation) -> ObligationId {
        let id = self.obligations.allocate(obligation);
        self.journal.record(Mutation::ObligationAllocated);
        self.queue_task(Task::Oblige(id));

        id
    }

    /// Run one obligated check once.
    pub(in crate::check) fn run_obligation(
        &mut self,
        id: ObligationId,
    ) -> CompilerResult<Answer<()>> {
        if self.obligations.is_complete(id) {
            return Ok(Answer::Ready(()));
        }

        // copy the obligation for the borrow-free check
        let obligation = self.obligations.get(id)?.clone();
        let predicates = match obligation.condition() {
            Condition::Always => smallvec::SmallVec::new(),
            Condition::When(predicates) => predicates.clone(),
        };

        // gate conditional obligations on their predicates
        if !predicates.is_empty() {
            match self.decide_condition(&predicates)? {
                // skip obligations whose condition failed
                Answer::Ready(false) => {
                    self.obligations.complete(id);

                    return Ok(Answer::Ready(()));
                }
                Answer::Ready(true) => {}
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }

        // check the obligation under its own guard assumptions
        let mark = self.assume(&predicates)?;
        let decision = self.check_obligation(&obligation);
        self.release_assumptions(mark);
        let decision = decision?;

        match decision {
            Answer::Ready(diagnostic) => {
                if let Some(diagnostic) = diagnostic {
                    self.module_mut(obligation.source().module_id)
                        .diagnostics
                        .push(diagnostic);
                }
                self.obligations.complete(id);

                Ok(Answer::Ready(()))
            }
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Check one obligation against solved inputs.
    fn check_obligation(
        &mut self,
        obligation: &Obligation,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        match obligation {
            Obligation::Match(obligation) => {
                self.check_match_exhaustive(obligation.source, obligation.value, &obligation.cases)
            }
            Obligation::Pattern(obligation) => self.check_irrefutable_pattern(
                obligation.source,
                obligation.pattern,
                obligation.value,
            ),
            Obligation::Try(obligation) => self.check_try_propagates(
                obligation.source,
                obligation.value,
                obligation.return_type,
            ),
            Obligation::Place(obligation) => self.check_writable_place(obligation.place),
            Obligation::Layout(obligation) => self.check_layout(obligation.source, obligation.ty),
            Obligation::Heritage(obligation) => {
                self.check_class_heritage(obligation.source, obligation.symbol)
            }
        }
    }

    /// Return the diagnostic anchor for one source node.
    pub(in crate::check) fn source_anchor(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> (ModuleId, DiagnosticAnchor) {
        let module = source.module_id;
        let anchor = self.diagnostic_anchor(module, source.local_id);

        (module, anchor)
    }
}

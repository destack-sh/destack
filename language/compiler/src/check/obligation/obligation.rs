use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::{CheckError, CompilerError, CompilerResult, DiagnosticAnchor};

use crate::check::{Answer, CheckEvent, CheckState, Condition, Mutation, Place, Task};

/// Stable index of one collected obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct ObligationId(u32);

impl ObligationId {
    /// Return the obligation index.
    pub(in crate::check) fn index(self) -> usize {
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
    /// A pattern-bearing site must cover the matched value space.
    PatternCoverage(PatternCoverageObligation),
    /// Try propagation must fit the enclosing return type.
    TryPropagation(TryPropagationObligation),
    /// A place assignment must target writable storage.
    WritablePlace(WritablePlaceObligation),
    /// A type at a representation slot must have a computed representation.
    Representation(RepresentationObligation),
    /// A runtime-erased type must have a checkable dynamic surface.
    DynamicSafety(DynamicSafetyObligation),
    /// An extension must provide members required by its implemented interfaces.
    ExtensionConformance(ExtensionConformanceObligation),
    /// An implementation must not overlap a conflicting implementation.
    ImplementationCoherence(ImplementationCoherenceObligation),
    /// A declaration must satisfy its heritage graph rules.
    DeclarationHeritage(DeclarationHeritageObligation),
}

impl Obligation {
    /// Return the source node anchoring diagnostics.
    pub(in crate::check) fn source(&self) -> dir::GlobalNodeIdAny {
        match self {
            Self::PatternCoverage(obligation) => obligation.source,
            Self::TryPropagation(obligation) => obligation.source,
            Self::WritablePlace(obligation) => obligation.place.source,
            Self::Representation(obligation) => obligation.source,
            Self::DynamicSafety(obligation) => obligation.source,
            Self::ExtensionConformance(obligation) => obligation.source,
            Self::ImplementationCoherence(obligation) => obligation.source,
            Self::DeclarationHeritage(obligation) => obligation.source,
        }
    }

    /// Return the static condition under which this obligation exists.
    pub(in crate::check) fn condition(&self) -> &Condition {
        match self {
            Self::PatternCoverage(obligation) => &obligation.condition,
            Self::TryPropagation(obligation) => &obligation.condition,
            Self::WritablePlace(obligation) => &obligation.condition,
            Self::Representation(obligation) => &obligation.condition,
            Self::DynamicSafety(obligation) => &obligation.condition,
            Self::ExtensionConformance(obligation) => &obligation.condition,
            Self::ImplementationCoherence(obligation) => &obligation.condition,
            Self::DeclarationHeritage(obligation) => &obligation.condition,
        }
    }
}

/// Obliges a pattern-bearing site to cover the matched value space.
///
/// ```ds
/// match (value) { true => 1, false => 0 }
/// let { name } = user;
/// try {} catch ({ message }) {}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct PatternCoverageObligation {
    /// The checked source.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The matched value type.
    pub(in crate::check) value: dir::GlobalTypeId,
    /// The pattern coverage shape.
    pub(in crate::check) coverage: PatternCoverage,
}

/// A pattern-bearing site whose coverage must be checked.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum PatternCoverage {
    /// A match expression whose active cases must be exhaustive.
    Match {
        /// The active match cases in source order.
        cases: Vec<MatchCase>,
    },
    /// A binding pattern in a non-matching position that must be irrefutable.
    Binding {
        /// The checked pattern node.
        pattern: dir::GlobalNodeId<dir::Pattern>,
    },
    /// A catch binding pattern that must be irrefutable for the failure type.
    Catch {
        /// The checked pattern node.
        pattern: dir::GlobalNodeId<dir::Pattern>,
    },
}

/// Obliges a try expression to propagate through the enclosing return type.
///
/// ```ds
/// value?
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct TryPropagationObligation {
    /// The try expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The tried value type.
    pub(in crate::check) value: dir::GlobalTypeId,
    /// The enclosing function return type.
    pub(in crate::check) return_type: Option<dir::GlobalTypeId>,
}

/// Obliges an assignment target to accept writes.
///
/// ```ds
/// value = 2;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct WritablePlaceObligation {
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The place being written.
    pub(in crate::check) place: Place,
}

/// Obliges a declaration's heritage graph to be valid.
///
/// ```ds
/// class Admin extends User {}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct DeclarationHeritageObligation {
    /// The declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The checked declaration symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
}

/// Obliges a type to have one fixed representation.
///
/// ```ds
/// sizeOf<T>()
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct RepresentationObligation {
    /// The source expression requiring one fixed representation.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The type that must have one fixed representation.
    pub(in crate::check) ty: dir::GlobalTypeId,
}

/// Obliges a type to support runtime erasure.
///
/// ```ds
/// value is Printable
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct DynamicSafetyObligation {
    /// The source requiring runtime erasure.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The type that must support runtime erasure.
    pub(in crate::check) ty: dir::GlobalTypeId,
}

/// Obliges an extension to provide members required by its implemented interfaces.
///
/// ```ds
/// extension of User implements Display {}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ExtensionConformanceObligation {
    /// The extension declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The checked extension symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
}

/// Obliges an implementation to avoid overlapping conflicting implementations.
///
/// ```ds
/// extension of User implements Display {}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ImplementationCoherenceObligation {
    /// The extension declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The checked extension symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
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
                    self.record_event(CheckEvent::ObligationCheck {
                        obligation: id,
                        finished: true,
                    });

                    return Ok(Answer::Ready(()));
                }
                Answer::Ready(true) => {}
                Answer::Pending(blockers) => {
                    self.record_event(CheckEvent::ObligationCheck {
                        obligation: id,
                        finished: false,
                    });

                    return Ok(Answer::Pending(blockers));
                }
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
                self.record_event(CheckEvent::ObligationCheck {
                    obligation: id,
                    finished: true,
                });

                Ok(Answer::Ready(()))
            }
            Answer::Pending(blockers) => {
                self.record_event(CheckEvent::ObligationCheck {
                    obligation: id,
                    finished: false,
                });

                Ok(Answer::Pending(blockers))
            }
        }
    }

    /// Check one obligation against solved inputs.
    fn check_obligation(
        &mut self,
        obligation: &Obligation,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        match obligation {
            Obligation::PatternCoverage(obligation) => self.check_pattern_coverage(obligation),
            Obligation::TryPropagation(obligation) => self.check_try_propagates(
                obligation.source,
                obligation.value,
                obligation.return_type,
            ),
            Obligation::WritablePlace(obligation) => self.check_writable_place(obligation.place),
            Obligation::Representation(obligation) => {
                self.check_layout(obligation.source, obligation.ty)
            }
            Obligation::DynamicSafety(obligation) => {
                self.check_dynamic_safe(obligation.source, obligation.ty)
            }
            Obligation::ExtensionConformance(obligation) => {
                self.check_extension_conformance(obligation.source, obligation.symbol)
            }
            Obligation::ImplementationCoherence(obligation) => {
                self.check_implementation_coherence(obligation.source, obligation.symbol)
            }
            Obligation::DeclarationHeritage(obligation) => {
                self.check_heritage(obligation.source, obligation.symbol)
            }
        }
    }

    /// Check one pattern coverage obligation.
    fn check_pattern_coverage(
        &mut self,
        obligation: &PatternCoverageObligation,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        match &obligation.coverage {
            PatternCoverage::Match { cases } => {
                self.check_match_exhaustive(obligation.source, obligation.value, cases)
            }
            PatternCoverage::Binding { pattern } => self.check_irrefutable_pattern(
                obligation.source,
                *pattern,
                obligation.value,
                |anchor, module, missing| {
                    CheckError::RefutablePattern {
                        anchor,
                        module,
                        missing,
                    }
                    .help("handle the uncovered values with 'if let' or 'match'")
                },
            ),
            PatternCoverage::Catch { pattern } => self.check_irrefutable_pattern(
                obligation.source,
                *pattern,
                obligation.value,
                |anchor, module, missing| {
                    CheckError::RefutableCatchPattern {
                        anchor,
                        module,
                        missing,
                    }
                    .help("catch bindings must handle every failure value")
                },
            ),
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

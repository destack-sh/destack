use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexMap;

use crate::{CheckError, CompilerError, CompilerResult, DiagnosticAnchor};

use crate::check::{
    Answer, AutoInterface, CheckEvent, CheckState, Condition, Origin, Place, Task, answer,
};

/// Component-global id of one collected obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::check) struct ObligationId(u32);

impl ObligationId {
    /// Return the obligation id at one index.
    pub(in crate::check) fn at(index: usize) -> Self {
        Self(index as u32)
    }

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
    /// A type must satisfy one compiler-known auto interface.
    AutoInterface(AutoInterfaceObligation),
    /// A runtime predicate must have valid operands.
    RuntimePredicate(RuntimePredicateObligation),
    /// A for-in source must be enumerable.
    ForInSource(ForInSourceObligation),
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
            Self::AutoInterface(obligation) => obligation.source,
            Self::RuntimePredicate(obligation) => obligation.source,
            Self::ForInSource(obligation) => obligation.source,
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
            Self::AutoInterface(obligation) => &obligation.condition,
            Self::RuntimePredicate(obligation) => &obligation.condition,
            Self::ForInSource(obligation) => &obligation.condition,
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
    /// The tried value.
    pub(in crate::check) value: TryPropagationValue,
    /// The receiver of the propagated failure.
    pub(in crate::check) target: TryPropagationTarget,
}

/// A tried value available either immediately or after selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum TryPropagationValue {
    /// A known try value type.
    Type(dir::GlobalTypeId),
    /// A selected node whose checked type is the try value.
    Node(dir::GlobalNodeIdAny),
}

/// The receiver of one propagated failure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) enum TryPropagationTarget {
    /// A local try target failure type.
    Failure {
        /// The local failure accumulator type.
        ty: dir::GlobalTypeId,
    },
    /// The enclosing function return type.
    Return {
        /// The function return type, if propagation is inside a function.
        ty: Option<dir::GlobalTypeId>,
    },
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
    /// The type being overwritten.
    pub(in crate::check) ty: dir::GlobalTypeId,
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

/// Obliges a type to satisfy one compiler-known auto interface.
///
/// ```ds
/// T: DynamicSafe
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct AutoInterfaceObligation {
    /// The source requiring the interface.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The type that must satisfy the interface.
    pub(in crate::check) ty: dir::GlobalTypeId,
    /// The required auto interface.
    pub(in crate::check) interface: AutoInterface,
}

/// Obliges a runtime predicate to be executable.
///
/// ```ds
/// value instanceof User
/// "name" in value
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct RuntimePredicateObligation {
    /// The predicate expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The left operand expression.
    pub(in crate::check) left: dir::GlobalNodeIdAny,
    /// The right operand expression or type.
    pub(in crate::check) right: dir::GlobalNodeIdAny,
    /// The selected predicate.
    pub(in crate::check) predicate: dir::GuardResolution,
}

/// Obliges a for-in source to have enumerable string keys.
///
/// ```ds
/// for (const key in value) {}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ForInSourceObligation {
    /// The for-in expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The static condition under which this obligation exists.
    pub(in crate::check) condition: Condition,
    /// The source type that must be object-shaped.
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

/// Collected obligations in allocation order.
#[derive(Debug, Clone)]
pub(in crate::check) struct ObligationTable {
    /// The collected obligations keyed by absolute obligation id.
    obligations: IndexMap<ObligationId, Obligation>,
}

impl ObligationTable {
    /// Create an empty obligation table.
    pub(in crate::check) fn new() -> Self {
        Self {
            obligations: IndexMap::new(),
        }
    }

    /// Insert one exact obligation id.
    pub(in crate::check) fn insert(&mut self, id: ObligationId, obligation: Obligation) {
        self.obligations.insert(id, obligation);
    }

    /// Remove one exact obligation id.
    pub(in crate::check) fn remove(&mut self, id: ObligationId) -> Option<Obligation> {
        self.obligations.swap_remove(&id)
    }

    /// Return one collected obligation.
    pub(in crate::check) fn get(&self, id: ObligationId) -> CompilerResult<&Obligation> {
        self.obligations
            .get(&id)
            .ok_or_else(|| CompilerError::Internal {
                message: format!("check obligation {id:?} does not exist"),
            })
    }

    /// Return the number of collected obligations.
    pub(in crate::check) fn count(&self) -> usize {
        self.obligations.len()
    }
}

impl CheckState<'_> {
    /// Collect one obligation.
    pub(in crate::check) fn push_obligation(&mut self, obligation: Obligation) -> ObligationId {
        let id = self.solver.allocate_obligation(obligation);
        self.queue_task(Task::Oblige(id));

        id
    }

    /// Check one obligation once.
    pub(in crate::check) fn run_obligation(
        &mut self,
        id: ObligationId,
    ) -> CompilerResult<Answer<()>> {
        // copy the obligation for the borrow-free check
        let obligation = self.solver.obligations.get(id)?.clone();
        let predicates = match obligation.condition() {
            Condition::Always => smallvec::SmallVec::new(),
            Condition::When(predicates) => predicates.clone(),
        };

        // gate conditional obligations on their predicates
        if !predicates.is_empty() {
            match self.decide_condition(&predicates)? {
                // skip obligations whose condition failed
                Answer::Ready(false) => {
                    self.record_event(CheckEvent::ObligationChecked {
                        obligation: id,
                        is_finished: true,
                    });

                    return Ok(Answer::Ready(()));
                }
                Answer::Ready(true) => {}
                Answer::Pending(blockers) => {
                    self.record_event(CheckEvent::ObligationChecked {
                        obligation: id,
                        is_finished: false,
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
                self.record_event(CheckEvent::ObligationChecked {
                    obligation: id,
                    is_finished: true,
                });

                Ok(Answer::Ready(()))
            }
            Answer::Pending(blockers) => {
                self.record_event(CheckEvent::ObligationChecked {
                    obligation: id,
                    is_finished: false,
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
            Obligation::TryPropagation(obligation) => self.check_try_propagates(obligation),
            Obligation::WritablePlace(obligation) => self.check_writable_place(obligation),
            Obligation::Representation(obligation) => {
                self.check_layout(obligation.source, obligation.ty)
            }
            Obligation::AutoInterface(obligation) => {
                self.check_auto_interface(obligation.source, obligation.ty, obligation.interface)
            }
            Obligation::RuntimePredicate(obligation) => self.check_runtime_predicate(obligation),
            Obligation::ForInSource(obligation) => self.check_for_in_source(obligation),
            Obligation::ExtensionConformance(obligation) => {
                self.check_extension_conformance(obligation.source, obligation.symbol)
            }
            Obligation::ImplementationCoherence(obligation) => {
                self.check_implementation_coherence(obligation.source, obligation.symbol)
            }
            Obligation::DeclarationHeritage(obligation) => {
                self.check_declaration_heritage(obligation.source, obligation.symbol)
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

    /// Check one for-in source obligation.
    fn check_for_in_source(
        &mut self,
        obligation: &ForInSourceObligation,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let origin = Origin::Node(obligation.source);
        if answer!(self.is_keyed_type(origin, obligation.ty)?) {
            return Ok(Answer::Ready(None));
        }

        let (module, anchor) = self.source_anchor(obligation.source);
        let error = CheckError::ForInSourceNotObjectShaped { anchor, module };

        Ok(Answer::Ready(Some(error.into())))
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

use destack_dir as dir;

use crate::{CompilerError, CompilerResult};

use crate::check::{
    Answer, CheckEvent, CheckState, ExpectedType, FlowBranch, GenericTemplateId, Origin, Task,
    TaskFailure, TaskFailures, Variance, WriteTarget, answer,
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

/// One active match arm used for coverage.
///
/// Examples:
/// ```ds
/// match (value) { _ => value }
/// match (value) { Some(item) => item }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct PatternArm {
    /// The selected pattern.
    pub pattern: dir::GlobalNodeId<dir::Pattern>,
    /// Whether the arm has a guard expression.
    pub is_guarded: bool,
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
    /// A place assignment must target writable storage.
    WritablePlace(Box<WritablePlaceObligation>),
    /// A type at a representation slot must have a computed representation.
    Representation(RepresentationObligation),
    /// A runtime predicate must have valid operands.
    RuntimePredicate(Box<RuntimePredicateObligation>),
    /// A for-in source must be enumerable.
    ForInSource(ForInSourceObligation),
    /// An extension must provide members required by its implemented interfaces.
    ExtensionConformance(ExtensionConformanceObligation),
    /// An implementation must not overlap a conflicting implementation.
    ImplementationCoherence(ImplementationCoherenceObligation),
    /// A declaration must satisfy its heritage graph rules.
    DeclarationHeritage(DeclarationHeritageObligation),
    /// A class must initialize required fields on every constructor path.
    ClassInitialization(ClassInitializationObligation),
    /// A written type operation must be well-formed once its operands close.
    WellFormedType(WellFormedTypeObligation),
    /// A declaration's generic parameters must occur in its definition.
    ParameterUse(ParameterUseObligation),
}

impl Obligation {
    /// Return the source node anchoring diagnostics.
    pub(in crate::check) fn source(&self) -> dir::GlobalNodeIdAny {
        match self {
            Self::PatternCoverage(obligation) => obligation.source,
            Self::WritablePlace(obligation) => obligation.place.source,
            Self::Representation(obligation) => obligation.source,
            Self::RuntimePredicate(obligation) => obligation.source,
            Self::ForInSource(obligation) => obligation.source,
            Self::ExtensionConformance(obligation) => obligation.source,
            Self::ImplementationCoherence(obligation) => obligation.source,
            Self::DeclarationHeritage(obligation) => obligation.source,
            Self::ClassInitialization(obligation) => obligation.source,
            Self::WellFormedType(obligation) => obligation.source,
            Self::ParameterUse(obligation) => obligation.source,
        }
    }
}

/// Result of checking one obligation.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ObligationCheck {
    /// The obligation holds.
    Holds,
    /// The obligation failed for one or more known reasons.
    Fails(Vec<ObligationFailure>),
}

impl ObligationCheck {
    /// Return a successful obligation check.
    pub(in crate::check) fn holds() -> Self {
        Self::Holds
    }

    /// Return one failed obligation check.
    pub(in crate::check) fn fail(failure: ObligationFailure) -> Self {
        Self::Fails(vec![failure])
    }

    /// Return an obligation check from collected failures.
    pub(in crate::check) fn from_failures(failures: Vec<ObligationFailure>) -> Self {
        if failures.is_empty() {
            Self::Holds
        } else {
            Self::Fails(failures)
        }
    }

    /// Return the failed reasons.
    pub(in crate::check) fn into_failures(self) -> Vec<ObligationFailure> {
        match self {
            Self::Holds => Vec::new(),
            Self::Fails(failures) => failures,
        }
    }
}

/// Reason one completed obligation failed.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ObligationFailure {
    /// A match expression did not cover one remaining value.
    NonExhaustivePattern {
        /// The expression or pattern-bearing source.
        source: dir::GlobalNodeIdAny,
        /// A representative uncovered value.
        missing: UncoveredValue,
    },
    /// A non-matching binding pattern can reject one remaining value.
    RefutablePattern {
        /// The pattern-bearing source.
        source: dir::GlobalNodeIdAny,
        /// A representative uncovered value.
        missing: UncoveredValue,
    },
    /// A catch binding pattern can reject one remaining failure value.
    RefutableCatchPattern {
        /// The pattern-bearing source.
        source: dir::GlobalNodeIdAny,
        /// A representative uncovered value.
        missing: UncoveredValue,
    },
    /// A for-in source does not expose object keys.
    ForInSourceNotObjectShaped {
        /// The for-in expression.
        source: dir::GlobalNodeIdAny,
    },
    /// A type predicate can never hold.
    ImpossibleIs {
        /// The predicate expression.
        source: dir::GlobalNodeIdAny,
        /// The checked value type.
        value: dir::GlobalTypeId,
        /// The tested target type.
        target: dir::GlobalTypeId,
    },
    /// An instanceof predicate can never hold.
    ImpossibleInstanceOf {
        /// The predicate expression.
        source: dir::GlobalNodeIdAny,
        /// The checked value type.
        value: dir::GlobalTypeId,
        /// The tested class symbol.
        target: dir::GlobalSymbolId,
    },
    /// An in predicate has no executable implementation.
    InvalidInPredicate {
        /// The predicate expression.
        source: dir::GlobalNodeIdAny,
        /// The key type.
        key: dir::GlobalTypeId,
        /// The receiver type.
        receiver: dir::GlobalTypeId,
    },
    /// An indexed access has a non-indexable receiver.
    InvalidIndexReceiver {
        /// The written index type expression.
        source: dir::GlobalNodeIdAny,
        /// The indexed receiver type.
        receiver: dir::GlobalTypeId,
    },
    /// An indexed access key does not project from its receiver.
    InvalidIndexKey {
        /// The written index type expression.
        source: dir::GlobalNodeIdAny,
        /// The indexed receiver type.
        receiver: dir::GlobalTypeId,
        /// The supplied key type.
        key: dir::GlobalTypeId,
    },
    /// A written placement conflicts with a nominal declaration's concrete space.
    ConflictingDeclarationPlacement {
        /// The written placed type expression.
        source: dir::GlobalNodeIdAny,
        /// The nominal declaration with intrinsic placement.
        symbol: dir::GlobalSymbolId,
        /// The written space.
        written: dir::Space,
        /// The declaration's effective space.
        declared: dir::Space,
    },
    /// A value cannot be assigned to an imported binding.
    CannotAssignImportedBinding {
        /// The assignment target expression.
        source: dir::GlobalNodeIdAny,
        /// The imported binding symbol.
        symbol: dir::GlobalSymbolId,
    },
    /// A value cannot be assigned to an immutable binding.
    CannotAssignImmutableBinding {
        /// The assignment target expression.
        source: dir::GlobalNodeIdAny,
        /// The immutable binding symbol.
        symbol: dir::GlobalSymbolId,
    },
    /// A value cannot be written through a readonly member.
    CannotAssignReadonlyMember {
        /// The assignment target expression.
        source: dir::GlobalNodeIdAny,
        /// The selected member.
        member: dir::ProjectionField,
    },
    /// A non-exclusive overwrite requires overwrite-stable values.
    OverwriteStabilityNotSatisfied {
        /// The assignment target expression.
        source: dir::GlobalNodeIdAny,
        /// The overwritten value type.
        ty: dir::GlobalTypeId,
    },
    /// A type does not have finite by-value storage.
    CircularType {
        /// The source exposing the cycle.
        source: dir::GlobalNodeIdAny,
    },
    /// Shared storage contains a safe reference into local storage.
    LocalReferenceInSharedStorage {
        /// The stored type or field retaining the local reference.
        source: dir::GlobalNodeIdAny,
    },
    /// A type does not satisfy a compiler-known interface.
    AutoInterfaceNotSatisfied {
        /// The source requiring the interface.
        source: dir::GlobalNodeIdAny,
        /// The checked type.
        ty: dir::GlobalTypeId,
        /// The required interface.
        interface: dir::AutoInterface,
    },
    /// An extension does not implement one declared interface.
    InterfaceNotImplemented {
        /// The implementation clause source.
        source: dir::GlobalNodeIdAny,
        /// The implementing type.
        ty: dir::GlobalTypeId,
        /// The required interface instantiation.
        interface: dir::GlobalTypeId,
    },
    /// An implementation pair is outside both relevant packages.
    NonLocalImplementation {
        /// The extension declaration source.
        source: dir::GlobalNodeIdAny,
        /// The implemented interface.
        interface: dir::GlobalSymbolId,
        /// The implemented type symbol.
        ty: dir::GlobalSymbolId,
    },
    /// A blanket implementation is outside the interface package.
    ForeignBlanketImplementation {
        /// The extension declaration source.
        source: dir::GlobalNodeIdAny,
        /// The implemented interface.
        interface: dir::GlobalSymbolId,
    },
    /// An exported nonlocal extension has no source-level name.
    UnnamedExportedNonlocalExtension {
        /// The extension declaration source.
        source: dir::GlobalNodeIdAny,
        /// The extension target type.
        target: dir::GlobalTypeId,
    },
    /// An implementation overlaps another visible implementation.
    ConflictingImplementation {
        /// The extension declaration source.
        source: dir::GlobalNodeIdAny,
        /// The conflicting extension symbol.
        conflict: dir::GlobalSymbolId,
        /// The implemented interface.
        interface: dir::GlobalSymbolId,
        /// The implemented type.
        ty: dir::GlobalTypeId,
    },
    /// Heritage reaches the same declaration with incompatible arguments.
    ConflictingHeritage {
        /// The conflicting heritage clause source.
        source: dir::GlobalNodeIdAny,
        /// The declaration whose heritage is invalid.
        symbol: dir::GlobalSymbolId,
        /// The repeated heritage target.
        target: dir::GlobalSymbolId,
    },
    /// Heritage reaches its own declaration again.
    CircularHeritage {
        /// The heritage clause exposing the cycle.
        source: dir::GlobalNodeIdAny,
        /// The declaration whose heritage is circular.
        symbol: dir::GlobalSymbolId,
    },
    /// Heritage declarations impose incompatible concrete spaces.
    ConflictingHeritagePlacement {
        /// The first placement requirement clause.
        source: dir::GlobalNodeIdAny,
        /// The declaration imposing the first placement.
        symbol: dir::GlobalSymbolId,
        /// The conflicting placement requirement clause.
        conflict_source: dir::GlobalNodeIdAny,
        /// The declaration imposing the conflicting placement.
        conflict: dir::GlobalSymbolId,
    },
    /// Override modifier appears without an inherited member.
    InvalidOverride {
        /// The member declaration source.
        source: dir::GlobalNodeIdAny,
        /// The member key.
        member: dir::StaticKey,
    },
    /// Override target is not virtual or abstract.
    OverrideNotVirtual {
        /// The member declaration source.
        source: dir::GlobalNodeIdAny,
        /// The member key.
        member: dir::StaticKey,
    },
    /// Override type is not assignable to the inherited member type.
    IncompatibleOverride {
        /// The member declaration source.
        source: dir::GlobalNodeIdAny,
        /// The member key.
        member: dir::StaticKey,
        /// The overriding member type.
        source_ty: dir::GlobalTypeId,
        /// The inherited member type.
        target_ty: dir::GlobalTypeId,
    },
    /// Inherited member is shadowed without override.
    MissingOverride {
        /// The member declaration source.
        source: dir::GlobalNodeIdAny,
        /// The member key.
        member: dir::StaticKey,
    },
    /// Concrete class declares an abstract member.
    AbstractMemberInConcreteClass {
        /// The member declaration source.
        source: dir::GlobalNodeIdAny,
        /// The member key.
        member: dir::StaticKey,
    },
    /// Concrete class does not implement one inherited abstract member.
    UnimplementedAbstractMember {
        /// The class declaration source.
        source: dir::GlobalNodeIdAny,
        /// The member key.
        member: dir::StaticKey,
    },
    /// Class extends a final base class.
    FinalClassExtended {
        /// The class declaration source.
        source: dir::GlobalNodeIdAny,
        /// The final base symbol.
        base: dir::GlobalSymbolId,
    },
    /// Class field is not definitely initialized.
    FieldNotDefinitelyInitialized {
        /// The field declaration source.
        source: dir::GlobalNodeIdAny,
        /// The field symbol.
        field: dir::GlobalSymbolId,
    },
    /// Declared generic parameter never occurs in its definition.
    UnusedGenericParameter {
        /// The parameter declaration source.
        source: dir::GlobalNodeIdAny,
        /// The unused parameter symbol.
        parameter: dir::GlobalSymbolId,
    },
    /// Declared variance conflicts with the parameter's derived use.
    VarianceConflict {
        /// The parameter declaration source.
        source: dir::GlobalNodeIdAny,
        /// The conflicting parameter symbol.
        parameter: dir::GlobalSymbolId,
        /// The derived variance the declaration must admit.
        derived: Variance,
        /// The declared variance modifier.
        declared: dir::VarianceModifier,
    },
}

/// Representative value left uncovered by pattern coverage.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum UncoveredValue {
    /// A type-shaped uncovered value.
    Type(dir::GlobalTypeId),
    /// A tagged case uncovered value.
    VariantCase {
        /// The matched tagged type.
        ty: dir::GlobalTypeId,
        /// The uncovered case key.
        key: dir::StaticKey,
    },
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
    /// The matched value type.
    pub(in crate::check) value: ExpectedType,
    /// The pattern coverage shape.
    pub(in crate::check) coverage: PatternCoverage,
}

/// A pattern-bearing site whose coverage must be checked.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum PatternCoverage {
    /// A match expression whose active arms must be exhaustive.
    Match {
        /// The active match arms in source order.
        arms: Vec<PatternArm>,
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

/// Obliges an assignment target to accept writes.
///
/// ```ds
/// value = 2;
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct WritablePlaceObligation {
    /// The place being written.
    pub(in crate::check) place: WriteTarget,
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
    /// The checked declaration symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
}

/// Obliges a class to initialize required fields before construction completes.
///
/// ```ds
/// class User {
///     name: string;
///     constructor(name: string) { this.name = name; }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ClassInitializationObligation {
    /// The class declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The checked class symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The constructed receiver type.
    pub(in crate::check) receiver: dir::GlobalTypeId,
    /// The constructor completion branches.
    pub(in crate::check) constructor_branches: Vec<FlowBranch>,
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
    /// The type that must have one fixed representation.
    pub(in crate::check) ty: dir::GlobalTypeId,
}

/// Obliges a written type operation to be well-formed once its operands close.
///
/// ```ds
/// type Value = User["name"]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct WellFormedTypeObligation {
    /// The written type expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The interned operation type.
    pub(in crate::check) ty: dir::GlobalTypeId,
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
    /// The checked extension symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
}

/// Obliges a declaration's generic parameters to occur in its definition.
///
/// ```ds
/// class Box<T> { value: T }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ParameterUseObligation {
    /// The declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The checked declaration symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
}

/// One collected obligation with its assuming scope.
#[derive(Debug, Clone)]
pub(in crate::check) struct ObligationEntry {
    /// The obligation to check.
    pub(in crate::check) obligation: Obligation,
    /// The generic template whose predicates the check assumes.
    pub(in crate::check) scope: Option<GenericTemplateId>,
}

/// Collected obligations in allocation order.
#[derive(Debug, Clone)]
pub(in crate::check) struct ObligationTable {
    /// The collected obligations indexed by obligation id.
    obligations: Vec<ObligationEntry>,
}

impl ObligationTable {
    /// Create an empty obligation table.
    pub(in crate::check) fn new() -> Self {
        Self {
            obligations: Vec::new(),
        }
    }

    /// Append one obligation at the next id.
    pub(in crate::check) fn insert(&mut self, id: ObligationId, entry: ObligationEntry) {
        debug_assert_eq!(self.obligations.len(), id.index());
        self.obligations.push(entry);
    }

    /// Truncate obligations undone by one probe rollback.
    pub(in crate::check) fn truncate(&mut self, count: usize) {
        self.obligations.truncate(count);
    }

    /// Return one collected obligation.
    pub(in crate::check) fn get(&self, id: ObligationId) -> CompilerResult<&ObligationEntry> {
        self.obligations
            .get(id.index())
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
    /// Collect one obligation under one assuming scope.
    pub(in crate::check) fn push_obligation(
        &mut self,
        obligation: Obligation,
        scope: Option<GenericTemplateId>,
    ) -> ObligationId {
        let entry = ObligationEntry { obligation, scope };
        let id = self.solver.allocate_obligation(entry);
        self.queue_task(Task::Oblige(id));

        id
    }

    /// Check one obligation once, returning its failed judgments.
    pub(in crate::check) fn run_obligation(
        &mut self,
        id: ObligationId,
    ) -> CompilerResult<Answer<TaskFailures>> {
        // copy the obligation for the borrow-free check
        let entry = self.solver.obligations.get(id)?.clone();
        let origin = Origin::Node(entry.obligation.source(), entry.scope);
        let obligation = entry.obligation;
        let check = self.check_obligation(origin, &obligation)?;

        match check {
            Answer::Ready(check) => {
                let failures = check
                    .into_failures()
                    .into_iter()
                    .map(TaskFailure::Obligation)
                    .collect();
                self.record_event(CheckEvent::ObligationChecked {
                    obligation: id,
                    is_finished: true,
                });

                Ok(Answer::Ready(failures))
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
        origin: Origin,
        obligation: &Obligation,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        match obligation {
            Obligation::PatternCoverage(obligation) => {
                self.check_pattern_coverage(origin, obligation)
            }
            Obligation::WritablePlace(obligation) => self.check_writable_place(origin, obligation),
            Obligation::Representation(obligation) => {
                self.check_representation(origin, obligation.ty)
            }
            Obligation::RuntimePredicate(obligation) => {
                self.check_runtime_predicate(origin, obligation)
            }
            Obligation::ForInSource(obligation) => self.check_for_in_source(origin, obligation),
            Obligation::ExtensionConformance(obligation) => {
                self.check_extension_conformance(origin, obligation.symbol)
            }
            Obligation::ImplementationCoherence(obligation) => {
                self.check_implementation_coherence(origin, obligation.symbol)
            }
            Obligation::DeclarationHeritage(obligation) => {
                self.check_declaration_heritage(origin, obligation.symbol)
            }
            Obligation::ClassInitialization(obligation) => {
                self.check_class_initialization(origin, obligation)
            }
            Obligation::WellFormedType(obligation) => {
                self.check_well_formed_type(origin, obligation)
            }
            Obligation::ParameterUse(obligation) => self.check_parameter_use(obligation.symbol),
        }
    }

    /// Check one pattern coverage obligation.
    fn check_pattern_coverage(
        &mut self,
        origin: Origin,
        obligation: &PatternCoverageObligation,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        // bodies commit their nodes before obligations read them
        let value = match obligation.value {
            ExpectedType::Type(ty) => ty,
            ExpectedType::Node(site) => {
                let ty = self.require_node_type(site.node)?;

                answer!(self.flow_type_at(site, ty)?)
            }
        };

        match &obligation.coverage {
            PatternCoverage::Match { arms } => {
                self.check_match_exhaustive(origin, obligation.source, value, arms)
            }
            PatternCoverage::Binding { pattern } => self.check_irrefutable_pattern(
                origin,
                obligation.source,
                *pattern,
                value,
                |source, missing| ObligationFailure::RefutablePattern { source, missing },
            ),
            PatternCoverage::Catch { pattern } => self.check_irrefutable_pattern(
                origin,
                obligation.source,
                *pattern,
                value,
                |source, missing| ObligationFailure::RefutableCatchPattern { source, missing },
            ),
        }
    }

    /// Check one for-in source obligation.
    fn check_for_in_source(
        &mut self,
        origin: Origin,
        obligation: &ForInSourceObligation,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        if answer!(self.is_keyed_type(origin, obligation.ty)?) {
            return Ok(Answer::Ready(ObligationCheck::holds()));
        }

        let failure = ObligationFailure::ForInSourceNotObjectShaped {
            source: obligation.source,
        };

        Ok(Answer::Ready(ObligationCheck::fail(failure)))
    }
}

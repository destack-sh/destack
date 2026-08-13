use destack_dir as dir;
use smallvec::SmallVec;

use crate::{CompilerError, CompilerResult};

use crate::check::{
    AssignmentSelection, CheckEvent, CheckState, ExpectedType, GenericTemplateId, MoveSite, Origin,
    PendingWork, Variance,
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
    /// A moved place must be copyable to stay readable.
    UseAfterMove(UseAfterMoveObligation),
    /// An assignment must select a writable target.
    WritableTarget(Box<WritableTargetObligation>),
    /// A runtime predicate must have valid operands.
    RuntimePredicate(Box<RuntimePredicateObligation>),
    /// A for-in source must be enumerable.
    ForInSource(ForInSourceObligation),
    /// A class must initialize required fields on every constructor path.
    ClassInitialization(ClassInitializationObligation),
    /// A written type operation must be well-formed once its operands close.
    WellFormedType(WellFormedTypeObligation),
}

/// When one obligation may judge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ObligationPhase {
    /// Solves control-flow holes as soon as its node checks.
    Produce,
    /// Judges finished bodies at the final round, like borrowck.
    Judge,
}

impl Obligation {
    /// Return when this obligation may judge.
    pub(in crate::check) fn phase(&self) -> ObligationPhase {
        match self {
            Self::PatternCoverage(_) | Self::RuntimePredicate(_) | Self::ForInSource(_) => {
                ObligationPhase::Produce
            }
            Self::UseAfterMove(_)
            | Self::WritableTarget(_)
            | Self::ClassInitialization(_)
            | Self::WellFormedType(_) => ObligationPhase::Judge,
        }
    }

    /// Return the source node anchoring diagnostics.
    pub(in crate::check) fn source(&self) -> dir::GlobalNodeIdAny {
        match self {
            Self::PatternCoverage(obligation) => obligation.source,
            Self::UseAfterMove(obligation) => obligation.source,
            Self::WritableTarget(obligation) => obligation.target.source,
            Self::RuntimePredicate(obligation) => obligation.source,
            Self::ForInSource(obligation) => obligation.source,
            Self::ClassInitialization(obligation) => obligation.source,
            Self::WellFormedType(obligation) => obligation.source,
        }
    }

    /// Return the judged operand types stored on this obligation.
    pub(in crate::check) fn operand_types(&self) -> SmallVec<[dir::GlobalTypeId; 2]> {
        match self {
            Self::PatternCoverage(obligation) => match obligation.value {
                ExpectedType::Type(ty) => SmallVec::from_slice(&[ty]),
                ExpectedType::Node(_) => SmallVec::new(),
            },
            Self::WritableTarget(obligation) => SmallVec::from_slice(&[obligation.ty]),
            Self::ForInSource(obligation) => SmallVec::from_slice(&[obligation.ty]),
            Self::WellFormedType(obligation) => SmallVec::from_slice(&[obligation.ty]),
            Self::ClassInitialization(obligation) => SmallVec::from_slice(&[obligation.receiver]),
            Self::UseAfterMove(_) | Self::RuntimePredicate(_) => SmallVec::new(),
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
    /// The obligation stays undecided while its variables are open.
    Ambiguous(SmallVec<[dir::TypeVariableId; 2]>),
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
            Self::Holds | Self::Ambiguous(_) => Vec::new(),
            Self::Fails(failures) => failures,
        }
    }
}

/// Reason one completed obligation failed.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum ObligationFailure {
    /// A moved non-copyable place was read.
    UseAfterMove {
        /// The reading source node.
        source: dir::GlobalNodeIdAny,
        /// The moved place symbol.
        symbol: dir::GlobalSymbolId,
    },
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
        member: dir::MemberTarget,
    },
    /// A computed key cannot be assigned through a structural signature.
    CannotAssignStructuralIndex {
        /// The assignment target expression.
        source: dir::GlobalNodeIdAny,
        /// The structural receiver type.
        receiver: dir::GlobalTypeId,
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
    /// An extension redeclares a property another visible extension declares.
    DuplicateExtensionMember {
        /// The declaring member source node.
        source: dir::GlobalNodeIdAny,
        /// The duplicated member key.
        member: dir::StaticKey,
        /// The formatted extended target.
        target: String,
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
pub(in crate::check) struct WritableTargetObligation {
    /// The selected assignment target.
    pub(in crate::check) target: AssignmentSelection,
    /// The type being overwritten.
    pub(in crate::check) ty: dir::GlobalTypeId,
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
}

/// Obliges one read of a moved place to a value the move left intact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct UseAfterMoveObligation {
    /// The reading source node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The moved place symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The syntactic position that marked the move.
    pub(in crate::check) site: MoveSite,
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
    pub(in crate::check) predicate: dir::GuardDecision,
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

/// Obliges a declaration to satisfy every declared interface.
///
/// ```ds
/// struct Point implements Display { toString(): string }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct InterfaceConformanceObligation {
    /// The implementing declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The implementing declaration symbol.
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

/// Obliges an extension to leave other visible extensions' properties alone.
///
/// ```ds
/// extension of User { greeting(this): string { "hello" } }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct ExtensionCoherenceObligation {
    /// The extension declaration node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The checked extension symbol.
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
        let id = self.fulfill.allocate_obligation(entry);
        self.fulfill.register_work(PendingWork::Obligation(id));

        id
    }

    /// Check one obligation once, returning its failures.
    pub(in crate::check) fn run_obligation(
        &mut self,
        id: ObligationId,
    ) -> CompilerResult<Option<SmallVec<[dir::TypeVariableId; 2]>>> {
        // body obligations judge checked nodes: only check runs them
        if !self.is_checking() {
            return Ok(None);
        }

        // copy the obligation for the borrow-free check
        let entry = self.fulfill.obligations.get(id)?.clone();
        let origin = Origin::Node(entry.obligation.source(), entry.scope);
        let obligation = entry.obligation;
        let check = self.check_obligation(origin, &obligation)?;

        // stall the obligation while its variables stay open
        if let ObligationCheck::Ambiguous(stalls) = check {
            return Ok(Some(stalls));
        }

        // report every failure the check produced, then record the result
        for failure in check.into_failures() {
            self.report_obligation_failure(failure)?;
        }
        self.record_event(CheckEvent::ObligationChecked {
            obligation: id,
            is_finished: true,
        });

        Ok(None)
    }

    /// Check one obligation against solved inputs.
    fn check_obligation(
        &mut self,
        origin: Origin,
        obligation: &Obligation,
    ) -> CompilerResult<ObligationCheck> {
        // hold without checking once an operand already reported an error
        if self.any_error_operand(&obligation.operand_types())? {
            return Ok(ObligationCheck::holds());
        }

        match obligation {
            Obligation::PatternCoverage(obligation) => {
                self.check_pattern_coverage(origin, obligation)
            }
            Obligation::UseAfterMove(obligation) => self.check_use_after_move(origin, *obligation),
            Obligation::WritableTarget(obligation) => {
                self.check_writable_assignment(origin, obligation)
            }
            Obligation::RuntimePredicate(obligation) => {
                self.check_runtime_predicate(origin, obligation)
            }
            Obligation::ForInSource(obligation) => self.check_for_in_source(origin, obligation),
            Obligation::ClassInitialization(obligation) => {
                self.check_class_initialization(origin, obligation)
            }
            Obligation::WellFormedType(obligation) => {
                self.check_well_formed_type(origin, obligation)
            }
        }
    }

    /// Check one read of a moved place: copyable values read from a copy.
    fn check_use_after_move(
        &mut self,
        origin: Origin,
        obligation: UseAfterMoveObligation,
    ) -> CompilerResult<ObligationCheck> {
        let ty = self.symbol_type(obligation.symbol)?;

        // a once callable is consumed by the call that invoked it, whatever its ownership admits
        let survives = if self.consumes_once_callable(origin, &obligation.site, ty)? {
            false
        }
        // positions that preserve their source read the value in place
        else if self.move_site_borrows(origin, &obligation.site)? {
            true
        }
        // require an owned value, managed handles copy freely
        else if self.default_ownership(origin, ty)? != Some(dir::Ownership::Owned) {
            true
        }
        // only copyable values survive a use after their move
        else {
            self.satisfies_auto_interface(origin, ty, dir::AutoInterface::Copy)?
        };
        if survives {
            return Ok(ObligationCheck::Holds);
        }

        Ok(ObligationCheck::from_failures(vec![
            ObligationFailure::UseAfterMove {
                source: obligation.source,
                symbol: obligation.symbol,
            },
        ]))
    }

    /// Return whether one marked move position is a call consuming its once callable.
    fn consumes_once_callable(
        &mut self,
        origin: Origin,
        site: &MoveSite,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        if !self.is_callee_position(site) {
            return Ok(false);
        }
        let ty = self.normalize(origin, ty)?;
        let dir::Type::Function(function) = self.ty(ty)? else {
            return Ok(false);
        };

        Ok(function.multiplicity == dir::Multiplicity::Once)
    }

    /// Return whether one marked move position is the callee of its call.
    fn is_callee_position(&self, site: &MoveSite) -> bool {
        let Some(call) = site.call else {
            return false;
        };
        let node = self
            .module(call.module_id)
            .view()
            .get(call.local_id.into_typed::<dir::Expression>());

        matches!(node, dir::Expression::Call { left, .. } if left.into_global_any(call.module_id) == site.node)
    }

    /// Return whether one marked move position borrows its source.
    fn move_site_borrows(&mut self, origin: Origin, site: &MoveSite) -> CompilerResult<bool> {
        // argument and receiver positions read the selected call
        if let Some(call) = site.call {
            // rejected calls reported their own diagnostics and move nothing
            if matches!(
                self.decision(call),
                Some(dir::Decision::Rejected | dir::Decision::Poisoned)
            ) {
                return Ok(true);
            }

            // every call but a once call reads its callee place in place
            if self.is_callee_position(site) {
                return Ok(true);
            }

            let decisions = self.decisions(call.module_id);
            if let Some(resolution) = decisions.call_decision(call).cloned() {
                return self.call_position_borrows(origin, &resolution, site.node);
            }
            if let Some(resolution) = decisions.construct_decision(call) {
                // lend constructor arguments through borrowing parameters
                let binding = resolution.arguments.iter().find(|binding| {
                    matches!(
                        binding.source,
                        dir::ArgumentSource::Provided(provided) if provided == site.node
                    )
                });
                let Some(binding) = binding else {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "construct resolution {call:?} has no binding for move site {:?}",
                            site.node
                        ),
                    });
                };

                return self.type_head_borrows(origin, binding.argument_type);
            }

            return Err(CompilerError::Internal {
                message: format!(
                    "move site {} has no resolution for {}, decided as {:?}",
                    self.node_label(site.node),
                    self.node_label(call),
                    self.decision(call),
                ),
            });
        }

        // lend initializer and assignment positions into borrow bindings
        if let Some(target) = site.target {
            let ty = self.symbol_type(target)?;

            return self.type_head_borrows(origin, ty);
        }

        Ok(false)
    }

    /// Return whether one call position borrows its source in every runtime alternative.
    fn call_position_borrows(
        &mut self,
        origin: Origin,
        resolution: &dir::CallDecision,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<bool> {
        match resolution {
            dir::OperationResolution::One(call) => {
                self.call_arm_position_borrows(origin, call, source)
            }
            dir::OperationResolution::Union { arms, .. } => {
                for call in arms {
                    if !self.call_arm_position_borrows(origin, call, source)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
        }
    }

    /// Return whether one singular call position borrows its source.
    fn call_arm_position_borrows(
        &mut self,
        origin: Origin,
        call: &dir::Call,
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<bool> {
        // borrow an argument according to its selected parameter
        if let Some(binding) = call.arguments.iter().find(|binding| {
            matches!(
                binding.source,
                dir::ArgumentSource::Provided(provided) if provided == source
            )
        }) {
            return self.type_head_borrows(origin, binding.argument_type);
        }

        // treat the remaining marked position as the selected receiver
        let receiver = match &call.target {
            dir::CallTarget::Expression { .. } => return Ok(true),
            dir::CallTarget::Symbol { function, .. } => function.receiver.as_ref(),
            dir::CallTarget::Dynamic { dispatch, .. } => Some(&dispatch.receiver),
        };
        let Some(receiver) = receiver else {
            return Err(CompilerError::Internal {
                message: format!("static call has no binding for move site {source:?}"),
            });
        };
        let borrows = receiver
            .adjustments
            .iter()
            .any(|adjustment| matches!(adjustment, dir::ReceiverAdjustment::Borrow { .. }));

        Ok(borrows)
    }

    /// Return whether one type lends by default beneath its placement.
    fn type_head_borrows(&mut self, origin: Origin, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        let ownership = self.default_ownership(origin, ty)?;

        Ok(ownership == Some(dir::Ownership::Borrowed))
    }

    /// Check one pattern coverage obligation.
    fn check_pattern_coverage(
        &mut self,
        origin: Origin,
        obligation: &PatternCoverageObligation,
    ) -> CompilerResult<ObligationCheck> {
        // wait for authored values still undergoing contextual checking
        let value = match obligation.value {
            ExpectedType::Type(ty) => ty,
            ExpectedType::Node(node) => {
                let ty = self.node_type(node)?;
                let site = self.node_site(node)?;

                self.flow_type_at(site, ty)?
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
    ) -> CompilerResult<ObligationCheck> {
        if self.is_keyed_type(origin, obligation.ty)? {
            return Ok(ObligationCheck::holds());
        }

        let failure = ObligationFailure::ForInSourceNotObjectShaped {
            source: obligation.source,
        };

        Ok(ObligationCheck::fail(failure))
    }
}

use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;

use crate::sema::{
    AssignmentSelection, Check, CheckEvent, CheckId, CheckState, ExpectedType, GenericTemplateId,
    Origin, Variance,
};

/// One active match arm used for coverage.
///
/// Examples:
/// ```ds
/// match (value) { _ => value }
/// match (value) { Some(item) => item }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::sema) struct PatternArm {
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
pub(in crate::sema) enum Obligation {
    /// A site's patterns must cover the matched value space.
    PatternCoverage(PatternCoverageObligation),
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
    /// A range's written endpoints share one element type.
    RangeElement(RangeElementObligation),
}

/// When one obligation may judge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum ObligationPhase {
    /// Solves control-flow holes as soon as its node checks.
    Produce,
    /// Judges finished bodies at the final round, like borrowck.
    Judge,
}

impl Obligation {
    /// Return when this obligation may judge.
    pub(in crate::sema) fn phase(&self) -> ObligationPhase {
        match self {
            Self::PatternCoverage(_)
            | Self::RuntimePredicate(_)
            | Self::ForInSource(_)
            | Self::RangeElement(_) => ObligationPhase::Produce,
            Self::WritableTarget(_) | Self::ClassInitialization(_) | Self::WellFormedType(_) => {
                ObligationPhase::Judge
            }
        }
    }

    /// Return the source node anchoring diagnostics.
    pub(in crate::sema) fn source(&self) -> dir::GlobalNodeIdAny {
        match self {
            Self::PatternCoverage(obligation) => obligation.source,
            Self::WritableTarget(obligation) => obligation.target.source,
            Self::RuntimePredicate(obligation) => obligation.source,
            Self::ForInSource(obligation) => obligation.source,
            Self::ClassInitialization(obligation) => obligation.source,
            Self::WellFormedType(obligation) => obligation.source,
            Self::RangeElement(obligation) => obligation.source,
        }
    }

    /// Return the judged operand types stored on this obligation.
    pub(in crate::sema) fn operand_types(&self) -> SmallVec<[dir::GlobalTypeId; 2]> {
        match self {
            Self::PatternCoverage(obligation) => match obligation.value {
                ExpectedType::Type(ty) => SmallVec::from_slice(&[ty]),
                ExpectedType::Node(_) => SmallVec::new(),
            },
            Self::WritableTarget(obligation) => SmallVec::from_slice(&[obligation.ty]),
            Self::ForInSource(obligation) => SmallVec::from_slice(&[obligation.ty]),
            Self::WellFormedType(obligation) => SmallVec::from_slice(&[obligation.ty]),
            Self::ClassInitialization(obligation) => SmallVec::from_slice(&[obligation.receiver]),
            Self::RangeElement(obligation) => SmallVec::from_slice(&[obligation.element]),
            Self::RuntimePredicate(_) => SmallVec::new(),
        }
    }
}

/// Result of checking one obligation.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) enum ObligationCheck {
    /// The obligation holds.
    Holds,
    /// The obligation failed for one or more known reasons.
    Fails(Vec<ObligationFailure>),
    /// The obligation stays undecided while its variables are open.
    Ambiguous(SmallVec<[dir::TypeVariableId; 2]>),
}

impl ObligationCheck {
    /// Return a successful obligation check.
    pub(in crate::sema) fn holds() -> Self {
        Self::Holds
    }

    /// Return one failed obligation check.
    pub(in crate::sema) fn fail(failure: ObligationFailure) -> Self {
        Self::Fails(vec![failure])
    }

    /// Return an obligation check from collected failures.
    pub(in crate::sema) fn from_failures(failures: Vec<ObligationFailure>) -> Self {
        if failures.is_empty() {
            Self::Holds
        } else {
            Self::Fails(failures)
        }
    }

    /// Return the failed reasons.
    pub(in crate::sema) fn into_failures(self) -> Vec<ObligationFailure> {
        match self {
            Self::Holds | Self::Ambiguous(_) => Vec::new(),
            Self::Fails(failures) => failures,
        }
    }
}

/// Reason one completed obligation failed.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) enum ObligationFailure {
    /// A match expression did not cover one remaining value.
    NonExhaustivePattern {
        /// The source holding the patterns.
        source: dir::GlobalNodeIdAny,
        /// A representative uncovered value.
        missing: UncoveredValue,
    },
    /// A non-matching binding pattern can reject one remaining value.
    RefutablePattern {
        /// The source holding the pattern.
        source: dir::GlobalNodeIdAny,
        /// A representative uncovered value.
        missing: UncoveredValue,
    },
    /// A catch binding pattern can reject one remaining failure value.
    RefutableCatchPattern {
        /// The source holding the pattern.
        source: dir::GlobalNodeIdAny,
        /// A representative uncovered value.
        missing: UncoveredValue,
    },
    /// A for-in source does not expose object keys.
    ForInSourceNotObjectShaped {
        /// The for-in expression.
        source: dir::GlobalNodeIdAny,
    },
    /// A range's written endpoints carry different types.
    IncompatibleRangeEndpoints {
        /// The range expression.
        source: dir::GlobalNodeIdAny,
        /// The unified element type of the written endpoints.
        element: dir::GlobalTypeId,
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
        /// The implemented root.
        root: dir::TypeRoot,
    },
    /// An extension member redeclares a member of the root declaration.
    InherentMemberRedeclared {
        /// The redeclaring member source.
        source: dir::GlobalNodeIdAny,
        /// The member key.
        member: dir::StaticKey,
        /// The rendered root declaration type.
        target: String,
    },
    /// A blanket implementation is outside the interface package.
    ForeignBlanketImplementation {
        /// The extension declaration source.
        source: dir::GlobalNodeIdAny,
        /// The implemented interface.
        interface: dir::GlobalSymbolId,
    },
    /// A blanket extension member implements no declared interface member.
    UnanchoredBlanketMember {
        /// The extension declaration source.
        source: dir::GlobalNodeIdAny,
        /// The unanchored member key.
        member: String,
    },
    /// Extension parameter left unconstrained by the target and its conformances.
    UnconstrainedExtensionParameter {
        /// The parameter declaration source.
        source: dir::GlobalNodeIdAny,
        /// The unconstrained parameter.
        parameter: String,
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
pub(in crate::sema) enum UncoveredValue {
    /// A type-shaped uncovered value.
    Type(dir::GlobalTypeId),
    /// An enum variant left uncovered.
    VariantCase {
        /// The matched enum type.
        ty: dir::GlobalTypeId,
        /// The uncovered case key.
        key: dir::StaticKey,
    },
}

/// Obliges one site's patterns to cover the matched value space.
///
/// ```ds
/// match (value) { true => 1, false => 0 }
/// let { name } = user;
/// try {} catch ({ message }) {}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) struct PatternCoverageObligation {
    /// The checked source.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The matched value type.
    pub(in crate::sema) value: ExpectedType,
    /// The pattern coverage shape.
    pub(in crate::sema) coverage: PatternCoverage,
}

/// The patterns one site covers its matched value with.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) enum PatternCoverage {
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
pub(in crate::sema) struct WritableTargetObligation {
    /// The selected assignment target.
    pub(in crate::sema) target: AssignmentSelection,
    /// The type being overwritten.
    pub(in crate::sema) ty: dir::GlobalTypeId,
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
pub(in crate::sema) struct ClassInitializationObligation {
    /// The class declaration node.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The checked class symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The constructed receiver type.
    pub(in crate::sema) receiver: dir::GlobalTypeId,
}

/// Obliges a written type operation to be well-formed once its operands close.
///
/// ```ds
/// type Value = User["name"]
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) struct WellFormedTypeObligation {
    /// The written type expression.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The interned operation type.
    pub(in crate::sema) ty: dir::GlobalTypeId,
}

/// Obliges a runtime predicate to be executable.
///
/// ```ds
/// value instanceof User
/// "name" in value
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) struct RuntimePredicateObligation {
    /// The predicate expression.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The left operand expression.
    pub(in crate::sema) left: dir::GlobalNodeIdAny,
    /// The right operand expression or type.
    pub(in crate::sema) right: dir::GlobalNodeIdAny,
    /// The selected predicate.
    pub(in crate::sema) predicate: dir::GuardDecision,
}

/// Obliges a for-in source to have enumerable string keys.
///
/// ```ds
/// for (const key in value) {}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) struct ForInSourceObligation {
    /// The for-in expression.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The source type that must be object-shaped.
    pub(in crate::sema) ty: dir::GlobalTypeId,
}

/// Obliges a range's written endpoints to share one element type.
///
/// ```ds
/// low..high
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) struct RangeElementObligation {
    /// The range expression.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The unified element type of the written endpoints.
    pub(in crate::sema) element: dir::GlobalTypeId,
}

/// Obliges a declaration to satisfy every declared interface.
///
/// ```ds
/// struct Point implements Display { toString(): string }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) struct InterfaceConformanceObligation {
    /// The implementing declaration node.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The implementing declaration symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
}

/// Obliges an implementation to avoid overlapping conflicting implementations.
///
/// ```ds
/// extension of User implements Display {}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) struct ImplementationCoherenceObligation {
    /// The extension declaration node.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The checked extension symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
}

/// Obliges an extension to leave other visible extensions' properties alone.
///
/// ```ds
/// extension of User { greeting(this): string { "hello" } }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) struct ExtensionCoherenceObligation {
    /// The extension declaration node.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
    /// The checked extension symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
}

/// One collected obligation with its assuming scope.
#[derive(Debug, Clone)]
pub(in crate::sema) struct ObligationEntry {
    /// The obligation to check.
    pub(in crate::sema) obligation: Obligation,
    /// The generic template whose predicates the check assumes.
    pub(in crate::sema) scope: Option<GenericTemplateId>,
}

impl CheckState<'_> {
    /// Collect one obligation under one assuming scope.
    pub(in crate::sema) fn push_obligation(
        &mut self,
        obligation: Obligation,
        scope: Option<GenericTemplateId>,
    ) -> CompilerResult<CheckId> {
        let entry = ObligationEntry { obligation, scope };

        self.register_check(Check::Declared(entry))
    }

    /// Check one obligation once, returning its failures.
    pub(in crate::sema) fn run_obligation(
        &mut self,
        id: CheckId,
        entry: &ObligationEntry,
    ) -> CompilerResult<Option<SmallVec<[dir::TypeVariableId; 2]>>> {
        let origin = Origin::Node(entry.obligation.source(), entry.scope);
        let check = self.check_obligation(origin, &entry.obligation)?;

        // stall the obligation while its variables stay open
        if let ObligationCheck::Ambiguous(stalls) = check {
            return Ok(Some(stalls));
        }

        // report every failure the check produced, then record the result
        for failure in check.into_failures() {
            self.report_obligation_failure(failure)?;
        }
        self.record_event(CheckEvent::Checked {
            check: id,
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
            Obligation::RangeElement(obligation) => self.check_range_element(obligation),
        }
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

    /// Check one range element obligation.
    fn check_range_element(
        &mut self,
        obligation: &RangeElementObligation,
    ) -> CompilerResult<ObligationCheck> {
        // judge the element once inference solves every endpoint
        let stalls = self.open_type_variables([obligation.element])?;
        if !stalls.is_empty() {
            return Ok(ObligationCheck::Ambiguous(stalls));
        }

        // mismatched endpoints join into a union element
        let element = self.shallow_resolve(obligation.element)?;
        if !matches!(self.ty(element)?, dir::Type::Union(_)) {
            return Ok(ObligationCheck::holds());
        }

        let failure = ObligationFailure::IncompatibleRangeEndpoints {
            source: obligation.source,
            element,
        };

        Ok(ObligationCheck::fail(failure))
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

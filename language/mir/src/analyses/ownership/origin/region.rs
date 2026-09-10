use smallvec::SmallVec;

use crate::analyses::LoanId;
use crate::{
    BorrowedPath, Lifetime, LifetimeParameter, LifetimeSlot, LifetimeTerm, ReferenceKind, Storage,
    Type,
};

/// Lifetime region that keeps a borrowed value valid.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum Region {
    /// Global or static storage.
    Static,
    /// Explicit lifetime slot.
    Lifetime(LifetimeSlot),
    /// Managed storage, alive while reachable.
    Managed,
    /// Storage in the current function frame.
    Frame,
}

/// Origin that keeps a borrowed value valid.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Origin {
    /// The lifetime regions.
    regions: SmallVec<[Region; 2]>,
    /// Loans kept active by this value.
    pub(super) loans: SmallVec<[LoanId; 2]>,
}

impl Region {
    /// Return this region as a MIR lifetime.
    fn lifetime(&self) -> Lifetime {
        match self {
            Self::Static => Lifetime::static_storage(),
            Self::Lifetime(slot) => Lifetime::slot(slot.0),
            Self::Managed => Lifetime::managed(),
            Self::Frame => Lifetime::frame(),
        }
    }

    /// Return whether this region is local to the current function.
    fn is_local(&self) -> bool {
        match self {
            Self::Frame => true,
            Self::Static | Self::Lifetime(_) | Self::Managed => false,
        }
    }

    /// Return whether this region is covered by a required lifetime.
    fn is_covered_by(&self, required: &Lifetime, parameters: &[LifetimeParameter]) -> bool {
        required.accepts(&self.lifetime(), parameters)
    }
}

impl Origin {
    /// Create empty origin.
    pub(super) fn none() -> Self {
        Self::default()
    }

    /// Create origin from one origin.
    pub(super) fn one(origin: Region) -> Self {
        Self::new([origin])
    }

    /// Create origin from many regions.
    pub(super) fn new(regions: impl IntoIterator<Item = Region>) -> Self {
        let mut unique = SmallVec::new();

        // retain insertion order while removing duplicates
        for region in regions {
            if !unique.contains(&region) {
                unique.push(region);
            }
        }

        Self {
            regions: unique,
            loans: SmallVec::new(),
        }
    }

    /// Create origin from one MIR lifetime.
    pub(super) fn from_lifetime(lifetime: &Lifetime) -> Self {
        Self::new(lifetime.terms.iter().map(|term| match term {
            LifetimeTerm::Static => Region::Static,
            LifetimeTerm::Frame => Region::Frame,
            LifetimeTerm::Managed => Region::Managed,
            LifetimeTerm::Slot(slot) => Region::Lifetime(*slot),
            LifetimeTerm::Parameter(index) => {
                unreachable!("region parameter {index} of a type declaration reached a body")
            }
        }))
    }

    /// Derive a reference path's origin from its lifetime or implicit managed storage.
    pub(super) fn from_path(borrowed: &BorrowedPath) -> Self {
        match borrowed.kind {
            ReferenceKind::Managed if borrowed.lifetime.is_empty() => Self::one(Region::Managed),
            _ => Self::from_lifetime(&borrowed.lifetime),
        }
    }

    /// Create origin for one managed reference type.
    pub(super) fn from_managed(ty: &Type) -> Self {
        if ty
            .reference_storage()
            .and_then(Storage::heap_space)
            .is_none()
        {
            unreachable!("managed reference has non-heap storage")
        }
        ty.reference_lifetime()
            .filter(|lifetime| !lifetime.is_empty())
            .map(Self::from_lifetime)
            .unwrap_or_else(|| Self::one(Region::Managed))
    }

    /// Create origin for one reference-like type.
    pub(super) fn from_reference(ty: &Type) -> Self {
        match ty.reference_kind() {
            Some(ReferenceKind::Managed) => Self::from_managed(ty),
            Some(ReferenceKind::Borrowed) => ty
                .reference_lifetime()
                .filter(|lifetime| !lifetime.is_empty())
                .map(Self::from_lifetime)
                .unwrap_or_default(),
            Some(ReferenceKind::Unique) => Self::one(Region::Frame),
            None => Self::none(),
        }
    }

    /// Return this origin carrying additional loans.
    pub(super) fn with_loans(mut self, loans: &[LoanId]) -> Self {
        for &loan in loans {
            if !self.loans.contains(&loan) {
                self.loans.push(loan);
            }
        }
        self
    }

    /// Return whether this origin is empty.
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty() && self.loans.is_empty()
    }

    /// Return whether any region may escape the function.
    pub fn has_escaping_region(&self) -> bool {
        self.regions.iter().any(|region| !region.is_local())
    }

    /// Return whether any region is local to this function.
    pub fn has_local_region(&self) -> bool {
        self.regions.iter().any(Region::is_local)
    }

    /// Return whether this origin has a proven lifetime region.
    pub fn has_region(&self) -> bool {
        !self.regions.is_empty()
    }

    /// Return whether this origin satisfies a required MIR lifetime.
    pub fn is_covered_by(&self, required: &Lifetime, parameters: &[LifetimeParameter]) -> bool {
        self.regions
            .iter()
            .all(|region| region.is_covered_by(required, parameters))
    }

    /// Return whether this origin lives at least as long as `other`.
    pub fn outlives(&self, other: &Origin, parameters: &[LifetimeParameter]) -> bool {
        other.regions.is_empty()
            || (!self.regions.is_empty()
                && self.regions.iter().all(|region| {
                    other
                        .regions
                        .iter()
                        .all(|shorter| shorter.lifetime().accepts(&region.lifetime(), parameters))
                }))
    }

    /// Merge two origin sets.
    pub(super) fn merge(&self, other: &Self) -> Self {
        Self::new(self.regions.iter().chain(&other.regions).cloned())
            .with_loans(&self.loans)
            .with_loans(&other.loans)
    }

    /// Add one issuing loan.
    pub(super) fn with_loan(mut self, loan: LoanId) -> Self {
        if !self.loans.contains(&loan) {
            self.loans.push(loan);
        }

        self
    }

    /// Return loans retained by this origin.
    pub fn loans(&self) -> &[LoanId] {
        &self.loans
    }
}

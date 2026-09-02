use smallvec::SmallVec;

use crate::{Access, LocalNodeIdAny, Path, Place, Value};

/// Dense identity of one borrow loan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LoanId(u32);

impl LoanId {
    /// Return this identity's dense table index.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Loan definitions for one MIR function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoanTable {
    /// Loans in dense identity order.
    loans: Vec<Loan>,
    /// Root loans sorted by carrying SSA value.
    roots: Vec<Root>,
    /// Loans carried by structural paths inside SSA values.
    representations: Vec<Representation>,
}

/// One borrow loan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loan {
    /// The borrowed storage.
    target: LoanTarget,
    /// Access granted by the borrow.
    pub access: Access,
    /// The SSA value carrying the borrowed reference.
    pub representation: Value,
    /// Loans from which this loan was reborrowed.
    parents: SmallVec<[LoanId; 2]>,
    /// The operation that issued the loan.
    pub issued_at: LocalNodeIdAny,
}

/// Storage borrowed by one loan.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LoanTarget {
    /// One concrete MIR place.
    Place {
        /// The canonical borrowed place.
        place: Place,
        /// The value through which the storage was borrowed.
        source: Option<Value>,
    },
    /// One referent admitted by a function parameter path.
    Parameter { value: Value, path: Path },
}

/// One structural value path carrying a loan.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Representation {
    /// The SSA value carrying the loan.
    value: Value,
    /// The structural path inside the value.
    path: Path,
    /// The carried loan.
    loan: LoanId,
}

/// One root value carrying a loan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Root {
    /// The SSA value carrying the loan.
    value: Value,
    /// The carried loan.
    loan: LoanId,
}

impl LoanTable {
    /// Create an empty loan table for one function.
    pub(crate) fn new() -> Self {
        Self {
            loans: Vec::new(),
            roots: Vec::new(),
            representations: Vec::new(),
        }
    }

    /// Return the number of loans.
    pub fn len(&self) -> usize {
        self.loans.len()
    }

    /// Return whether this table contains no loans.
    pub fn is_empty(&self) -> bool {
        self.loans.is_empty()
    }

    /// Iterate every loan with its identity.
    pub fn iter(&self) -> impl Iterator<Item = (LoanId, &Loan)> {
        self.loans
            .iter()
            .enumerate()
            .map(|(index, loan)| (LoanId(index as u32), loan))
    }

    /// Return one loan by identity.
    pub fn get(&self, loan: LoanId) -> &Loan {
        self.loans
            .get(loan.index())
            .unwrap_or_else(|| unreachable!("missing loan {loan:?}"))
    }

    /// Return the loan carried by one exact value path.
    pub fn carried(&self, value: Value, path: &Path) -> Option<LoanId> {
        if path.is_root() {
            return self.root(value);
        }

        self.representations.iter().find_map(|representation| {
            (representation.value == value && &representation.path == path)
                .then_some(representation.loan)
        })
    }

    /// Return the root loan carried by one value.
    pub fn root(&self, value: Value) -> Option<LoanId> {
        let index = self
            .roots
            .binary_search_by_key(&value, |root| root.value)
            .ok()?;

        Some(self.roots[index].loan)
    }

    /// Return the active loan conflicting with a new loan.
    pub fn conflict(
        &self,
        loan: &Loan,
        active: &[LoanId],
        mut may_overlap: impl FnMut(&Place, &Place) -> bool,
    ) -> Option<LoanId> {
        active.iter().copied().find(|&current| {
            let current_loan = self.get(current);
            !loan.parents.contains(&current)
                && current_loan.may_overlap(loan, &mut may_overlap)
                && (current_loan.is_exclusive() || loan.is_exclusive())
        })
    }

    /// Insert one loan carried by one value path.
    pub(crate) fn insert(&mut self, value: Value, path: Path, loan: Loan) -> LoanId {
        let id = LoanId(self.loans.len() as u32);
        self.loans.push(loan);

        // retain the common root form in the sparse value index
        if path.is_root() {
            self.roots.push(Root { value, loan: id });
        }
        // retain uncommon structural paths sparsely
        else {
            self.representations.push(Representation {
                value,
                path,
                loan: id,
            });
        }

        id
    }

    /// Sort root loans for direct lookup.
    pub(crate) fn sort(&mut self) {
        self.roots.sort_unstable_by_key(|root| root.value);
    }

    /// Replace the parent loans of one loan.
    pub(crate) fn set_parents(&mut self, loan: LoanId, parents: impl IntoIterator<Item = LoanId>) {
        self.get_mut(loan).parents = parents.into_iter().collect();
    }

    /// Return the loan blocking a place change.
    pub fn blocking_change(
        &self,
        place: &Place,
        active: &[LoanId],
        mut may_overlap: impl FnMut(&Place, &Place) -> bool,
    ) -> Option<LoanId> {
        active
            .iter()
            .copied()
            .find(|loan| self.get(*loan).blocks(place, &mut may_overlap))
    }

    /// Return one mutable loan by identity.
    fn get_mut(&mut self, loan: LoanId) -> &mut Loan {
        self.loans
            .get_mut(loan.index())
            .unwrap_or_else(|| unreachable!("missing loan {loan:?}"))
    }
}

impl Loan {
    /// Create a loan over one concrete MIR place.
    pub fn new(
        place: Place,
        source: Option<Value>,
        access: Access,
        representation: Value,
        parents: impl IntoIterator<Item = LoanId>,
        issued_at: LocalNodeIdAny,
    ) -> Self {
        Self {
            target: LoanTarget::Place { place, source },
            access,
            representation,
            parents: parents.into_iter().collect(),
            issued_at,
        }
    }

    /// Create a loan admitted by one function parameter path.
    pub(crate) fn parameter(
        value: Value,
        path: Path,
        access: Access,
        issued_at: LocalNodeIdAny,
    ) -> Self {
        Self {
            target: LoanTarget::Parameter { value, path },
            access,
            representation: value,
            parents: SmallVec::new(),
            issued_at,
        }
    }

    /// Return the concrete borrowed place when this loan has one.
    pub fn place(&self) -> Option<&Place> {
        match &self.target {
            LoanTarget::Place { place, .. } => Some(place),
            LoanTarget::Parameter { .. } => None,
        }
    }

    /// Return the value through which concrete storage was borrowed.
    pub fn source(&self) -> Option<Value> {
        match self.target {
            LoanTarget::Place { source, .. } => source,
            LoanTarget::Parameter { .. } => None,
        }
    }

    /// Return loans from which this loan was reborrowed.
    pub fn parents(&self) -> &[LoanId] {
        &self.parents
    }

    /// Return whether this loan excludes overlapping access.
    pub fn is_exclusive(&self) -> bool {
        self.access.is_exclusive()
    }

    /// Return whether this loan may overlap another loan.
    fn may_overlap(
        &self,
        other: &Self,
        may_overlap: &mut impl FnMut(&Place, &Place) -> bool,
    ) -> bool {
        match (&self.target, &other.target) {
            (LoanTarget::Place { place: left, .. }, LoanTarget::Place { place: right, .. }) => {
                may_overlap(left, right)
            }
            (
                LoanTarget::Parameter {
                    value: left,
                    path: left_path,
                },
                LoanTarget::Parameter {
                    value: right,
                    path: right_path,
                },
            ) => {
                left == right && (left_path.contains(right_path) || right_path.contains(left_path))
            }
            (LoanTarget::Place { .. }, LoanTarget::Parameter { .. })
            | (LoanTarget::Parameter { .. }, LoanTarget::Place { .. }) => false,
        }
    }

    /// Return whether this loan blocks one concrete place change.
    fn blocks(&self, place: &Place, may_overlap: &mut impl FnMut(&Place, &Place) -> bool) -> bool {
        match &self.target {
            LoanTarget::Place {
                place: borrowed, ..
            } => may_overlap(borrowed, place),
            LoanTarget::Parameter { .. } => false,
        }
    }
}

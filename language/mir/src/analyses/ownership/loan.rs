use destack_core::BitSet;
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
        self.roots_of(value).next()
    }

    /// Iterate every root loan one value holds, one per reborrowed argument for a call result.
    pub fn roots_of(&self, value: Value) -> impl Iterator<Item = LoanId> + '_ {
        let start = self.roots.partition_point(|root| root.value < value);
        let end = self.roots.partition_point(|root| root.value <= value);

        self.roots[start..end].iter().map(|root| root.loan)
    }

    /// Return the active loan conflicting with a new loan.
    pub fn conflict(
        &self,
        loan: &Loan,
        active: &[LoanId],
        mut may_overlap: impl FnMut(&Place, &Place) -> bool,
        mut is_exclusive: impl FnMut(&Loan) -> bool,
    ) -> Option<LoanId> {
        // authorize reborrows through every ancestor loan, including cyclic loop origins
        let mut parents = BitSet::new(self.len());
        let mut pending = loan.parents.clone();
        while let Some(parent) = pending.pop() {
            if !parents.contains(parent.index()) {
                parents.insert(parent.index());
                pending.extend_from_slice(self.get(parent).parents());
            }
        }

        active.iter().copied().find(|&current| {
            let current_loan = self.get(current);
            !parents.contains(current.index())
                && current_loan.may_overlap(loan, &mut may_overlap)
                && (is_exclusive(current_loan) || is_exclusive(loan))
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

    /// Return whether this loan may write through its reference.
    pub fn writes(&self) -> bool {
        self.access.can_write()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestModule;

    /// Permit ancestral reborrows and reject an independent overlapping exclusive borrow.
    #[test]
    fn test_allow_ancestors_and_reject_overlapping_siblings() {
        let program = TestModule::new(
            r#"
type Pair { first: int32; second: int32; }
type Box { pair: Pair; }

function test(): void {
    local l0: Box

entry:
    v0: int32 = 7
    v1: Pair = aggregate (v0, v0)
    v2: Box = aggregate (v1)
    local.set l0, v2
    v3: ref<Box, borrowed, 'frame, mutable, frame> = local.address l0
    v4: ref<Pair, borrowed, 'frame, mutable, frame> = field.address v3, 0
    v5: ref<int32, borrowed, 'frame, mutable, frame> = field.address v4, 0
    v6: ref<int32, borrowed, 'frame, mutable, frame> = field.address v4, 1
    v7: ref<int32, borrowed, 'frame, mutable, frame> = field.address v4, 0
    v8: ref<int32, borrowed, 'frame, readonly, frame> = field.address v4, 0
    v9: ref<int32, borrowed, 'frame, readonly, frame> = field.address v4, 0
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let origins = analyses.origin(function, &program.tree);
        let constants = analyses.constant(function, &program.tree);
        let loans = origins.loans();
        let [
            root,
            parent,
            first,
            second,
            overlapping,
            reader,
            other_reader,
        ] = [3, 4, 5, 6, 7, 8, 9].map(|value| loans.root(Value(value)).unwrap());
        for (loan, active, expected) in [
            (first, vec![root, parent], None),
            (second, vec![first], None),
            (overlapping, vec![first], Some(first)),
            (reader, vec![first], Some(first)),
            (first, vec![reader], Some(reader)),
            (other_reader, vec![reader], None),
        ] {
            let actual = loans.conflict(
                loans.get(loan),
                &active,
                |left, right| left.may_overlap(right, &constants, function, &program.tree),
                Loan::writes,
            );

            assert_eq!(
                actual,
                expected,
                "borrow {:?}",
                loans.get(loan).representation
            );
        }
    }

    /// Permit reborrows from either selected parent and reject an overlapping sibling.
    #[test]
    fn test_allow_both_selected_reborrow_parents() {
        let program = TestModule::new(
            r#"
type Pair { first: int32; second: int32; }

function test(v0: boolean): void {
    local l0: Pair
    local l1: Pair

entry(v0: boolean):
    v1: int32 = 7
    v2: Pair = aggregate (v1, v1)
    local.set l0, v2
    local.set l1, v2
    v3: ref<Pair, borrowed, 'frame, mutable, frame> = local.address l0
    v4: ref<Pair, borrowed, 'frame, mutable, frame> = local.address l1
    v5: ref<Pair, borrowed, 'frame, mutable, frame> = select v0, v3, v4
    v6: ref<int32, borrowed, 'frame, mutable, frame> = field.address v5, 0
    v7: ref<int32, borrowed, 'frame, mutable, frame> = field.address v5, 0
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let origins = analyses.origin(function, &program.tree);
        let constants = analyses.constant(function, &program.tree);
        let loans = origins.loans();
        let [left, right, first, sibling] =
            [3, 4, 6, 7].map(|value| loans.root(Value(value)).unwrap());

        assert_eq!(loans.get(first).parents(), &[left, right]);
        for (active, expected) in [(vec![left, right], None), (vec![first], Some(first))] {
            let actual = loans.conflict(
                loans.get(sibling),
                &active,
                |left, right| left.may_overlap(right, &constants, function, &program.tree),
                Loan::writes,
            );

            assert_eq!(actual, expected);
        }
    }

    /// Permit a loop reborrow whose ancestry includes its previous iteration.
    #[test]
    fn test_allow_reborrows_through_cyclic_ancestry() {
        let program = TestModule::new(
            r#"
external function identity<'a>(ref<int32, borrowed, 'a, mutable, frame>): ref<int32, borrowed, 'a, mutable, frame>

function test(v0: boolean): void {
    local l0: int32

entry(v0: boolean):
    v1: int32 = 7
    local.set l0, v1
    v2: ref<int32, borrowed, 'frame, mutable, frame> = local.address l0
    jump loop(v2)

loop(v3: ref<int32, borrowed, 'frame, mutable, frame>):
    v4: ref<int32, borrowed, 'frame, mutable, frame> = call identity(v3): <'frame>(ref<int32, borrowed, 'frame, mutable, frame>) => ref<int32, borrowed, 'frame, mutable, frame>
    branch v0 => loop(v4) | exit

exit:
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let origins = analyses.origin(function, &program.tree);
        let constants = analyses.constant(function, &program.tree);
        let loans = origins.loans();
        let root = loans.root(Value(2)).unwrap();
        let reborrow = loans.root(Value(4)).unwrap();

        assert_eq!(loans.get(reborrow).parents(), &[root, reborrow]);
        let conflict = loans.conflict(
            loans.get(reborrow),
            &[root, reborrow],
            |left, right| left.may_overlap(right, &constants, function, &program.tree),
            Loan::writes,
        );

        assert_eq!(conflict, None);
    }
}

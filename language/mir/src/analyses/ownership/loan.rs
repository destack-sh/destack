use smallvec::SmallVec;
use tspp_core::BitSet;

use crate::{
    Access, ConstantTable, FunctionId, LocalNodeIdAny, Path, Place, PlaceTable, StorageSet, Tree,
    Value,
};

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

/// The storage one access touches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessTarget {
    /// One precise place.
    Place(Place),
    /// Any storage of some spaces an unrelated operation may access.
    Spaces(StorageSet),
}

/// Storage borrowed by one loan.
#[derive(Debug, Clone, PartialEq, Eq)]
enum LoanTarget {
    /// One concrete MIR place.
    Place {
        /// The canonical borrowed place.
        place: Place,
        /// The storage of the reference through which this loan was borrowed.
        source: Option<Place>,
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

    /// Append unseen ancestors to the given loans and record every included identity.
    pub fn extend_parents(&self, loans: &mut Vec<LoanId>, included: &mut BitSet) {
        // visit each newly included loan once, including cyclic loop origins
        loans.retain(|loan| included.insert(loan.index()));
        let mut index = 0;
        while index < loans.len() {
            for &parent in self.get(loans[index]).parents() {
                if included.insert(parent.index()) {
                    loans.push(parent);
                }
            }
            index += 1;
        }
    }

    /// Return the active loan conflicting with a new loan.
    pub fn conflict(
        &self,
        loan: &Loan,
        active: &[LoanId],
        constants: &ConstantTable,
        places: &PlaceTable,
        function: FunctionId,
        tree: &Tree,
    ) -> Option<LoanId> {
        // authorize reborrows through every ancestor loan, including cyclic loop origins
        let mut parents = BitSet::new(self.len());
        let mut pending = loan.parents.to_vec();
        self.extend_parents(&mut pending, &mut parents);

        active.iter().copied().find(|&current| {
            let current_loan = self.get(current);
            !parents.contains(current.index())
                && current_loan.conflicts(loan, loan.issued_at, constants, places, function, tree)
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

    /// Return the loan blocking a place change at one operation.
    pub fn blocking_change(
        &self,
        place: &Place,
        at: LocalNodeIdAny,
        active: &[LoanId],
        places: &PlaceTable,
        mut may_overlap: impl FnMut(&Place, &Place) -> bool,
    ) -> Option<LoanId> {
        active
            .iter()
            .copied()
            .find(|loan| self.get(*loan).blocks(place, at, places, &mut may_overlap))
    }

    /// Return one mutable loan by identity.
    fn get_mut(&mut self, loan: LoanId) -> &mut Loan {
        self.loans
            .get_mut(loan.index())
            .unwrap_or_else(|| unreachable!("missing loan {loan:?}"))
    }
}

#[allow(clippy::too_many_arguments)]
impl Loan {
    /// Create a loan over one concrete MIR place.
    pub fn new(
        place: Place,
        source: Option<Place>,
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

    /// Return the storage of the reference through which this loan was borrowed.
    pub fn source(&self) -> Option<&Place> {
        match &self.target {
            LoanTarget::Place { source, .. } => source.as_ref(),
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

    /// Return whether this loan and one place root apart at one operation, one at a fresh allocation.
    ///
    /// A fresh allocation has no alias yet, so no other root addresses its storage.
    pub fn isolates(&self, place: &Place, at: LocalNodeIdAny, places: &PlaceTable) -> bool {
        self.place().is_some_and(|borrowed| {
            borrowed.origin != place.origin
                && (places.is_fresh_at(borrowed, at) || places.is_fresh_at(place, at))
        })
    }

    /// Return whether these loans exclude one another at one operation, or one may retag or release the other.
    pub fn conflicts(
        &self,
        other: &Self,
        at: LocalNodeIdAny,
        constants: &ConstantTable,
        places: &PlaceTable,
        function: FunctionId,
        tree: &Tree,
    ) -> bool {
        match (&self.target, &other.target) {
            // preserve access guarantees and the validity of selected cases both ways
            (LoanTarget::Place { place: left, .. }, LoanTarget::Place { place: right, .. }) => {
                self.forbids_place(right, other.access, at, constants, places, function, tree)
                    || other.forbids_place(left, self.access, at, constants, places, function, tree)
            }
            // compare incoming loans by their parameter and structural path
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
                self.access.conflicts(other.access)
                    && left == right
                    && (left_path.contains(right_path) || right_path.contains(left_path))
            }
            (LoanTarget::Place { .. }, LoanTarget::Parameter { .. })
            | (LoanTarget::Parameter { .. }, LoanTarget::Place { .. }) => false,
        }
    }

    /// Return whether one access to a target at one operation breaks this loan's guarantee.
    ///
    /// An access to some spaces breaks only a loan the caller knows those spaces expose.
    pub fn forbids(
        &self,
        target: &AccessTarget,
        access: Access,
        at: LocalNodeIdAny,
        constants: &ConstantTable,
        places: &PlaceTable,
        function: FunctionId,
        tree: &Tree,
    ) -> bool {
        match target {
            // violate the loan's access, or retag or release its place
            AccessTarget::Place(place) => {
                self.forbids_place(place, access, at, constants, places, function, tree)
            }
            // violate the loan's access in these spaces, or retag or release its place
            AccessTarget::Spaces(spaces) => {
                let Some(borrowed) = self.place() else {
                    return false;
                };
                let representation = tree.get(function).expect_value_type(self.representation);
                let loan_spaces = tree
                    .type_definition(tree.storage_type(representation))
                    .reference_storage_set()
                    .unwrap_or(StorageSet::ANY);
                let may_change = access.can_write()
                    && !borrowed.is_constant(tree)
                    && (borrowed.is_retaggable(function, tree)
                        || borrowed.is_releasable(function, tree));

                !spaces.is_disjoint(loan_spaces) && (self.access.conflicts(access) || may_change)
            }
        }
    }

    /// Return whether one access to a place at one operation breaks this loan's guarantee.
    fn forbids_place(
        &self,
        place: &Place,
        access: Access,
        at: LocalNodeIdAny,
        constants: &ConstantTable,
        places: &PlaceTable,
        function: FunctionId,
        tree: &Tree,
    ) -> bool {
        let Some(borrowed) = self.place() else {
            return false;
        };

        // address a fresh allocation through loans rooted at it alone
        let is_writing = access.can_write();
        if self.isolates(place, at, places) || !(is_writing || self.access.conflicts(access)) {
            return false;
        }

        // compare the access, then what the write may retag or release
        let is_overlapping = place.may_overlap(borrowed, constants, places, function, tree);
        let may_change = is_writing
            && !borrowed.is_constant(tree)
            && (place.may_replace_case(borrowed, constants, places, function, tree)
                || place.may_release(borrowed, is_overlapping, constants, places, function, tree));

        (is_overlapping && self.access.conflicts(access)) || may_change
    }

    /// Return whether this loan blocks one concrete place change at one operation.
    fn blocks(
        &self,
        place: &Place,
        at: LocalNodeIdAny,
        places: &PlaceTable,
        may_overlap: &mut impl FnMut(&Place, &Place) -> bool,
    ) -> bool {
        match &self.target {
            LoanTarget::Place {
                place: borrowed, ..
            } => !self.isolates(place, at, places) && may_overlap(borrowed, place),
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
    store l0, v2
    v3: ref<Box, borrowed, 'frame, exclusive> = address l0
    v4: ref<Pair, borrowed, 'frame, exclusive> = address (*v3).0
    v5: ref<int32, borrowed, 'frame, exclusive> = address (*v4).0
    v6: ref<int32, borrowed, 'frame, exclusive> = address (*v4).1
    v7: ref<int32, borrowed, 'frame, exclusive> = address (*v4).0
    v8: ref<int32, borrowed, 'frame, readonly> = address (*v4).0
    v9: ref<int32, borrowed, 'frame, readonly> = address (*v4).0
    v10: ref<int32, borrowed, 'frame, mutable> = address (*v4).0
    v11: ref<int32, borrowed, 'frame, immutable> = address (*v4).0
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let origins = analyses.origin(program.entry_function_id(), &program.tree);
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        let places = analyses.place(program.entry_function_id(), &program.tree);
        let loans = origins.loans();
        let [
            root,
            parent,
            first,
            second,
            overlapping,
            reader,
            other_reader,
            mutable,
            immutable,
        ] = [3, 4, 5, 6, 7, 8, 9, 10, 11].map(|value| loans.root(Value(value)).unwrap());

        // retain the complete ancestry when only the leaf borrow remains live
        let mut live = vec![first];
        let mut included = BitSet::new(loans.len());
        loans.extend_parents(&mut live, &mut included);
        assert_eq!(
            included.iter().collect::<Vec<_>>(),
            [root, parent, first].map(LoanId::index)
        );

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
                &constants,
                &places,
                program.entry_function_id(),
                &program.tree,
            );

            assert_eq!(
                actual,
                expected,
                "borrow {:?}",
                loans.get(loan).representation
            );
        }

        // compare all access pairs on the same field
        let accesses = [reader, mutable, immutable, first];
        let expected = [
            [None, None, None, Some(first)],
            [None, None, Some(immutable), Some(first)],
            [None, Some(mutable), None, Some(first)],
            [Some(reader), Some(mutable), Some(immutable), Some(first)],
        ];
        let actual = accesses.map(|loan| {
            accesses.map(|active| {
                loans.conflict(
                    loans.get(loan),
                    &[active],
                    &constants,
                    &places,
                    program.entry_function_id(),
                    &program.tree,
                )
            })
        });

        assert_eq!(actual, expected);
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
    store l0, v2
    store l1, v2
    v3: ref<Pair, borrowed, 'frame, exclusive> = address l0
    v4: ref<Pair, borrowed, 'frame, exclusive> = address l1
    v5: ref<Pair, borrowed, 'frame, exclusive> = select v0, v3, v4
    v6: ref<int32, borrowed, 'frame, exclusive> = address (*v5).0
    v7: ref<int32, borrowed, 'frame, exclusive> = address (*v5).0
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let origins = analyses.origin(program.entry_function_id(), &program.tree);
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        let places = analyses.place(program.entry_function_id(), &program.tree);
        let loans = origins.loans();
        let [left, right, first, sibling] =
            [3, 4, 6, 7].map(|value| loans.root(Value(value)).unwrap());

        assert_eq!(loans.get(first).parents(), &[left, right]);
        for (active, expected) in [(vec![left, right], None), (vec![first], Some(first))] {
            let actual = loans.conflict(
                loans.get(sibling),
                &active,
                &constants,
                &places,
                program.entry_function_id(),
                &program.tree,
            );

            assert_eq!(actual, expected);
        }
    }

    /// Preserve borrowed inline cases while permitting payload writes and retained referents.
    #[test]
    fn test_preserve_borrowed_variant_cases() {
        let program = TestModule::new(
            r#"
type Inner = variant<uint1> { 0uint1 = void; 1uint1 = int32; };
type Packet = variant<uint1> { 0uint1 = void; 1uint1 = (int32, ref<int32, managed, mutable, local>, Inner); };
type Single = variant<uint1> { 0uint1 = int32; };

function test(v0: Packet, v1: Single): void {
    local l0: Packet
    local l1: Single

entry(v0: Packet, v1: Single):
    store l0, v0
    store l1, v1
    v2: ref<Packet, borrowed, 'frame, mutable> = address l0
    v3: ref<int32, borrowed, 'frame, readonly> = address (l0 as 1).0
    v4: ref<int32, borrowed, 'frame, mutable> = address (l0 as 1).0
    v5: ref<ref<int32, managed, mutable, local>, borrowed, 'frame, readonly> = address (l0 as 1).1
    v6: ref<int32, borrowed, 'frame, readonly> = address (*(l0 as 1).1)
    v7: ref<Single, borrowed, 'frame, mutable> = address l1
    v8: ref<int32, borrowed, 'frame, readonly> = address (l1 as 0)
    v9: ref<Inner, borrowed, 'frame, mutable> = address (l0 as 1).2
    v10: ref<int32, borrowed, 'frame, readonly> = address ((l0 as 1).2 as 1)
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let origins = analyses.origin(program.entry_function_id(), &program.tree);
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        let places = analyses.place(program.entry_function_id(), &program.tree);
        let loans = origins.loans();

        // compare both borrowing orders for each storage relationship
        for (left, right, expected) in [
            (2, 3, true),
            (2, 4, true),
            (3, 4, false),
            (2, 5, true),
            (4, 5, false),
            (2, 6, false),
            (5, 6, false),
            (7, 8, false),
            (2, 10, true),
            (9, 10, true),
            (4, 10, false),
            (9, 3, false),
            (7, 10, false),
        ] {
            let left = loans.root(Value(left)).unwrap();
            let right = loans.root(Value(right)).unwrap();
            let actual = [(left, right), (right, left)].map(|(borrowed, active)| {
                loans.conflict(
                    loans.get(borrowed),
                    &[active],
                    &constants,
                    &places,
                    program.entry_function_id(),
                    &program.tree,
                )
            });
            let expected = if expected {
                [Some(right), Some(left)]
            } else {
                [None, None]
            };

            assert_eq!(actual, expected, "loans {left:?}, {right:?}");
        }
    }

    /// Permit a loop reborrow whose ancestry includes its previous iteration.
    #[test]
    fn test_allow_reborrows_through_cyclic_ancestry() {
        let program = TestModule::new(
            r#"
external function identity<'a>(ref<int32, borrowed, 'a, exclusive>): ref<int32, borrowed, 'a, exclusive>

function test(v0: boolean): void {
    local l0: int32

entry(v0: boolean):
    v1: int32 = 7
    store l0, v1
    v2: ref<int32, borrowed, 'frame, exclusive> = address l0
    jump loop(v2)

loop(v3: ref<int32, borrowed, 'frame, exclusive>):
    v4: ref<int32, borrowed, 'frame, exclusive> = call identity(v3): <'frame>(ref<int32, borrowed, 'frame, exclusive>) => ref<int32, borrowed, 'frame, exclusive>
    branch v0 => loop(v4) | exit

exit:
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let origins = analyses.origin(program.entry_function_id(), &program.tree);
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        let places = analyses.place(program.entry_function_id(), &program.tree);
        let loans = origins.loans();
        let root = loans.root(Value(2)).unwrap();
        let reborrow = loans.root(Value(4)).unwrap();

        assert_eq!(loans.get(reborrow).parents(), &[root, reborrow]);
        let conflict = loans.conflict(
            loans.get(reborrow),
            &[root, reborrow],
            &constants,
            &places,
            program.entry_function_id(),
            &program.tree,
        );

        assert_eq!(conflict, None);
    }
}

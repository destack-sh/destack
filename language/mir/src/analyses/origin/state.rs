use crate::analyses::LoanId;
use crate::{Lattice, LocalId, Path, Place, PlaceOrigin, Projection, ReferenceKind, TypeId, Value};

use super::context::OriginContext;
use super::region::{Origin, Region};

/// Origin at one MIR program point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OriginState {
    /// Origin carried by SSA values.
    bindings: Vec<ValueBinding>,
    /// Stored place bindings.
    places: Vec<PlaceBinding>,
    /// Loans escaped through aliasable storage.
    escaped_loans: Vec<LoanId>,
}

/// Origin for one value path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueBinding {
    /// SSA value carrying the borrowed path.
    pub value: Value,
    /// Path inside the value.
    pub path: Path,
    /// Origin that keeps the path live.
    pub origin: Origin,
}

/// Origin stored in one place path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceBinding {
    /// The exact place carrying the borrowed value.
    pub place: Place,
    /// Origin that keeps the path live.
    pub origin: Origin,
}

impl OriginState {
    /// Create empty origin flow.
    pub fn new() -> Self {
        Self::default()
    }

    /// Iterate all value bindings.
    pub fn bindings(&self) -> impl Iterator<Item = &ValueBinding> {
        self.bindings.iter()
    }

    /// Iterate all place bindings.
    pub fn places(&self) -> impl Iterator<Item = &PlaceBinding> {
        self.places.iter()
    }

    /// Bind one value to borrow origin.
    pub(super) fn insert(&mut self, value: Value, origin: Origin) {
        self.insert_at(value, Path::root(), origin);
    }

    /// Bind one value path to borrow origin.
    pub(super) fn insert_at(&mut self, value: Value, path: Path, origin: Origin) {
        if origin.is_empty() {
            return;
        }

        // replace existing bindings instead of growing duplicate entries
        if let Some(current) = self
            .bindings
            .iter_mut()
            .find(|binding| binding.value == value && binding.path == path)
        {
            current.origin = origin;
            return;
        }

        self.bindings.push(ValueBinding {
            value,
            path,
            origin,
        });
    }

    /// Bind all origin paths for one value.
    pub(super) fn insert_bindings(&mut self, value: Value, bindings: Vec<(Path, Origin)>) {
        self.bindings.retain(|binding| binding.value != value);

        // insert each structural path once
        for (path, origin) in bindings {
            self.insert_at(value, path, origin);
        }
    }

    /// Replace borrow origin below one value path.
    pub(super) fn replace_paths(
        &mut self,
        value: Value,
        base: Path,
        bindings: Vec<(Path, Origin)>,
    ) {
        self.bindings
            .retain(|binding| binding.value != value || !base.contains(&binding.path));

        // insert each origin below the replacement path
        for (path, origin) in bindings {
            let path = base.clone().with_path(&path);
            self.insert_at(value, path, origin);
        }
    }

    /// Merge borrow origin below one value path.
    pub(super) fn merge_paths(&mut self, value: Value, base: Path, bindings: Vec<(Path, Origin)>) {
        // merge each origin below the shared path
        for (path, origin) in bindings {
            let path = base.clone().with_path(&path);
            self.merge_origins_at(value, &path, &origin);
        }
    }

    /// Return borrow origin for one value path.
    pub(super) fn get_at(&self, value: Value, path: &Path) -> Option<&Origin> {
        self.bindings.iter().find_map(|binding| {
            (binding.value == value && &binding.path == path).then_some(&binding.origin)
        })
    }

    /// Merge origin at one value path.
    pub(super) fn merge_origins_at(&mut self, value: Value, path: &Path, origin: &Origin) {
        // accumulate origin from repeated paths
        let origin = self
            .get_at(value, path)
            .map(|current| current.merge(origin))
            .unwrap_or_else(|| origin.clone());

        self.insert_at(value, path.clone(), origin);
    }

    /// Bind one place path to borrow origin.
    fn insert_place(&mut self, place: Place, origin: Origin) {
        if origin.is_empty() {
            return;
        }

        // replace the previous value stored in this exact place path
        if let Some(current) = self
            .places
            .iter_mut()
            .find(|binding| binding.place == place)
        {
            current.origin = origin;
            return;
        }

        self.places.push(PlaceBinding { place, origin });
    }

    /// Return borrow origin stored in one exact place.
    pub(super) fn get_place(&self, place: &Place) -> Option<&Origin> {
        self.places
            .iter()
            .find_map(|binding| (&binding.place == place).then_some(&binding.origin))
    }

    /// Replace every borrowed path stored in one place.
    pub(super) fn insert_place_bindings(&mut self, place: Place, bindings: Vec<(Path, Origin)>) {
        self.places
            .retain(|binding| !place.contains(&binding.place));

        // retain each exact stored path
        for (path, origin) in bindings {
            self.insert_place(place.clone().with_path(&path), origin);
        }
    }

    /// Mark borrow loans carried into aliasable storage as escaped.
    pub(super) fn escape_loans(&mut self, bindings: &[(Path, Origin)]) {
        for (_, origin) in bindings {
            Self::merge_loans(&mut self.escaped_loans, origin.loans());
        }
    }

    /// Release the loans of the storage one assignment overwrites, as rustc's `loan_killed_at`.
    pub(super) fn kill(&mut self, cx: &OriginContext<'_>, written: &Place) {
        let killed = cx
            .loans
            .iter()
            .filter(|(_, loan)| {
                loan.place()
                    .is_some_and(|place| place.contains(written) || written.contains(place))
            })
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        if killed.is_empty() {
            return;
        }
        for origin in self.origins_mut() {
            origin.loans.retain(|loan| !killed.contains(loan));
        }
    }

    /// Iterate every origin carried by a value or stored in a place.
    fn origins_mut(&mut self) -> impl Iterator<Item = &mut Origin> {
        let bindings = self.bindings.iter_mut().map(|binding| &mut binding.origin);
        let places = self.places.iter_mut().map(|binding| &mut binding.origin);

        bindings.chain(places)
    }

    /// Return active loan identities retained by live values and places.
    pub fn active_loans(
        &self,
        mut is_value_live: impl FnMut(Value) -> bool,
        mut is_place_live: impl FnMut(&Place) -> bool,
    ) -> Vec<LoanId> {
        let mut loans = self.escaped_loans.clone();

        // collect loans carried by live SSA values
        for binding in self.bindings() {
            if is_value_live(binding.value) {
                Self::merge_loans(&mut loans, binding.origin.loans());
            }
        }

        // collect loans carried by live memory places
        for binding in self.places() {
            if is_place_live(&binding.place) {
                Self::merge_loans(&mut loans, binding.origin.loans());
            }
        }

        loans
    }

    /// Return loans retained through aliasable storage.
    pub fn escaped_loans(&self) -> &[LoanId] {
        &self.escaped_loans
    }

    /// Return origin carried by one value.
    pub fn value(&self, value: Value) -> Origin {
        self.value_path(value, &Path::root())
    }

    /// Return origin carried by one value path.
    pub fn value_path(&self, value: Value, path: &Path) -> Origin {
        if let Some(origin) = self.get_at(value, path) {
            return origin.clone();
        }
        if !path.is_root() {
            return Origin::none();
        }

        let mut origin = Origin::none();

        // merge sparse child paths for whole-value checks
        for binding in &self.bindings {
            if binding.value == value {
                origin = origin.merge(&binding.origin);
            }
        }

        origin
    }

    /// Return origin stored in one place.
    pub fn place(&self, cx: &OriginContext<'_>, place: &Place) -> Origin {
        let mut origin = self
            .get_place(place)
            .cloned()
            .unwrap_or_else(|| self.storage_origin(cx, place));

        // merge sparse child paths for whole-place checks
        for binding in &self.places {
            if &binding.place != place && place.contains(&binding.place) {
                origin = origin.merge(&binding.origin);
            }
        }

        origin
    }

    /// Return origin of one place's storage itself.
    pub(super) fn storage_origin(&self, cx: &OriginContext<'_>, place: &Place) -> Origin {
        match (place.origin, place.path.first()) {
            (PlaceOrigin::Local(local), Some(Projection::Deref)) => self
                .get_place(&Place::local(local))
                .cloned()
                .unwrap_or_else(|| Self::local_reference_origin(cx, local)),
            (PlaceOrigin::Local(_), _) => Origin::one(Region::Frame),
            (PlaceOrigin::Global(_), _) => Origin::one(Region::Static),
            (PlaceOrigin::Value(value), _) => self.storage(cx, value, &place.path),
        }
    }

    /// Return the origin one reference local's type gives its storage before any store.
    fn local_reference_origin(cx: &OriginContext<'_>, local: LocalId) -> Origin {
        let ty = cx.tree.get(local).ty;
        let ty = cx.tree.get(cx.tree.storage_type(TypeId::from(ty)));
        match ty.reference_kind() {
            Some(ReferenceKind::Managed | ReferenceKind::Borrowed) => Origin::from_reference(ty),
            Some(ReferenceKind::Unique) => Origin::one(Region::Frame),
            None => Origin::none(),
        }
    }

    /// Return origin paths carried by one value.
    pub fn value_bindings(&self, cx: &OriginContext<'_>, value: Value) -> Vec<(Path, Origin)> {
        let ty = cx.function.expect_value_type(value);
        let paths = cx.tree.type_origin_paths(TypeId::from(ty));
        if paths.is_empty() {
            let origin = self.value(value);
            if origin.is_empty() {
                return Vec::new();
            }

            return vec![(Path::root(), origin)];
        }

        paths
            .into_iter()
            .map(|borrowed| {
                let origin = self.value_path(value, &borrowed.path);

                (borrowed.path, origin, borrowed.kind)
            })
            // skip handle paths holding no tracked handle
            .filter(|(_, origin, kind)| *kind == ReferenceKind::Borrowed || !origin.is_empty())
            .map(|(path, origin, _)| (path, origin))
            .collect()
    }

    /// Bind successor parameter origin.
    pub(super) fn bind(&mut self, cx: &OriginContext<'_>, argument: Value, parameter: Value) {
        if argument == parameter {
            return;
        }

        // replace the successor parameter with every origin path from this edge
        let bindings = self.value_bindings(cx, argument);
        self.insert_bindings(parameter, bindings);

        // bind index values embedded in dynamic paths
        for binding in &mut self.bindings {
            binding.path.replace_value(argument, parameter);
        }
    }

    /// Merge unique loans into one destination.
    fn merge_loans(loans: &mut Vec<LoanId>, added: &[LoanId]) {
        for &loan in added {
            if !loans.contains(&loan) {
                loans.push(loan);
            }
        }
    }
}

impl Lattice for OriginState {
    /// Merge origin carried by two incoming edges.
    fn meet(&self, other: &Self) -> Self {
        let mut merged = Self::new();

        // merge value origin from both predecessors
        for predecessor in [self, other] {
            for binding in predecessor.bindings() {
                merged.merge_origins_at(binding.value, &binding.path, &binding.origin);
            }
        }

        // merge stored origin from both predecessors
        for predecessor in [self, other] {
            for binding in predecessor.places() {
                let origin = merged
                    .get_place(&binding.place)
                    .map(|current| current.merge(&binding.origin))
                    .unwrap_or_else(|| binding.origin.clone());
                merged.insert_place(binding.place.clone(), origin);
            }
        }

        // retain every loan escaped through aliasable storage
        Self::merge_loans(&mut merged.escaped_loans, &self.escaped_loans);
        Self::merge_loans(&mut merged.escaped_loans, &other.escaped_loans);

        merged
    }
}

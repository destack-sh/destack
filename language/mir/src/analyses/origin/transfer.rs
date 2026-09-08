use crate::{
    AddressKind, Instruction, Intrinsic, LocalNodeId, Path, Place, PlaceOrigin, Projection,
    ReferenceKind, Type, TypeId, Value,
};

use super::context::OriginContext;
use super::region::{Origin, Region};
use super::state::OriginState;

impl OriginState {
    /// Advance through one MIR instruction.
    pub fn advance(&mut self, cx: &OriginContext<'_>, instruction_id: LocalNodeId<Instruction>) {
        let instruction = cx.tree.get(instruction_id);

        // bind a borrowed call result before transferring stored origin
        cx.define_call_result(self, instruction_id, instruction);

        self.transfer_instruction(cx, instruction);

        // bind loans onto the value this instruction defines
        let Some(destination) = instruction.destination() else {
            return;
        };

        // merge the origin of every borrowed place this instruction loans
        let mut issued = Origin::none();
        for loan_id in cx.loans.roots_of(destination) {
            let loan = cx.loans.get(loan_id);
            let place = loan
                .place()
                .unwrap_or_else(|| unreachable!("instruction loan has no concrete place"));
            issued = issued.merge(&self.place(cx, place).with_loan(loan_id));
        }

        if issued.is_empty() {
            return;
        }

        // keep the regions a call signature mapped beside the reborrows
        match instruction {
            Instruction::Call { .. } => self.merge_origins_at(destination, &Path::root(), &issued),
            _ => self.insert(destination, issued),
        }
    }

    /// Copy origin from one value into another.
    fn copy(&mut self, cx: &OriginContext<'_>, source: Value, destination: Value) {
        let place = cx.places.get(source);
        self.copy_place(cx, place, destination);

        // carry the source value's loans into the copy
        if let Some(loans) = self.carried_loans(cx, source, destination) {
            self.merge_origins_at(destination, &Path::root(), &loans);
        }
    }

    /// Return the loans one copied reference-like value carries into its copy.
    fn carried_loans(
        &self,
        cx: &OriginContext<'_>,
        source: Value,
        destination: Value,
    ) -> Option<Origin> {
        // keep the same borrow through a reference-like copy under another type
        let ty = cx
            .tree
            .storage_type(TypeId::from(cx.function.expect_value_type(destination)));
        cx.tree.get(ty).reference_kind()?;
        let carried = self.value(source);
        if carried.loans().is_empty() {
            return None;
        }
        Some(Origin::new([]).with_loans(carried.loans()))
    }

    /// Copy origin from one place into one value.
    fn copy_place(&mut self, cx: &OriginContext<'_>, place: &Place, destination: Value) {
        if !Self::carries(cx, destination) {
            return;
        }

        let root = self.place(cx, place);
        self.insert(destination, root);

        // preserve nested borrowed paths
        let ty = cx.function.expect_value_type(destination);
        for borrowed in cx.tree.type_origin_paths(TypeId::from(ty)) {
            let source = place.clone().with_path(&borrowed.path);
            let origin = self.place(cx, &source);
            self.insert_at(destination, borrowed.path, origin);
        }
    }

    /// Load stored origin into one value.
    fn load(&mut self, cx: &OriginContext<'_>, place: &Place, destination: Value) {
        if !Self::carries(cx, destination) {
            return;
        }

        let ty = cx.function.expect_value_type(destination);
        let paths = cx.tree.type_origin_paths(TypeId::from(ty));
        let bindings = if paths.is_empty() {
            let origin = self
                .get_place(place)
                .cloned()
                .unwrap_or_else(|| Self::type_origin(cx, destination));

            vec![(Path::root(), origin)]
        } else {
            paths
                .into_iter()
                .map(|borrowed| {
                    let stored = place.clone().with_path(&borrowed.path);
                    let mut origin = self
                        .get_place(&stored)
                        .cloned()
                        .unwrap_or_else(|| Origin::from_path(&borrowed));
                    if let Some(loan) = cx.loans.carried(destination, &borrowed.path) {
                        origin = origin.with_loan(loan);
                    }

                    (borrowed.path, origin)
                })
                .collect()
        };

        self.insert_bindings(destination, bindings);
    }

    /// Merge origin from alternative values.
    fn merge_values(
        &mut self,
        cx: &OriginContext<'_>,
        values: impl IntoIterator<Item = Value>,
        destination: Value,
    ) {
        if !Self::carries(cx, destination) {
            return;
        }

        let mut bindings: Vec<(Path, Origin)> = Vec::new();

        // merge each structural path across every source
        for value in values {
            for (path, origin) in self.value_bindings(cx, value) {
                if let Some((_, current)) =
                    bindings.iter_mut().find(|(current, _)| *current == path)
                {
                    *current = current.merge(&origin);
                } else {
                    bindings.push((path, origin));
                }
            }
        }

        self.insert_bindings(destination, bindings);
    }

    /// Return the origin path one projection selects inside a value.
    fn value_projection(cx: &OriginContext<'_>, value: Value, projection: Projection) -> Path {
        // read through applications, dropping the projection at a newtype layer
        let mut ty = TypeId::from(cx.function.expect_value_type(value));
        loop {
            ty = match cx.tree.get(ty) {
                Type::Newtype { .. } => return Path::root(),
                Type::Application { base, .. } => *base,
                _ => return Path::root().with_projection(projection),
            };
        }
    }

    /// Replace one destination projection from a value.
    fn replace(
        &mut self,
        cx: &OriginContext<'_>,
        source: Value,
        destination: Value,
        projection: Projection,
    ) {
        let base = Self::value_projection(cx, destination, projection);
        let bindings = self.value_bindings(cx, source);
        self.replace_paths(destination, base, bindings);
    }

    /// Merge one value into a destination projection.
    fn merge_projection(
        &mut self,
        cx: &OriginContext<'_>,
        source: Value,
        destination: Value,
        projection: Projection,
    ) {
        let base = Self::value_projection(cx, destination, projection);
        let bindings = self.value_bindings(cx, source);
        self.merge_paths(destination, base, bindings);
    }

    /// Project one structural value path into a destination value.
    fn project(
        &mut self,
        cx: &OriginContext<'_>,
        source: Value,
        destination: Value,
        projection: Projection,
    ) {
        let prefix = Self::value_projection(cx, source, projection);
        let bindings = self
            .value_bindings(cx, source)
            .into_iter()
            .filter_map(|(path, origin)| path.strip_prefix(&prefix).map(|path| (path, origin)))
            .collect();

        self.insert_bindings(destination, bindings);
    }

    /// Transfer one MIR instruction.
    fn transfer_instruction(&mut self, cx: &OriginContext<'_>, instruction: &Instruction) {
        match instruction {
            Instruction::LocalGet { destination, local } => {
                self.load(cx, &Place::local(*local), *destination);
            }
            Instruction::LocalSet { local, value } => {
                let place = Place::local(*local);
                let bindings = self.value_bindings(cx, *value);
                self.kill(cx, &place);
                self.insert_place_bindings(place, bindings);
            }
            Instruction::Store { pointer, value } => {
                let place = cx.places.get(*pointer).clone();
                let mut bindings = self.value_bindings(cx, *value);
                if Self::storage_outlives_reference(cx, &place) {
                    self.escape_loans(&bindings);
                }
                self.root_handles(cx, &place, &mut bindings, *value);
                self.kill(cx, &place);
                self.insert_place_bindings(place, bindings);
            }
            Instruction::Aggregate {
                destination,
                values,
            } => {
                self.aggregate(cx, *destination, cx.tree.get_values(*values));
            }
            Instruction::Select {
                destination,
                then_value,
                else_value,
                ..
            } => {
                self.merge_values(cx, [*then_value, *else_value], *destination);
            }
            Instruction::FieldGet {
                destination,
                aggregate,
                field,
            } => {
                self.project(
                    cx,
                    *aggregate,
                    *destination,
                    Projection::Field { index: *field },
                );
            }
            Instruction::ElementGet {
                destination,
                aggregate,
                index,
            } => {
                self.project(
                    cx,
                    *aggregate,
                    *destination,
                    Projection::Element { index: *index },
                );
            }
            Instruction::FieldSet {
                destination,
                aggregate,
                value,
                field,
            } => {
                self.copy(cx, *aggregate, *destination);
                self.replace(
                    cx,
                    *value,
                    *destination,
                    Projection::Field { index: *field },
                );
            }
            Instruction::ElementSet {
                destination,
                aggregate,
                value,
                index,
            } => {
                self.copy(cx, *aggregate, *destination);
                self.replace(
                    cx,
                    *value,
                    *destination,
                    Projection::Element { index: *index },
                );
            }
            Instruction::Load {
                destination,
                pointer,
                ..
            } => {
                self.load(cx, cx.places.get(*pointer), *destination);
            }
            // a projected place carries the origin of the place it projects from
            Instruction::FieldAddr {
                destination,
                aggregate: base,
                kind: AddressKind::Projection,
                ..
            }
            | Instruction::ElementAddr {
                destination,
                base,
                kind: AddressKind::Projection,
                ..
            }
            | Instruction::VariantPayloadAddr {
                destination,
                variant: base,
                kind: AddressKind::Projection,
                ..
            } => {
                let origin = self.value(*base);
                if !origin.is_empty() {
                    self.insert(*destination, origin);
                }
            }
            Instruction::Cast {
                destination,
                argument,
                ..
            }
            | Instruction::FunctionEnvironment {
                destination,
                function: argument,
            }
            | Instruction::FunctionBind {
                destination,
                environment: argument,
                ..
            }
            | Instruction::DynamicBind {
                destination,
                payload: argument,
                ..
            }
            | Instruction::DynamicPayload {
                destination,
                dynamic: argument,
                ..
            } => {
                self.copy(cx, *argument, *destination);
            }
            // keep the origin of the reference a reinterpret reads
            Instruction::Intrinsic {
                destination: Some(destination),
                intrinsic: Intrinsic::Transmute,
                arguments,
            } => {
                if let [argument] = cx.tree.get_values(*arguments) {
                    self.copy(cx, *argument, *destination);
                }
            }
            Instruction::NewZeroed { destination, .. }
            | Instruction::NewUninit { destination, .. }
            | Instruction::NewSliceZeroed { destination, .. }
            | Instruction::NewSliceUninit { destination, .. } => {
                let origin = Self::destination(cx, *destination);
                self.insert(*destination, origin);
            }
            Instruction::NewComplete {
                destination, value, ..
            } => {
                let origin = Self::destination(cx, *destination);
                self.insert(*destination, origin);

                let place = cx.places.get(*destination).clone();
                let mut bindings = self.value_bindings(cx, *value);
                if Self::storage_outlives_reference(cx, &place) {
                    self.escape_loans(&bindings);
                }
                self.root_handles(cx, &place, &mut bindings, *value);
                self.insert_place_bindings(place, bindings);
            }
            // null references and constants outlive every frame
            Instruction::Const { destination, .. } => {
                let ty = cx.function.expect_value_type(*destination);
                let bindings = cx
                    .tree
                    .type_origin_paths(TypeId::from(ty))
                    .into_iter()
                    .map(|borrowed| (borrowed.path, Origin::one(Region::Static)))
                    .collect();
                self.insert_bindings(*destination, bindings);
            }
            Instruction::VariantPayload {
                destination,
                variant,
                case,
            } => {
                self.project(
                    cx,
                    *variant,
                    *destination,
                    Projection::Variant { case: *case },
                );
            }
            // bind the absent cases to static and the built case to its payload
            Instruction::VariantNew {
                destination,
                case,
                payload,
                ..
            } => {
                let ty = cx.function.expect_value_type(*destination);
                let built = Path::root().with_projection(Projection::Variant { case: *case });
                let absent = cx
                    .tree
                    .type_origin_paths(TypeId::from(ty))
                    .into_iter()
                    .filter(|borrowed| borrowed.path.strip_prefix(&built).is_none())
                    .map(|borrowed| (borrowed.path, Origin::one(Region::Static)))
                    .collect();
                self.insert_bindings(*destination, absent);
                if let Some(payload) = payload {
                    self.replace(
                        cx,
                        *payload,
                        *destination,
                        Projection::Variant { case: *case },
                    );
                }
            }
            _ => {}
        }
    }

    /// Populate origin for one aggregate construction.
    fn aggregate(&mut self, cx: &OriginContext<'_>, destination: Value, values: &[Value]) {
        let ty = cx.function.expect_value_type(destination);

        // bind each logical slot to its destination path
        for (index, value) in values.iter().copied().enumerate() {
            if let Some(projection) = Self::aggregate_projection(cx, ty, index) {
                self.merge_projection(cx, value, destination, projection);
            } else {
                let source = self.value(value);
                let current = self.value(destination);
                self.insert(destination, current.merge(&source));
            }
        }
    }

    /// Return whether one value type carries origin.
    fn carries(cx: &OriginContext<'_>, value: Value) -> bool {
        let ty = TypeId::from(cx.function.expect_value_type(value));

        cx.tree.type_lifetime(ty).is_some() || !cx.tree.type_origin_paths(ty).is_empty()
    }

    /// Root stored handles in the written storage when it outlives their regions.
    fn root_handles(
        &self,
        cx: &OriginContext<'_>,
        place: &Place,
        bindings: &mut [(Path, Origin)],
        value: Value,
    ) {
        let ty = cx.function.expect_value_type(value);
        let paths = cx.tree.type_origin_paths(TypeId::from(ty));
        for (path, origin) in bindings.iter_mut() {
            let is_handle = paths
                .iter()
                .any(|borrowed| &borrowed.path == path && borrowed.kind == ReferenceKind::Managed);
            if !is_handle {
                continue;
            }
            let slot = self.storage_origin(cx, &place.clone().with_path(path));
            if slot.outlives(origin, &cx.function.lifetimes) {
                *origin = slot;
            }
        }
    }

    /// Return the path for one logical aggregate slot.
    fn aggregate_projection(
        cx: &OriginContext<'_>,
        ty: LocalNodeId<Type>,
        index: usize,
    ) -> Option<Projection> {
        match cx.tree.get(ty) {
            Type::Struct { .. } | Type::Tuple { .. } => Some(Projection::Field {
                index: index as u32,
            }),
            Type::FixedArray { .. } => Some(Projection::Element {
                index: index as u32,
            }),
            Type::Newtype { .. } => None,
            Type::Application { base, .. } => Self::aggregate_projection(cx, *base, index),
            _ => unreachable!("aggregate destination is not an aggregate type"),
        }
    }

    /// Return origin implied by one value type.
    fn type_origin(cx: &OriginContext<'_>, value: Value) -> Origin {
        let ty = cx.function.expect_value_type(value);

        cx.tree
            .type_lifetime(TypeId::from(ty))
            .map(|lifetime| Origin::from_lifetime(&lifetime))
            .unwrap_or_default()
    }

    /// Return origin for one storage-producing destination.
    fn destination(cx: &OriginContext<'_>, destination: Value) -> Origin {
        let ty = cx.function.expect_value_type(destination);
        let ty = cx.tree.get(cx.tree.storage_type(TypeId::from(ty)));

        match ty.reference_kind() {
            Some(ReferenceKind::Managed) => Origin::from_managed(ty),
            Some(ReferenceKind::Unique) => Origin::one(Region::Frame),
            Some(ReferenceKind::Borrowed) | None => Origin::none(),
        }
    }

    /// Return origin implied by one value path used as storage.
    pub(super) fn storage(&self, cx: &OriginContext<'_>, value: Value, path: &Path) -> Origin {
        if let Some(origin) = self.get_at(value, path) {
            return origin.clone();
        }
        if !path.is_root()
            && let Some(origin) = self.get_at(value, &Path::root())
        {
            return origin.clone();
        }

        let ty = cx.function.expect_value_type(value);
        let ty = cx.tree.get(cx.tree.storage_type(TypeId::from(ty)));
        match ty.reference_kind() {
            Some(ReferenceKind::Managed | ReferenceKind::Borrowed) => self
                .get_at(value, &Path::root())
                .cloned()
                .unwrap_or_else(|| Origin::from_reference(ty)),
            Some(ReferenceKind::Unique) => Origin::one(Region::Frame),
            None => Origin::none(),
        }
    }

    /// Return whether addressed storage may outlive its reference value.
    fn storage_outlives_reference(cx: &OriginContext<'_>, place: &Place) -> bool {
        match (place.origin, place.path.first()) {
            (PlaceOrigin::Local(local), Some(Projection::Deref)) => {
                let ty = cx.tree.get(local).ty;
                let ty = cx.tree.get(cx.tree.storage_type(TypeId::from(ty)));

                matches!(
                    ty.reference_kind(),
                    Some(ReferenceKind::Managed | ReferenceKind::Borrowed)
                )
            }
            (PlaceOrigin::Local(_), _) => false,
            (PlaceOrigin::Global(_), _) => true,
            (PlaceOrigin::Value(value), _) => matches!(
                cx.function.reference_kind(value, cx.tree),
                Some(ReferenceKind::Managed | ReferenceKind::Borrowed)
            ),
        }
    }
}

use destack_core::{FxIndexMap, FxIndexSet};

use crate::{
    Access, Analysis, Block, CastOperator, Constant, ConstantTable, ControlTable, FunctionId,
    Instruction, Intrinsic, LocalId, LocalNodeId, Mutation, Place, PlaceOrigin, PlaceType,
    Projection, Reference, Storage, Substitution, Tree, Type, Value,
};

/// Canonical places for one MIR function.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlaceTable {
    /// The canonical place for each SSA value.
    values: Vec<Place>,
    /// The values naming a fresh allocation, reached through no other root until stored.
    fresh: FxIndexSet<Value>,
    /// The locals whose address some instruction takes.
    exposed: FxIndexSet<LocalId>,
}

impl Place {
    /// Return the storage containing this place.
    pub fn storage(&self, function: FunctionId, tree: &Tree) -> Option<Storage> {
        if self.path.projections.contains(&Projection::Deref) {
            let reference = self.reference_type(function, tree)?;

            tree.type_definition(reference).reference_storage()
        } else {
            match self.origin {
                PlaceOrigin::Global(global) => Some(Storage::global(tree.get(global).space)),
                PlaceOrigin::Local(_) | PlaceOrigin::Value(_) => Some(Storage::Frame),
            }
        }
    }

    /// Return whether two structural places may overlap.
    pub fn may_overlap(
        &self,
        other: &Self,
        constants: &ConstantTable,
        places: &PlaceTable,
        function: FunctionId,
        tree: &Tree,
    ) -> bool {
        // compare inline projections after the common prefix
        if self.origin == other.origin {
            let common = self
                .path
                .projections
                .iter()
                .zip(&other.path.projections)
                .take_while(|(left, right)| left == right)
                .count();
            let left = &self.path.projections[common..];
            let right = &other.path.projections[common..];
            if !left.contains(&Projection::Deref) && !right.contains(&Projection::Deref) {
                return match (left.first(), right.first()) {
                    (Some(left), Some(right)) => {
                        !Self::projections_are_disjoint(left, right, constants)
                    }
                    _ => true,
                };
            }
        }

        // a local whose address is never taken is reached through no reference
        let is_frame_storage = |place: &Self| {
            matches!(place.origin, PlaceOrigin::Local(_))
                && !place.path.projections.contains(&Projection::Deref)
        };
        let is_private_local = |place: &Self| {
            matches!(place.origin, PlaceOrigin::Local(local) if !places.is_exposed(local))
                && !place.path.projections.contains(&Projection::Deref)
        };
        let dereferences = |place: &Self| place.path.projections.contains(&Projection::Deref);
        if (is_private_local(self) && dereferences(other))
            || (is_private_local(other) && dereferences(self))
        {
            return false;
        }

        // a caller-provided reference never reaches this frame's own locals
        let is_parameter_pointee = |place: &Self| {
            matches!(place.origin, PlaceOrigin::Value(value)
                if tree.get(function).parameters.iter().any(|parameter| parameter.value == value))
                && place.path.projections.first() == Some(&Projection::Deref)
        };
        if (is_frame_storage(self) && is_parameter_pointee(other))
            || (is_frame_storage(other) && is_parameter_pointee(self))
        {
            return false;
        }

        // the pointees of two reference parameters alias only when both references admit an alias
        let parameter_reference = |place: &Self| {
            let PlaceOrigin::Value(value) = place.origin else {
                return None;
            };
            if place.path.projections.first() != Some(&Projection::Deref) {
                return None;
            }
            let parameter = tree
                .get(function)
                .parameters
                .iter()
                .find(|parameter| parameter.value == value)?;

            Some(tree.type_definition(tree.storage_type(parameter.ty)))
        };
        let is_unaliased = |reference: &Type| {
            reference.reference_kind() == Some(Reference::Unique)
                || matches!(
                    reference.reference_access(),
                    Some(Access::Exclusive | Access::Immutable)
                )
        };
        if self.origin != other.origin
            && let (Some(left), Some(right)) =
                (parameter_reference(self), parameter_reference(other))
            && (is_unaliased(left) || is_unaliased(right))
        {
            return false;
        }

        // separate root values until a reference is followed
        let is_value_storage = |place: &Self| {
            matches!(place.origin, PlaceOrigin::Value(_))
                && !place.path.projections.contains(&Projection::Deref)
        };
        if is_value_storage(self) || is_value_storage(other) {
            return false;
        }
        if !self.path.projections.contains(&Projection::Deref)
            && !other.path.projections.contains(&Projection::Deref)
        {
            return false;
        }

        self.storage(function, tree)
            .zip(other.storage(function, tree))
            .is_none_or(|(left, right)| left.storage_set(tree).may_alias(right.storage_set(tree)))
    }

    /// Iterate inline variants whose cases this place selects.
    pub fn variants(&self) -> impl Iterator<Item = Self> + '_ {
        self.path
            .projections
            .iter()
            .enumerate()
            .rev()
            .take_while(|(_, projection)| **projection != Projection::Deref)
            .filter_map(|(index, projection)| {
                if !matches!(projection, Projection::Variant { .. }) {
                    return None;
                }
                let mut variant = self.clone();
                variant.path.projections.truncate(index);

                Some(variant)
            })
    }

    /// Return whether a write to this place replaces the unique owner one borrow reaches through.
    pub fn may_replace_owner(&self, borrowed: &Self, function: FunctionId, tree: &Tree) -> bool {
        // require the borrow to dereference the written place
        if self.origin != borrowed.origin {
            return false;
        }
        let written = self.path.projections.as_slice();
        let Some(rest) = borrowed.path.projections.strip_prefix(written) else {
            return false;
        };
        if rest.first() != Some(&Projection::Deref) {
            return false;
        }

        // require the written place to hold unique storage
        match self.ty(function, tree) {
            Some(PlaceType::Value(ty)) => tree
                .get(Substitution::resolve(ty, tree))
                .is_unique_storage(),
            _ => false,
        }
    }

    /// Return whether replacing this place may change a case selected by the borrowed place.
    pub fn may_replace_case(
        &self,
        borrowed: &Self,
        constants: &ConstantTable,
        places: &PlaceTable,
        function: FunctionId,
        tree: &Tree,
    ) -> bool {
        borrowed.variants().any(|mut variant| {
            // permit writes confined to the selected payload
            let length = variant.path.projections.len();
            variant.push(borrowed.path.projections[length].clone());
            if variant.contains(self) {
                return false;
            }
            variant.path.projections.truncate(length);
            if !self.contains(&variant) {
                return false;
            }

            // omit variants whose case cannot change
            let Some(PlaceType::Value(mut ty)) = variant.ty(function, tree) else {
                unreachable!("a variant projection requires a value");
            };
            loop {
                ty = Substitution::resolve(ty, tree);
                match tree.get(ty) {
                    Type::Uninit { value } | Type::ManuallyDrop { value } => ty = *value,
                    _ => break,
                }
            }
            let Type::Variant { cases, .. } = tree.get(ty) else {
                unreachable!("a variant projection requires a variant");
            };

            cases.len() > 1 && self.may_overlap(&variant, constants, places, function, tree)
        })
    }

    /// Return whether two projections are proven disjoint.
    fn projections_are_disjoint(
        left: &Projection,
        right: &Projection,
        constants: &ConstantTable,
    ) -> bool {
        // compare the projected index intervals
        let Some(left) = Self::projection_interval(left, constants) else {
            return false;
        };
        let Some(right) = Self::projection_interval(right, constants) else {
            return false;
        };

        left.1 <= right.0 || right.1 <= left.0
    }

    /// Return the half-open index interval covered by one projection.
    fn projection_interval(
        projection: &Projection,
        constants: &ConstantTable,
    ) -> Option<(u128, u128)> {
        match projection {
            Projection::Field { index }
            | Projection::Element { index }
            | Projection::Variant { case: index } => {
                let start = u128::from(*index);
                let end = start.checked_add(1)?;

                Some((start, end))
            }
            Projection::Index { index } => {
                let start = Self::projection_constant(*index, constants)?;
                let end = start.checked_add(1)?;

                Some((start, end))
            }
            Projection::Elements | Projection::Deref => None,
            Projection::Slice { start, length } => {
                let start = Self::projection_constant(*start, constants)?;
                let length = Self::projection_constant(*length, constants)?;
                let end = start.checked_add(length)?;

                Some((start, end))
            }
        }
    }

    /// Return the integer constant defined for one value.
    fn projection_constant(value: Value, constants: &ConstantTable) -> Option<u128> {
        match constants.constant(value)? {
            Constant::Int { value, .. } => u128::try_from(*value).ok(),
            Constant::UInt { value, .. } => Some(*value),
            _ => None,
        }
    }
}

impl PlaceTable {
    /// Analyse storage origins and address projections for one function.
    pub fn analyse(function: FunctionId, graph: &ControlTable, tree: &Tree) -> Self {
        let mut resolutions = vec![Resolution::Unknown; tree.get(function).value_types().len()];
        let mut fresh = FxIndexSet::default();

        // root function parameters in their incoming values
        for parameter in &tree.get(function).parameters {
            Self::set(
                &mut resolutions,
                parameter.value,
                Resolution::Known(Place::value(parameter.value).with_projection(Projection::Deref)),
            );
        }

        // forward stored references through locals whose addresses are never taken
        let exposed = Self::exposed_locals(function, tree);
        let forwarded = Self::forwarded_locals(function, &exposed, tree);
        let mut exits: FxIndexMap<LocalNodeId<Block>, FxIndexMap<LocalId, Resolution>> =
            FxIndexMap::default();

        // solve block parameters, held references, and address derivations together
        let mut is_changed = true;
        while is_changed {
            is_changed = false;

            // resolve instruction places in each block
            for &block_id in tree.get(function).blocks() {
                let block = tree.get(block_id);

                // merge every copyable block parameter from its incoming arguments
                if Some(block_id) != tree.get(function).entry() {
                    for (index, parameter) in block.parameters.iter().enumerate() {
                        let ty = Substitution::resolve(parameter.ty, tree);
                        let resolution = if tree.get(ty).is_unique_storage() {
                            Resolution::Known(
                                Place::value(parameter.value).with_projection(Projection::Deref),
                            )
                        } else {
                            Self::parameter(block_id, index, graph, &resolutions, tree)
                        };
                        is_changed |= Self::set(&mut resolutions, parameter.value, resolution);
                    }
                }

                // merge the references the locals hold at entry from every predecessor
                let mut held = Self::merge_held(block_id, graph, &exits, &forwarded);

                // derive instruction destinations from their storage operands
                for &instruction_id in &block.instructions {
                    let instruction = tree.get(instruction_id);
                    match instruction {
                        Instruction::Store {
                            place:
                                Place {
                                    origin: PlaceOrigin::Local(local),
                                    path,
                                },
                            value,
                        } if path.is_root() && forwarded.contains(local) => {
                            held.insert(*local, Self::copy(*value, &resolutions));
                        }
                        Instruction::Address {
                            destination,
                            place:
                                Place {
                                    origin: PlaceOrigin::Local(local),
                                    path,
                                },
                            ..
                        } if path.projections.first() == Some(&Projection::Deref)
                            && forwarded.contains(local) =>
                        {
                            // project through the reference this local holds
                            let mut place = match held.get(local) {
                                Some(Resolution::Known(place)) => place.clone(),
                                _ => Place::local(*local).with_projection(Projection::Deref),
                            };
                            place
                                .path
                                .projections
                                .extend_from_slice(&path.projections[1..]);
                            let resolution = Resolution::Known(place);
                            is_changed |= Self::set(&mut resolutions, *destination, resolution);
                        }
                        Instruction::Load {
                            destination,
                            place:
                                Place {
                                    origin: PlaceOrigin::Local(local),
                                    path,
                                },
                            ..
                        } if path.is_root() && forwarded.contains(local) => {
                            // resolve the reference held by this local
                            let resolution = match held.get(local) {
                                Some(Resolution::Opaque) => Resolution::Known(
                                    Place::local(*local).with_projection(Projection::Deref),
                                ),
                                Some(resolution) => resolution.clone(),
                                None => Resolution::Unknown,
                            };
                            is_changed |= Self::set(&mut resolutions, *destination, resolution);
                        }
                        _ => {
                            if let Some(destination) = instruction.destination() {
                                let resolution =
                                    Self::instruction(destination, instruction, &resolutions, tree);
                                is_changed |= Self::set(&mut resolutions, destination, resolution);
                                if Self::allocates(instruction, &fresh) {
                                    fresh.insert(destination);
                                }
                            }
                        }
                    }
                }

                // record the references held at exit
                if exits.get(&block_id) != Some(&held) {
                    exits.insert(block_id, held);
                    is_changed = true;
                }
            }
        }

        // preserve an opaque root for unresolved or conflicting values
        let values = resolutions
            .into_iter()
            .enumerate()
            .map(|(index, resolution)| match resolution {
                Resolution::Known(place) => place,
                Resolution::Unknown | Resolution::Opaque => {
                    Place::value(Value::new(index as u32)).with_projection(Projection::Deref)
                }
            })
            .collect();

        Self {
            values,
            fresh,
            exposed,
        }
    }

    /// Return whether one instruction defines a fresh allocation or reinterprets one.
    fn allocates(instruction: &Instruction, fresh: &FxIndexSet<Value>) -> bool {
        match instruction {
            Instruction::NewZeroed { .. }
            | Instruction::NewUninit { .. }
            | Instruction::NewComplete { .. }
            | Instruction::NewSliceZeroed { .. }
            | Instruction::NewSliceUninit { .. } => true,
            Instruction::Cast {
                operator: CastOperator::Bitcast,
                argument,
                ..
            } => fresh.contains(argument),
            _ => false,
        }
    }

    /// Return whether one value names a fresh allocation.
    pub fn is_fresh(&self, value: Value) -> bool {
        self.fresh.contains(&value)
    }

    /// Return whether some instruction takes one local's address.
    pub fn is_exposed(&self, local: LocalId) -> bool {
        self.exposed.contains(&local)
    }

    /// Return the canonical place for one value.
    pub fn get(&self, value: Value) -> &Place {
        self.values
            .get(value.id() as usize)
            .unwrap_or_else(|| unreachable!("missing place for value {value:?}"))
    }

    /// Return one projected canonical place.
    pub fn project(&self, value: Value, projection: Projection) -> Place {
        self.get(value).clone().with_projection(projection)
    }

    /// Resolve the reference root of an explicit memory operand.
    pub fn resolve_place(&self, place: &Place) -> Place {
        match (place.origin, place.path.projections.split_first()) {
            (PlaceOrigin::Value(value), Some((Projection::Deref, projections))) => {
                let mut resolved = self.get(value).clone();
                resolved.path.projections.extend_from_slice(projections);

                resolved
            }
            _ => place.clone(),
        }
    }

    /// Resolve one instruction destination.
    fn instruction(
        destination: Value,
        instruction: &Instruction,
        resolutions: &[Resolution],
        tree: &Tree,
    ) -> Resolution {
        match instruction {
            Instruction::Address { place, .. } => Self::resolve(place, resolutions),
            Instruction::Select {
                then_value,
                else_value,
                ..
            } => {
                let left = Self::copy(*then_value, resolutions);
                let right = Self::copy(*else_value, resolutions);

                left.merge(right)
            }

            Instruction::Copy { value, .. } | Instruction::NewComplete { value, .. } => {
                Self::copy(*value, resolutions)
            }

            // preserve the storage named by a reinterpreted address
            Instruction::Cast {
                operator: CastOperator::Bitcast,
                argument,
                ..
            } => Self::copy(*argument, resolutions),
            Instruction::Intrinsic {
                intrinsic: Intrinsic::Transmute | Intrinsic::SpaceCast,
                arguments,
                ..
            } => match tree.get_values(*arguments) {
                [argument] => Self::copy(*argument, resolutions),
                _ => unreachable!("representation cast requires one argument"),
            },
            _ => Resolution::Known(Place::value(destination).with_projection(Projection::Deref)),
        }
    }

    /// Return the locals whose address some instruction takes.
    fn exposed_locals(function: FunctionId, tree: &Tree) -> FxIndexSet<LocalId> {
        let mut exposed = FxIndexSet::default();
        for &block_id in tree.get(function).blocks() {
            for &instruction_id in &tree.get(block_id).instructions {
                if let Instruction::Address {
                    place:
                        Place {
                            origin: PlaceOrigin::Local(local),
                            path,
                        },
                    ..
                } = tree.get(instruction_id)
                    && !path.projections.contains(&Projection::Deref)
                {
                    exposed.insert(*local);
                }
            }
        }

        exposed
    }

    /// Return copyable reference locals accessed exclusively through local loads and stores.
    fn forwarded_locals(
        function: FunctionId,
        exposed: &FxIndexSet<LocalId>,
        tree: &Tree,
    ) -> FxIndexSet<LocalId> {
        (0..tree.get(function).locals().len())
            .filter_map(|index| {
                let local = tree.get(function).locals()[index];
                let held = Substitution::resolve(tree.get(local).ty, tree);

                let held = tree.get(held);

                (!exposed.contains(&local)
                    && held.is_reference_representation()
                    && !held.is_unique_storage())
                .then_some(local)
            })
            .collect()
    }

    /// Merge the references the forwarded locals hold at one block's entry.
    fn merge_held(
        block: LocalNodeId<Block>,
        graph: &ControlTable,
        exits: &FxIndexMap<LocalNodeId<Block>, FxIndexMap<LocalId, Resolution>>,
        forwarded: &FxIndexSet<LocalId>,
    ) -> FxIndexMap<LocalId, Resolution> {
        let mut held = FxIndexMap::default();
        for &local in forwarded {
            let mut merged = Resolution::Unknown;
            for predecessor in graph.predecessors(block) {
                let Some(incoming) = exits.get(&predecessor).and_then(|exit| exit.get(&local))
                else {
                    continue;
                };
                merged = merged.merge(incoming.clone());
            }
            if merged != Resolution::Unknown {
                held.insert(local, merged);
            }
        }

        held
    }

    /// Resolve one block parameter from incoming arguments.
    fn parameter(
        block: LocalNodeId<Block>,
        index: usize,
        graph: &ControlTable,
        resolutions: &[Resolution],
        tree: &Tree,
    ) -> Resolution {
        let mut resolution = Resolution::Unknown;
        let parameter = tree.get(block).parameters[index].value;

        // merge each exact incoming edge, including values produced by its terminator
        for (edge, target) in graph.incoming_edges(block, tree) {
            let source = tree.get(edge.source);
            let terminator = tree.get(source.terminator);
            let result_count = terminator.target_result_count(tree, edge.successor);
            let incoming = if index < result_count {
                Resolution::Known(Place::value(parameter).with_projection(Projection::Deref))
            } else {
                let arguments = target.arguments(tree);
                Self::copy(arguments[index - result_count], resolutions)
            };
            resolution = resolution.merge(incoming);
        }

        resolution
    }

    /// Resolve a reference dereference while the alias table is being built.
    fn resolve(place: &Place, resolutions: &[Resolution]) -> Resolution {
        match (place.origin, place.path.projections.split_first()) {
            (PlaceOrigin::Value(value), Some((Projection::Deref, projections))) => {
                match Self::copy(value, resolutions) {
                    Resolution::Known(mut place) => {
                        place.path.projections.extend_from_slice(projections);

                        Resolution::Known(place)
                    }
                    resolution => resolution,
                }
            }
            _ => Resolution::Known(place.clone()),
        }
    }

    /// Copy one place resolution.
    fn copy(value: Value, resolutions: &[Resolution]) -> Resolution {
        match resolutions.get(value.id() as usize) {
            Some(Resolution::Known(place)) => Resolution::Known(place.clone()),
            Some(Resolution::Opaque) => {
                Resolution::Known(Place::value(value).with_projection(Projection::Deref))
            }
            Some(Resolution::Unknown) | None => Resolution::Unknown,
        }
    }

    /// Replace one value resolution.
    fn set(resolutions: &mut [Resolution], value: Value, resolution: Resolution) -> bool {
        let index = value.id() as usize;
        let current = resolutions
            .get(index)
            .unwrap_or_else(|| unreachable!("value outside place table: {value:?}"));
        if current == &resolution || matches!(current, Resolution::Opaque) {
            return false;
        }

        // collapse conflicting places into an opaque root
        let resolution = match (current, resolution) {
            (Resolution::Known(current), Resolution::Known(next)) if current != &next => {
                Resolution::Opaque
            }
            (Resolution::Known(_), Resolution::Unknown) => return false,
            (_, resolution) => resolution,
        };

        resolutions[index] = resolution;

        true
    }
}

impl Analysis for PlaceTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL
        .union(Mutation::VALUE)
        .union(Mutation::LAYOUT);
}

/// Place resolution while building one function table.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Resolution {
    /// The value's place still depends on unresolved control flow.
    Unknown,
    /// The value has one canonical place.
    Known(Place),
    /// Incoming control flow carries different places.
    Opaque,
}

impl Resolution {
    /// Merge incoming locations while preserving unresolved and conflicting states.
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unknown, other) => other,
            (current, Self::Unknown) => current,
            (Self::Known(left), Self::Known(right)) if left == right => Self::Known(left),
            _ => Self::Opaque,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Space;
    use crate::analyses::tests::TestModule;

    /// Preserve agreeing references through selections and block arguments.
    #[test]
    fn test_preserve_common_storage_through_merges() {
        let program = TestModule::new(
            r#"
function test(v0: boolean, v1: ref<int32, borrowed, 'static, readonly, local>): ref<int32, borrowed, 'static, readonly, local> {
entry(v0: boolean, v1: ref<int32, borrowed, 'static, readonly, local>):
    v2: ref<int32, borrowed, 'static, readonly, local> = select v0, v1, v1
    branch v0 => join(v1) | join(v2)

join(v3: ref<int32, borrowed, 'static, readonly, local>):
    return v3
}
"#,
        );
        let mut analyses = program.function_analyses();
        let table = analyses.place(program.entry_function_id(), &program.tree);

        assert_eq!(
            [Value(2), Value(3)].map(|value| table.get(value)),
            [
                &Place::value(Value(1)).with_projection(Projection::Deref),
                &Place::value(Value(1)).with_projection(Projection::Deref)
            ]
        );
    }

    /// Preserve a distinct origin when incoming references address different storage.
    #[test]
    fn test_retain_value_origins_for_distinct_incoming_storage() {
        let program = TestModule::new(
            r#"
function test(v0: boolean, v1: ref<int32, unique, mutable, local>, v2: ref<int32, unique, mutable, local>): ref<int32, unique, mutable, local> {
entry(v0: boolean, v1: ref<int32, unique, mutable, local>, v2: ref<int32, unique, mutable, local>):
    branch v0 => join(v1) | join(v2)

join(v3: ref<int32, unique, mutable, local>):
    return v3
}
"#,
        );
        let mut analyses = program.function_analyses();
        let table = analyses.place(program.entry_function_id(), &program.tree);

        assert_eq!(
            table.get(Value(3)),
            &Place::value(Value(3)).with_projection(Projection::Deref)
        );
    }

    /// Compose field, element, and slice projections from the original storage.
    #[test]
    fn test_compose_address_projections() {
        let program = TestModule::new(
            r#"
type Object {
    values: [int32; 4];
}

function test<'a>(v0: ref<Object, borrowed, 'a, mutable, local>, v1: uint64, v2: uint64): void {
entry(v0: ref<Object, borrowed, 'a, mutable, local>, v1: uint64, v2: uint64):
    v3: ref<[int32; 4], borrowed, 'a, mutable, local> = address (*v0).0
    v4: ref<int32, borrowed, 'a, mutable, local> = address (*v3)[v1]
    v5: slice<int32, borrowed, 'a, mutable, local> = address (*v3)[v1; v2]
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let table = analyses.place(program.entry_function_id(), &program.tree);
        let field = Place::value(Value(0))
            .with_projection(Projection::Deref)
            .with_projection(Projection::Field { index: 0 });

        assert_eq!(
            [Value(3), Value(4), Value(5)].map(|value| table.get(value).clone()),
            [
                field.clone(),
                field
                    .clone()
                    .with_projection(Projection::Index { index: Value(1) }),
                field.with_projection(Projection::Slice {
                    start: Value(1),
                    length: Value(2)
                }),
            ]
        );
    }

    /// Forward unaddressed locals and preserve places through representation casts.
    #[test]
    fn test_forward_local_references() {
        let program = TestModule::new(
            r#"
function test<'a>(v0: ref<int32, borrowed, 'a, readonly, local>, v1: ref<int32, borrowed, 'a, readonly, local>): void {
    local l0: ref<int32, borrowed, 'a, readonly, local>
    local l1: ref<int32, borrowed, 'a, readonly, local>

entry(v0: ref<int32, borrowed, 'a, readonly, local>, v1: ref<int32, borrowed, 'a, readonly, local>):
    store l0, v0
    store l1, v0
    v2: ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'frame, mutable, frame> = address l1
    store (*v2), v1
    jump done

done:
    v3: ref<int32, borrowed, 'a, readonly, local> = load l0
    v4: ref<int32, borrowed, 'a, readonly, local> = load l1
    v5: usize = cast.bit v3 -> usize
    v6: ref<int32, borrowed, 'a, readonly, local> = intrinsic.memory.raw.transmute(v5)
    v7: ref<int32, borrowed, 'a, readonly, local> = intrinsic.space.cast(v6)
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let table = analyses.place(program.entry_function_id(), &program.tree);

        assert_eq!(
            [Value(3), Value(4), Value(5), Value(6), Value(7)]
                .map(|value| table.get(value).clone()),
            [
                Place::value(Value(0)).with_projection(Projection::Deref),
                Place::value(Value(4)).with_projection(Projection::Deref),
                Place::value(Value(0)).with_projection(Projection::Deref),
                Place::value(Value(0)).with_projection(Projection::Deref),
                Place::value(Value(0)).with_projection(Projection::Deref),
            ]
        );
    }

    /// Constant propagation distinguishes projected array elements.
    #[test]
    fn test_separate_elements_with_distinct_constant_indices() {
        let program = TestModule::new(
            r#"
function test<'a>(v0: ref<[int32; 4], borrowed, 'a, mutable, local>): void {
entry(v0: ref<[int32; 4], borrowed, 'a, mutable, local>):
    v1: uint64 = 0
    v2: uint64 = 1
    v3: uint64 = add v1, v2
    v4: ref<int32, borrowed, 'a, mutable, local> = address (*v0)[v1]
    v5: ref<int32, borrowed, 'a, mutable, local> = address (*v0)[v3]
    return
}
"#,
        );

        let function_id = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let constants = analyses.constant(function_id, &program.tree);
        let places = analyses.place(function_id, &program.tree);

        // compare projections using direct and propagated constants
        assert!(!places.get(Value(4)).may_overlap(
            places.get(Value(5)),
            &constants,
            &places,
            function_id,
            &program.tree
        ));
    }

    /// Preserve the pointee storage of a reference merged through a local.
    #[test]
    fn test_resolve_merged_reference_storage() {
        let program = TestModule::new(
            r#"
function test(v0: boolean, v1: ref<int32, borrowed, 'static, readonly, shared>, v2: ref<int32, borrowed, 'static, readonly, shared>): void {
    local l0: ref<int32, borrowed, 'static, readonly, shared>

entry(v0: boolean, v1: ref<int32, borrowed, 'static, readonly, shared>, v2: ref<int32, borrowed, 'static, readonly, shared>):
    branch v0 => left | right

left:
    store l0, v1
    jump join

right:
    store l0, v2
    jump join

join:
    v3: ref<int32, borrowed, 'static, readonly, shared> = load l0
    return
}
"#,
        );
        let function = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let places = analyses.place(program.entry_function_id(), &program.tree);
        let local = Place::local(program.tree.get(function).local(0));
        let reference = places.get(Value(3));

        assert_eq!(reference, &local.clone().with_projection(Projection::Deref));
        assert_eq!(
            local.storage(program.entry_function_id(), &program.tree),
            Some(Storage::Frame)
        );
        assert_eq!(
            reference.storage(program.entry_function_id(), &program.tree),
            Some(Storage::Heap(Space::Shared))
        );

        // distinguish frame descriptors from their possibly aliased heap referents
        let constants = analyses.constant(program.entry_function_id(), &program.tree);
        let selected = [
            local,
            reference.clone(),
            Place::value(Value(1)),
            Place::value(Value(1)).with_projection(Projection::Deref),
        ];
        let actual = selected.each_ref().map(|left| {
            selected.each_ref().map(|right| {
                left.may_overlap(
                    right,
                    &constants,
                    &places,
                    program.entry_function_id(),
                    &program.tree,
                )
            })
        });
        assert_eq!(
            actual,
            [
                [true, false, false, false],
                [false, true, false, true],
                [false, false, true, false],
                [false, true, false, true],
            ]
        );
    }
}

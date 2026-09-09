use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_core::{FxIndexMap, FxIndexSet};

use crate::{
    Analysis, Block, CastOperator, ControlTable, Function, FunctionCache, GlobalId, Instruction,
    Intrinsic, LocalId, LocalNodeId, Mutation, Path, Projection, Tree, Value,
};

/// Canonical places for one MIR function.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlaceTable {
    /// The canonical place for each SSA value.
    values: Vec<Place>,
}

/// Root storage for one analyzed MIR place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum PlaceOrigin {
    /// A function-local stack slot.
    Local(LocalId),
    /// A module global.
    Global(GlobalId),
    /// Storage rooted in an SSA value.
    Value(Value),
}

/// One storage location derived from MIR address values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Place {
    /// The root storage.
    pub origin: PlaceOrigin,
    /// The structural path from the root.
    pub path: Path,
}

impl Place {
    /// Create a place from one origin.
    pub fn new(origin: PlaceOrigin) -> Self {
        Self {
            origin,
            path: Path::root(),
        }
    }

    /// Create a place rooted in one local.
    pub fn local(local: LocalId) -> Self {
        Self::new(PlaceOrigin::Local(local))
    }

    /// Create a place rooted in one global.
    pub fn global(global: GlobalId) -> Self {
        Self::new(PlaceOrigin::Global(global))
    }

    /// Create a place rooted in one SSA value.
    pub fn value(value: Value) -> Self {
        Self::new(PlaceOrigin::Value(value))
    }

    /// Return this place with one extra projection.
    pub fn with_projection(mut self, projection: Projection) -> Self {
        self.path.push(projection);

        self
    }

    /// Return this place with another path appended.
    pub fn with_path(mut self, path: &Path) -> Self {
        self.path = self.path.with_path(path);

        self
    }

    /// Append one projection.
    pub fn push(&mut self, projection: Projection) {
        self.path.push(projection);
    }

    /// Return whether this place contains another place.
    pub fn contains(&self, other: &Self) -> bool {
        self.origin == other.origin && self.path.contains(&other.path)
    }
}

impl PlaceTable {
    /// Build canonical places for one function.
    pub fn build(function: &Function, tree: &Tree) -> Self {
        let graph = ControlTable::build(function, tree);
        let mut resolutions = vec![Resolution::Unknown; function.value_types().len()];

        // root function parameters in their incoming values
        for parameter in &function.parameters {
            Self::set(
                &mut resolutions,
                parameter.value,
                Resolution::Known(Place::value(parameter.value)),
            );
        }

        // forward the reference a local holds through its loads, unless its address escapes
        let forwarded = Self::forwarded_locals(function, tree);
        let mut exits: FxIndexMap<LocalNodeId<Block>, FxIndexMap<LocalId, Resolution>> =
            FxIndexMap::default();

        // solve block parameters, held references, and address derivations together
        let mut is_changed = true;
        while is_changed {
            is_changed = false;

            for &block_id in function.blocks() {
                let block = tree.get(block_id);

                // merge every block parameter from its incoming arguments
                if Some(block_id) != function.entry() {
                    for (index, parameter) in block.parameters.iter().enumerate() {
                        let resolution =
                            Self::parameter(block_id, index, &graph, &resolutions, tree);
                        is_changed |= Self::set(&mut resolutions, parameter.value, resolution);
                    }
                }

                // merge the references the locals hold at entry from every predecessor
                let mut held = Self::merge_held(block_id, &graph, &exits, &forwarded);

                // derive instruction destinations from their storage operands
                for &instruction_id in &block.instructions {
                    let instruction = tree.get(instruction_id);
                    match instruction {
                        Instruction::LocalSet { local, value } if forwarded.contains(local) => {
                            held.insert(*local, Self::copy(*value, &resolutions));
                            continue;
                        }
                        Instruction::LocalGet { destination, local }
                            if forwarded.contains(local) =>
                        {
                            // forward the place of a local holding one reference, else name its dereference
                            let resolution = match held.get(local) {
                                Some(Resolution::Opaque) => Resolution::Known(
                                    Place::local(*local).with_projection(Projection::Deref),
                                ),
                                Some(resolution) => resolution.clone(),
                                None => Resolution::Unknown,
                            };
                            is_changed |= Self::set(&mut resolutions, *destination, resolution);
                            continue;
                        }
                        _ => {}
                    }
                    let Some(destination) = instruction.destination() else {
                        continue;
                    };
                    let resolution =
                        Self::instruction(destination, instruction, &resolutions, tree);
                    is_changed |= Self::set(&mut resolutions, destination, resolution);
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
                Resolution::Unknown | Resolution::Opaque => Place::value(Value::new(index as u32)),
            })
            .collect();

        Self { values }
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

    /// Resolve one instruction destination.
    fn instruction(
        destination: Value,
        instruction: &Instruction,
        resolutions: &[Resolution],
        tree: &Tree,
    ) -> Resolution {
        match instruction {
            Instruction::LocalAddr { local, .. } => Resolution::Known(Place::local(*local)),
            Instruction::GlobalAddr { global, .. } => Resolution::Known(Place::global(*global)),
            Instruction::FieldAddr {
                aggregate, field, ..
            } => Self::resolve_projection(
                *aggregate,
                Projection::Field { index: *field },
                resolutions,
            ),
            Instruction::ElementAddr { base, index, .. } => {
                Self::resolve_projection(*base, Projection::Index { index: *index }, resolutions)
            }
            Instruction::VariantPayloadAddr { variant, case, .. } => {
                Self::resolve_projection(*variant, Projection::Variant { case: *case }, resolutions)
            }
            Instruction::SliceView {
                source,
                start,
                length,
                ..
            } => Self::resolve_projection(
                *source,
                Projection::Slice {
                    start: *start,
                    length: *length,
                },
                resolutions,
            ),
            // keep the storage a reinterpreted address names
            Instruction::Cast {
                operator: CastOperator::Bitcast,
                argument,
                ..
            } => Self::copy(*argument, resolutions),
            Instruction::Intrinsic {
                intrinsic: Intrinsic::Transmute,
                arguments,
                ..
            } => match tree.get_values(*arguments) {
                [argument] => Self::copy(*argument, resolutions),
                _ => Resolution::Known(Place::value(destination)),
            },
            _ => Resolution::Known(Place::value(destination)),
        }
    }

    /// Return the locals holding a reference with an address kept inside the frame.
    fn forwarded_locals(function: &Function, tree: &Tree) -> FxIndexSet<LocalId> {
        let mut exposed = FxIndexSet::default();
        for &block_id in function.blocks() {
            for &instruction_id in &tree.get(block_id).instructions {
                if let Instruction::LocalAddr { local, .. } = tree.get(instruction_id) {
                    exposed.insert(*local);
                }
            }
        }

        function
            .locals()
            .iter()
            .copied()
            .filter(|local| {
                let held = tree.represented(tree.get(*local).ty);

                !exposed.contains(local) && tree.get(held).is_reference_representation()
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
                let Some(incoming) = exits.get(predecessor).and_then(|exit| exit.get(&local))
                else {
                    continue;
                };
                merged = match (&merged, incoming) {
                    (_, Resolution::Unknown) => merged,
                    (Resolution::Unknown, incoming) => incoming.clone(),
                    (Resolution::Known(current), Resolution::Known(next)) if current == next => {
                        merged
                    }
                    _ => Resolution::Opaque,
                };
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
        let mut place = None;

        // merge the matching argument from every incoming edge
        for &predecessor in graph.predecessors(block) {
            let predecessor_id = predecessor;
            let predecessor = tree.get(predecessor_id);
            let terminator = tree.get(predecessor.terminator);
            for (edge, target) in terminator
                .targets(tree, predecessor_id)
                .into_iter()
                .filter(|(_, target)| target.block == block)
            {
                let Some(parameters) = terminator.target_parameters(tree, edge.successor, target)
                else {
                    return Resolution::Opaque;
                };
                let parameter = tree.get(block).parameters[index].value;
                let Some(argument_index) = parameters
                    .iter()
                    .position(|candidate| candidate.value == parameter)
                else {
                    let argument = Place::value(parameter);
                    if place.as_ref().is_none_or(|place| place == &argument) {
                        place = Some(argument);
                        continue;
                    }

                    return Resolution::Opaque;
                };
                let Some(&argument) = target.arguments(tree).get(argument_index) else {
                    return Resolution::Opaque;
                };

                match resolutions.get(argument.id() as usize) {
                    Some(Resolution::Known(argument))
                        if place.as_ref().is_none_or(|place| place == argument) =>
                    {
                        place = Some(argument.clone());
                    }
                    Some(Resolution::Unknown) => {}
                    Some(Resolution::Known(_) | Resolution::Opaque) | None => {
                        return Resolution::Opaque;
                    }
                }
            }
        }

        place.map(Resolution::Known).unwrap_or(Resolution::Unknown)
    }

    /// Resolve one projected place.
    fn resolve_projection(
        value: Value,
        projection: Projection,
        resolutions: &[Resolution],
    ) -> Resolution {
        match resolutions.get(value.id() as usize) {
            Some(Resolution::Known(place)) => {
                Resolution::Known(place.clone().with_projection(projection))
            }
            Some(Resolution::Opaque) => {
                Resolution::Known(Place::value(value).with_projection(projection))
            }
            Some(Resolution::Unknown) | None => Resolution::Unknown,
        }
    }

    /// Copy one place resolution.
    fn copy(value: Value, resolutions: &[Resolution]) -> Resolution {
        match resolutions.get(value.id() as usize) {
            Some(Resolution::Known(place)) => Resolution::Known(place.clone()),
            Some(Resolution::Opaque) => Resolution::Known(Place::value(value)),
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

impl PlaceTable {
    /// Compute canonical places for one function.
    pub(crate) fn compute(function: &Function, tree: &Tree, _analyses: &mut FunctionCache) -> Self {
        Self::build(function, tree)
    }
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

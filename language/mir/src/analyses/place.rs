use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Analysis, Block, ControlTable, Function, FunctionCache, GlobalId, Instruction, LocalId,
    LocalNodeId, Mutation, Path, Projection, Tree, Value,
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

        // solve block parameters and address derivations together
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

                // derive instruction destinations from their storage operands
                for &instruction_id in &block.instructions {
                    let instruction = tree.get(instruction_id);
                    let Some(destination) = instruction.destination() else {
                        continue;
                    };
                    let resolution = Self::instruction(destination, instruction, &resolutions);
                    is_changed |= Self::set(&mut resolutions, destination, resolution);
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
            Instruction::TensorCast {
                tensor: argument, ..
            }
            | Instruction::TensorView { view: argument, .. }
            | Instruction::Pin {
                value: argument, ..
            } => Self::copy(*argument, resolutions),
            _ => Resolution::Known(Place::value(destination)),
        }
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
        let mut visited = Vec::new();

        // merge the matching argument from every incoming edge
        for &predecessor in graph.predecessors(block) {
            if visited.contains(&predecessor) {
                continue;
            }
            visited.push(predecessor);

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
    fn set(resolutions: &mut Vec<Resolution>, value: Value, resolution: Resolution) -> bool {
        let index = value.id() as usize;
        if index >= resolutions.len() {
            resolutions.resize(index + 1, Resolution::Unknown);
        }
        let current = &resolutions[index];
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
    const INVALIDATED_BY: Mutation = Mutation::CONTROL.union(Mutation::VALUE);
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

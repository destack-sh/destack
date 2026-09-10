use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_core::{FxIndexMap, FxIndexSet};

use crate::{
    Analysis, Block, CastOperator, ControlTable, Function, GlobalId, Instruction, Intrinsic,
    LocalId, LocalNodeId, Mutation, Path, Projection, Tree, Value,
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
    /// Analyse storage origins and address projections for one function.
    pub fn analyse(function: &Function, graph: &ControlTable, tree: &Tree) -> Self {
        let mut resolutions = vec![Resolution::Unknown; function.value_types().len()];

        // root function parameters in their incoming values
        for parameter in &function.parameters {
            Self::set(
                &mut resolutions,
                parameter.value,
                Resolution::Known(Place::value(parameter.value)),
            );
        }

        // forward stored references through locals whose addresses are never taken
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
                            Self::parameter(block_id, index, graph, &resolutions, tree);
                        is_changed |= Self::set(&mut resolutions, parameter.value, resolution);
                    }
                }

                // merge the references the locals hold at entry from every predecessor
                let mut held = Self::merge_held(block_id, graph, &exits, &forwarded);

                // derive instruction destinations from their storage operands
                for &instruction_id in &block.instructions {
                    let instruction = tree.get(instruction_id);
                    match instruction {
                        Instruction::LocalSet { local, value } if forwarded.contains(local) => {
                            held.insert(*local, Self::copy(*value, &resolutions));
                        }
                        Instruction::LocalGet { destination, local }
                            if forwarded.contains(local) =>
                        {
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
            Instruction::Select {
                then_value,
                else_value,
                ..
            } => {
                let left = Self::copy(*then_value, resolutions);
                let right = Self::copy(*else_value, resolutions);

                left.merge(right)
            }
            Instruction::NewComplete { value, .. } => Self::copy(*value, resolutions),

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
            _ => Resolution::Known(Place::value(destination)),
        }
    }

    /// Return reference locals accessed exclusively through local loads and stores.
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
                Resolution::Known(Place::value(parameter))
            } else {
                let arguments = target.arguments(tree);
                Self::copy(arguments[index - result_count], resolutions)
            };
            resolution = resolution.merge(incoming);
        }

        resolution
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
    use crate::analyses::tests::TestModule;

    /// Preserve agreeing references through selections and block arguments.
    #[test]
    fn test_merge_matching_places() {
        let program = TestModule::new(
            r#"
function test(v0: boolean, v1: ref<int32, unique, mutable, local>): ref<int32, unique, mutable, local> {
entry(v0: boolean, v1: ref<int32, unique, mutable, local>):
    v2: ref<int32, unique, mutable, local> = select v0, v1, v1
    branch v0 => join(v1) | join(v2)

join(v3: ref<int32, unique, mutable, local>):
    return v3
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let table = analyses.place(function, &program.tree);

        assert_eq!(
            table.values,
            vec![
                Place::value(Value(0)),
                Place::value(Value(1)),
                Place::value(Value(1)),
                Place::value(Value(1))
            ]
        );
    }

    /// Keep conflicting incoming references opaque while preserving their projections.
    #[test]
    fn test_merge_distinct_places() {
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
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let table = analyses.place(function, &program.tree);

        assert_eq!(
            table.values,
            vec![
                Place::value(Value(0)),
                Place::value(Value(1)),
                Place::value(Value(2)),
                Place::value(Value(3))
            ]
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
    v3: ref<[int32; 4], borrowed, 'a, mutable, local> = field.address v0, 0
    v4: ref<int32, borrowed, 'a, mutable, local> = element.address v3, v1
    v5: slice<int32, borrowed, 'a, mutable, local> = slice.view v3, v1, v2
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let table = analyses.place(function, &program.tree);
        let field = Place::value(Value(0)).with_projection(Projection::Field { index: 0 });

        assert_eq!(
            table.values,
            vec![
                Place::value(Value(0)),
                Place::value(Value(1)),
                Place::value(Value(2)),
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
function test<'a>(v0: ref<int32, borrowed, 'a, readonly, local>): void {
    local l0: ref<int32, borrowed, 'a, readonly, local>
    local l1: ref<int32, borrowed, 'a, readonly, local>

entry(v0: ref<int32, borrowed, 'a, readonly, local>):
    local.set l0, v0
    local.set l1, v0
    v1: ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'frame, readonly, frame> = local.address l1
    jump done

done:
    v2: ref<int32, borrowed, 'a, readonly, local> = local.get l0
    v3: ref<int32, borrowed, 'a, readonly, local> = local.get l1
    v4: usize = cast.bit v2 -> usize
    v5: ref<int32, borrowed, 'a, readonly, local> = intrinsic.memory.raw.transmute(v4)
    v6: ref<int32, borrowed, 'a, readonly, local> = intrinsic.space.cast(v5)
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let table = analyses.place(function, &program.tree);

        assert_eq!(
            table.values,
            vec![
                Place::value(Value(0)),
                Place::local(function.locals()[1]),
                Place::value(Value(0)),
                Place::value(Value(3)),
                Place::value(Value(0)),
                Place::value(Value(0)),
                Place::value(Value(0)),
            ]
        );
    }
}

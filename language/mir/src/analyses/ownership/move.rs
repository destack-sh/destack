use destack_core::{FxIndexMap, FxIndexSet};

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Analysis, Function, Instruction, Intrinsic, Local, LocalId, Mutation, NodeTable, Place,
    PlaceOrigin, PlaceTable, Projection, Reference, Tree, Type, TypeId, Value,
};

/// Dense structural paths whose initialization can change independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveTable {
    /// Move paths in dense identifier order.
    paths: Vec<MovePath>,
    /// Dense identifiers keyed by canonical place.
    ids: FxIndexMap<Place, MovePathId>,
    /// Root paths keyed by SSA value.
    values: Vec<Option<MovePathId>>,
    /// Root paths keyed by function local.
    locals: NodeTable<Local, Option<MovePathId>>,
    /// The owned pointee path each load or store address moves through.
    pointees: FxIndexMap<Value, MovePathId>,
    /// The reference results that assert initialized pointee storage.
    initialized_pointees: FxIndexSet<Value>,
}

impl MoveTable {
    /// Build move paths for one function.
    pub fn analyse(function: &Function, places: &PlaceTable, tree: &Tree) -> Self {
        let mut table = Self {
            paths: Vec::new(),
            ids: FxIndexMap::default(),
            values: vec![None; function.value_types().len()],
            locals: NodeTable::from_nodes(function.locals(), || None),
            pointees: FxIndexMap::default(),
            initialized_pointees: FxIndexSet::default(),
        };

        // create roots for every SSA value
        for (index, ty) in function.value_types().iter().enumerate() {
            let Some(ty) = ty else {
                continue;
            };

            // create an independent root for the SSA value
            let value = Value::new(index as u32);
            let path = table.insert(Place::value(value), *ty, None);
            table.values[index] = Some(path);
        }

        // create roots for every local
        for &local in function.locals() {
            let ty = tree.get(local).ty;

            // create an independent root for the local
            let path = table.insert(Place::local(local), ty, None);
            *table.locals.get_mut(local) = Some(path);
        }

        // collect types that MIR moves or reconstructs by projection
        let mut projected_types = FxIndexSet::default();
        let mut elements: FxIndexMap<TypeId, FxIndexSet<u32>> = FxIndexMap::default();
        for &block in function.blocks() {
            for &instruction in &tree.get(block).instructions {
                // retain only the static array elements this function projects
                if let Instruction::ElementGet {
                    aggregate, index, ..
                }
                | Instruction::ElementSet {
                    aggregate, index, ..
                } = tree.get(instruction)
                {
                    let ty = tree.represented(function.expect_value_type(*aggregate));
                    elements.entry(ty).or_default().insert(*index);
                }

                // collect types used by structural projections
                let aggregate = match tree.get(instruction) {
                    Instruction::FieldGet {
                        destination,
                        aggregate,
                        ..
                    } if table.value(*destination).is_some() => Some(*aggregate),
                    Instruction::ElementGet {
                        destination,
                        aggregate,
                        ..
                    } if table.value(*destination).is_some() => Some(*aggregate),
                    Instruction::FieldSet { aggregate, .. }
                    | Instruction::ElementSet { aggregate, .. } => Some(*aggregate),
                    _ => None,
                };
                if let Some(aggregate) = aggregate
                    && table.value(aggregate).is_some()
                {
                    projected_types.insert(function.expect_value_type(aggregate));
                }
            }
        }

        // expand every root of each projected type to the same shape
        let mut index = 0;
        while index < table.paths.len() {
            let path = MovePathId::new(index as u32);
            let ty = table.get(path).ty;
            if projected_types.contains(&ty) && table.children(path).is_empty() {
                let place = table.get(path).place.clone();
                table.expand(path, place, ty, &elements, tree);
            }

            index += 1;
        }

        // track pointee storage independently of the SSA values that own it
        for (index, ty) in function.value_types().iter().enumerate() {
            if ty.is_some_and(|ty| tree.get(tree.represented(ty)).is_unique_reference()) {
                let value = Value::new(index as u32);
                if let Some(path) = table.track_pointee(value, function, tree, places, &elements) {
                    table.pointees.insert(value, path);
                }
            }
        }

        // track the owned pointee each move-only load or store addresses
        for &block in function.blocks() {
            for &instruction in &tree.get(block).instructions {
                // record explicit assertions of initialized reference storage
                if let Some(destination) =
                    initialized_reference(tree.get(instruction), function, tree)
                    && let Some(path) =
                        table.track_pointee(destination, function, tree, places, &elements)
                {
                    table.pointees.insert(destination, path);
                    table.initialized_pointees.insert(destination);
                }

                // track each address used by a load or store
                let pointer = match tree.get(instruction) {
                    Instruction::Load { pointer, .. } | Instruction::Store { pointer, .. } => {
                        *pointer
                    }
                    _ => continue,
                };
                if table.pointees.contains_key(&pointer) {
                    continue;
                }

                // record the pointee this address moves through
                if let Some(path) = table.track_pointee(pointer, function, tree, places, &elements)
                {
                    table.pointees.insert(pointer, path);
                }
            }
        }

        // apply each observed structural shape to every value and storage root of that type
        let expanded = table
            .paths
            .iter()
            .filter(|path| !path.children.is_empty())
            .map(|path| path.ty)
            .chain(projected_types.iter().copied())
            .collect::<FxIndexSet<_>>();
        let mut index = 0;
        while index < table.paths.len() {
            let path = MovePathId::new(index as u32);
            let current = table.get(path);
            if current.children.is_empty() && expanded.contains(&current.ty) {
                let place = current.place.clone();
                let ty = current.ty;
                table.expand(path, place, ty, &elements, tree);
            }
            index += 1;
        }

        table
    }

    /// Return the owned pointee path one load or store address moves through, when it is tracked.
    pub fn pointee(&self, pointer: Value) -> Option<MovePathId> {
        self.pointees.get(&pointer).copied()
    }

    /// Return the storage one reference result asserts initialized.
    pub fn initialized_pointee(&self, destination: Value) -> Option<MovePathId> {
        self.initialized_pointees
            .contains(&destination)
            .then(|| self.pointee(destination))
            .flatten()
    }

    /// Track the owned storage one address points at: a local projection or a unique pointee.
    fn track_pointee(
        &mut self,
        pointer: Value,
        function: &Function,
        tree: &Tree,
        places: &PlaceTable,
        elements: &FxIndexMap<TypeId, FxIndexSet<u32>>,
    ) -> Option<MovePathId> {
        // distinguish addressed locals from allocated storage reached through an owner
        let mut place = places.get(pointer).clone();
        let root = match place.origin {
            PlaceOrigin::Local(local) => {
                if place.path.first() == Some(&Projection::Deref) {
                    let ty = tree.get(tree.represented(tree.get(local).ty));
                    let Type::Reference {
                        kind: Reference::Unique,
                        pointee,
                        ..
                    } = ty
                    else {
                        return None;
                    };
                    let root = Place::local(local).with_projection(Projection::Deref);

                    self.insert(root, *pointee, None)
                } else {
                    self.local(local)?
                }
            }
            PlaceOrigin::Value(value) => {
                let mut ty = tree.represented(function.expect_value_type(value));
                while let Type::Uninit { value } | Type::ManuallyDrop { value } = tree.type_definition(ty) {
                    ty = tree.represented(*value);
                }
                let Type::Reference {
                    kind: Reference::Unique,
                    pointee,
                    ..
                } = tree.type_definition(ty)
                else {
                    return None;
                };
                if place.path.first() != Some(&Projection::Deref) {
                    place.path.projections.insert(0, Projection::Deref);
                }
                let root = Place::value(value).with_projection(Projection::Deref);

                self.insert(root, *pointee, None)
            }
            PlaceOrigin::Global(_) => return None,
        };
        let prefix = self.get(root).place.path.projections.len();

        // expand the path one projection at a time down to the addressed storage
        let mut current = root;
        for (depth, projection) in place.path.projections.iter().enumerate().skip(prefix) {
            // expand the children of this step on first arrival
            if self.children(current).is_empty() {
                let ty = self.get(current).ty;
                let prefix = self.get(current).place.clone();
                self.expand(current, prefix, ty, elements, tree);
            }

            // step to the child the projection names
            let child = self.children(current).iter().copied().find(|child| {
                self.get(*child).place.path.projections.get(depth) == Some(projection)
            })?;
            current = child;
        }

        Some(current)
    }

    /// Return the number of move paths.
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    /// Return whether the table has no move paths.
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// Return one move path.
    pub fn get(&self, id: MovePathId) -> &MovePath {
        &self.paths[id.index()]
    }

    /// Return the root path for one SSA value.
    pub fn value(&self, value: Value) -> Option<MovePathId> {
        self.values.get(value.id() as usize).copied().flatten()
    }

    /// Return the root path for one local.
    pub fn local(&self, local: LocalId) -> Option<MovePathId> {
        *self.locals.get(local)
    }

    /// Return the path for one canonical place.
    pub fn place(&self, place: &Place) -> Option<MovePathId> {
        self.ids.get(place).copied()
    }

    /// Return the nearest move path containing one place.
    pub fn containing(&self, place: &Place) -> Option<MovePathId> {
        let mut place = place.clone();

        // walk toward the root until one tracked path contains the projection
        loop {
            if let Some(path) = self.place(&place) {
                return Some(path);
            }
            place.path.projections.pop()?;
        }
    }

    /// Return child paths in structural field order.
    pub fn children(&self, id: MovePathId) -> &[MovePathId] {
        &self.get(id).children
    }

    /// Iterate over root paths in dense identifier order.
    pub fn roots(&self) -> impl DoubleEndedIterator<Item = MovePathId> + '_ {
        self.paths.iter().enumerate().filter_map(|(index, path)| {
            path.parent
                .is_none()
                .then_some(MovePathId::new(index as u32))
        })
    }

    /// Return the structural root containing one path.
    pub fn root(&self, path: MovePathId) -> MovePathId {
        let mut root = path;

        // walk to the structural root
        while let Some(parent) = self.get(root).parent {
            root = parent;
        }

        root
    }

    /// Map one descendant between equal move-path trees.
    pub fn map(&self, path: MovePathId, from: MovePathId, to: MovePathId) -> MovePathId {
        let source = &self.get(from).place;
        let path = &self.get(path).place;
        if !source.contains(path) {
            unreachable!("move path is outside mapped root");
        }

        // append the relative source path to the destination root
        let offset = source.path.projections.len();
        let mut destination = self.get(to).place.clone();
        destination
            .path
            .projections
            .extend(path.path.projections[offset..].iter().cloned());

        self.place(&destination)
            .unwrap_or_else(|| unreachable!("mapped move paths have different shapes"))
    }

    /// Return the path and every structural descendant.
    pub fn descendants(&self, root: MovePathId) -> impl Iterator<Item = MovePathId> + '_ {
        self.paths.iter().enumerate().filter_map(move |(index, _)| {
            let id = MovePathId::new(index as u32);
            (id == root || self.is_ancestor(root, id)).then_some(id)
        })
    }

    /// Return whether `ancestor` structurally contains `path`.
    pub fn is_ancestor(&self, ancestor: MovePathId, path: MovePathId) -> bool {
        let mut parent = self.get(path).parent;

        // walk the parent chain to find the ancestor
        while let Some(current) = parent {
            if current == ancestor {
                return true;
            }

            parent = self.get(current).parent;
        }

        false
    }

    /// Expand one structural type into direct children.
    fn expand(
        &mut self,
        parent: MovePathId,
        place: Place,
        ty: TypeId,
        elements: &FxIndexMap<TypeId, FxIndexSet<u32>>,
        tree: &Tree,
    ) {
        let children = match tree.type_definition(ty) {
            Type::Struct { fields, .. } => fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    (
                        Projection::Field {
                            index: index as u32,
                        },
                        tree.get(*field).ty,
                    )
                })
                .collect::<Vec<_>>(),
            Type::Tuple { elements, .. } => elements
                .iter()
                .copied()
                .enumerate()
                .map(|(index, ty)| {
                    (
                        Projection::Field {
                            index: index as u32,
                        },
                        ty,
                    )
                })
                .collect(),
            Type::Newtype { inner, .. } => vec![(Projection::Field { index: 0 }, *inner)],
            // expand each case's payload behind its discriminant
            Type::Variant { cases, .. } => cases
                .iter()
                .enumerate()
                .map(|(case, payload)| (Projection::Variant { case: case as u32 }, payload.ty))
                .collect(),
            // track only used elements and retain whether they cover the entire array
            Type::FixedArray { element, length } => {
                let mut indices = elements
                    .get(&ty)
                    .into_iter()
                    .flat_map(|indices| indices.iter().copied())
                    .collect::<Vec<_>>();
                indices.sort_unstable();
                self.paths[parent.index()].is_exhaustive = tree
                    .static_value(*length)
                    .length()
                    .is_some_and(|length| length == indices.len() as u64);

                indices
                    .into_iter()
                    .map(|index| (Projection::Element { index }, *element))
                    .collect()
            }
            // expand an application through the type it stands for
            Type::Application { .. } => {
                let applied = tree.represented(ty);
                if applied != ty {
                    self.expand(parent, place, applied, elements, tree);
                }
                return;
            }
            _ => return,
        };

        // preserve source field order for deterministic diagnostics and drops
        for (projection, ty) in children {
            let child_place = place.clone().with_projection(projection);
            let child = self.insert(child_place, ty, Some(parent));
            self.paths[parent.index()].children.push(child);
        }
    }

    /// Insert one path or return its existing identifier.
    fn insert(&mut self, place: Place, ty: TypeId, parent: Option<MovePathId>) -> MovePathId {
        if let Some(id) = self.ids.get(&place) {
            if self.get(*id).parent != parent {
                unreachable!("move path has conflicting structural parents");
            }

            return *id;
        }

        // allocate and index the new move path
        let id = MovePathId::new(self.paths.len() as u32);
        self.paths.push(MovePath {
            place: place.clone(),
            ty,
            parent,
            children: Vec::new(),
            is_exhaustive: true,
        });
        self.ids.insert(place, id);

        id
    }
}

impl Analysis for MoveTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL
        .union(Mutation::VALUE)
        .union(Mutation::LAYOUT);
}

/// One independently movable structural place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovePath {
    /// The canonical storage place.
    pub place: Place,
    /// The value type stored at this path.
    pub ty: TypeId,
    /// The direct structural parent.
    pub parent: Option<MovePathId>,
    /// Direct structural children.
    pub children: Vec<MovePathId>,
    /// Whether the tracked children cover the complete aggregate.
    pub is_exhaustive: bool,
}

/// Dense identifier for one move path.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct MovePathId(u32);

impl MovePathId {
    /// Create an identifier from its dense index.
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Return the dense index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Return the reference result of an explicit initialization assertion.
fn initialized_reference(
    instruction: &Instruction,
    function: &Function,
    tree: &Tree,
) -> Option<Value> {
    // select reference transmutes with one operand and one result
    let Instruction::Intrinsic {
        destination: Some(destination),
        intrinsic: Intrinsic::Transmute,
        arguments,
    } = instruction
    else {
        return None;
    };
    let [argument] = tree.get_values(*arguments) else {
        unreachable!("transmute requires one argument");
    };
    let source = tree.get(tree.represented(function.expect_value_type(*argument)));
    let target = tree.get(tree.represented(function.expect_value_type(*destination)));

    // recognize the declared transition from uninitialized to initialized pointee storage
    let (
        Type::Reference {
            pointee: source, ..
        },
        Type::Reference {
            pointee: target, ..
        },
    ) = (source, target)
    else {
        return None;
    };
    let Type::Uninit { value } = tree.get(tree.represented(*source)) else {
        return None;
    };

    (tree.represented(*value) == tree.represented(*target)).then_some(*destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestModule;

    /// Track only the moved elements of a large fixed array.
    #[test]
    fn test_track_array_elements() {
        let program = TestModule::new(
            r#"
function test(v0: [ref<int32, unique, mutable, local>; 1048576]): ref<int32, unique, mutable, local> {
entry(v0: [ref<int32, unique, mutable, local>; 1048576]):
    v1: ref<int32, unique, mutable, local> = element.get v0, 19
    v2: ref<int32, unique, mutable, local> = element.get v0, 29
    return v1
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let paths = analyses.moves(function, &program.tree);
        let array = paths.value(Value(0)).unwrap();
        let actual = paths
            .descendants(array)
            .map(|path| paths.get(path).place.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            [
                Place::value(Value(0)),
                Place::value(Value(0)).with_projection(Projection::Element { index: 19 }),
                Place::value(Value(0)).with_projection(Projection::Element { index: 29 }),
            ]
        );
    }

    /// Keep an owner's initialization independent from the contents moved through it.
    #[test]
    fn test_separate_owner_and_pointee_move_paths() {
        let program = TestModule::new(
            r#"
function test(v0: ref<ref<int32, unique, mutable, local>, unique, mutable, local>): ref<int32, unique, mutable, local> {
entry(v0: ref<ref<int32, unique, mutable, local>, unique, mutable, local>):
    v1: ref<int32, unique, mutable, local> = load v0
    return v1
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let paths = analyses.moves(function, &program.tree);
        let owner = paths.value(Value(0)).unwrap();
        let pointee = paths.pointee(Value(0)).unwrap();
        let moved = paths.value(Value(1)).unwrap();

        assert_eq!(paths.get(owner).place, Place::value(Value(0)));
        assert_eq!(
            paths.get(pointee).place,
            Place::value(Value(0)).with_projection(Projection::Deref)
        );
        assert_eq!(paths.get(moved).place, Place::value(Value(1)));
        assert_eq!(paths.get(owner).parent, None);
        assert_eq!(paths.get(pointee).parent, None);
        assert_eq!(paths.get(moved).parent, None);
    }
}

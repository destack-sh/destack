use destack_core::{FxIndexMap, FxIndexSet};

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Analysis, FunctionId, Instruction, Local, LocalId, Mutation, NodeTable, Place, PlaceOrigin,
    PlaceTable, Projection, Reference, Substitution, Tree, Type, TypeId, Value, is_copy,
};

/// Dense structural paths whose initialization can change independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveTable {
    /// The function the paths belong to.
    function: FunctionId,
    /// Move paths in dense identifier order.
    paths: Vec<MovePath>,
    /// Dense identifiers keyed by canonical place.
    ids: FxIndexMap<Place, MovePathId>,
    /// Root paths keyed by SSA value.
    values: Vec<Option<MovePathId>>,
    /// Root paths keyed by function local.
    locals: NodeTable<Local, Option<MovePathId>>,
}

impl MoveTable {
    /// Build move paths for one function.
    pub fn analyse(function: FunctionId, places: &PlaceTable, tree: &Tree) -> Self {
        let mut table = Self {
            function,
            paths: Vec::new(),
            ids: FxIndexMap::default(),
            values: vec![None; tree.get(function).value_types().len()],
            locals: NodeTable::from_nodes(tree.get(function).locals(), || None),
        };

        // create roots for every SSA value
        for index in 0..tree.get(function).value_types().len() {
            let ty = tree.get(function).value_types()[index];
            let Some(ty) = ty else {
                continue;
            };

            // create an independent root for the SSA value
            let value = Value::new(index as u32);
            let path = table.insert(Place::value(value), ty, None, tree);
            table.values[index] = Some(path);
        }

        // create roots for every local
        for &local in tree.get(function).locals() {
            let ty = tree.get(local).ty;

            // create an independent root for the local
            let path = table.insert(Place::local(local), ty, None, tree);
            *table.locals.get_mut(local) = Some(path);
        }

        // collect types that MIR moves or reconstructs by projection
        let mut projected_types = FxIndexSet::default();
        let mut elements: FxIndexMap<TypeId, FxIndexSet<u32>> = FxIndexMap::default();
        for block_index in 0..tree.get(function).blocks().len() {
            let block = tree.get(function).blocks()[block_index];
            for index in 0..tree.get(block).instructions.len() {
                let instruction = tree.get(block).instructions[index];
                // retain only the static array elements this function projects
                if let Instruction::ElementGet {
                    aggregate, index, ..
                }
                | Instruction::ElementSet {
                    aggregate, index, ..
                } = &tree.get(instruction).clone()
                {
                    let ty = Substitution::resolve(
                        tree.get(function).expect_value_type(*aggregate),
                        tree,
                    );
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
                    projected_types.insert(tree.get(function).expect_value_type(aggregate));
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

        // track the storage selected by explicit memory operands
        for block_index in 0..tree.get(function).blocks().len() {
            let block = tree.get(function).blocks()[block_index];
            for index in 0..tree.get(block).instructions.len() {
                let instruction = tree.get(block).instructions[index];
                let instruction = tree.get(instruction).clone();
                let Some(place) = instruction.place() else {
                    continue;
                };
                let place = places.resolve_place(place);
                table.track_place(&place, tree, &elements);
            }
        }

        // apply each observed shape to every value and storage root of that type
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
            if current.children.is_empty()
                && (expanded.contains(&current.ty) || table.owns_pointee(path, tree))
            {
                let place = current.place.clone();
                let ty = current.ty;
                table.expand(path, place, ty, &elements, tree);
            }
            index += 1;
        }

        table
    }

    /// Return whether one path owns a pointee outside every containing allocation's type.
    fn owns_pointee(&self, path: MovePathId, tree: &Tree) -> bool {
        let ty = tree.storage_type(self.get(path).ty);
        let Type::Reference {
            kind: Reference::Unique,
            pointee,
            ..
        } = tree.type_definition(ty)
        else {
            return false;
        };
        if matches!(tree.get(tree.storage_type(*pointee)), Type::Uninit { .. }) {
            return false;
        }

        // bound recursive ownership at its first repeated pointee type
        let pointee = tree.storage_type(*pointee);
        let mut parent = self.get(path).parent;
        while let Some(current) = parent {
            if tree.storage_type(self.get(current).ty) == pointee {
                return false;
            }
            parent = self.get(current).parent;
        }

        true
    }

    /// Return the owned pointee path of one unique reference value.
    pub fn pointee(&self, pointer: Value) -> Option<MovePathId> {
        self.pointee_of(self.value(pointer)?)
    }

    /// Return the owned pointee path of one unique reference path.
    pub fn pointee_of(&self, path: MovePathId) -> Option<MovePathId> {
        self.children(path).iter().copied().find(|child| {
            self.get(*child).place.path.projections.last() == Some(&Projection::Deref)
        })
    }

    /// Track structural storage, crossing only owned references.
    fn track_place(
        &mut self,
        place: &Place,
        tree: &Tree,
        elements: &FxIndexMap<TypeId, FxIndexSet<u32>>,
    ) -> Option<MovePathId> {
        let mut current = match place.origin {
            PlaceOrigin::Local(local) => self.local(local)?,
            PlaceOrigin::Value(value) => self.value(value)?,
            PlaceOrigin::Global(_) => return None,
        };

        // visit each structural component and each owned allocation
        for (depth, projection) in place.path.projections.iter().enumerate() {
            if self.children(current).is_empty() {
                let path = self.get(current);
                self.expand(current, path.place.clone(), path.ty, elements, tree);
            }
            current = self.children(current).iter().copied().find(|child| {
                self.get(*child).place.path.projections.get(depth) == Some(projection)
            })?;
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

    /// Return the path and every structural descendant in preorder.
    pub fn descendants(&self, root: MovePathId) -> impl Iterator<Item = MovePathId> + '_ {
        let mut pending = vec![root];

        std::iter::from_fn(move || {
            let path = pending.pop()?;
            pending.extend(self.children(path).iter().rev().copied());

            Some(path)
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
        let children = match &{
            let ty = Substitution::resolve(ty, tree);
            tree.get(ty).clone()
        } {
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
            Type::Newtype { value, .. } => vec![(Projection::Field { index: 0 }, *value)],
            // expand the allocation a unique reference owns
            Type::Reference {
                kind: Reference::Unique,
                pointee,
                ..
            } => match tree.get(tree.storage_type(*pointee)) {
                Type::Uninit { .. } => return,
                _ => vec![(Projection::Deref, *pointee)],
            },
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
            _ => return,
        };

        // preserve source field order for deterministic diagnostics and drops
        for (projection, ty) in children {
            let child_place = place.clone().with_projection(projection);
            let child = self.insert(child_place, ty, Some(parent), tree);
            self.paths[parent.index()].children.push(child);
        }
    }

    /// Insert one path or return its existing identifier.
    fn insert(
        &mut self,
        place: Place,
        ty: TypeId,
        parent: Option<MovePathId>,
        tree: &Tree,
    ) -> MovePathId {
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
            is_copy: is_copy(tree, ty, &tree.get(self.function).generics),
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
    /// Whether values at this path copy on use, so no use moves them.
    pub is_copy: bool,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestModule;

    /// Track only the moved elements of a large fixed array, each with the allocation it owns.
    #[test]
    fn test_track_array_elements() {
        let program = TestModule::new(
            r#"
function test(v0: [ref<int32, unique, mutable>; 1048576]): ref<int32, unique, mutable> {
entry(v0: [ref<int32, unique, mutable>; 1048576]):
    v1: ref<int32, unique, mutable> = element.get v0, 19
    v2: ref<int32, unique, mutable> = element.get v0, 29
    return v1
}
"#,
        );
        let mut analyses = program.function_analyses();
        let paths = analyses.moves(program.entry_function_id(), &program.tree);
        let array = paths.value(Value(0)).unwrap();
        let actual = paths
            .descendants(array)
            .map(|path| paths.get(path).place.clone())
            .collect::<Vec<_>>();

        let element = |index| Place::value(Value(0)).with_projection(Projection::Element { index });
        assert_eq!(
            actual,
            [
                Place::value(Value(0)),
                element(19),
                element(19).with_projection(Projection::Deref),
                element(29),
                element(29).with_projection(Projection::Deref),
            ]
        );
    }

    /// Keep an owner's initialization independent from the contents moved through it.
    #[test]
    fn test_own_the_pointee_path_under_its_reference() {
        let program = TestModule::new(
            r#"
function test(v0: ref<ref<int32, unique, mutable>, unique, mutable>): ref<int32, unique, mutable> {
entry(v0: ref<ref<int32, unique, mutable>, unique, mutable>):
    v1: ref<int32, unique, mutable> = load (*v0)
    return v1
}
"#,
        );
        let mut analyses = program.function_analyses();
        let paths = analyses.moves(program.entry_function_id(), &program.tree);
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
        assert_eq!(paths.get(pointee).parent, Some(owner));
        assert_eq!(paths.children(owner), [pointee]);
        assert_eq!(paths.get(moved).parent, None);
    }
}

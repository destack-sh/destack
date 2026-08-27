use destack_core::{FxIndexMap, FxIndexSet};

use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    Analysis, Function, FunctionCache, Instruction, Local, LocalId, Mutation, NodeTable, Place,
    PlaceTable, Projection, Tree, Type, TypeId, Value,
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
}

impl MoveTable {
    /// Build move paths for one function.
    pub fn build(function: &Function, tree: &Tree, places: &PlaceTable) -> Self {
        let mut table = Self {
            paths: Vec::new(),
            ids: FxIndexMap::default(),
            values: vec![None; function.value_types().len()],
            locals: NodeTable::from_nodes(function.locals(), || None),
        };

        // create roots for every move-only SSA value
        for (index, ty) in function.value_types().iter().enumerate() {
            let Some(ty) = ty else {
                continue;
            };
            if tree.ty(*ty).copy(tree).is_yes() {
                continue;
            }

            let value = Value::new(index as u32);
            let path = table.insert(places.get(value).clone(), *ty, None);
            table.values[index] = Some(path);
        }

        // create roots for every move-only local
        for &local in function.locals() {
            let ty = tree.get(local).ty;
            if tree.ty(ty).copy(tree).is_yes() {
                continue;
            }

            let path = table.insert(Place::local(local), ty, None);
            *table.locals.get_mut(local) = Some(path);
        }

        // collect types that MIR moves or reconstructs by projection
        let mut projected_types = FxIndexSet::default();
        for &block in function.blocks() {
            for &instruction in &tree.get(block).instructions {
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
                table.expand(path, place, ty, tree);
            }

            index += 1;
        }

        table
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

        while let Some(current) = parent {
            if current == ancestor {
                return true;
            }

            parent = self.get(current).parent;
        }

        false
    }

    /// Expand one structural type into direct children.
    fn expand(&mut self, parent: MovePathId, place: Place, ty: TypeId, tree: &Tree) {
        let children = match tree.ty(ty) {
            Type::Struct { fields, .. } => fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    (
                        Projection::Field {
                            index: index as u32,
                        },
                        field.ty,
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
            Type::FixedArray {
                element, length, ..
            } => (0..*length)
                .map(|index| {
                    let index = u32::try_from(index)
                        .unwrap_or_else(|_| unreachable!("fixed array index exceeds MIR range"));

                    (Projection::Element { index }, *element)
                })
                .collect(),
            Type::Application { base, .. } => {
                self.expand(parent, place, *base, tree);
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

        let id = MovePathId::new(self.paths.len() as u32);
        self.paths.push(MovePath {
            place: place.clone(),
            ty,
            parent,
            children: Vec::new(),
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

impl MoveTable {
    /// Compute move paths for one function.
    pub(crate) fn compute(function: &Function, tree: &Tree, analyses: &mut FunctionCache) -> Self {
        let places = analyses.place(function, tree);

        Self::build(function, tree, &places)
    }
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

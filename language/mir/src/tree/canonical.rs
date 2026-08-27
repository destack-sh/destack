use std::fmt::Write;

use destack_core::{FxIndexMap, FxIndexSet};

use super::tree::Tree;
use super::r#type::TypeId;
use crate::TypeFold;

impl Tree {
    /// Return the strongly connected type component containing one type.
    pub fn cycle_members(&self, entry: TypeId) -> Vec<TypeId> {
        // gather the types reachable from the entry
        let mut visited = FxIndexSet::default();
        self.collect_reachable(entry, &mut visited);

        // retain types that can reach the entry
        let mut members = Vec::new();
        for id in visited {
            let mut visited = FxIndexSet::default();
            self.collect_reachable(id, &mut visited);
            if visited.contains(&entry) {
                members.push(id);
            }
        }

        members
    }

    /// Return the canonical serialization of one type graph.
    pub fn canonical_key(&self, root: TypeId) -> String {
        let mut indices = FxIndexMap::default();
        let mut key = String::new();
        self.canonical_write(root, &mut indices, &mut key);

        key
    }

    /// Return whether two type graphs are equal, cycles included.
    pub fn types_equal(&self, left: TypeId, right: TypeId) -> bool {
        let mut assumed = FxIndexMap::default();

        self.types_equal_assuming(left, right, &mut assumed)
    }

    /// Compare two type graphs under an assumption set.
    fn types_equal_assuming(
        &self,
        left: TypeId,
        right: TypeId,
        assumed: &mut FxIndexMap<(TypeId, TypeId), ()>,
    ) -> bool {
        if left == right {
            return true;
        }
        if assumed.insert((left, right), ()).is_some() {
            return true;
        }

        // compare the nodes shallowly with their children masked out
        let mut left_type = self.ty(left).clone();
        let mut right_type = self.ty(right).clone();
        left_type.map_types(&mut |_| left);
        right_type.map_types(&mut |_| left);
        if left_type != right_type {
            return false;
        }

        // recurse into the paired children
        let left_children = self.child_type_ids(left);
        let right_children = self.child_type_ids(right);
        if left_children.len() != right_children.len() {
            return false;
        }

        left_children
            .into_iter()
            .zip(right_children)
            .all(|(left, right)| self.types_equal_assuming(left, right, assumed))
    }

    /// Collect every type reachable from one canonical type.
    fn collect_reachable(&self, root: TypeId, visited: &mut FxIndexSet<TypeId>) {
        let mut queue = vec![root];
        while let Some(id) = queue.pop() {
            if !visited.insert(id) {
                continue;
            }
            for child in self.child_type_ids(id) {
                queue.push(child);
            }
        }
    }

    /// Return the direct child types of one canonical type.
    fn child_type_ids(&self, id: TypeId) -> Vec<TypeId> {
        let mut children = Vec::new();
        let mut ty = self.ty(id).clone();
        ty.map_types(&mut |child| {
            children.push(child);
            child
        });

        children
    }

    /// Serialize one node into the canonical key, indexing revisits.
    fn canonical_write(
        &self,
        id: TypeId,
        indices: &mut FxIndexMap<TypeId, usize>,
        key: &mut String,
    ) {
        // reference already serialized nodes by their visit index
        if let Some(index) = indices.get(&id) {
            let _ = write!(key, "@{index}");

            return;
        }

        // preserve nested nominal identities
        if !indices.is_empty()
            && let Some(symbol) = self.type_symbol(id)
        {
            let _ = write!(key, "#{symbol:?}");

            return;
        }
        indices.insert(id, indices.len());

        // serialize the payload with its child ids masked out
        let mut children = Vec::new();
        let mut canonical = self.ty(id).clone();
        canonical.map_types(&mut |child| {
            children.push(child);
            TypeId::new(u32::MAX)
        });
        let _ = write!(key, "{canonical:?}(");
        for child in children {
            self.canonical_write(child, indices, key);
            key.push(';');
        }
        key.push(')');
    }
}

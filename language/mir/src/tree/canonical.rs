use std::fmt::Write;

use destack_core::FxIndexMap;

use super::node::TypeId;
use super::tree::Tree;
use super::r#type::Type;

impl Tree {
    /// Return each member of one cycle component with its canonical key.
    ///
    /// Distinct dir spellings of one cyclic type lower to isomorphic graphs;
    /// keying every member by its canonical serialization lets later walks
    /// unify on the first interned identity, entered at any member.
    pub fn canonical_component(&self, entry: TypeId) -> Vec<(TypeId, String)> {
        // gather the nodes reachable from the entry
        let mut forward = Vec::new();
        let mut visited = FxIndexMap::default();
        self.collect_reachable(entry, &mut visited);
        for (id, _) in &visited {
            forward.push(*id);
        }

        // keep the members that reach the entry back: the cycle component
        let mut members = Vec::new();
        for id in forward {
            let mut visited = FxIndexMap::default();
            self.collect_reachable(id, &mut visited);
            if visited.contains_key(&entry) {
                members.push(id);
            }
        }

        // key each member by its canonical serialization
        members
            .into_iter()
            .map(|member| (member, self.canonical_key(member)))
            .collect()
    }

    /// Return the canonical serialization of one type graph.
    pub fn canonical_key(&self, root: TypeId) -> String {
        let mut indices = FxIndexMap::default();
        let mut key = String::new();
        self.canonical_write(root, &mut indices, &mut key);

        key
    }

    /// Return whether two type graphs are equal, cycles included.
    ///
    /// A revisited pair holds by assumption, so unrolled and rolled
    /// spellings of one recursive type compare equal.
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
        let mut left_type = self.get(left).clone();
        let mut right_type = self.get(right).clone();
        left_type.map_child_type_ids(&mut |_| left);
        right_type.map_child_type_ids(&mut |_| left);
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

    /// Collect every type reachable from one node through child edges.
    fn collect_reachable(&self, root: TypeId, visited: &mut FxIndexMap<TypeId, ()>) {
        let mut queue = vec![root];
        while let Some(id) = queue.pop() {
            if visited.insert(id, ()).is_some() {
                continue;
            }
            for child in self.child_type_ids(id) {
                queue.push(child);
            }
        }
    }

    /// Return the direct child types of one node, including struct field types.
    fn child_type_ids(&self, id: TypeId) -> Vec<TypeId> {
        let mut children = Vec::new();
        match self.get(id) {
            // struct children reach through their field nodes
            Type::Struct { fields, .. } => {
                for field in fields {
                    children.push(self.get(*field).ty);
                }
            }
            other => {
                let mut other = other.clone();
                other.map_child_type_ids(&mut |child| {
                    children.push(child);
                    child
                });
            }
        }

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

        // nominal identities serialize by name: isomorphic structure never
        //  unifies distinct nominals, and the root stays structural so
        //  isomorphic cycle spellings still meet
        if !indices.is_empty()
            && let Some(symbol) = self.type_symbol(id)
        {
            let _ = write!(key, "#{symbol:?}");

            return;
        }
        indices.insert(id, indices.len());

        match self.get(id) {
            // serialize struct types field by named field
            Type::Struct { fields, copy } => {
                let _ = write!(key, "struct{copy:?}(");
                for field in fields.clone() {
                    let field = self.get(field).clone();
                    let _ = write!(key, "{:?}:", field.name);
                    self.canonical_write(field.ty, indices, key);
                    key.push(';');
                }
                key.push(')');
            }
            // serialize the family payload with children canonicalized away
            other => {
                let mut children = Vec::new();
                let mut canonical = other.clone();
                canonical.map_child_type_ids(&mut |child| {
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
    }
}

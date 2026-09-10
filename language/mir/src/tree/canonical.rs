use std::fmt::Write;

use destack_core::FxIndexMap;

use super::node::TypeId;
use super::tree::Tree;
use super::r#type::Type;

impl Tree {
    /// Return each member of one cycle component with its canonical key.
    pub fn canonical_component(&self, entry: TypeId) -> Vec<(TypeId, String)> {
        // gather the nodes reachable from the entry
        let mut forward = Vec::new();
        let mut visited = FxIndexMap::default();
        self.collect_reachable(entry, &mut visited);
        for (id, _) in &visited {
            forward.push(*id);
        }

        // keep the members with a path back to the entry: the cycle component
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
    pub fn types_equal(&self, left: TypeId, right: TypeId) -> bool {
        let mut assumed = FxIndexMap::default();

        self.types_equal_assuming(left, right, &mut assumed)
    }

    /// Return whether two types share one representation, regions aside.
    pub fn same_representation(&self, left: TypeId, right: TypeId) -> bool {
        self.types_equal(self.represented(left), self.represented(right))
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

        // reject two declared nominals, which match by identity alone
        if self.type_declaration(left).is_some() && self.type_declaration(right).is_some() {
            return false;
        }

        // assume a pair already under comparison matches
        if assumed.insert((left, right), ()).is_some() {
            return true;
        }

        // compare struct heads by copy and field names, leaving field types to the children
        if let (
            Type::Struct {
                fields: left_fields,
                copy: left_copy,
            },
            Type::Struct {
                fields: right_fields,
                copy: right_copy,
            },
        ) = (self.get(left), self.get(right))
        {
            let names_match = left_fields.len() == right_fields.len()
                && left_fields
                    .iter()
                    .zip(right_fields)
                    .all(|(left, right)| self.get(*left).name == self.get(*right).name);
            if left_copy != right_copy || !names_match {
                return false;
            }
        } else {
            // compare other nodes shallowly with lifetimes erased and children masked out
            let mut left_type = self.get(left).erased_lifetime();
            let mut right_type = self.get(right).erased_lifetime();
            left_type.map_child_type_ids(&mut |_| left);
            right_type.map_child_type_ids(&mut |_| left);
            if left_type != right_type {
                return false;
            }
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
    pub fn child_type_ids(&self, id: TypeId) -> Vec<TypeId> {
        let mut children = Vec::new();
        match self.get(id) {
            // read struct children through their field nodes
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

        // serialize nominal identities by name, keeping the root structural so
        //  isomorphic cycles still meet
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

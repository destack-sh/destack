use std::hash::Hash;

use destack_core::stable_hash_value;

use crate::{Attribute, Field, LocalNodeId, Tree, Type, TypeId};

impl Tree {
    /// Find one type equal by structure.
    pub fn find_type(&self, ty: &Type) -> Option<TypeId> {
        let hash = Self::intern_hash(ty);
        let ids = self.type_index.get(&hash)?;

        ids.iter().copied().find(|id| self.get(*id) == ty)
    }

    /// Intern one type by structural equality.
    pub fn intern_type(&mut self, ty: Type) -> TypeId {
        // reuse an equal structural type
        let hash = Self::intern_hash(&ty);
        if let Some(ids) = self.type_index.get(&hash) {
            for id in ids {
                if self.get(*id) == &ty {
                    return *id;
                }
            }
        }

        // allocate and index one new structural type
        let local_id = self.types.allocate(ty);
        let id = self.insert_node(local_id);
        self.type_index.entry(hash).or_default().push(id);

        id
    }

    /// Intern one field and its attributes.
    pub fn intern_field(&mut self, field: Field, attributes: Vec<Attribute>) -> LocalNodeId<Field> {
        // reuse an equal field declaration
        let hash = Self::intern_hash(&(&field, &attributes));
        if let Some(ids) = self.field_index.get(&hash) {
            for id in ids {
                if self.get(*id) == &field && self.attributes(*id) == attributes {
                    return *id;
                }
            }
        }

        // allocate and index one new field declaration
        let local_id = self.fields.allocate(field);
        let id = self.insert_node(local_id);
        self.field_index.entry(hash).or_default().push(id);
        if !attributes.is_empty() {
            self.attributes_by_node_id.insert(id.id, attributes);
        }

        id
    }

    /// Compute one in-memory interning hash.
    pub(crate) fn intern_hash(value: &impl Hash) -> u64 {
        stable_hash_value(value)
    }
}

use std::fmt::{Debug, Formatter};
use std::hash::Hash;

use destack_core::{FrozenArena, SectionEntry, stable_hash_value};
use destack_serde::{Reflect, Schema, Type as ReflectType};
use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smallvec::SmallVec;


use crate::{
    Copy, Field, Space, SpaceJoinId, Static, StaticId, Storage, StorageJoinId, Tree, Type,
};

/// Compact identity of one interned type.
#[repr(transparent)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub struct TypeId(pub u32);

impl TypeId {
    /// Return the zero-based interning index.
    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Compact identity of one interned struct field.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct FieldId(pub u32);

/// The interned types and compile-time values of one tree.
///
/// Interning appends through a shared reference.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Reflect)]
pub struct TypeTable {
    /// The structural types, each beside the copy decision written for it.
    types: Interned<(Type, Copy)>,
    /// The struct fields.
    fields: Interned<Field>,
    /// The compile-time values.
    statics: Interned<Static>,
    /// The space join expressions.
    space_joins: Interned<Vec<Space>>,
    /// The storage join expressions.
    storage_joins: Interned<Vec<Storage>>,
}

impl Tree {
    /// Find one type equal by structure and copy decision.
    pub fn find_type(&self, ty: &Type, copy: Copy) -> Option<TypeId> {
        self.types.types.find(&(ty.clone(), copy)).map(TypeId)
    }

    /// Intern one reference or slice type at its referent typed uninitialized, the storage alone.
    pub fn emptied_type(&self, ty: TypeId) -> TypeId {
        let mut emptied = self.get(ty).clone();
        match &mut emptied {
            Type::Reference { pointee, .. } => {
                *pointee = self.intern_type(Type::Uninit { value: *pointee }, Copy::No);
            }
            Type::Slice { element, .. } => {
                *element = self.intern_type(Type::Uninit { value: *element }, Copy::No);
            }
            _ => unreachable!("emptied storage outside a unique reference or slice"),
        }

        self.intern_type(emptied, self.copy(ty))
    }

    /// Intern one type by structural equality, beside the copy decision written for it.
    pub fn intern_type(&self, ty: Type, copy: Copy) -> TypeId {
        TypeId(self.types.types.intern((ty, copy)))
    }

    /// Return the copy decision written for one type, a declaration naming its definition's.
    pub fn copy(&self, ty: TypeId) -> Copy {
        match self.types.types.get(ty.0) {
            (Type::Declaration { declaration }, _) => match self.get(*declaration).definition {
                Some(definition) => self.copy(definition),
                None => Copy::No,
            },
            (_, copy) => *copy,
        }
    }

    /// Intern one field by structural equality.
    pub fn intern_field(&self, field: Field) -> FieldId {
        FieldId(self.types.fields.intern(field))
    }

    /// Intern one compile-time value by structural equality.
    pub fn intern_static(&self, value: Static) -> StaticId {
        StaticId(self.types.statics.intern(value))
    }

    /// Return one interned compile-time value.
    pub fn static_value(&self, id: StaticId) -> &Static {
        self.types.statics.get(id.0)
    }

    /// Intern one space join expression in the supplied order.
    pub fn intern_space_join(&self, spaces: impl IntoIterator<Item = Space>) -> Space {
        Space::Join(SpaceJoinId(
            self.types.space_joins.intern(spaces.into_iter().collect()),
        ))
    }

    /// Return the members of one space join.
    pub fn space_join(&self, id: SpaceJoinId) -> &[Space] {
        self.types.space_joins.get(id.0)
    }

    /// Intern one storage join expression in the supplied order.
    pub fn intern_storage_join(&self, storages: impl IntoIterator<Item = Storage>) -> Storage {
        Storage::Join(StorageJoinId(
            self.types
                .storage_joins
                .intern(storages.into_iter().collect()),
        ))
    }

    /// Return the members of one storage join.
    pub fn storage_join(&self, id: StorageJoinId) -> &[Storage] {
        self.types.storage_joins.get(id.0)
    }

    /// Iterate every interned type in interning order.
    pub fn types(&self) -> impl Iterator<Item = (TypeId, &Type)> {
        self.types
            .types
            .iter()
            .enumerate()
            .map(|(index, (ty, _))| (TypeId(index as u32), ty))
    }

    /// Return the number of interned types.
    pub fn type_count(&self) -> usize {
        self.types.types.len()
    }

    /// Resolve one type id.
    #[inline]
    pub(crate) fn type_at(&self, id: TypeId) -> &Type {
        &self.types.types.get(id.0).0
    }

    /// Resolve one field id.
    #[inline]
    pub(crate) fn field_at(&self, id: FieldId) -> &Field {
        self.types.fields.get(id.0)
    }
}

/// Values interned by structural equality, appended through a shared reference.
struct Interned<T> {
    /// The values in interning order.
    values: FrozenArena<T>,
    /// The value indices grouped by structural hash.
    index: Mutex<FxHashMap<u64, SmallVec<[u32; 1]>>>,
}

impl<T: Hash + Eq> Interned<T> {
    /// Find one equal value.
    fn find(&self, value: &T) -> Option<u32> {
        let index = self.index.lock();
        let candidates = index.get(&stable_hash_value(value))?;

        candidates
            .iter()
            .copied()
            .find(|candidate| self.get(*candidate) == value)
    }

    /// Intern one value, reusing an equal one.
    fn intern(&self, value: T) -> u32 {
        let hash = stable_hash_value(&value);
        let mut index = self.index.lock();

        // reuse an equal value
        if let Some(candidates) = index.get(&hash)
            && let Some(existing) = candidates
                .iter()
                .copied()
                .find(|candidate| self.get(*candidate) == &value)
        {
            return existing;
        }

        // append and index one new value while holding the index lock
        let id = self.values.push(value);
        index.entry(hash).or_default().push(id);

        id
    }
}

impl<T> Interned<T> {
    /// Return one interned value.
    #[inline]
    fn get(&self, id: u32) -> &T {
        self.values
            .get(id)
            .unwrap_or_else(|| unreachable!("interned id {id} is out of range"))
    }

    /// Return the number of interned values.
    fn len(&self) -> usize {
        self.values.len()
    }

    /// Iterate the values in interning order.
    fn iter(&self) -> impl Iterator<Item = &T> {
        self.values.iter()
    }
}

impl<T: Hash> From<FrozenArena<T>> for Interned<T> {
    fn from(values: FrozenArena<T>) -> Self {
        // rebuild the hash index over the restored values
        let mut index = FxHashMap::<u64, SmallVec<[u32; 1]>>::default();
        for (position, value) in values.iter().enumerate() {
            index
                .entry(stable_hash_value(value))
                .or_default()
                .push(position as u32);
        }

        Self {
            values,
            index: Mutex::new(index),
        }
    }
}

impl<T: Serialize> Serialize for Interned<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.values.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de> + Hash> Deserialize<'de> for Interned<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        FrozenArena::deserialize(deserializer).map(Self::from)
    }
}

impl<T: Reflect> Reflect for Interned<T> {
    fn reflect(schema: &mut Schema) -> ReflectType {
        FrozenArena::<T>::reflect(schema)
    }
}

impl<T> Default for Interned<T> {
    fn default() -> Self {
        Self {
            values: FrozenArena::new(),
            index: Mutex::new(FxHashMap::default()),
        }
    }
}

impl<T: Clone + Hash> Clone for Interned<T> {
    fn clone(&self) -> Self {
        Self::from(self.values.clone())
    }
}

impl<T: Debug> Debug for Interned<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

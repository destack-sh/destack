use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;
use std::sync::Arc;

use destack_core::Arena;
use indexmap::IndexMap;
use smallvec::{Array, SmallVec};

use crate::{ModuleId, ProfileId, SourcePartKey, StringId};

/// One adapter for image-sensitive live values.
pub trait ImageAdapter {
    /// Adapt one embedded string id.
    fn adapt_string_id(&mut self, string_id: &mut StringId);
}

/// Adapt one live value through one image adapter.
pub trait AdaptImage {
    /// Adapt this value through one image adapter.
    fn adapt_image(&mut self, adapter: &mut impl ImageAdapter);
}

macro_rules! impl_adapt_image_noop {
    ($($ty:ty),* $(,)?) => {
        $(
            impl AdaptImage for $ty {
                fn adapt_image(&mut self, _adapter: &mut impl ImageAdapter) {
                }
            }
        )*
    };
}

impl AdaptImage for StringId {
    fn adapt_image(&mut self, adapter: &mut impl ImageAdapter) {
        adapter.adapt_string_id(self);
    }
}

impl<T> AdaptImage for Option<T>
where
    T: AdaptImage,
{
    fn adapt_image(&mut self, adapter: &mut impl ImageAdapter) {
        if let Some(value) = self {
            value.adapt_image(adapter);
        }
    }
}

impl<T> AdaptImage for Vec<T>
where
    T: AdaptImage,
{
    fn adapt_image(&mut self, adapter: &mut impl ImageAdapter) {
        for value in self {
            value.adapt_image(adapter);
        }
    }
}

impl<T, const N: usize> AdaptImage for [T; N]
where
    T: AdaptImage,
{
    fn adapt_image(&mut self, adapter: &mut impl ImageAdapter) {
        for value in self {
            value.adapt_image(adapter);
        }
    }
}

impl<T> AdaptImage for Box<T>
where
    T: AdaptImage,
{
    fn adapt_image(&mut self, adapter: &mut impl ImageAdapter) {
        self.as_mut().adapt_image(adapter);
    }
}

impl<T> AdaptImage for Arc<T>
where
    T: Clone + AdaptImage,
{
    fn adapt_image(&mut self, adapter: &mut impl ImageAdapter) {
        Arc::make_mut(self).adapt_image(adapter);
    }
}

impl<T> AdaptImage for Arena<T>
where
    T: AdaptImage,
{
    fn adapt_image(&mut self, adapter: &mut impl ImageAdapter) {
        for value in self.iter_mut() {
            value.adapt_image(adapter);
        }
    }
}

impl<T> AdaptImage for SmallVec<T>
where
    T: Array,
    T::Item: AdaptImage,
{
    fn adapt_image(&mut self, adapter: &mut impl ImageAdapter) {
        for value in self {
            value.adapt_image(adapter);
        }
    }
}

impl<K, V, S> AdaptImage for HashMap<K, V, S>
where
    K: Eq + Hash + AdaptImage,
    V: AdaptImage,
    S: BuildHasher + Clone,
{
    fn adapt_image(&mut self, adapter: &mut impl ImageAdapter) {
        let hasher = self.hasher().clone();
        let entries = std::mem::replace(self, HashMap::with_hasher(hasher.clone()));
        let mut rebuilt = HashMap::with_capacity_and_hasher(entries.len(), hasher);

        for (mut key, mut value) in entries {
            key.adapt_image(adapter);
            value.adapt_image(adapter);
            rebuilt.insert(key, value);
        }

        *self = rebuilt;
    }
}

impl<T, S> AdaptImage for HashSet<T, S>
where
    T: Eq + Hash + AdaptImage,
    S: BuildHasher + Clone,
{
    fn adapt_image(&mut self, adapter: &mut impl ImageAdapter) {
        let hasher = self.hasher().clone();
        let entries = std::mem::replace(self, HashSet::with_hasher(hasher.clone()));
        let mut rebuilt = HashSet::with_capacity_and_hasher(entries.len(), hasher);

        for mut entry in entries {
            entry.adapt_image(adapter);
            rebuilt.insert(entry);
        }

        *self = rebuilt;
    }
}

impl<K, V, S> AdaptImage for IndexMap<K, V, S>
where
    K: Eq + Hash + AdaptImage,
    V: AdaptImage,
    S: BuildHasher + Clone,
{
    fn adapt_image(&mut self, adapter: &mut impl ImageAdapter) {
        let hasher = self.hasher().clone();
        let entries = std::mem::replace(self, IndexMap::with_hasher(hasher.clone()));
        let mut rebuilt = IndexMap::with_capacity_and_hasher(entries.len(), hasher);

        for (mut key, mut value) in entries {
            key.adapt_image(adapter);
            value.adapt_image(adapter);
            rebuilt.insert(key, value);
        }

        *self = rebuilt;
    }
}

impl<A, B> AdaptImage for (A, B)
where
    A: AdaptImage,
    B: AdaptImage,
{
    fn adapt_image(&mut self, adapter: &mut impl ImageAdapter) {
        self.0.adapt_image(adapter);
        self.1.adapt_image(adapter);
    }
}

impl<A, B, C> AdaptImage for (A, B, C)
where
    A: AdaptImage,
    B: AdaptImage,
    C: AdaptImage,
{
    fn adapt_image(&mut self, adapter: &mut impl ImageAdapter) {
        self.0.adapt_image(adapter);
        self.1.adapt_image(adapter);
        self.2.adapt_image(adapter);
    }
}

impl<A, B, C, D> AdaptImage for (A, B, C, D)
where
    A: AdaptImage,
    B: AdaptImage,
    C: AdaptImage,
    D: AdaptImage,
{
    fn adapt_image(&mut self, adapter: &mut impl ImageAdapter) {
        self.0.adapt_image(adapter);
        self.1.adapt_image(adapter);
        self.2.adapt_image(adapter);
        self.3.adapt_image(adapter);
    }
}

impl<T> AdaptImage for PhantomData<T> {
    fn adapt_image(&mut self, _adapter: &mut impl ImageAdapter) {}
}

impl_adapt_image_noop!(
    (),
    bool,
    char,
    u8,
    u16,
    u32,
    u64,
    usize,
    i8,
    i16,
    i32,
    i64,
    isize,
    f32,
    f64,
    ModuleId,
    ProfileId,
    SourcePartKey,
);

use std::hash::{BuildHasher, Hash};
use std::sync::Arc;

use destack_core::StringId;
use destack_dir as dir;
use destack_source::{AdaptImage, ImageAdapter, ModuleId, ProfileId};
use indexmap::IndexMap;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{Loader, ModuleEdgeRelation};

/// Self-contained image form for one live artifact value.
pub trait Image: Serialize + DeserializeOwned + Sized {
    /// The live value represented by this image.
    type Live;

    /// Build one image from one live value.
    fn dehydrate(live: &Self::Live, context: &mut impl DehydrationContext) -> Self;

    /// Rehydrate one live value from this image.
    fn rehydrate(self, context: &mut impl HydrationContext) -> Self::Live;
}

/// One dehydration context for one persisted artifact image.
pub trait DehydrationContext {
    /// Dehydrate one live string id into one image-local string id.
    fn dehydrate_string_id(&mut self, string_id: StringId) -> StringId;
}

/// One hydration context for one persisted artifact image.
pub trait HydrationContext {
    /// Rehydrate one image-local string id into one live string id.
    fn rehydrate_string_id(&mut self, string_id: StringId) -> StringId;
}

/// One structural field that participates in image adaptation.
pub(crate) trait Field {
    /// Dehydrate this field through one image context.
    fn dehydrate(&mut self, context: &mut impl DehydrationContext);

    /// Rehydrate this field through one image context.
    fn rehydrate(&mut self, context: &mut impl HydrationContext);
}

/// One adapter that maps live string ids into image-local ids.
struct DehydratingImageAdapter<'a, C> {
    /// The active dehydration context.
    context: &'a mut C,
}

impl<C> ImageAdapter for DehydratingImageAdapter<'_, C>
where
    C: DehydrationContext,
{
    fn adapt_string_id(&mut self, string_id: &mut StringId) {
        *string_id = self.context.dehydrate_string_id(*string_id);
    }
}

/// One adapter that maps image-local string ids back into live ids.
struct RehydratingImageAdapter<'a, C> {
    /// The active hydration context.
    context: &'a mut C,
}

impl<C> ImageAdapter for RehydratingImageAdapter<'_, C>
where
    C: HydrationContext,
{
    fn adapt_string_id(&mut self, string_id: &mut StringId) {
        *string_id = self.context.rehydrate_string_id(*string_id);
    }
}

macro_rules! impl_passthrough_field {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Field for $ty {
                fn dehydrate(&mut self, _context: &mut impl DehydrationContext) {
                }

                fn rehydrate(&mut self, _context: &mut impl HydrationContext) {
                }
            }
        )*
    };
}

macro_rules! impl_contextual_field {
    ($($ty:path),* $(,)?) => {
        $(
            impl Field for $ty {
                fn dehydrate(&mut self, context: &mut impl DehydrationContext) {
                    let mut adapter = DehydratingImageAdapter { context };
                    self.adapt_image(&mut adapter);
                }

                fn rehydrate(&mut self, context: &mut impl HydrationContext) {
                    let mut adapter = RehydratingImageAdapter { context };
                    self.adapt_image(&mut adapter);
                }
            }
        )*
    };
}

impl Field for StringId {
    fn dehydrate(&mut self, context: &mut impl DehydrationContext) {
        *self = context.dehydrate_string_id(*self);
    }

    fn rehydrate(&mut self, context: &mut impl HydrationContext) {
        *self = context.rehydrate_string_id(*self);
    }
}

impl<T> Field for Option<T>
where
    T: Field,
{
    fn dehydrate(&mut self, context: &mut impl DehydrationContext) {
        if let Some(value) = self {
            value.dehydrate(context);
        }
    }

    fn rehydrate(&mut self, context: &mut impl HydrationContext) {
        if let Some(value) = self {
            value.rehydrate(context);
        }
    }
}

impl<T> Field for Vec<T>
where
    T: Field,
{
    fn dehydrate(&mut self, context: &mut impl DehydrationContext) {
        for value in self {
            value.dehydrate(context);
        }
    }

    fn rehydrate(&mut self, context: &mut impl HydrationContext) {
        for value in self {
            value.rehydrate(context);
        }
    }
}

impl<T> Field for Arc<T>
where
    T: Clone + Field,
{
    fn dehydrate(&mut self, context: &mut impl DehydrationContext) {
        Arc::make_mut(self).dehydrate(context);
    }

    fn rehydrate(&mut self, context: &mut impl HydrationContext) {
        Arc::make_mut(self).rehydrate(context);
    }
}

impl<K, V, S> Field for IndexMap<K, V, S>
where
    K: Eq + Hash + Field,
    V: Field,
    S: BuildHasher + Clone,
{
    fn dehydrate(&mut self, context: &mut impl DehydrationContext) {
        let hasher = self.hasher().clone();
        let entries = std::mem::replace(self, IndexMap::with_hasher(hasher.clone()));
        let mut rebuilt = IndexMap::with_capacity_and_hasher(entries.len(), hasher);

        for (mut key, mut value) in entries {
            key.dehydrate(context);
            value.dehydrate(context);
            rebuilt.insert(key, value);
        }

        *self = rebuilt;
    }

    fn rehydrate(&mut self, context: &mut impl HydrationContext) {
        let hasher = self.hasher().clone();
        let entries = std::mem::replace(self, IndexMap::with_hasher(hasher.clone()));
        let mut rebuilt = IndexMap::with_capacity_and_hasher(entries.len(), hasher);

        for (mut key, mut value) in entries {
            key.rehydrate(context);
            value.rehydrate(context);
            rebuilt.insert(key, value);
        }

        *self = rebuilt;
    }
}

impl<A, B> Field for (A, B)
where
    A: Field,
    B: Field,
{
    fn dehydrate(&mut self, context: &mut impl DehydrationContext) {
        self.0.dehydrate(context);
        self.1.dehydrate(context);
    }

    fn rehydrate(&mut self, context: &mut impl HydrationContext) {
        self.0.rehydrate(context);
        self.1.rehydrate(context);
    }
}

impl<A, B, C, D> Field for (A, B, C, D)
where
    A: Field,
    B: Field,
    C: Field,
    D: Field,
{
    fn dehydrate(&mut self, context: &mut impl DehydrationContext) {
        self.0.dehydrate(context);
        self.1.dehydrate(context);
        self.2.dehydrate(context);
        self.3.dehydrate(context);
    }

    fn rehydrate(&mut self, context: &mut impl HydrationContext) {
        self.0.rehydrate(context);
        self.1.rehydrate(context);
        self.2.rehydrate(context);
        self.3.rehydrate(context);
    }
}

impl_passthrough_field!(
    ModuleId,
    ProfileId,
    ModuleEdgeRelation,
    Loader,
    dir::LocalNodeIdAny,
    dir::LocalScopeId,
    dir::LocalSymbolId,
    dir::SymbolSpace
);

impl<T> Field for dir::LocalNodeId<T>
where
    T: dir::Node,
{
    fn dehydrate(&mut self, _context: &mut impl DehydrationContext) {}

    fn rehydrate(&mut self, _context: &mut impl HydrationContext) {}
}

impl_contextual_field!(
    dir::Tree,
    dir::SymbolTable,
    dir::TypeTable,
    dir::CaptureTable,
    dir::ModuleBinding,
    dir::ModuleBindingExports,
    dir::ModuleResolution,
    dir::StaticKey,
    dir::Export,
    dir::NamespaceExport,
);

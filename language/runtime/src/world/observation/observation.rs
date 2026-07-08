use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::host::ResourceId;
use crate::runtime::{RuntimeId, WorkerId};
use crate::world::topology::LabelSet;

use super::{ObservationCategory, ObservationScope};

/// One emitted observable fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    /// The observation category.
    pub category: ObservationCategory,
    /// The observation scope.
    pub scope: ObservationScope,
    /// Stable observation name.
    name: Cow<'static, str>,
    /// Structured observation labels.
    labels: LabelSet,
    /// Structured observation annotations.
    annotations: LabelSet,
}

impl Observation {
    /// Create one observation from explicit parts.
    pub fn new(
        category: ObservationCategory,
        scope: ObservationScope,
        name: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            category,
            scope,
            name: name.into(),
            labels: LabelSet::new(),
            annotations: LabelSet::new(),
        }
    }

    /// Create one world-scoped structured-annotation observation.
    pub fn world_annotations<K, V>(
        category: ObservationCategory,
        name: impl Into<String>,
        annotations: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self::annotations(category, ObservationScope::world(), name, annotations)
    }

    /// Create one runtime-scoped structured-annotation observation.
    pub fn runtime_annotations<K, V>(
        category: ObservationCategory,
        runtime_id: RuntimeId,
        name: impl Into<String>,
        annotations: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self::annotations(
            category,
            ObservationScope::runtime(runtime_id),
            name,
            annotations,
        )
    }

    /// Create one worker-scoped structured-annotation observation.
    pub fn worker_annotations<K, V>(
        category: ObservationCategory,
        runtime_id: Option<RuntimeId>,
        worker_id: WorkerId,
        name: impl Into<String>,
        annotations: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self::annotations(
            category,
            ObservationScope::worker(runtime_id, worker_id),
            name,
            annotations,
        )
    }

    /// Create one entity-scoped structured-annotation observation.
    pub fn entity_annotations<K, V>(
        category: ObservationCategory,
        entity_id: impl Into<String>,
        name: impl Into<String>,
        annotations: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self::annotations(
            category,
            ObservationScope::entity(entity_id),
            name,
            annotations,
        )
    }

    /// Create one edge-scoped structured-annotation observation.
    pub fn edge_annotations<K, V>(
        category: ObservationCategory,
        edge_id: impl Into<String>,
        name: impl Into<String>,
        annotations: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self::annotations(category, ObservationScope::edge(edge_id), name, annotations)
    }

    /// Create one structured-annotation observation.
    pub fn annotations<K, V>(
        category: ObservationCategory,
        scope: ObservationScope,
        name: impl Into<String>,
        entries: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        let mut annotations = LabelSet::new();
        for (key, value) in entries {
            annotations.insert(key.into().into_boxed_str(), value.into().into_boxed_str());
        }

        Self {
            category,
            scope,
            name: Cow::Owned(name.into()),
            labels: LabelSet::new(),
            annotations,
        }
    }

    /// Create one resource-attached observation.
    pub fn resource_attached(worker_id: WorkerId, resource_id: ResourceId) -> Self {
        Self::annotations(
            ObservationCategory::Resource,
            ObservationScope::resource(resource_id),
            "runtime.resource.attached",
            [
                ("worker_id", worker_id.0.to_string()),
                ("resource_id", resource_id.local_id.to_string()),
            ],
        )
    }

    /// Create one resource-detached observation.
    pub fn resource_detached(worker_id: WorkerId, resource_id: ResourceId) -> Self {
        Self::annotations(
            ObservationCategory::Resource,
            ObservationScope::resource(resource_id),
            "runtime.resource.detached",
            [
                ("worker_id", worker_id.0.to_string()),
                ("resource_id", resource_id.local_id.to_string()),
            ],
        )
    }

    /// Return the stable observation name.
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Return one copy of this observation with one additional label.
    pub fn label(mut self, key: impl Into<Box<str>>, value: impl Into<Box<str>>) -> Self {
        self.labels.insert(key, value);
        self
    }

    /// Return the value for one observation annotation when present.
    pub fn annotation(&self, key: &str) -> Option<&str> {
        self.annotations.get(key)
    }

    /// Return the value for one observation label when present.
    pub fn label_value(&self, key: &str) -> Option<&str> {
        self.labels.get(key)
    }
}

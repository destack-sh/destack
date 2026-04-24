use std::collections::BTreeMap;

use crate::platform::runtime::{
    BranchFilterValue, CheckpointFilterValue, ImageFilterValue, ResourceFilterValue,
    RevisionFilterValue, RuntimeExecutionMode, RuntimeFilterValue, RuntimeLabelSelectorValue,
    RuntimeLabelValue, RuntimeWorldKind, TopologyEdgeFilterValue, TopologyEdgeIdValue,
    TopologyEdgeKindValue, TopologyEntityFilterValue, TopologyEntityIdValue,
    TopologyEntityKindValue, WorkerFilterValue, WorldCreateOptionsValue,
};
use crate::runtime::control::inspect::{
    BranchListFilter, CheckpointListFilter, EdgeListFilter, EntityListFilter, ImageListFilter,
    ResourceListFilter, RevisionListFilter, RuntimeListFilter, WorkerListFilter,
};
use crate::runtime::observe::ObservationOptions;
use destack_workspace::{
    ExecutionMode, RuntimeAuthority, RuntimeOptions, RuntimeWorkerOptions, RuntimeWorld,
};

use super::RuntimeHandleCodec;

/// Request and filter normalization for low-level runtime bindings.
pub(crate) struct RuntimeRequestCodec;

impl RuntimeRequestCodec {
    /// Convert optional runtime labels into one label map.
    pub(crate) fn labels_from_value(
        labels: Option<Vec<RuntimeLabelValue>>,
    ) -> BTreeMap<String, String> {
        let mut values = BTreeMap::new();

        // copy label entries
        for label in labels.into_iter().flatten() {
            values.insert(label.key, label.value);
        }

        values
    }

    /// Convert optional label selectors into runtime selector tuples.
    pub(crate) fn label_selectors_from_value(
        selectors: Option<Vec<RuntimeLabelSelectorValue>>,
    ) -> Vec<(String, Option<String>)> {
        let mut values = Vec::new();

        // copy selector entries
        for selector in selectors.into_iter().flatten() {
            values.push((selector.key, selector.value));
        }

        values
    }

    /// Convert one optional topology entity id into one semantic selector.
    pub(crate) fn topology_entity_id_from_value(
        value: Option<TopologyEntityIdValue>,
    ) -> Option<String> {
        value.map(|value| value.0)
    }

    /// Convert one optional topology edge id into one semantic selector.
    pub(crate) fn topology_edge_id_from_value(
        value: Option<TopologyEdgeIdValue>,
    ) -> Option<String> {
        value.map(|value| value.0)
    }

    /// Convert one optional topology entity kind into one semantic selector.
    pub(crate) fn topology_entity_kind_from_value(
        value: Option<TopologyEntityKindValue>,
    ) -> Option<String> {
        value.map(|value| value.0)
    }

    /// Convert one optional topology edge kind into one semantic selector.
    pub(crate) fn topology_edge_kind_from_value(
        value: Option<TopologyEdgeKindValue>,
    ) -> Option<String> {
        value.map(|value| value.0)
    }

    /// Convert one world-create request into runtime options and labels.
    pub(crate) fn create_world_from_value(
        options: WorldCreateOptionsValue,
    ) -> (RuntimeOptions, BTreeMap<String, String>) {
        let mut runtime_options = RuntimeOptions::default();

        // optional execution mode
        if let Some(execution) = options.execution {
            let execution = match execution {
                RuntimeExecutionMode::Fast => ExecutionMode::Fast,
                RuntimeExecutionMode::Deterministic => ExecutionMode::Deterministic,
                RuntimeExecutionMode::Record => ExecutionMode::Record,
                RuntimeExecutionMode::Replay => ExecutionMode::Replay,
            };
            runtime_options.set_execution_mode(execution);
        }

        // optional world kind
        if let Some(world) = options.world {
            runtime_options.policy.world = match world {
                RuntimeWorldKind::Host => RuntimeWorld::Host,
                RuntimeWorldKind::Simulation => RuntimeWorld::Simulation,
            };
        }

        // simulation worlds default to virtual time
        if runtime_options.policy.world == RuntimeWorld::Simulation {
            runtime_options.policy.time.authority = RuntimeAuthority::Simulation;
            runtime_options.policy.random.authority = RuntimeAuthority::Simulation;
        }

        let labels = Self::labels_from_value(options.labels);

        (runtime_options, labels)
    }

    /// Build one runtime-create options object from decoded runtime fields.
    pub(crate) fn runtime_create_options(
        name: Option<String>,
        labels: Option<Vec<RuntimeLabelValue>>,
    ) -> RuntimeOptions {
        RuntimeOptions {
            name,
            labels: Self::labels_from_value(labels),
            ..RuntimeOptions::default()
        }
    }

    /// Build one worker-create options object from decoded worker fields.
    pub(crate) fn worker_create_options(
        name: Option<String>,
        labels: Option<Vec<RuntimeLabelValue>>,
    ) -> RuntimeOptions {
        RuntimeOptions {
            primary_worker: RuntimeWorkerOptions {
                name,
                labels: Self::labels_from_value(labels),
            },
            ..RuntimeOptions::default()
        }
    }

    /// Convert one world kind and execution mode pair into runtime options.
    pub(crate) fn runtime_options_from_create(
        execution: Option<RuntimeExecutionMode>,
        world: Option<RuntimeWorldKind>,
    ) -> RuntimeOptions {
        let (runtime_options, _) = Self::create_world_from_value(WorldCreateOptionsValue {
            engine: None,
            execution,
            world,
            labels: None,
        });

        runtime_options
    }

    /// Convert one optional low-level list limit into one optional Rust limit.
    pub(crate) fn list_limit(limit: Option<u32>) -> Option<usize> {
        limit.filter(|limit| *limit > 0).map(|limit| limit as usize)
    }

    /// Convert one optional low-level list limit into one bounded Rust limit.
    pub(crate) fn list_limit_or_max(limit: Option<u32>) -> usize {
        Self::list_limit(limit).unwrap_or(usize::MAX)
    }

    /// Convert one runtime filter into the shared inspect filter.
    pub(crate) fn runtime_filter_from_value(
        filter: Option<RuntimeFilterValue>,
    ) -> RuntimeListFilter {
        let Some(filter) = filter else {
            return RuntimeListFilter::default();
        };

        RuntimeListFilter {
            name: filter.name,
            labels: Self::label_selectors_from_value(filter.labels),
        }
    }

    /// Convert one worker filter into the shared inspect filter.
    pub(crate) fn worker_filter_from_value(filter: Option<WorkerFilterValue>) -> WorkerListFilter {
        let Some(filter) = filter else {
            return WorkerListFilter::default();
        };

        WorkerListFilter {
            runtime_id: filter.runtime_id.map(RuntimeHandleCodec::decode_runtime_id),
            name: filter.name,
            has_pending_work: filter.has_pending_work,
            labels: Self::label_selectors_from_value(filter.labels),
        }
    }

    /// Convert one branch filter into the shared inspect filter.
    pub(crate) fn branch_filter_from_value(filter: Option<BranchFilterValue>) -> BranchListFilter {
        let Some(filter) = filter else {
            return BranchListFilter::default();
        };

        BranchListFilter {
            name: filter.name,
            labels: Self::label_selectors_from_value(filter.labels),
        }
    }

    /// Convert one revision filter into the shared inspect filter.
    pub(crate) fn revision_filter_from_value(
        filter: Option<RevisionFilterValue>,
    ) -> RevisionListFilter {
        let Some(filter) = filter else {
            return RevisionListFilter::default();
        };

        RevisionListFilter {
            branch_id: filter.branch_id.map(RuntimeHandleCodec::decode_branch_id),
        }
    }

    /// Convert one checkpoint filter into the shared inspect filter.
    pub(crate) fn checkpoint_filter_from_value(
        filter: Option<CheckpointFilterValue>,
    ) -> CheckpointListFilter {
        let Some(filter) = filter else {
            return CheckpointListFilter::default();
        };

        CheckpointListFilter {
            revision: filter
                .revision_id
                .map(RuntimeHandleCodec::decode_revision_id),
            name: filter.name,
            labels: Self::label_selectors_from_value(filter.labels),
        }
    }

    /// Convert one image filter into the shared inspect filter.
    pub(crate) fn image_filter_from_value(filter: Option<ImageFilterValue>) -> ImageListFilter {
        let Some(filter) = filter else {
            return ImageListFilter::default();
        };

        ImageListFilter {
            revision: filter
                .revision_id
                .map(RuntimeHandleCodec::decode_revision_id),
        }
    }

    /// Convert one resource filter into the shared inspect filter.
    pub(crate) fn resource_filter_from_value(
        filter: Option<ResourceFilterValue>,
    ) -> ResourceListFilter {
        let Some(filter) = filter else {
            return ResourceListFilter::default();
        };

        ResourceListFilter {
            runtime_id: filter.runtime_id.map(RuntimeHandleCodec::decode_runtime_id),
            worker_id: filter.worker_id.map(RuntimeHandleCodec::decode_worker_id),
            kind: filter.kind,
            label: filter.label,
        }
    }

    /// Convert one topology entity filter into the shared inspect filter.
    pub(crate) fn entity_filter_from_value(
        filter: Option<TopologyEntityFilterValue>,
    ) -> EntityListFilter {
        let Some(filter) = filter else {
            return EntityListFilter::default();
        };

        EntityListFilter {
            kind: Self::topology_entity_kind_from_value(filter.kind),
            labels: Self::label_selectors_from_value(filter.labels),
        }
    }

    /// Convert one topology edge filter into the shared inspect filter.
    pub(crate) fn edge_filter_from_value(
        filter: Option<TopologyEdgeFilterValue>,
    ) -> EdgeListFilter {
        let Some(filter) = filter else {
            return EdgeListFilter::default();
        };

        EdgeListFilter {
            kind: Self::topology_edge_kind_from_value(filter.kind),
            from: Self::topology_entity_id_from_value(filter.from),
            to: Self::topology_entity_id_from_value(filter.to),
            labels: Self::label_selectors_from_value(filter.labels),
        }
    }

    /// Build observation options from low-level observation flags.
    pub(crate) fn observation_options_from_flags(
        trace: Option<bool>,
        topology: Option<bool>,
        resources: Option<bool>,
        scheduler: Option<bool>,
        diagnostics: Option<bool>,
        profiles: Option<bool>,
    ) -> ObservationOptions {
        let trace = trace.unwrap_or(false);

        ObservationOptions {
            runtime: trace,
            topology: topology.unwrap_or(false),
            resource: resources.unwrap_or(false),
            scheduler: scheduler.unwrap_or(false),
            diagnostic: diagnostics.unwrap_or(false),
            telemetry: profiles.unwrap_or(false),
            domain: trace,
        }
    }
}

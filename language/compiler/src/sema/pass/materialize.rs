use std::sync::Arc;

use destack_artifact::{DiagnosticRecord, DirMaterialized};
use destack_repository::ArtifactAttemptRecorder;

use crate::CompilerResult;
use crate::sema::CheckState;
use crate::sema::materialize::InstanceWorklist;

impl CheckState<'_> {
    /// Run the materialize pass: evaluate open computations and close instances.
    pub(in crate::sema) fn run_materialize(&mut self) -> CompilerResult<()> {
        // load the external modules this module reads
        self.import_external_modules()?;

        // materialize the module's own entries, then the instances they reach
        let recorder = self.recorder;
        let mut worklist = InstanceWorklist::default();
        ArtifactAttemptRecorder::breakdown_maybe(recorder, "materialize.entries", || {
            self.materialize_module_entries(&mut worklist)
        })?;
        ArtifactAttemptRecorder::breakdown_maybe(recorder, "materialize.bodies", || {
            self.materialize_member_bodies(&mut worklist)
        })?;

        // decide the bounds and witnesses inside one inference scope
        self.with_scope(|state| {
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "materialize.bounds", || {
                state.fill_parameter_bounds()
            })?;
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "materialize.witnesses", || {
                state.walk_nominal_witnesses(&mut worklist)
            })?;
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "materialize.instances", || {
                state.materialize_instances(&mut worklist)
            })?;

            // close the world over the instances and their own instances
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "materialize.closure", || {
                state.materialize_instance_closure(&mut worklist)
            })
        })?;

        // resolve the types and decisions emitted by generated bodies
        self.write_back()
    }

    /// Convert materialized state into one materialized DIR module.
    pub(in crate::sema) fn into_materialized(
        mut self,
    ) -> CompilerResult<(DirMaterialized, Vec<DiagnosticRecord>)> {
        // take the tail segments this pass wrote
        let diagnostics = self.collect_diagnostics()?;
        let roots = self.module.expanded.roots.clone();
        let module_state = self.module;
        let patch = module_state.patch.clone();
        let types = module_state.types_tail.finish();

        Ok((
            DirMaterialized {
                patch,
                bindings: Arc::new(module_state.bindings_tail),
                resolutions: Arc::new(module_state.resolutions),
                types: Arc::new(types),
                generics: Arc::new(module_state.generics_tail),
                decisions: Arc::new(module_state.decisions_tail),
                coercions: Arc::new(module_state.coercions_tail),
                representations: Arc::new(module_state.representations_tail),
                roots,
            },
            diagnostics,
        ))
    }
}

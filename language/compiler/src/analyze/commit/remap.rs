use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{LocalTypeId, TypeTable};

impl Compiler {
    /// Synchronize appended snapshot types into committed tables once.
    pub(in crate::analyze::commit) fn synchronize_snapshot_type_tail_for_commit(
        &self,
        snapshot_types: &TypeTable,
        committed_types: &mut TypeTable,
        committed_type_floor: u32,
        is_synchronized: &mut bool,
    ) -> AnalyzeResult<()> {
        // avoid repeated tail copies inside one apply pass
        if *is_synchronized {
            return Ok(());
        }

        // snapshot clones must contain at least the committed prefix
        if snapshot_types.type_count() < committed_type_floor {
            return Err(AnalyzeError::Internal {
                message: "snapshot type table smaller than committed floor".to_string(),
            });
        }

        // commit tables should still be at the floor before tail sync
        if committed_types.type_count() != committed_type_floor {
            return Err(AnalyzeError::Internal {
                message: "committed type table changed before snapshot tail sync".to_string(),
            });
        }

        // copy appended snapshot types in id order to preserve identity mapping
        let snapshot_type_count = snapshot_types.type_count();
        for type_id in committed_type_floor..snapshot_type_count {
            let type_id = LocalTypeId::new(type_id);
            let source_id = snapshot_types.get_type_source(type_id);
            let ty = snapshot_types.get_type(type_id).clone();

            let inserted_type_id = if snapshot_types.is_imported_type(type_id) {
                committed_types.insert_imported_type_from_any(ty, source_id)
            } else {
                committed_types.insert_type_from_any(ty, source_id)
            };

            // copied tails must keep exact ids because appended types may reference each other
            if inserted_type_id != type_id {
                return Err(AnalyzeError::Internal {
                    message: "snapshot tail sync produced non-identical type ids".to_string(),
                });
            }
        }

        *is_synchronized = true;
        Ok(())
    }

    /// Resolve one committed type id from a snapshot type id.
    pub(in crate::analyze::commit) fn committed_type_id_for_snapshot_type(
        &self,
        snapshot_type_id: LocalTypeId,
        snapshot_types: &TypeTable,
        committed_types: &mut TypeTable,
        committed_type_floor: u32,
        is_synchronized: &mut bool,
    ) -> AnalyzeResult<LocalTypeId> {
        // pre-snapshot ids are shared between committed tables and snapshots
        if snapshot_type_id.0 < committed_type_floor {
            return Ok(snapshot_type_id);
        }

        // copy the appended snapshot tail on first remap request
        self.synchronize_snapshot_type_tail_for_commit(
            snapshot_types,
            committed_types,
            committed_type_floor,
            is_synchronized,
        )?;

        // after synchronization, requested ids must exist in committed tables
        if snapshot_type_id.0 >= committed_types.type_count() {
            return Err(AnalyzeError::Internal {
                message: "missing committed type for snapshot type id".to_string(),
            });
        }

        Ok(snapshot_type_id)
    }
}

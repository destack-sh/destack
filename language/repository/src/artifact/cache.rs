use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tspp_artifact::{
    ArtifactCache, ArtifactCacheError, ArtifactCacheManifest, ArtifactCachePublication,
    ArtifactDependency, ArtifactEntry, ArtifactOutcome, ArtifactPack, ArtifactPackReference,
    ArtifactPackVersion, ArtifactVersion,
};
use tspp_core::{BlobStore, StringPool, stable_hash_value};
use tspp_source::PackageId;

use crate::repository::RevisionState;
use crate::{ArtifactSelection, Repository, RepositoryError, Revision, RevisionPin};

/// Stable artifact pack shards for each package or repository group.
const ARTIFACT_PACK_SHARDS: u64 = 32;

/// One repository artifact generation queued for persistent storage.
#[derive(Debug)]
pub(crate) struct ArtifactCacheWrite {
    /// Repository path owning the persisted manifest.
    repository: PathBuf,
    /// Exact immutable repository revision.
    revision: Revision,
    /// Artifact selection generation observed when this write was queued.
    generation: u64,
    /// Revision state containing the latest selected artifacts.
    state: Arc<RevisionState>,
    /// Shared interned strings referenced by artifacts.
    strings: Arc<StringPool>,
    /// Shared immutable Blob bytes referenced by artifacts.
    blobs: Arc<BlobStore>,
}

impl ArtifactCacheWrite {
    /// Return one write for an unpersisted artifact selection.
    fn pending(revision: &RevisionPin) -> Option<Self> {
        let repository = revision.repository();
        let state = revision.state().clone();
        let selection = state.artifacts.read();
        if !selection.needs_persistence() {
            return None;
        }
        let generation = selection.generation();
        drop(selection);

        Some(Self {
            repository: repository.path().to_path_buf(),
            revision: revision.revision(),
            generation,
            state,
            strings: repository.string_pool().clone(),
            blobs: repository.blob_store().clone(),
        })
    }

    /// Return the repository path receiving this write.
    pub(crate) fn repository(&self) -> &Path {
        &self.repository
    }

    /// Return the immutable repository revision queued by this write.
    pub(crate) fn revision(&self) -> Revision {
        self.revision
    }

    /// Return the artifact selection generation queued by this write.
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    /// Persist successful artifacts selected by this exact revision.
    pub(crate) fn write(
        &self,
        cache: &ArtifactCache,
        worker_count: usize,
    ) -> Result<u64, ArtifactCacheError> {
        let selection = self.state.artifacts.read();
        let generation = selection.generation();
        if !selection.needs_persistence() {
            return Ok(generation);
        }
        let entries = selection.current_entries();
        drop(selection);
        let entries = entries
            .into_iter()
            .filter(|entry| matches!(entry.outcome(), ArtifactOutcome::Ok))
            .collect::<Vec<_>>();

        // assign selected artifacts to stable pack shards
        let mut entries_by_pack =
            BTreeMap::<(Option<PackageId>, usize), Vec<&ArtifactEntry>>::new();
        for entry in &entries {
            let package = entry.version.package_id();
            let shard = Self::pack_shard(entry.version);
            entries_by_pack
                .entry((package, shard))
                .or_default()
                .push(entry.as_ref());
        }
        let entries_by_pack = entries_by_pack.into_iter().collect::<Vec<_>>();

        // index the preceding artifact pack references
        let publication = cache.publish()?;
        let manifest = publication.load_manifest::<Revision>(&self.repository)?;
        let cached = manifest
            .iter()
            .flat_map(|manifest| manifest.packs.iter().copied())
            .map(|selected| ((selected.package, selected.version), selected))
            .collect::<BTreeMap<_, _>>();

        // retain unchanged packs and collect changed pack ordinals
        let mut selected = vec![None; entries_by_pack.len()];
        let mut changed = Vec::new();
        for (index, ((package, _), entries)) in entries_by_pack.iter().enumerate() {
            let version = ArtifactPackVersion::new(entries.iter().map(|entry| entry.version));
            let selected_pack = cached.get(&(*package, version)).copied();
            match selected_pack {
                Some(pack) => selected[index] = Some(pack),
                None => changed.push((index, *package, version)),
            }
        }

        // encode independent changed packs across bounded workers
        let worker_count = worker_count.max(1).min(changed.len().max(1));
        if worker_count == 1 {
            for (index, package, version) in changed {
                let entries = &entries_by_pack[index].1;
                selected[index] = Some(self.write_pack(&publication, package, version, entries)?);
            }
        } else {
            std::thread::scope(|scope| -> Result<(), ArtifactCacheError> {
                let entries_by_pack = &entries_by_pack;
                let changed = &changed;
                let publication = &publication;
                let mut handles = Vec::with_capacity(worker_count);
                for worker in 0..worker_count {
                    let changes = changed.iter().skip(worker).step_by(worker_count);
                    handles.push(scope.spawn(move || {
                        let mut completed = Vec::new();
                        for &(index, package, version) in changes {
                            let entries = &entries_by_pack[index].1;
                            let pack = self.write_pack(publication, package, version, entries)?;
                            completed.push((index, pack));
                        }

                        Ok::<_, ArtifactCacheError>(completed)
                    }));
                }

                for handle in handles {
                    let completed = handle.join().map_err(|_| {
                        ArtifactCacheError::Internal("artifact cache encoder panicked".to_string())
                    })??;
                    for (index, pack) in completed {
                        selected[index] = Some(pack);
                    }
                }

                Ok(())
            })?;
        }

        // require every retained or rewritten pack in stable order
        let mut selected = selected
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                ArtifactCacheError::Internal("artifact cache pack was not written".to_string())
            })?;
        selected.sort_unstable_by_key(|pack| (pack.package, pack.version));

        // skip an equal manifest already selected for this revision
        if manifest.as_ref().is_some_and(|manifest| {
            manifest.revision == self.revision && manifest.packs.as_ref() == selected
        }) {
            return Ok(generation);
        }

        // publish the manifest only after every selected pack exists
        let manifest =
            ArtifactCacheManifest::new(cache.build_id(), &self.repository, self.revision, selected);
        publication.write_manifest(&self.repository, &manifest)?;

        Ok(generation)
    }

    /// Mark one successfully written selection generation as persistent.
    pub(crate) fn mark_persisted(&self, generation: u64) {
        self.state.artifacts.write().mark_persisted(generation);
    }

    /// Return the stable pack shard for one artifact version.
    fn pack_shard(version: ArtifactVersion) -> usize {
        let hash = match version.module_id() {
            Some(module) => stable_hash_value(&module),
            None => stable_hash_value(&version.key),
        };

        (hash % ARTIFACT_PACK_SHARDS) as usize
    }

    /// Write one self-contained pack from successful artifact entries.
    fn write_pack(
        &self,
        cache: &ArtifactCachePublication<'_>,
        package: Option<PackageId>,
        version: ArtifactPackVersion,
        entries: &[&ArtifactEntry],
    ) -> Result<ArtifactPackReference, ArtifactCacheError> {
        // build one self-contained artifact pack
        let bytes = ArtifactPack::encode(
            cache.build_id(),
            package,
            entries,
            &self.strings,
            &self.blobs,
        )?;
        let checksum = cache.write_pack(version, bytes)?;

        Ok(ArtifactPackReference::new(package, version, checksum))
    }
}

impl Repository {
    /// Restore cached artifacts into one imported physical revision.
    pub fn restore_artifacts(
        &self,
        revision: Revision,
        worker_count: usize,
    ) -> Result<usize, RepositoryError> {
        let Some(cache) = self.artifact_cache() else {
            return Ok(0);
        };
        let loaded = cache.load::<Revision>(self.path(), worker_count)?;
        let Some((manifest, packs)) = loaded else {
            return Ok(0);
        };
        let saved_revision = manifest.revision;

        // install every exact interned string under one pool write
        let strings = packs.iter().flat_map(ArtifactPack::strings);
        self.string_pool().ensure_all(strings);

        // install generated Blobs before restoring their artifact records
        for pack in &packs {
            for (blob, bytes) in pack.blobs() {
                // reuse bytes already owned by the repository host
                if self.contains_blob(blob)? {
                    continue;
                }

                // retain each new Blob once in its final aligned allocation
                let retained = self.retain_blob(bytes)?;
                if retained != blob {
                    return Err(RepositoryError::InvalidArtifact {
                        message: format!("cached Blob {blob} restored as {retained}"),
                    });
                }
            }
        }

        // restore independent pack metadata across bounded workers
        let entries = self.restore_artifact_packs(packs, worker_count)?;

        // restore every artifact selected by the saved packs
        let selection = ArtifactSelection::from_entries(entries)?;

        // invalidate changed source observations across physical restarts
        let selection = if saved_revision == revision {
            selection
        } else {
            let mut sources = selection
                .dependencies()
                .filter_map(|dependency| match dependency {
                    ArtifactDependency::Source(source) => Some(*source),
                    ArtifactDependency::Artifact(_) | ArtifactDependency::Projection(_) => None,
                })
                .collect::<Vec<_>>();
            sources.sort_unstable();
            sources.dedup();
            let mut invalidated = Vec::new();
            for source in sources {
                if !self.source_dependency_holds(revision, source)? {
                    invalidated.push(source);
                }
            }

            selection.fork(&invalidated, self.artifact_table())
        };

        let state = self.revision(revision)?;
        let restored = selection.len();
        state.artifacts.write().adopt(&selection)?;

        Ok(restored)
    }

    /// Restore artifact metadata from independent immutable packs.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    fn restore_artifact_packs(
        &self,
        packs: Vec<ArtifactPack>,
        worker_count: usize,
    ) -> Result<Vec<Arc<ArtifactEntry>>, RepositoryError> {
        let entry_count = packs.iter().map(ArtifactPack::entry_count).sum();
        let worker_count = worker_count.max(1).min(packs.len().max(1));
        if worker_count == 1 {
            let mut entries = Vec::with_capacity(entry_count);
            for pack in packs {
                entries.extend(
                    pack.restore(self.artifact_table())
                        .map_err(RepositoryError::from)?,
                );
            }

            return Ok(entries);
        }

        // distribute complete packs without copying their retained bytes
        let mut groups = (0..worker_count).map(|_| Vec::new()).collect::<Vec<_>>();
        for (index, pack) in packs.into_iter().enumerate() {
            groups[index % worker_count].push(pack);
        }

        // decode each group into its final artifact entries
        std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(worker_count);
            for packs in groups {
                handles.push(scope.spawn(move || {
                    let mut entries = Vec::new();
                    for pack in packs {
                        entries.extend(
                            pack.restore(self.artifact_table())
                                .map_err(RepositoryError::from)?,
                        );
                    }

                    Ok::<_, RepositoryError>(entries)
                }));
            }

            let mut entries = Vec::with_capacity(entry_count);
            for handle in handles {
                let restored = handle.join().map_err(|_| {
                    RepositoryError::from(ArtifactCacheError::Internal(
                        "artifact cache restorer panicked".to_string(),
                    ))
                })??;
                entries.extend(restored);
            }

            Ok(entries)
        })
    }

    /// Restore artifact metadata on bare WebAssembly.
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn restore_artifact_packs(
        &self,
        packs: Vec<ArtifactPack>,
        _worker_count: usize,
    ) -> Result<Vec<Arc<ArtifactEntry>>, RepositoryError> {
        let entry_count = packs.iter().map(ArtifactPack::entry_count).sum();
        let mut entries = Vec::with_capacity(entry_count);
        for pack in packs {
            entries.extend(
                pack.restore(self.artifact_table())
                    .map_err(RepositoryError::from)?,
            );
        }

        Ok(entries)
    }
}

impl RevisionPin {
    /// Queue successful artifacts selected by this exact revision for persistence.
    pub fn persist_artifacts(&self) -> bool {
        let Some(writer) = self.repository().host.artifact_cache_writer() else {
            return false;
        };
        let Some(write) = ArtifactCacheWrite::pending(self) else {
            return false;
        };

        writer.enqueue(write)
    }
}

impl Repository {
    /// Wait for every queued artifact cache write on this host.
    pub fn flush_artifact_cache(&self) -> Result<(), RepositoryError> {
        self.host
            .flush_artifact_cache()
            .map_err(RepositoryError::from)
    }
}

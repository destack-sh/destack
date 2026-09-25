use std::ops::Range;
use std::sync::{Arc, OnceLock};

use tspp_core::{Blob, BlobStore, StringId, StringPool};
use tspp_source::PackageId;

use super::codec::{ArtifactPackDecoder, ArtifactPackEncoder};
use crate::{
    ArtifactCacheError, ArtifactDependency, ArtifactEntry, ArtifactError, ArtifactPayload,
    ArtifactResult, ArtifactTable, ArtifactVersion, BuildId, DiagnosticRecord,
};

/// One artifact record in an immutable pack.
#[derive(Debug)]
struct ArtifactPackEntry {
    /// The exact artifact version.
    version: ArtifactVersion,
    /// The encoded dependency and diagnostic byte range.
    metadata: Range<usize>,
}

/// One encoded payload and its first decoded value.
#[derive(Debug)]
struct ArtifactPackPayload {
    /// Encoded payload byte range.
    bytes: Range<usize>,
    /// Interned strings referenced by the encoded payload.
    strings: Range<usize>,
    /// Immutable Blobs referenced by the encoded payload.
    blobs: Range<usize>,
    /// Decoded payload or its exact codec failure.
    decoded: OnceLock<Result<ArtifactPayload, ArtifactError>>,
}

impl ArtifactPackPayload {
    /// Retain one encoded payload range.
    fn new(bytes: Range<usize>, strings: Range<usize>, blobs: Range<usize>) -> Self {
        Self {
            bytes,
            strings,
            blobs,
            decoded: OnceLock::new(),
        }
    }
}

/// Immutable bytes and lazy payloads shared by one pack.
#[derive(Debug)]
struct ArtifactPackStorage {
    /// Retained immutable pack bytes.
    bytes: Vec<u8>,
    /// Payloads in artifact directory order.
    payloads: Box<[ArtifactPackPayload]>,
    /// Interned strings referenced by individual payloads.
    strings: Box<[StringId]>,
    /// Immutable Blobs referenced by individual payloads.
    blobs: Box<[Blob]>,
}

/// One encoded artifact payload retained by an artifact entry.
#[derive(Debug)]
pub(crate) struct ArtifactPackRecord {
    /// Shared pack storage.
    storage: Arc<ArtifactPackStorage>,
    /// Payload ordinal in pack directory order.
    ordinal: u32,
}

impl ArtifactPackRecord {
    /// Retain one encoded payload by its pack ordinal.
    fn new(storage: Arc<ArtifactPackStorage>, ordinal: u32) -> Self {
        Self { storage, ordinal }
    }

    /// Decode the payload once and return its owned artifact value.
    pub(crate) fn payload(&self) -> Result<ArtifactPayload, crate::ArtifactError> {
        let payload = &self.storage.payloads[self.ordinal as usize];
        let decoded = payload.decoded.get_or_init(|| {
            let bytes = &self.storage.bytes[payload.bytes.clone()];

            tspp_serde::from_slice_fixed(bytes).map_err(Into::into)
        });

        decoded.clone()
    }

    /// Return the exact encoded payload byte length.
    fn encoded_len(&self) -> usize {
        let payload = &self.storage.payloads[self.ordinal as usize];

        payload.bytes.len()
    }

    /// Append the exact encoded payload and its referenced resources.
    fn append(
        &self,
        encoder: &mut ArtifactPackEncoder,
        strings: &mut Vec<StringId>,
        blobs: &mut Vec<Blob>,
    ) -> Range<usize> {
        let payload = &self.storage.payloads[self.ordinal as usize];
        strings.extend_from_slice(&self.storage.strings[payload.strings.clone()]);
        blobs.extend_from_slice(&self.storage.blobs[payload.blobs.clone()]);
        let start = encoder.len();
        encoder.write_bytes(&self.storage.bytes[payload.bytes.clone()]);

        start..encoder.len()
    }
}

/// One immutable artifact pack encoded as independent records.
#[derive(Debug)]
pub struct ArtifactPack {
    /// The package shared by every stored artifact, when package-owned.
    package: Option<PackageId>,
    /// Interned strings stored in this pack.
    strings: Box<[(StringId, Range<usize>)]>,
    /// Immutable Blob identities and byte ranges stored in this pack.
    blobs: Box<[(Blob, Range<usize>)]>,
    /// Artifact versions and encoded record ranges.
    entries: Box<[ArtifactPackEntry]>,
    /// End of the header and artifact record bytes.
    records_end: usize,
    /// Immutable artifact record bytes and lazy payloads.
    storage: Arc<ArtifactPackStorage>,
}

impl ArtifactPack {
    /// Artifact pack file marker.
    const MAGIC: &'static [u8; 8] = b"DSTACKAP";
    /// The current persistent artifact pack format.
    pub const FORMAT: u32 = 6;

    /// Encode one immutable artifact pack.
    pub fn encode(
        build_id: BuildId,
        package: Option<PackageId>,
        entries: &[&ArtifactEntry],
        strings: &StringPool,
        blobs: &BlobStore,
    ) -> Result<Vec<u8>, ArtifactCacheError> {
        let mut entries = entries.to_vec();
        entries.sort_unstable_by_key(|entry| entry.version);
        Self::validate_entries(package, &entries)?;

        // write the fixed header and reserve its section offsets
        let record_bytes = entries
            .iter()
            .filter_map(|entry| match &entry.result {
                ArtifactResult::Cached(record) => Some(record.encoded_len()),
                ArtifactResult::Ok(_) | ArtifactResult::Failed(_) => None,
            })
            .sum();
        let mut writer = ArtifactPackEncoder::with_capacity(record_bytes);
        writer.write_bytes(Self::MAGIC);
        writer.write_u32(Self::FORMAT);
        writer.write_bytes(build_id.as_bytes());
        writer.write_package(package);
        let resources_offset = writer.reserve_u64();
        let directory_offset = writer.reserve_u64();

        // append each record directly and build its compact directory
        let mut string_ids = Vec::new();
        let mut blob_ids = Vec::new();
        let mut payload_strings = Vec::new();
        let mut payload_blobs = Vec::new();
        let mut directory = ArtifactPackEncoder::with_capacity(entries.len() * 64);
        directory.write_len(entries.len())?;
        for entry in &entries {
            let version_len = directory.reserve_u32();
            let version = directory.write_value(&entry.version, &mut string_ids)?;
            directory.patch_len(version_len, version.len())?;

            let diagnostics = entry.diagnostics();
            let metadata = (entry.dependencies.as_ref(), diagnostics.as_ref());
            let metadata = writer.write_value(&metadata, &mut string_ids)?;

            // append owned payloads or copy retained cache records directly
            payload_strings.clear();
            payload_blobs.clear();
            let payload_range = match &entry.result {
                ArtifactResult::Ok(payload) => {
                    let range = writer.write_value(payload, &mut payload_strings)?;
                    payload_blobs.extend(payload.blobs());

                    range
                }
                ArtifactResult::Cached(record) => {
                    record.append(&mut writer, &mut payload_strings, &mut payload_blobs)
                }
                ArtifactResult::Failed(_failure) => {
                    return Err(ArtifactCacheError::Invalid(format!(
                        "artifact {:?} has no successful payload",
                        entry.version
                    )));
                }
            };

            // canonicalize the resources referenced by this payload
            payload_strings.sort_unstable();
            payload_strings.dedup();
            payload_blobs.sort_unstable();
            payload_blobs.dedup();
            if !payload_blobs
                .windows(2)
                .all(|items| items[0].id < items[1].id)
            {
                return Err(ArtifactCacheError::Invalid(
                    "one Blob identity has multiple byte lengths".to_string(),
                ));
            }

            directory.write_len(metadata.len())?;
            directory.write_len(payload_range.len())?;
            directory.write_string_ids(&payload_strings)?;
            directory.write_blobs(&payload_blobs)?;

            string_ids.extend_from_slice(&payload_strings);
            blob_ids.extend_from_slice(&payload_blobs);
            blob_ids.extend(
                diagnostics
                    .iter()
                    .flat_map(|record| record.diagnostic.blobs()),
            );
        }

        // append each referenced interned string once
        string_ids.sort_unstable();
        string_ids.dedup();
        let offset = writer.len() as u64;
        writer.patch_u64(resources_offset, offset);
        writer.write_len(string_ids.len())?;
        for id in string_ids {
            writer.write_string(id, strings)?;
        }

        // append each referenced immutable Blob once
        blob_ids.sort_unstable();
        blob_ids.dedup();
        if !blob_ids.windows(2).all(|items| items[0].id < items[1].id) {
            return Err(ArtifactCacheError::Invalid(
                "one Blob identity has multiple byte lengths".to_string(),
            ));
        }
        writer.write_len(blob_ids.len())?;
        for blob in blob_ids {
            writer.write_blob(blob, blobs)?;
        }

        // append the directory and publish both section offsets
        let offset = writer.len() as u64;
        writer.patch_u64(directory_offset, offset);
        writer.write_bytes(&directory.finish());

        Ok(writer.finish())
    }

    /// Decode and validate one complete artifact pack.
    pub fn decode(bytes: Vec<u8>, build_id: BuildId) -> Result<Self, ArtifactCacheError> {
        let mut reader = ArtifactPackDecoder::new(&bytes);
        reader.read_magic(Self::MAGIC)?;
        reader.read_format(Self::FORMAT)?;
        reader.read_build(build_id)?;
        let package = reader.read_package()?;
        let resources_offset = reader.read_offset()?;
        let directory_offset = reader.read_offset()?;
        let records_offset = reader.position();
        if !(records_offset <= resources_offset && resources_offset <= directory_offset) {
            return Err(ArtifactCacheError::Invalid(
                "artifact pack section offsets are not ordered".to_string(),
            ));
        }
        reader.seek(resources_offset)?;
        let strings = reader.read_strings()?;
        let blobs = reader.read_blobs()?;
        if reader.position() != directory_offset {
            return Err(ArtifactCacheError::Invalid(
                "artifact pack resources do not end at its directory".to_string(),
            ));
        }

        // decode the entry directory
        let entry_count = reader.read_len()?;
        let mut directory = Vec::with_capacity(entry_count);
        let mut payload_strings = Vec::new();
        let mut payload_blobs = Vec::new();
        for _ in 0..entry_count {
            let version = reader.read_byte_slice()?;
            let version: ArtifactVersion = tspp_serde::from_slice_fixed(version)?;
            let metadata_len = reader.read_len()?;
            let payload_len = reader.read_len()?;
            let payload_string_range = reader.read_string_ids(&mut payload_strings)?;
            let payload_blob_range = reader.read_blob_ids(&mut payload_blobs)?;
            directory.push((
                version,
                metadata_len,
                payload_len,
                payload_string_range,
                payload_blob_range,
            ));
        }

        // retain each independently encoded record range
        let mut entries = Vec::with_capacity(entry_count);
        let mut payloads = Vec::with_capacity(entry_count);
        let mut record_offset = records_offset;
        for (version, metadata_len, payload_len, strings, blobs) in directory {
            if version.package_id() != package {
                return Err(ArtifactCacheError::Invalid(format!(
                    "artifact {version:?} does not belong in package {package:?}"
                )));
            }
            let metadata = Self::record_range(&mut record_offset, metadata_len, resources_offset)?;
            let payload = Self::record_range(&mut record_offset, payload_len, resources_offset)?;
            entries.push(ArtifactPackEntry { version, metadata });
            payloads.push(ArtifactPackPayload::new(payload, strings, blobs));
        }
        if record_offset != resources_offset {
            return Err(ArtifactCacheError::Invalid(
                "artifact pack records do not end at its resources".to_string(),
            ));
        }
        reader.finish()?;

        // require canonical identity order
        if !strings.windows(2).all(|items| items[0].0 < items[1].0)
            || !blobs.windows(2).all(|items| items[0].0.id < items[1].0.id)
            || !entries
                .windows(2)
                .all(|items| items[0].version < items[1].version)
        {
            return Err(ArtifactCacheError::Invalid(
                "pack identities are not strictly ordered".to_string(),
            ));
        }

        // validate every payload resource reference
        let contains_string = |id| strings.binary_search_by_key(&id, |item| item.0).is_ok();
        let contains_blob = |blob: Blob| {
            blobs
                .binary_search_by_key(&blob.id, |item| item.0.id)
                .is_ok_and(|index| blobs[index].0 == blob)
        };
        if payload_strings
            .iter()
            .copied()
            .any(|id| !contains_string(id))
            || payload_blobs
                .iter()
                .copied()
                .any(|blob| !contains_blob(blob))
        {
            return Err(ArtifactCacheError::Invalid(
                "artifact payload references a missing pack resource".to_string(),
            ));
        }

        // retain the encoded records for lazy payload reads
        let storage = Arc::new(ArtifactPackStorage {
            bytes,
            payloads: payloads.into_boxed_slice(),
            strings: payload_strings.into_boxed_slice(),
            blobs: payload_blobs.into_boxed_slice(),
        });

        Ok(Self {
            package,
            strings: strings.into_boxed_slice(),
            blobs: blobs.into_boxed_slice(),
            entries: entries.into_boxed_slice(),
            records_end: resources_offset,
            storage,
        })
    }

    /// Return the common package owner, when these artifacts are package-owned.
    pub fn package(&self) -> Option<PackageId> {
        self.package
    }

    /// Return the number of artifact entries in this pack.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Iterate the exact artifact versions stored in this pack.
    pub fn versions(&self) -> impl Iterator<Item = ArtifactVersion> + '_ {
        self.entries.iter().map(|entry| entry.version)
    }

    /// Iterate interned strings stored in this self-contained pack.
    pub fn strings(&self) -> impl Iterator<Item = (StringId, &str)> + '_ {
        self.strings.iter().map(|(id, text)| {
            let bytes = &self.storage.bytes[text.clone()];

            // safety: pack construction and decoding validate every string range
            let text = unsafe { std::str::from_utf8_unchecked(bytes) };

            (*id, text)
        })
    }

    /// Iterate immutable Blobs stored in this self-contained pack.
    pub fn blobs(&self) -> impl Iterator<Item = (Blob, &[u8])> + '_ {
        self.blobs
            .iter()
            .map(|(blob, bytes)| (*blob, &self.storage.bytes[bytes.clone()] as &[u8]))
    }

    /// Restore every artifact entry after its resources have been installed.
    pub fn restore(
        mut self,
        artifacts: &ArtifactTable,
    ) -> Result<Box<[Arc<ArtifactEntry>]>, ArtifactCacheError> {
        self.retain_records()?;

        (0..self.entries.len())
            .map(|ordinal| self.restore_entry(artifacts, ordinal))
            .collect()
    }

    /// Restore one exact artifact entry.
    fn restore_entry(
        &self,
        artifacts: &ArtifactTable,
        ordinal: usize,
    ) -> Result<Arc<ArtifactEntry>, ArtifactCacheError> {
        let entry = &self.entries[ordinal];
        let bytes = &self.storage.bytes[entry.metadata.clone()];
        let (dependencies, diagnostics): (Arc<[ArtifactDependency]>, Arc<[DiagnosticRecord]>) =
            tspp_serde::from_slice_fixed(bytes)?;
        let record = ArtifactPackRecord::new(self.storage.clone(), ordinal as u32);
        let entry = artifacts.restore(entry.version, dependencies, record, diagnostics);

        Ok(entry)
    }

    /// Retain only bytes referenced by restored artifact records.
    fn retain_records(&mut self) -> Result<(), ArtifactCacheError> {
        let storage = Arc::get_mut(&mut self.storage).ok_or_else(|| {
            ArtifactCacheError::Internal(
                "artifact pack storage was shared before restoration".to_string(),
            )
        })?;

        // copy record bytes into one tight retained allocation
        storage.bytes = storage.bytes[..self.records_end].to_vec();
        self.strings = Box::new([]);
        self.blobs = Box::new([]);

        Ok(())
    }

    /// Advance one encoded record range within the record section.
    fn record_range(
        offset: &mut usize,
        len: usize,
        end: usize,
    ) -> Result<Range<usize>, ArtifactCacheError> {
        let start = *offset;
        let next = start
            .checked_add(len)
            .filter(|next| *next <= end)
            .ok_or_else(|| {
                ArtifactCacheError::Invalid("artifact pack record range is invalid".to_string())
            })?;
        *offset = next;

        Ok(start..next)
    }

    /// Require one ordered successful artifact set for the selected package.
    fn validate_entries(
        package: Option<PackageId>,
        entries: &[&ArtifactEntry],
    ) -> Result<(), ArtifactCacheError> {
        if !entries
            .windows(2)
            .all(|items| items[0].version < items[1].version)
        {
            return Err(ArtifactCacheError::Invalid(
                "artifact pack versions are not unique".to_string(),
            ));
        }
        if let Some(entry) = entries
            .iter()
            .find(|entry| entry.version.package_id() != package)
        {
            return Err(ArtifactCacheError::Invalid(format!(
                "artifact {:?} does not belong in package {package:?}",
                entry.version
            )));
        }

        Ok(())
    }
}

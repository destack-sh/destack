use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use destack_core::fnv1a_64;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::core::HostRequestContext;
use crate::platform::PlatformError;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::os::DocumentAccess;
use crate::platform::os::abi_generated::{DocumentAccessGrantValue, DocumentDescriptorValue};

use super::storage::{ensure_document_state_directory, source_path_from_document_descriptor};

/// Directory name for persisted document-access state.
const DOCUMENT_ACCESS_DIRECTORY: &str = "document-access";

/// Persisted document-access grant record.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredDocumentAccessGrant {
    /// Stable persisted grant identifier.
    id: String,
    /// Granted document descriptor.
    document: DocumentDescriptorValue,
    /// Granted access mode.
    access: DocumentAccess,
    /// Grant persistence timestamp in UTC nanoseconds.
    persisted_unix_ns: u64,
}

/// Persist document-access grants for one document set.
pub(crate) fn persist_document_access_grants(
    context: &HostRequestContext,
    documents: Vec<DocumentDescriptorValue>,
    access: DocumentAccess,
) -> RuntimeResult<Vec<DocumentAccessGrantValue>> {
    let mut grants = Vec::with_capacity(documents.len());

    // persist one grant per document
    for document in documents {
        validate_persistable_document(&document)?;

        let grant_id = document_access_grant_id(&document, access);
        let persisted_unix_ns = read_document_access_grant(context, &grant_id)?
            .map(|existing| existing.persisted_unix_ns)
            .unwrap_or_else(current_unix_timestamp_ns);
        let grant = StoredDocumentAccessGrant {
            id: grant_id.clone(),
            document,
            access,
            persisted_unix_ns,
        };

        write_document_access_grant(context, &grant)?;
        grants.push(document_access_grant_value_from_record(grant));
    }

    Ok(grants)
}

/// List persisted document-access grants.
pub(crate) fn list_document_access_grants(
    context: &HostRequestContext,
) -> RuntimeResult<Vec<DocumentAccessGrantValue>> {
    let records = read_document_access_grants(context)?;
    let grants = records
        .into_iter()
        .map(document_access_grant_value_from_record)
        .collect();

    Ok(grants)
}

/// Revoke persisted document-access grants by identifier.
pub(crate) fn revoke_document_access_grants(
    context: &HostRequestContext,
    ids: &[String],
) -> RuntimeResult<u32> {
    let mut revoked = 0u32;

    // remove one persisted grant record per requested identifier
    for id in ids {
        let path = document_access_grant_path(context, id)?;

        if !path.exists() {
            continue;
        }

        fs::remove_file(&path).map_err(|error| {
            document_access_error(
                "failed to revoke document access grant",
                &path,
                Some(PlatformErrorCode::IoPermissionDenied),
                error,
            )
        })?;

        revoked += 1;
    }

    Ok(revoked)
}

/// Return one grant value from one persisted record.
fn document_access_grant_value_from_record(
    record: StoredDocumentAccessGrant,
) -> DocumentAccessGrantValue {
    DocumentAccessGrantValue {
        id: record.id,
        document: record.document,
        access: record.access,
        persisted_unix_ns: record.persisted_unix_ns,
    }
}

/// Return the stable identifier for one persisted document-access grant.
fn document_access_grant_id(document: &DocumentDescriptorValue, access: DocumentAccess) -> String {
    let identity = format!("{}:{}", document.uri, access as u8);
    let hash = fnv1a_64(identity.as_bytes());

    format!("document-access-{hash:016x}")
}

/// Reject one document that this backend cannot persist yet.
fn validate_persistable_document(document: &DocumentDescriptorValue) -> RuntimeResult<()> {
    source_path_from_document_descriptor(document)?;

    Ok(())
}

/// Read every persisted document-access grant.
fn read_document_access_grants(
    context: &HostRequestContext,
) -> RuntimeResult<Vec<StoredDocumentAccessGrant>> {
    let directory = document_access_directory(context)?;
    let entries = fs::read_dir(&directory).map_err(|error| {
        document_access_error(
            "failed to read document access directory",
            &directory,
            Some(PlatformErrorCode::IoInvalidData),
            error,
        )
    })?;
    let mut grants = Vec::new();

    // decode one persisted grant per file
    for entry in entries {
        let entry = entry.map_err(|error| {
            document_access_error(
                "failed to decode document access directory entry",
                &directory,
                Some(PlatformErrorCode::IoInvalidData),
                error,
            )
        })?;
        let path = entry.path();

        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }

        let Some(grant) = read_document_access_grant_path(&path)? else {
            continue;
        };
        grants.push(grant);
    }

    Ok(grants)
}

/// Read one persisted document-access grant by identifier.
fn read_document_access_grant(
    context: &HostRequestContext,
    id: &str,
) -> RuntimeResult<Option<StoredDocumentAccessGrant>> {
    let path = document_access_grant_path(context, id)?;

    read_document_access_grant_path(&path)
}

/// Read one persisted document-access grant from one concrete path.
fn read_document_access_grant_path(
    path: &PathBuf,
) -> RuntimeResult<Option<StoredDocumentAccessGrant>> {
    // missing state should decode as one absent grant
    if !path.exists() {
        return Ok(None);
    }

    let payload = fs::read_to_string(path).map_err(|error| {
        document_access_error(
            "failed to read document access grant",
            &path,
            Some(PlatformErrorCode::IoInvalidData),
            error,
        )
    })?;
    let grant = serde_json::from_str(&payload).map_err(|error| {
        document_access_error(
            "failed to decode document access grant",
            path,
            Some(PlatformErrorCode::IoInvalidData),
            error,
        )
    })?;

    Ok(Some(grant))
}

/// Write one persisted document-access grant.
fn write_document_access_grant(
    context: &HostRequestContext,
    grant: &StoredDocumentAccessGrant,
) -> RuntimeResult<()> {
    let path = document_access_grant_path(context, &grant.id)?;
    let payload = serde_json::to_vec_pretty(grant).map_err(|error| {
        document_access_error(
            "failed to encode document access grant",
            &path,
            Some(PlatformErrorCode::IoInvalidData),
            error,
        )
    })?;

    fs::write(&path, payload).map_err(|error| {
        document_access_error(
            "failed to write document access grant",
            &path,
            Some(PlatformErrorCode::IoPermissionDenied),
            error,
        )
    })
}

/// Return the persisted document-access directory.
fn document_access_directory(context: &HostRequestContext) -> RuntimeResult<PathBuf> {
    ensure_document_state_directory(context, DOCUMENT_ACCESS_DIRECTORY)
}

/// Return one persisted document-access grant path.
fn document_access_grant_path(context: &HostRequestContext, id: &str) -> RuntimeResult<PathBuf> {
    let directory = document_access_directory(context)?;

    Ok(directory.join(format!("{id}.json")))
}

/// Return the current UTC unix timestamp in nanoseconds.
fn current_unix_timestamp_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|value| value.as_nanos().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

/// Return one persisted document-access error.
fn document_access_error(
    message: &str,
    path: &PathBuf,
    code: Option<PlatformErrorCode>,
    error: impl std::fmt::Display,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::generic(
        code,
        format!(
            "destack.os.document.access: {message} {}: {error}",
            path.display()
        ),
    ))
    .boxed()
}

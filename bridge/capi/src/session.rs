use std::ffi::c_char;
use std::slice;

use destack as rust;

use crate::core::{DestackError, DestackStatus, read_string, return_status, write_out};
use crate::generated::{
    DestackArtifactKey, DestackArtifactRecord, DestackArtifactSidecarArray, DestackArtifactVersion,
    DestackChangeArray, DestackCommit, DestackDiagnosticArray, DestackDirChecked, DestackDirParsed,
    DestackDirResolved, DestackModule, DestackProfileId, DestackRevision, DestackSessionFileArray,
};
use crate::source::{DestackEdits, DestackSource};

/// C ABI session handle.
#[repr(C)]
#[derive(Debug)]
pub struct DestackSession {
    /// Rust session.
    pub(crate) session: rust::Session,
}

/// Open one session from one source input.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_open(
    source: *const DestackSource,
    out: *mut *mut DestackSession,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let source = unsafe { source.as_ref() }.ok_or("source is null")?;
        let session = bridge(rust::Session::open(source.value.clone()))?;

        write_out(
            out,
            Box::into_raw(Box::new(DestackSession { session })),
            "session output is null",
        )
    })
}

/// Destroy one session handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_destroy(session: *mut DestackSession) {
    if session.is_null() {
        return;
    }

    drop(unsafe { Box::from_raw(session) });
}

/// Return the current session revision.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_revision(
    session: *const DestackSession,
    out: *mut DestackRevision,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let revision = bridge(session.session.revision())?;
        let revision = DestackRevision::from_bridge(revision)?;

        write_out(out, revision, "revision output is null")
    })
}

/// Return editable repository file paths.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_files(
    session: *const DestackSession,
    out: *mut DestackSessionFileArray,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let files = bridge(session.session.files())?;
        let files = DestackSessionFileArray::from_bridge(files)?;

        write_out(out, files, "session file array output is null")
    })
}

/// Edit files through the current session revision.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_edit(
    session: *mut DestackSession,
    edits: *const DestackEdits,
    out: *mut DestackCommit,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_mut() }.ok_or("session is null")?;
        let edits = unsafe { edits.as_ref() }.ok_or("edit list is null")?;
        let result = bridge(session.session.edit(edits.value.clone()))?;
        let result = DestackCommit::from_bridge(result)?;

        write_out(out, result, "commit output is null")
    })
}

/// Edit files when the current revision still matches.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_edit_at(
    session: *mut DestackSession,
    revision: *const DestackRevision,
    edits: *const DestackEdits,
    out: *mut DestackCommit,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_mut() }.ok_or("session is null")?;
        let revision = unsafe { revision.as_ref() }.ok_or("revision is null")?;
        let edits = unsafe { edits.as_ref() }.ok_or("edit list is null")?;
        let revision = revision.to_bridge()?;
        let result = bridge(session.session.edit_at(revision, edits.value.clone()))?;
        let result = DestackCommit::from_bridge(result)?;

        write_out(out, result, "commit output is null")
    })
}

/// Reload tracked files from this session backing source.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_reload(
    session: *mut DestackSession,
    out: *mut DestackChangeArray,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_mut() }.ok_or("session is null")?;
        let changes = bridge(session.session.reload())?;
        let changes = DestackChangeArray::from_bridge(changes)?;

        write_out(out, changes, "change array output is null")
    })
}

/// Load one module path into the current session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_load_module(
    session: *mut DestackSession,
    path: *const c_char,
    out: *mut DestackModule,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_mut() }.ok_or("session is null")?;
        let path = read_string(path)?;
        let module = bridge(session.session.load_module(path))?;
        let module = DestackModule::from_bridge(module)?;

        write_out(out, module, "module output is null")
    })
}

/// Provide root artifacts for one immutable revision.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_provide(
    session: *const DestackSession,
    revision: DestackRevision,
    keys: *const *const DestackArtifactKey,
    len: usize,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let revision = revision.to_bridge()?;
        let keys = artifact_keys_to_bridge(keys, len)?;

        bridge(session.session.provide(revision, keys))
    })
}

/// Require one root artifact for one immutable revision.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_require(
    session: *const DestackSession,
    revision: DestackRevision,
    key: *const DestackArtifactKey,
    out: *mut DestackArtifactVersion,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let revision = revision.to_bridge()?;
        let key = unsafe { key.as_ref() }.ok_or("artifact key is null")?;
        let version = bridge(session.session.require(revision, key.value.clone()))?;
        let version = DestackArtifactVersion::from_bridge(version)?;

        write_out(out, version, "artifact version output is null")
    })
}

/// Return one raw artifact record for one immutable revision.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_artifact_record(
    session: *const DestackSession,
    revision: DestackRevision,
    key: *const DestackArtifactKey,
    out: *mut DestackArtifactRecord,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let revision = revision.to_bridge()?;
        let key = unsafe { key.as_ref() }.ok_or("artifact key is null")?;
        let record = bridge(session.session.artifact_record(revision, key.value.clone()))?;
        let record = DestackArtifactRecord::from_bridge(record)?;

        write_out(out, record, "artifact record output is null")
    })
}

/// Return the parsed DIR artifact for one loaded module.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_parse(
    session: *const DestackSession,
    revision: DestackRevision,
    module: DestackModule,
    out: *mut DestackDirParsed,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let revision = revision.to_bridge()?;
        let module = module.to_bridge()?;
        let parsed = bridge(session.session.parse(revision, module))?;
        let parsed = DestackDirParsed::from_bridge(parsed)?;

        write_out(out, parsed, "parsed DIR output is null")
    })
}

/// Return the resolved DIR artifact for one loaded module profile.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_resolve(
    session: *const DestackSession,
    revision: DestackRevision,
    module: DestackModule,
    profile: DestackProfileId,
    out: *mut DestackDirResolved,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let revision = revision.to_bridge()?;
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let resolved = bridge(session.session.resolve(revision, module, profile))?;
        let resolved = DestackDirResolved::from_bridge(resolved)?;

        write_out(out, resolved, "resolved DIR output is null")
    })
}

/// Return the checked DIR artifact for one loaded module profile.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_check(
    session: *const DestackSession,
    revision: DestackRevision,
    module: DestackModule,
    profile: DestackProfileId,
    out: *mut DestackDirChecked,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let revision = revision.to_bridge()?;
        let module = module.to_bridge()?;
        let profile = profile.to_bridge()?;
        let checked = bridge(session.session.check(revision, module, profile))?;
        let checked = DestackDirChecked::from_bridge(checked)?;

        write_out(out, checked, "checked DIR output is null")
    })
}

/// Return diagnostics for one immutable revision.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_diagnostics(
    session: *const DestackSession,
    revision: DestackRevision,
    key: *const DestackArtifactKey,
    out: *mut DestackDiagnosticArray,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let revision = revision.to_bridge()?;
        let key = if key.is_null() {
            None
        } else {
            let key = unsafe { key.as_ref() }.ok_or("artifact key is null")?;
            Some(key.value.clone())
        };
        let diagnostics = bridge(session.session.diagnostics(revision, key))?;
        let diagnostics = DestackDiagnosticArray::from_bridge(diagnostics)?;

        write_out(out, diagnostics, "diagnostic array output is null")
    })
}

/// Return sidecars for one artifact key in one immutable revision.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_sidecars(
    session: *const DestackSession,
    revision: DestackRevision,
    key: *const DestackArtifactKey,
    out: *mut DestackArtifactSidecarArray,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let revision = revision.to_bridge()?;
        let key = unsafe { key.as_ref() }.ok_or("artifact key is null")?;
        let sidecars = bridge(session.session.sidecars(revision, key.value.clone()))?;
        let sidecars = DestackArtifactSidecarArray::from_bridge(sidecars)?;

        write_out(out, sidecars, "artifact sidecar array output is null")
    })
}

/// Convert C artifact key handles into bridge artifact keys.
fn artifact_keys_to_bridge(
    keys: *const *const DestackArtifactKey,
    len: usize,
) -> Result<Vec<rust::ArtifactKey>, String> {
    if len == 0 {
        return Ok(Vec::new());
    }
    if keys.is_null() {
        return Err("artifact key pointer is null".to_string());
    }

    let keys = unsafe { slice::from_raw_parts(keys, len) };
    let mut values = Vec::with_capacity(keys.len());
    for key in keys {
        let key = unsafe { key.as_ref() }.ok_or("artifact key is null")?;
        values.push(key.value.clone());
    }

    Ok(values)
}

/// Convert one Rust bridge result into a C ABI result.
fn bridge<T>(result: rust::Result<T>) -> Result<T, String> {
    result.map_err(|error| error.to_string())
}

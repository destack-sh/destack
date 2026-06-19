use std::ffi::c_char;
use std::slice;

use destack as rust;

use crate::core::{DestackError, DestackStatus, c_string, read_string, return_status, write_out};
use crate::generated::{
    DestackArtifactKey, DestackArtifactRecord, DestackArtifactSidecarArray, DestackArtifactVersion,
    DestackBuildOutput, DestackBuildRequest, DestackByteArray, DestackChangeArray,
    DestackCheckOutput, DestackCommit, DestackContent, DestackContentId, DestackDiagnosticArray,
    DestackDirResolved, DestackFormatOutput, DestackFormatRequest, DestackLintOutput,
    DestackLintRequest, DestackModule, DestackParseOutput, DestackProfileId, DestackRevision,
    DestackSessionFileArray,
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
        let revision =
            DestackRevision::from_bridge(rust::language::Revision::from_repository(revision))?;

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
pub unsafe extern "C" fn destack_session_edit_if_current(
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
        let revision = repository_revision(revision)?;
        let result = bridge(
            session
                .session
                .edit_if_current(revision, edits.value.clone()),
        )?;
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

/// Return one module path in the current session.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_module(
    session: *mut DestackSession,
    path: *const c_char,
    out: *mut DestackModule,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_mut() }.ok_or("session is null")?;
        let path = read_string(path)?;
        let module = bridge(session.session.module(path))?;
        let module = DestackModule::from_bridge(module.into_bridge())?;

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
        let revision = repository_revision(&revision)?;
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
        let revision = repository_revision(&revision)?;
        let key = unsafe { key.as_ref() }.ok_or("artifact key is null")?;
        let key = artifact_key(key)?;
        let version = bridge(session.session.require(revision, key))?;
        let version = DestackArtifactVersion::from_bridge(version.into())?;

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
        let revision = repository_revision(&revision)?;
        let key = unsafe { key.as_ref() }.ok_or("artifact key is null")?;
        let key = artifact_key(key)?;
        let record = bridge(session.session.artifact_record(revision, key))?;
        let record = DestackArtifactRecord::from_bridge(record)?;

        write_out(out, record, "artifact record output is null")
    })
}

/// Build one typed language output for one immutable revision.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_build(
    session: *const DestackSession,
    revision: DestackRevision,
    request: *const DestackBuildRequest,
    out: *mut DestackBuildOutput,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let request = unsafe { request.as_ref() }.ok_or("build request is null")?;
        let revision = repository_revision(&revision)?;
        let request = build_request(request)?;
        let output = bridge(session.session.build(revision, request))?;
        let output = DestackBuildOutput::from_bridge(output)?;

        write_out(out, output, "build output is null")
    })
}

/// Return one shared content payload by exact content id.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_content(
    session: *const DestackSession,
    id: DestackContentId,
    out: *mut DestackContent,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let id = content_id(id)?;
        let content = bridge(session.session.content(id))?;
        let content = DestackContent::from_bridge(content)?;

        write_out(out, content, "content output is null")
    })
}

/// Return one text content payload by exact content id.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_text(
    session: *const DestackSession,
    id: DestackContentId,
    out: *mut *mut c_char,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let id = content_id(id)?;
        let text = bridge(session.session.text(id))?;
        let text = c_string(text)?;

        write_out(out, text, "text output is null")
    })
}

/// Return one binary content payload by exact content id.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_bytes(
    session: *const DestackSession,
    id: DestackContentId,
    out: *mut DestackByteArray,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let id = content_id(id)?;
        let bytes = bridge(session.session.bytes(id))?;
        let bytes = DestackByteArray::from_vec(bytes);

        write_out(out, bytes, "byte array output is null")
    })
}

/// Parse one loaded module.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_parse(
    session: *const DestackSession,
    revision: DestackRevision,
    module: DestackModule,
    out: *mut DestackParseOutput,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let revision = repository_revision(&revision)?;
        let module = bridge_module(module)?;
        let output = bridge(session.session.parse(revision, module))?;
        let output = DestackParseOutput::from_bridge(output)?;

        write_out(out, output, "parse output is null")
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
        let revision = repository_revision(&revision)?;
        let module = bridge_module(module)?;
        let profile = profile_id(profile)?;
        let resolved = bridge(session.session.resolve(revision, module, profile))?;
        let resolved = DestackDirResolved::from_bridge(resolved)?;

        write_out(out, resolved, "resolved DIR output is null")
    })
}

/// Check one loaded module profile.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_check(
    session: *const DestackSession,
    revision: DestackRevision,
    module: DestackModule,
    profile: DestackProfileId,
    out: *mut DestackCheckOutput,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let revision = repository_revision(&revision)?;
        let module = bridge_module(module)?;
        let profile = profile_id(profile)?;
        let output = bridge(session.session.check(revision, module, profile))?;
        let output = DestackCheckOutput::from_bridge(output)?;

        write_out(out, output, "check output is null")
    })
}

/// Format one document for one immutable revision.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_format(
    session: *const DestackSession,
    revision: DestackRevision,
    request: *const DestackFormatRequest,
    out: *mut DestackFormatOutput,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let request = unsafe { request.as_ref() }.ok_or("format request is null")?;
        let revision = repository_revision(&revision)?;
        let request = format_request(request)?;
        let output = bridge(session.session.format(revision, request))?;
        let output = DestackFormatOutput::from_bridge(output)?;

        write_out(out, output, "format output is null")
    })
}

/// Lint one scope for one immutable revision.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn destack_session_lint(
    session: *const DestackSession,
    revision: DestackRevision,
    request: *const DestackLintRequest,
    out: *mut DestackLintOutput,
    error: *mut *mut DestackError,
) -> DestackStatus {
    return_status(error, || {
        let session = unsafe { session.as_ref() }.ok_or("session is null")?;
        let request = unsafe { request.as_ref() }.ok_or("lint request is null")?;
        let revision = repository_revision(&revision)?;
        let request = lint_request(request)?;
        let output = bridge(session.session.lint(revision, request))?;
        let output = DestackLintOutput::from_bridge(output)?;

        write_out(out, output, "lint output is null")
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
        let revision = repository_revision(&revision)?;
        let key = if key.is_null() {
            None
        } else {
            let key = unsafe { key.as_ref() }.ok_or("artifact key is null")?;
            Some(artifact_key(key)?)
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
        let revision = repository_revision(&revision)?;
        let key = unsafe { key.as_ref() }.ok_or("artifact key is null")?;
        let key = artifact_key(key)?;
        let sidecars = bridge(session.session.sidecars(revision, key))?;
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
        values.push(artifact_key(key)?);
    }

    Ok(values)
}

/// Convert one C revision into one repository revision.
fn repository_revision(revision: &DestackRevision) -> Result<rust::Revision, String> {
    revision
        .to_bridge()?
        .into_repository()
        .map_err(|error| error.to_string())
}

/// Convert one C module into one Rust bridge module.
fn bridge_module(module: DestackModule) -> Result<rust::Module, String> {
    rust::Module::try_from(module.to_bridge()?).map_err(|error| error.to_string())
}

/// Convert one C profile id into one source profile id.
fn profile_id(profile: DestackProfileId) -> Result<rust::ProfileId, String> {
    profile
        .to_bridge()?
        .into_source()
        .map_err(|error| error.to_string())
}

/// Convert one C content id into one source content id.
fn content_id(content: DestackContentId) -> Result<rust::ContentId, String> {
    content
        .to_bridge()?
        .into_source()
        .map_err(|error| error.to_string())
}

/// Convert one C artifact key handle into one artifact key.
fn artifact_key(key: &DestackArtifactKey) -> Result<rust::ArtifactKey, String> {
    key.value
        .clone()
        .into_artifact()
        .map_err(|error| error.to_string())
}

/// Convert one C build request into one Rust build request.
fn build_request(request: &DestackBuildRequest) -> Result<rust::BuildRequest, String> {
    rust::BuildRequest::try_from(request.to_bridge()?).map_err(|error| error.to_string())
}

/// Convert one C format request into one Rust format request.
fn format_request(request: &DestackFormatRequest) -> Result<rust::FormatRequest, String> {
    rust::FormatRequest::try_from(request.to_bridge()?).map_err(|error| error.to_string())
}

/// Convert one C lint request into one Rust lint request.
fn lint_request(request: &DestackLintRequest) -> Result<rust::LintRequest, String> {
    rust::LintRequest::try_from(request.to_bridge()?).map_err(|error| error.to_string())
}

/// Convert one Rust bridge result into a C ABI result.
fn bridge<T>(result: rust::Result<T>) -> Result<T, String> {
    result.map_err(|error| error.to_string())
}

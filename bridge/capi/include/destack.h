#ifndef DESTACK_H
#define DESTACK_H

#include "destack/core.h"

#ifdef __cplusplus
extern "C" {
#endif

#include "destack/generated.h"

DestackStatus destack_source_file_system(
    const char *path,
    DestackSource **out,
    DestackError **error
);
DestackStatus destack_source_memory(
    const char *root,
    DestackSource **out,
    DestackError **error
);
void destack_source_destroy(DestackSource *source);
DestackStatus destack_source_add_text(
    DestackSource *source,
    const char *path,
    const char *text,
    DestackError **error
);
DestackStatus destack_source_add_bytes(
    DestackSource *source,
    const char *path,
    const uint8_t *bytes,
    size_t len,
    DestackError **error
);

DestackEdits *destack_edits_new(void);
void destack_edits_destroy(DestackEdits *edits);
DestackStatus destack_edits_add_set_text(
    DestackEdits *edits,
    const char *path,
    const char *text,
    DestackError **error
);
DestackStatus destack_edits_add_edit_text(
    DestackEdits *edits,
    const char *path,
    const DestackTextEdit *text_edits,
    size_t len,
    DestackError **error
);
DestackStatus destack_edits_add_set_bytes(
    DestackEdits *edits,
    const char *path,
    const uint8_t *bytes,
    size_t len,
    DestackError **error
);
DestackStatus destack_edits_add_remove(
    DestackEdits *edits,
    const char *path,
    DestackError **error
);
DestackStatus destack_edits_add_move(
    DestackEdits *edits,
    const char *from,
    const char *to,
    DestackError **error
);

DestackStatus destack_session_open(
    const DestackSource *source,
    DestackSession **out,
    DestackError **error
);
void destack_session_destroy(DestackSession *session);
DestackStatus destack_session_revision(
    const DestackSession *session,
    DestackRevision *out,
    DestackError **error
);
DestackStatus destack_session_files(
    const DestackSession *session,
    DestackSessionFileArray *out,
    DestackError **error
);
DestackStatus destack_session_edit(
    DestackSession *session,
    const DestackEdits *edits,
    DestackCommit *out,
    DestackError **error
);
DestackStatus destack_session_edit_if_current(
    DestackSession *session,
    const DestackRevision *revision,
    const DestackEdits *edits,
    DestackCommit *out,
    DestackError **error
);
DestackStatus destack_session_reload(
    DestackSession *session,
    DestackChangeArray *out,
    DestackError **error
);
DestackStatus destack_session_module(
    DestackSession *session,
    const char *path,
    DestackModule *out,
    DestackError **error
);
DestackStatus destack_session_provide(
    const DestackSession *session,
    DestackRevision revision,
    const DestackArtifactKey *const *keys,
    size_t len,
    DestackError **error
);
DestackStatus destack_session_require(
    const DestackSession *session,
    DestackRevision revision,
    const DestackArtifactKey *key,
    DestackArtifactVersion *out,
    DestackError **error
);
DestackStatus destack_session_artifact_record(
    const DestackSession *session,
    DestackRevision revision,
    const DestackArtifactKey *key,
    DestackArtifactRecord *out,
    DestackError **error
);
DestackStatus destack_session_build(
    const DestackSession *session,
    DestackRevision revision,
    const DestackBuildRequest *request,
    DestackBuildOutput *out,
    DestackError **error
);
DestackStatus destack_session_content(
    const DestackSession *session,
    DestackContentId id,
    DestackContent *out,
    DestackError **error
);
DestackStatus destack_session_text(
    const DestackSession *session,
    DestackContentId id,
    char **out,
    DestackError **error
);
DestackStatus destack_session_bytes(
    const DestackSession *session,
    DestackContentId id,
    DestackByteArray *out,
    DestackError **error
);
DestackStatus destack_session_parse(
    const DestackSession *session,
    DestackRevision revision,
    DestackModule module,
    DestackParseOutput *out,
    DestackError **error
);
DestackStatus destack_session_resolve(
    const DestackSession *session,
    DestackRevision revision,
    DestackModule module,
    DestackProfileId profile,
    DestackDirResolved *out,
    DestackError **error
);
DestackStatus destack_session_check(
    const DestackSession *session,
    DestackRevision revision,
    DestackModule module,
    DestackProfileId profile,
    DestackCheckOutput *out,
    DestackError **error
);
DestackStatus destack_session_format(
    const DestackSession *session,
    DestackRevision revision,
    const DestackFormatRequest *request,
    DestackFormatOutput *out,
    DestackError **error
);
DestackStatus destack_session_lint(
    const DestackSession *session,
    DestackRevision revision,
    const DestackLintRequest *request,
    DestackLintOutput *out,
    DestackError **error
);
DestackStatus destack_session_diagnostics(
    const DestackSession *session,
    DestackRevision revision,
    const DestackArtifactKey *key,
    DestackDiagnosticArray *out,
    DestackError **error
);
DestackStatus destack_session_sidecars(
    const DestackSession *session,
    DestackRevision revision,
    const DestackArtifactKey *key,
    DestackArtifactSidecarArray *out,
    DestackError **error
);

#ifdef __cplusplus
}
#endif

#endif

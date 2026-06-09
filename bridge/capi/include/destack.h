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

DestackFileUpdate *destack_file_update_new(void);
void destack_file_update_destroy(DestackFileUpdate *update);
DestackStatus destack_file_update_set_base(
    DestackFileUpdate *update,
    const char *revision,
    DestackError **error
);
DestackStatus destack_file_update_add_set_text(
    DestackFileUpdate *update,
    const char *path,
    const char *text,
    DestackError **error
);
DestackStatus destack_file_update_add_edit_text(
    DestackFileUpdate *update,
    const char *path,
    const DestackTextEdit *edits,
    size_t len,
    DestackError **error
);
DestackStatus destack_file_update_add_set_bytes(
    DestackFileUpdate *update,
    const char *path,
    const uint8_t *bytes,
    size_t len,
    DestackError **error
);
DestackStatus destack_file_update_add_remove(
    DestackFileUpdate *update,
    const char *path,
    DestackError **error
);
DestackStatus destack_file_update_add_move(
    DestackFileUpdate *update,
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
DestackStatus destack_session_update(
    DestackSession *session,
    const DestackFileUpdate *update,
    DestackFileUpdateResult *out,
    DestackError **error
);
DestackStatus destack_session_reload(
    DestackSession *session,
    DestackFileChangeArray *out,
    DestackError **error
);
DestackStatus destack_session_load_module(
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
DestackStatus destack_session_parse(
    const DestackSession *session,
    DestackRevision revision,
    DestackModule module,
    DestackDirParsed *out,
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
    DestackDirChecked *out,
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

#ifndef DESTACK_H
#define DESTACK_H

#include "destack/core.h"

#ifdef __cplusplus
extern "C" {
#endif

#include "destack/generated.h"

DestackStatus destack_local_workspace_server_open(
    const char *home,
    DestackLocalWorkspaceServer **out,
    DestackError **error
);
void destack_local_workspace_server_destroy(DestackLocalWorkspaceServer *server);
DestackStatus destack_local_workspace_server_dispatch(
    const DestackLocalWorkspaceServer *server,
    const uint8_t *bytes,
    size_t len,
    DestackProtocolPayloadArray *out,
    DestackError **error
);
void destack_protocol_payload_array_destroy(DestackProtocolPayloadArray array);

#ifdef __cplusplus
}
#endif

#endif

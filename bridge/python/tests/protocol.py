from __future__ import annotations

from collections import deque
from collections.abc import Callable

from destack._generated.protocol.defaults import protocol_limits, protocol_version
from destack._generated.protocol.envelope import (
    ProtocolMessage,
    ProtocolMessageRequest,
    ProtocolMessageResponse,
    ProtocolRequest,
    ProtocolResponse,
    RequestId,
    RequestOptions,
)
from destack._generated.protocol.handshake import (
    HandshakeResponse,
    ServerDescriptor,
)
from destack._generated.protocol.query.model import (
    WorkspaceQueryResponseCurrentRevision,
)
from destack._generated.protocol.repository.revision import Revision
from destack._generated.protocol.request import WorkspaceRequestPing
from destack._generated.protocol.response import (
    WorkspaceResponse,
    WorkspaceResponseHandshake,
    WorkspaceResponsePong,
    WorkspaceResponseQueryResult,
    WorkspaceResponseRootClosed,
    WorkspaceResponseRootOpened,
    WorkspaceResponseRootReloaded,
)
from destack._generated.protocol.root import (
    RootClosedResponse,
    RootId,
    RootOpenedResponse,
    RootReloadResponse,
)
from destack._generated.protocol.workspace.message import UpdateBatch
from destack.protocol.codec import decode_frame, encode_frame
from destack.protocol.connection import Connection
from destack.protocol.workspace.client import open_remote_workspace


def test_roundtrip_ping_request_frame() -> None:
    """Roundtrip one protocol frame through the public codec."""

    message = ProtocolMessageRequest(
        request=ProtocolRequest(
            id=RequestId(field_0=7),
            options=RequestOptions(timeout_ms=None, priority=None, trace_id=None),
            payload=WorkspaceRequestPing(),
        )
    )

    # encode and decode the exact request frame
    frame = encode_frame(message)
    decoded = decode_frame(frame)

    assert decoded == message


def test_open_remote_workspace() -> None:
    """Open and drive a workspace through the protocol client."""

    transport = MemoryTransport(handle_workspace_request)
    connection = Connection(transport)

    # open and drive the remote workspace facade
    workspace = open_remote_workspace(connection=connection, workspace="/workspace")
    revision = workspace.revision()
    reload = workspace.reload()
    workspace.close()

    assert workspace.root() == "/workspace"
    assert revision == Revision(field_0=bytes([3]) * 32)
    assert reload.updates == []


def handle_workspace_request(message: ProtocolMessage) -> ProtocolMessage:
    """Return one test response for a workspace protocol request."""

    assert isinstance(message, ProtocolMessageRequest)
    payload = message.request.payload

    # negotiate workspace protocol
    if payload.kind == "handshake":
        return response(
            message,
            WorkspaceResponseHandshake(
                handshake=HandshakeResponse(
                    protocol=protocol_version,
                    server=ServerDescriptor(
                        name="test-workspace",
                        version="0",
                        build=None,
                    ),
                    limits=protocol_limits,
                )
            ),
        )

    # open a root handle
    if payload.kind == "openRoot":
        assert payload.open_root.root == "/workspace"

        return response(
            message,
            WorkspaceResponseRootOpened(
                root_opened=RootOpenedResponse(
                    handle=RootId(field_0=11),
                    root="/workspace",
                    diagnostics=[],
                    messages=[],
                )
            ),
        )

    # answer the current revision query
    if payload.kind == "query":
        assert payload.query.kind == "currentRevision"

        return response(
            message,
            WorkspaceResponseQueryResult(
                query_result=WorkspaceQueryResponseCurrentRevision(
                    current_revision=Revision(field_0=bytes([3]) * 32)
                )
            ),
        )

    # reload without changes
    if payload.kind == "reloadRoot":
        assert payload.reload_root.reason == "manual"

        return response(
            message,
            WorkspaceResponseRootReloaded(
                root_reloaded=RootReloadResponse(
                    handle=RootId(field_0=11),
                    updates=UpdateBatch(updates=[], messages=[]),
                )
            ),
        )

    # close the root handle
    if payload.kind == "closeRoot":
        return response(
            message,
            WorkspaceResponseRootClosed(
                root_closed=RootClosedResponse(handle=RootId(field_0=11))
            ),
        )

    raise AssertionError(f"unexpected request: {payload.kind}")


def response(
    request: ProtocolMessageRequest,
    payload: WorkspaceResponse,
) -> ProtocolMessageResponse:
    """Wrap one protocol response."""

    return ProtocolMessageResponse(
        response=ProtocolResponse(id=request.request.id, payload=payload)
    )


class MemoryTransport:
    """In-memory transport for synchronous protocol tests."""

    def __init__(self, handler: Callable[[ProtocolMessage], ProtocolMessage]) -> None:
        self._handler = handler
        self._frames: deque[bytes] = deque()

    def send(self, data: bytes) -> None:
        """Send one frame through the test handler."""

        request = decode_frame(data)
        response = self._handler(request)
        self._frames.append(encode_frame(response))

    def receive(self) -> bytes:
        """Receive one queued test frame."""

        return self._frames.popleft()

    def close(self) -> None:
        """Close this test transport."""

        self._frames.clear()

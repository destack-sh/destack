from __future__ import annotations

import builtins

from dataclasses import replace

from destack._generated.protocol.envelope import ProtocolNotification
from destack._generated.protocol.notification import WorkspaceNotificationPayloadChunk
from destack._generated.protocol.payload import (
    BinaryPayload,
    PayloadBodyDeferred,
    PayloadBodyInline,
    PayloadChunkNotification,
)
from destack._generated.protocol.query import (
    QueryResponsePayload,
    WorkspaceQueryResponse,
    WorkspaceQueryResponseQuery,
    WorkspaceQueryResponseQueryBatch,
)
from destack._generated.protocol.response import (
    WorkspaceResponse,
    WorkspaceResponseQueryResult,
)
from ..error import ProtocolRequestError


class PayloadReceiver:
    """Deferred payload chunks received while waiting for a response."""

    def __init__(self) -> None:
        self._pending: dict[int, PayloadStream] = {}
        self._completed: dict[int, builtins.bytes] = {}
        self._response: WorkspaceResponse | None = None

    def ingest_notification(self, notification: ProtocolNotification) -> bool:
        """Ingest one notification if it carries payload bytes."""

        payload = notification.payload
        if isinstance(payload, WorkspaceNotificationPayloadChunk):
            self.ingest(payload.payload_chunk)
            return True

        return False

    def ingest(self, chunk: PayloadChunkNotification) -> None:
        """Ingest one payload chunk."""

        if chunk.done and chunk.index + 1 != chunk.total:
            raise ProtocolRequestError("payload chunk done marker is inconsistent")

        stream = self._pending.get(chunk.id)
        if stream is None:
            stream = PayloadStream(chunk.total)
            self._pending[chunk.id] = stream

        stream.insert(chunk.index, bytes(chunk.bytes))
        if stream.is_complete:
            del self._pending[chunk.id]
            self._completed[chunk.id] = stream.bytes()

    def resolve(self, response: WorkspaceResponse) -> WorkspaceResponse | None:
        """Resolve deferred payloads referenced by one response."""

        self._response = response

        return self.resolve_required()

    def resolve_required(self) -> WorkspaceResponse | None:
        """Resolve the stored response if all payload chunks are present."""

        if self._response is None:
            return None

        if not isinstance(self._response, WorkspaceResponseQueryResult):
            return self._response

        query = self._resolve_query_response(self._response.query_result)
        if query is None:
            return None

        return replace(self._response, query_result=query)

    def _resolve_query_response(
        self,
        response: WorkspaceQueryResponse,
    ) -> WorkspaceQueryResponse | None:
        """Resolve one query response payload."""

        if isinstance(response, WorkspaceQueryResponseQuery):
            payload = self._resolve_payload(response.query.payload)
            if payload is None:
                return None

            return replace(response, query=QueryResponsePayload(payload=payload))

        if isinstance(response, WorkspaceQueryResponseQueryBatch):
            batch: list[QueryResponsePayload] = []
            for item in response.query_batch:
                payload = self._resolve_payload(item.payload)
                if payload is None:
                    return None

                batch.append(QueryResponsePayload(payload=payload))

            return replace(response, query_batch=batch)

        return response

    def _resolve_payload(self, payload: BinaryPayload) -> BinaryPayload | None:
        """Resolve one binary payload if its bytes are present."""

        if isinstance(payload.body, PayloadBodyInline):
            return payload

        if not isinstance(payload.body, PayloadBodyDeferred):
            raise ProtocolRequestError("unknown payload body")

        payload_id = payload.body.id
        bytes_ = self._completed.get(payload_id)
        if bytes_ is None:
            return None
        if len(bytes_) != payload.body.total_bytes:
            raise ProtocolRequestError("payload size mismatch")

        del self._completed[payload_id]

        return BinaryPayload(body=PayloadBodyInline(bytes=bytes_))


class PayloadStream:
    """Pending payload stream."""

    def __init__(self, total: int) -> None:
        if not isinstance(total, int) or total <= 0:
            raise ProtocolRequestError("payload chunk total must be positive")

        self._chunks: list[builtins.bytes | None] = [None] * total
        self._received = 0

    @property
    def is_complete(self) -> bool:
        """Return whether all chunks have arrived."""

        return self._received == len(self._chunks)

    def insert(self, index: int, data: builtins.bytes) -> None:
        """Insert one chunk."""

        if not isinstance(index, int) or index < 0 or index >= len(self._chunks):
            raise ProtocolRequestError("payload chunk index out of range")
        if self._chunks[index] is not None:
            raise ProtocolRequestError("duplicate payload chunk")

        self._chunks[index] = data
        self._received += 1

    def bytes(self) -> builtins.bytes:
        """Merge chunks into one byte buffer."""

        chunks = []
        for chunk in self._chunks:
            if chunk is None:
                raise ProtocolRequestError("payload chunks missing at finalize")

            chunks.append(chunk)

        return b"".join(chunks)

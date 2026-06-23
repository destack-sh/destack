from __future__ import annotations

from collections import deque
from collections.abc import Sequence
from typing import Protocol

from destack.protocol.codec import decode_payload_frame, encode_payload_frame

from .transport import TransportError


class EmbeddedServer(Protocol):
    """Server that can dispatch one workspace protocol payload."""

    def dispatch(self, data: bytes) -> Sequence[bytes]:
        """Dispatch one encoded protocol message payload."""


class EmbeddedTransport:
    """Embedded transport backed by an in-process workspace server."""

    def __init__(self, server: EmbeddedServer) -> None:
        self._server = server
        self._queue: deque[bytes] = deque()
        self._closed = False

    def send(self, data: bytes) -> None:
        """Send one binary frame."""

        if self._closed:
            raise TransportError("embedded transport is closed")

        payload = decode_payload_frame(data)
        responses = self._server.dispatch(payload)
        self._queue.extend(encode_payload_frame(response) for response in responses)

    def receive(self) -> bytes:
        """Receive one binary frame."""

        try:
            return self._queue.popleft()
        except IndexError as error:
            raise TransportError("embedded transport has no pending frame") from error

    def close(self) -> None:
        """Close the transport."""

        self._closed = True
        self._queue.clear()


__all__ = [
    "EmbeddedServer",
    "EmbeddedTransport",
]

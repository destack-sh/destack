from __future__ import annotations

import base64
import hashlib
import os
import socket
import ssl
from typing import Protocol
from urllib.parse import urlparse

WEBSOCKET_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
WEBSOCKET_VERSION = "13"
DEFAULT_WS_PORT = 80
DEFAULT_WSS_PORT = 443
FRAME_PAYLOAD_16 = 126
FRAME_PAYLOAD_64 = 127
FRAME_MASK_BIT = 0x80
FRAME_OPCODE_BINARY = 0x02
FRAME_OPCODE_CLOSE = 0x08
FRAME_OPCODE_PING = 0x09
FRAME_OPCODE_PONG = 0x0A


class Transport(Protocol):
    """Binary transport for workspace protocol frames."""

    def send(self, data: bytes) -> None:
        """Send one binary frame."""

    def receive(self) -> bytes:
        """Receive one binary frame."""

    def close(self) -> None:
        """Close the transport."""


class TransportError(Exception):
    """Error thrown by workspace protocol transports."""


class WebSocketTransport:
    """Synchronous WebSocket binary transport."""

    def __init__(self, sock: socket.socket) -> None:
        self._socket = sock
        self._closed = False

    @classmethod
    def connect(cls, url: str) -> WebSocketTransport:
        """Connect to one WebSocket endpoint."""

        endpoint = WebSocketEndpoint.parse(url)
        sock = socket.create_connection((endpoint.host, endpoint.port))

        if endpoint.is_secure:
            context = ssl.create_default_context()
            sock = context.wrap_socket(sock, server_hostname=endpoint.host)

        key = base64.b64encode(os.urandom(16)).decode("ascii")
        request = endpoint.handshake_request(key)
        sock.sendall(request.encode("ascii"))

        response = receive_http_response(sock)
        validate_handshake_response(response, key)

        return cls(sock)

    def send(self, data: bytes) -> None:
        """Send one binary WebSocket message."""

        if self._closed:
            raise TransportError("websocket transport is closed")

        frame = encode_websocket_frame(FRAME_OPCODE_BINARY, data, is_masked=True)
        self._socket.sendall(frame)

    def receive(self) -> bytes:
        """Receive one binary WebSocket message."""

        while not self._closed:
            frame = read_websocket_frame(self._socket)

            if frame.opcode == FRAME_OPCODE_BINARY:
                return frame.payload

            if frame.opcode == FRAME_OPCODE_PING:
                pong = encode_websocket_frame(
                    FRAME_OPCODE_PONG, frame.payload, is_masked=True
                )
                self._socket.sendall(pong)
                continue

            if frame.opcode == FRAME_OPCODE_PONG:
                continue

            if frame.opcode == FRAME_OPCODE_CLOSE:
                self.close()
                raise TransportError("websocket transport closed")

            raise TransportError(f"unsupported websocket opcode: {frame.opcode}")

        raise TransportError("websocket transport is closed")

    def close(self) -> None:
        """Close the WebSocket transport."""

        if self._closed:
            return

        self._closed = True
        try:
            self._socket.close()
        except OSError:
            return


class WebSocketEndpoint:
    """Parsed WebSocket endpoint."""

    def __init__(
        self, url: str, host: str, port: int, resource: str, is_secure: bool
    ) -> None:
        self.url = url
        self.host = host
        self.port = port
        self.resource = resource
        self.is_secure = is_secure

    @classmethod
    def parse(cls, url: str) -> WebSocketEndpoint:
        """Parse one WebSocket endpoint URL."""

        parsed = urlparse(url)
        if parsed.scheme not in {"ws", "wss"}:
            raise TransportError(f"unsupported websocket scheme: {parsed.scheme}")
        if parsed.hostname is None:
            raise TransportError("websocket url is missing a host")

        is_secure = parsed.scheme == "wss"
        port = parsed.port or (DEFAULT_WSS_PORT if is_secure else DEFAULT_WS_PORT)
        resource = parsed.path or "/"
        if parsed.query:
            resource = f"{resource}?{parsed.query}"

        return cls(
            url=url,
            host=parsed.hostname,
            port=port,
            resource=resource,
            is_secure=is_secure,
        )

    def handshake_request(self, key: str) -> str:
        """Return one HTTP upgrade request."""

        host = self.host
        if self.port not in {DEFAULT_WS_PORT, DEFAULT_WSS_PORT}:
            host = f"{host}:{self.port}"

        return (
            f"GET {self.resource} HTTP/1.1\r\n"
            f"Host: {host}\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\n"
            f"Sec-WebSocket-Version: {WEBSOCKET_VERSION}\r\n"
            "\r\n"
        )


class WebSocketFrame:
    """One decoded WebSocket frame."""

    def __init__(self, opcode: int, payload: bytes) -> None:
        self.opcode = opcode
        self.payload = payload


def receive_http_response(sock: socket.socket) -> bytes:
    """Read one HTTP response header block."""

    data = bytearray()
    while b"\r\n\r\n" not in data:
        chunk = sock.recv(4096)
        if not chunk:
            raise TransportError("websocket handshake ended early")

        data.extend(chunk)

    return bytes(data)


def validate_handshake_response(response: bytes, key: str) -> None:
    """Validate one WebSocket handshake response."""

    header = response.split(b"\r\n\r\n", 1)[0].decode("iso-8859-1")
    lines = header.split("\r\n")
    if not lines or " 101 " not in lines[0]:
        raise TransportError("websocket handshake was not accepted")

    headers = {}
    for line in lines[1:]:
        if ":" in line:
            name, value = line.split(":", 1)
            headers[name.strip().lower()] = value.strip()

    expected = websocket_accept_key(key)
    actual = headers.get("sec-websocket-accept")
    if actual != expected:
        raise TransportError("websocket handshake returned an invalid accept key")


def websocket_accept_key(key: str) -> str:
    """Return the expected WebSocket accept key."""

    digest = hashlib.sha1((key + WEBSOCKET_GUID).encode("ascii")).digest()

    return base64.b64encode(digest).decode("ascii")


def encode_websocket_frame(opcode: int, payload: bytes, is_masked: bool) -> bytes:
    """Encode one WebSocket frame."""

    length = len(payload)
    first = 0x80 | opcode
    mask = os.urandom(4) if is_masked else b""
    marker = FRAME_MASK_BIT if is_masked else 0

    if length < FRAME_PAYLOAD_16:
        header = bytes([first, marker | length])
    elif length <= 0xFFFF:
        header = bytes([first, marker | FRAME_PAYLOAD_16]) + length.to_bytes(2, "big")
    else:
        header = bytes([first, marker | FRAME_PAYLOAD_64]) + length.to_bytes(8, "big")

    if not is_masked:
        return header + payload

    return header + mask + mask_payload(payload, mask)


def read_websocket_frame(sock: socket.socket) -> WebSocketFrame:
    """Read one WebSocket frame."""

    header = read_exact(sock, 2)
    first = header[0]
    second = header[1]
    opcode = first & 0x0F
    is_final = (first & 0x80) != 0
    is_masked = (second & FRAME_MASK_BIT) != 0
    length = second & 0x7F

    if not is_final:
        raise TransportError("websocket continuation frames are not supported")

    if length == FRAME_PAYLOAD_16:
        length = int.from_bytes(read_exact(sock, 2), "big")
    elif length == FRAME_PAYLOAD_64:
        length = int.from_bytes(read_exact(sock, 8), "big")

    mask = read_exact(sock, 4) if is_masked else b""
    payload = read_exact(sock, length)
    if is_masked:
        payload = mask_payload(payload, mask)

    return WebSocketFrame(opcode=opcode, payload=payload)


def read_exact(sock: socket.socket, length: int) -> bytes:
    """Read exactly this many bytes from one socket."""

    data = bytearray()
    while len(data) < length:
        chunk = sock.recv(length - len(data))
        if not chunk:
            raise TransportError("websocket transport ended early")

        data.extend(chunk)

    return bytes(data)


def mask_payload(payload: bytes, mask: bytes) -> bytes:
    """Apply one WebSocket mask."""

    return bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))

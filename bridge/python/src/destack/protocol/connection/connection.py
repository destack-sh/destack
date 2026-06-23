from __future__ import annotations

from collections.abc import Callable
from dataclasses import replace

from ..codec import decode_frame, encode_frame
from destack._generated.protocol.defaults import (
    client_descriptor,
    protocol_limits,
    protocol_range,
)
from destack._generated.protocol.envelope import (
    ProtocolMessageNotification,
    ProtocolMessageRequest,
    ProtocolMessageResponse,
    ProtocolNotification,
    ProtocolRequest,
    ProtocolResponse,
    RequestId,
    RequestOptions,
)
from destack._generated.protocol.handshake import (
    ClientDescriptor,
    HandshakeRequest,
    HandshakeResponse,
    ProtocolLimits,
)
from destack._generated.protocol.request import (
    WorkspaceRequest,
    WorkspaceRequestHandshake,
)
from destack._generated.protocol.response import (
    WorkspaceResponse,
    WorkspaceResponseError,
    WorkspaceResponseHandshake,
)
from destack._generated.protocol.version import ProtocolRange
from .error import ProtocolRequestError
from .payload.receiver import PayloadReceiver
from .transport import Transport, WebSocketTransport

NotificationHandler = Callable[[ProtocolNotification], None]


class Connection:
    """Synchronous connection to one workspace protocol endpoint."""

    def __init__(
        self,
        transport: Transport,
        on_notification: NotificationHandler | None = None,
    ) -> None:
        self._transport = transport
        self._on_notification = on_notification
        self._next_request_id = 1
        self._handshake: HandshakeResponse | None = None
        self._is_closed = False

    @classmethod
    def connect_websocket(
        cls,
        url: str,
        on_notification: NotificationHandler | None = None,
        client: ClientOptions | None = None,
    ) -> Connection:
        """Connect to one WebSocket workspace endpoint."""

        transport = WebSocketTransport.connect(url)
        connection = cls(transport=transport, on_notification=on_notification)
        connection.handshake(client)

        return connection

    @property
    def handshake_response(self) -> HandshakeResponse | None:
        """Return the negotiated handshake response, if any."""

        return self._handshake

    def handshake(self, options: ClientOptions | None = None) -> HandshakeResponse:
        """Negotiate the workspace protocol for this connection."""

        if self._handshake is not None:
            return self._handshake

        options = options or ClientOptions()
        request = WorkspaceRequestHandshake(
            handshake=HandshakeRequest(
                protocol=options.protocol or protocol_range,
                client=options.client_descriptor(),
                limits=options.limits or protocol_limits,
            ),
        )
        response = self.request(request)

        if isinstance(response, WorkspaceResponseHandshake):
            self._handshake = response.handshake
            return response.handshake

        if isinstance(response, WorkspaceResponseError):
            raise ProtocolRequestError.from_protocol(response.error)

        raise ProtocolRequestError(f"unexpected handshake response: {response.kind}")

    def request(
        self,
        payload: WorkspaceRequest,
        options: RequestOptions | None = None,
    ) -> WorkspaceResponse:
        """Send one workspace request and wait for its response."""

        if self._is_closed:
            raise ProtocolRequestError("workspace connection is closed")

        request_id = self._next_request_id
        self._next_request_id += 1
        request_options = options or RequestOptions(
            timeout_ms=None, priority=None, trace_id=None
        )
        request = ProtocolRequest(
            id=RequestId(field_0=request_id),
            options=request_options,
            payload=payload,
        )
        message = ProtocolMessageRequest(request=request)

        self._transport.send(encode_frame(message))

        return self._receive_response(request_id)

    def close(self) -> None:
        """Close this connection."""

        if self._is_closed:
            return

        self._is_closed = True
        self._transport.close()

    def _receive_response(self, request_id: int) -> WorkspaceResponse:
        """Receive messages until this request receives its response."""

        payloads = PayloadReceiver()
        response: WorkspaceResponse | None = None

        while response is None:
            message = decode_frame(self._transport.receive())

            if isinstance(message, ProtocolMessageResponse):
                response = self._response_for_id(message.response, request_id)
                continue

            if isinstance(message, ProtocolMessageNotification):
                is_payload = payloads.ingest_notification(message.notification)
                if not is_payload and self._on_notification is not None:
                    self._on_notification(message.notification)
                continue

            raise ProtocolRequestError("workspace connection received a request")

        response = payloads.resolve(response)
        if response is None:
            response = self._receive_payloads(request_id, payloads)

        if isinstance(response, WorkspaceResponseError):
            raise ProtocolRequestError.from_protocol(response.error)

        return response

    def _receive_payloads(
        self,
        request_id: int,
        payloads: PayloadReceiver,
    ) -> WorkspaceResponse:
        """Receive payload chunks required by one response."""

        while True:
            message = decode_frame(self._transport.receive())

            if isinstance(message, ProtocolMessageNotification):
                is_payload = payloads.ingest_notification(message.notification)
                if not is_payload and self._on_notification is not None:
                    self._on_notification(message.notification)
            elif isinstance(message, ProtocolMessageResponse):
                self._response_for_id(message.response, request_id)
                raise ProtocolRequestError(
                    "workspace connection received duplicate response"
                )
            else:
                raise ProtocolRequestError("workspace connection received a request")

            response = payloads.resolve_required()
            if response is not None:
                return response

    def _response_for_id(
        self,
        response: ProtocolResponse,
        request_id: int,
    ) -> WorkspaceResponse:
        """Return one response payload after checking its request id."""

        if response.id.field_0 != request_id:
            raise ProtocolRequestError(
                f"workspace connection received response id {response.id.field_0}, expected {request_id}"
            )

        return response.payload


class ClientOptions:
    """Options for workspace protocol negotiation."""

    def __init__(
        self,
        protocol: ProtocolRange | None = None,
        limits: ProtocolLimits | None = None,
        client: ClientDescriptor | None = None,
        name: str | None = None,
        version: str | None = None,
        build: str | None = None,
    ) -> None:
        self.protocol = protocol
        self.limits = limits
        self.client = client
        self.name = name
        self.version = version
        self.build = build

    def client_descriptor(self) -> ClientDescriptor:
        """Return the effective client descriptor."""

        descriptor = self.client or client_descriptor

        return replace(
            descriptor,
            name=self.name or descriptor.name,
            version=self.version or descriptor.version,
            build=self.build if self.build is not None else descriptor.build,
        )


def connect_endpoint(
    url: str,
    on_notification: NotificationHandler | None = None,
    client: ClientOptions | None = None,
) -> Connection:
    """Connect to one WebSocket workspace endpoint."""

    return Connection.connect_websocket(
        url=url,
        on_notification=on_notification,
        client=client,
    )

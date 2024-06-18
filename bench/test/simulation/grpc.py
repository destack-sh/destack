import asyncio
from typing import Collection, Optional

from grpclib._typing import IServable
from grpclib.client import Channel
from grpclib.encoding.base import CodecBase, StatusDetailsCodecBase
from grpclib.protocol import H2Protocol
from grpclib.server import Server as GrpcServer


class SimulatedServer(asyncio.AbstractServer):
    def get_loop(self) -> asyncio.AbstractEventLoop:
        raise NotImplementedError

    def is_serving(self) -> bool:
        raise NotImplementedError

    async def start_serving(self) -> None:
        raise NotImplementedError

    async def serve_forever(self) -> None:
        raise NotImplementedError

    def close(self) -> None:
        pass

    async def wait_closed(self) -> None:
        pass


class SimulatedTransport(asyncio.Transport):
    def __init__(
        self,
        protocol: H2Protocol,
    ) -> None:
        super().__init__()
        self._loop = asyncio.get_event_loop()
        self._protocol = protocol

    def _write_soon(self, data: bytes) -> None:
        if not self._protocol.connection.is_closing():
            self._protocol.data_received(data)

    def write(self, data: bytes) -> None:
        if data:
            self._loop.call_soon(self._write_soon, data)

    def is_closing(self) -> bool:
        return False

    def close(self) -> None:
        pass


class SimulatedChannel:
    """Simulated in-memory channel to connect clients to gRPC services"""

    def __init__(
        self,
        services: Collection["IServable"],
        codec: Optional[CodecBase] = None,
        status_details_codec: Optional[StatusDetailsCodecBase] = None,
    ) -> None:
        self._services = services
        self._codec = codec
        self._status_details_codec = status_details_codec

    async def open(self) -> Channel:
        self._server = GrpcServer(
            self._services,
            codec=self._codec,
            status_details_codec=self._status_details_codec,
        )
        self._server._server = SimulatedServer()
        self._server._server_closed_fut = self._server._loop.create_future()
        self._server_protocol = self._server._protocol_factory()

        self._channel = Channel(
            codec=self._codec,
            status_details_codec=self._status_details_codec,
        )
        self._channel._protocol = self._channel._protocol_factory()

        self._channel._protocol.connection_made(SimulatedTransport(self._server_protocol))
        self._server_protocol.connection_made(SimulatedTransport(self._channel._protocol))
        return self._channel

    def close(self):
        if self._channel is not None:
            if self._channel._protocol is not None:
                self._channel._protocol.connection_lost(None)
            self._channel.close()
            self._channel = None

        if self._server is not None:
            self._server_protocol.connection_lost(None)
            self._server.close()

    async def wait_closed(self):
        if self._server is not None:
            await self._server.wait_closed()
            self._server = None

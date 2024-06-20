import asyncio
from typing import Collection

from grpclib._typing import IServable
from grpclib.client import Channel
from grpclib.protocol import H2Protocol
from grpclib.server import Server as GrpcServer

from bench.utils.oracle import Oracle


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
        *,
        protocol: H2Protocol,
        oracle: Oracle,
        latency_min: float,
        latency_mean: float,
    ) -> None:
        super().__init__()
        self._protocol = protocol
        self._oracle = oracle
        self._latency_min = latency_min
        self._latency_mean = latency_mean

    def _write_soon(self, data: bytes) -> None:
        if not self._protocol.connection.is_closing():
            self._protocol.data_received(data)

    def write(self, data: bytes) -> None:
        if data:
            latency = self._oracle.random.expovariate(1 / self._latency_mean) + self._latency_min
            self._oracle.call_later(latency, self._write_soon, data)

    def is_closing(self) -> bool:
        return False

    def close(self) -> None:
        pass


class SimulatedChannel:
    """Simulated in-memory channel to connect clients to gRPC services"""

    def __init__(
        self,
        *,
        services: Collection["IServable"],
        oracle: Oracle,
        latency_min: float,
        latency_mean: float,
    ) -> None:
        self._services = services
        self._oracle = oracle
        self._latency_min = latency_min
        self._latency_mean = latency_mean

    async def open(self) -> Channel:
        self._server = GrpcServer(self._services)
        self._server._server = SimulatedServer()
        self._server._server_closed_fut = self._server._loop.create_future()
        self._server_protocol = self._server._protocol_factory()

        self._channel = Channel()
        self._channel._protocol = self._channel._protocol_factory()

        self._server_transport = SimulatedTransport(
            protocol=self._server_protocol,
            oracle=self._oracle,
            latency_min=self._latency_min,
            latency_mean=self._latency_mean,
        )
        self._client_transport = SimulatedTransport(
            protocol=self._channel._protocol,
            oracle=self._oracle,
            latency_min=self._latency_min,
            latency_mean=self._latency_mean,
        )
        self._channel._protocol.connection_made(self._server_transport)
        self._server_protocol.connection_made(self._client_transport)
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

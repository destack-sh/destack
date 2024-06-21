import asyncio
from typing import Collection

import structlog
from grpclib._typing import IServable
from grpclib.client import Channel
from grpclib.protocol import H2Protocol
from grpclib.server import Server as GrpcServer
from opentelemetry import trace

from bench.utils.oracle import Oracle

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


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
        assert latency_min >= 0, f"latency_min={latency_min} must be >= 0"
        assert latency_mean >= 0, f"latency_mean={latency_mean} must be >= 0"
        self._latency_min = latency_min
        self._latency_mean = latency_mean

    def _write_soon(self, data: bytes) -> None:
        if not self._protocol.connection.is_closing():
            self._protocol.data_received(data)

    def write(self, data: bytes) -> None:
        if data:
            if self._latency_mean == 0:
                latency = self._latency_min
            else:
                latency = (
                    self._oracle.random.expovariate(1 / self._latency_mean) + self._latency_min
                )
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
        latency_min: float = 0.0,
        latency_mean: float = 0.0,
    ) -> None:
        self._services = services
        self._oracle = oracle
        self._latency_min = latency_min
        self._latency_mean = latency_mean
        self._server: GrpcServer | None = None
        self._server_protocol: H2Protocol | None = None
        self._server_transport: SimulatedTransport | None = None
        self._client_transport: SimulatedTransport | None = None
        self._channel: Channel | None = None

    @property
    def channel(self) -> Channel:
        assert self._channel is not None, f"{self!r} not opened yet"
        return self._channel

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
        if self._channel:
            if self._channel._protocol:
                self._channel._protocol.connection_lost(None)
            self._channel.close()
            self._channel = None
        if self._server:
            if self._server_protocol:
                self._server_protocol.connection_lost(None)
            self._server.close()

    async def wait_closed(self):
        if self._server is not None:
            await self._server.wait_closed()
            self._server = None

    async def __aenter__(self) -> Channel:
        return await self.open()

    async def __aexit__(self, exc_type, exc_value, traceback):
        self.close()
        await self.wait_closed()

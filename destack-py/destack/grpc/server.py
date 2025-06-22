import asyncio
from collections.abc import Collection
from typing import cast

import grpclib.client
import grpclib.server
import structlog
from grpclib._typing import IServable
from opentelemetry import trace

from destack.grpc.health import HealthService
from destack.grpc.network import Network
from destack.grpc.service import ServiceBase
from destack.language import Oracle
from destack.utils.telemetry import export_now

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class GrpcServer(grpclib.server.Server):
    """gRPC server with extra bells and whistles."""

    def __init__(self, handlers: Collection["IServable"], network: Network, oracle: Oracle):
        assert all(isinstance(h, ServiceBase) for h in handlers), (
            f"unexpected handlers: {handlers!r}"
        )
        self._services: tuple[ServiceBase, ...] = cast(tuple[ServiceBase, ...], tuple(handlers))
        self._health_service = HealthService(
            id="health",
            services=self._services,
            network=network,
            oracle=oracle,
            on_error=None,
        )
        super().__init__((*handlers, self._health_service))
        self._host: str | None = None
        self._port: int | None = None

    def __str__(self):
        return f"services={self._services}, host={self._host}, port={self._port}"

    def __repr__(self):
        return f"<DestackServer {self}>"

    async def start(self, host: str | None = None, port: int | None = None, **kwargs) -> None:
        self._host = host
        self._port = port
        await asyncio.gather(*(h.start() for h in self._services))
        logger.info("server.start", server=self)
        await super().start(host=host, port=port, **kwargs)  # wait until closed

    def close(self) -> None:
        for task in self._services:
            task.stop()
        super().close()
        export_now()
        logger.debug("server.close", server=self)

    async def wait_closed(self) -> None:
        await super().wait_closed()
        await asyncio.gather(*(h.wait_stopped() for h in self._services))
        logger.debug("server.wait_closed", server=self)

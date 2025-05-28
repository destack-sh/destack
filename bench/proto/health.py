from typing import (
    Callable,
    Collection,
    Mapping,
    override,
)

import structlog
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from opentelemetry import trace

from bench.pb2 import HealthBase, HealthCheckRequest, HealthCheckResponse, ServiceKind
from bench.utils.oracle import Oracle

from .network import Network
from .service import ServiceBase

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class HealthService(ServiceBase, HealthBase):
    """Health check service."""

    kind = ServiceKind.PUBLIC  # :ServiceKind
    name = "health"

    def __init__(
        self,
        id: str,
        services: Collection[ServiceBase],
        network: Network,
        oracle: Oracle,
        on_error: Callable[[BaseException], None] | None,
    ):
        super().__init__(
            id=id,
            logger=logger,
            tracer=tracer,
            network=network,
            oracle=oracle,
            on_error=on_error,
        )
        self._services = services

    @override
    async def check(self, request: HealthCheckRequest, headers: Mapping) -> HealthCheckResponse:
        # NOTE :Robustness :Monitoring: check health properly
        response = HealthCheckResponse(status=HealthCheckResponse.ServingStatus.SERVING)
        logger.trace("health.check", service=self, span="current")
        return response

    @override
    async def watch(self, request: HealthCheckRequest, headers: Mapping) -> HealthCheckResponse:
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)

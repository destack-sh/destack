import asyncio
import contextvars
import functools
from typing import Collection, TYPE_CHECKING, Mapping, final, Callable, TypeVar

from betterproto import ServiceStub
from grpclib import GRPCError, Status as GRPCStatus
from grpclib._typing import IServable
import grpclib.server
import structlog

from bench.proto.wire import RpcMetadata
from bench.server.utils import parse_metadata
from bench.utils.casing import to_casing, Casing
from bench.utils.monitoring import Monitored

logger = structlog.get_logger(__name__)

ServiceStubT = TypeVar("ServiceStubT", bound=ServiceStub)


class BenchServiceBase(IServable if TYPE_CHECKING else object):
    """gRPC service with some extra stuff for custom loops, auth, logging, metadata, ..."""

    def __init__(self):
        self._stream: contextvars.ContextVar[grpclib.server.Stream] = contextvars.ContextVar(
            "stream"
        )
        self._metadata: contextvars.ContextVar[RpcMetadata] = contextvars.ContextVar("metadata")
        self._log = structlog.get_logger(self.__class__.__name__)

    @property
    def stream(self) -> grpclib.server.Stream:
        """The current gRPC request stream."""
        return self._stream.get()

    @property
    def metadata(self) -> RpcMetadata:
        """The received metadata in the current gRPC request stream."""
        return self._metadata.get()

    async def start_quick(self) -> None:
        """Start the service. Should be ready for service when returning."""
        raise NotImplementedError

    def close(self) -> None:
        """Close the service.."""
        raise NotImplementedError

    async def wait_closed(self) -> None:
        """Wait for the service to be fully closed."""
        raise NotImplementedError

    def __mapping__(self) -> Mapping[str, grpclib.const.Handler]:
        patched_mapping = {}
        for method, handler in super().__mapping__().items():
            patched_mapping[method] = self._wrap_rpc(method, handler)
        return patched_mapping

    def _wrap_rpc_func(
        self, func: Callable, method_name: str, handler: grpclib.const.Handler
    ) -> Callable:
        return func

    @final
    def _wrap_rpc(self, method: str, handler: grpclib.const.Handler) -> grpclib.const.Handler:
        func, cardinality, request_type, reply_type = handler
        service_slug = to_casing(self.__class__.__name__, Casing.SNAKE)
        method_slug = to_casing(method.split("/")[-1], Casing.SNAKE)
        rpc_name = f"{service_slug}.{method_slug}"
        func = self._wrap_rpc_func(func, method_slug, handler)

        @functools.wraps(func)
        async def wrapped_method(stream: grpclib.server.Stream) -> None:
            start = asyncio.get_running_loop().time()
            try:
                self._stream.set(stream)
                metadata = parse_metadata(stream.metadata)
                self._metadata.set(metadata)
                logger.info(rpc_name, service=self, method=method, metadata=metadata)
                await func(stream)
                duration = asyncio.get_running_loop().time() - start
                logger.info(f"{rpc_name}.done", service=self, method=method, duration=duration)
            except GRPCError as e:
                duration = asyncio.get_running_loop().time() - start
                logger.error(
                    f"{rpc_name}.error", service=self, method=method, duration=duration, error=e
                )
                raise  # pass through
            except Exception as e:
                # any remaining errors are internal server errors
                duration = asyncio.get_running_loop().time() - start
                logger.exception(
                    f"{rpc_name}.internal_error",
                    service=self,
                    method=method,
                    duration=duration,
                    error=e,
                )
                raise GRPCError(GRPCStatus.INTERNAL, str(e)) from e

        return grpclib.const.Handler(wrapped_method, cardinality, request_type, reply_type)


class MonitoredServiceBase(BenchServiceBase, Monitored):
    pass


class BenchServer(grpclib.server.Server):
    """gRPC server with extra bells and whistles."""

    @functools.wraps(grpclib.server.Server.__init__)
    def __init__(self, handlers: Collection["IServable"], **kwargs):
        super().__init__(handlers, **kwargs)
        self._custom_handlers: tuple[BenchServiceBase, ...] = tuple(
            h for h in handlers if isinstance(h, BenchServiceBase)
        )
        self._host: str | None = None
        self._port: int | None = None

    def __str__(self):
        return f"services={self._custom_handlers}, host={self._host}, port={self._port}"

    def __repr__(self):
        return f"<BenchServer {self}>"

    @functools.wraps(grpclib.server.Server.start)
    async def start(self, host: str = None, port: int = None, **kwargs) -> None:
        self._host = host
        self._port = port
        logger.info("server.start", server=self)
        await asyncio.gather(*(h.start_quick() for h in self._custom_handlers))
        await super().start(host=host, port=port, **kwargs)
        logger.info("server.start.done", server=self)

    def close(self) -> None:
        logger.info("server.close", server=self)
        for task in self._custom_handlers:
            task.close()
        super().close()

    @functools.wraps(grpclib.server.Server.wait_closed)
    async def wait_closed(self) -> None:
        await super().wait_closed()
        await asyncio.gather(*(h.wait_closed() for h in self._custom_handlers))
        logger.info("server.closed", server=self)

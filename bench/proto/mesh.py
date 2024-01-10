import asyncio
import contextvars
import functools
from typing import Collection, TYPE_CHECKING, Mapping, final, Callable, TypeVar, cast

from betterproto import ServiceStub
from betterproto.grpc.grpclib_server import ServiceBase
from grpclib import GRPCError, Status as GRPCStatus
from grpclib._typing import IServable
import grpclib.server
from more_itertools import first
import structlog

from bench.proto.wire import RpcMetadata
from bench.server.utils import parse_metadata
from bench.utils.monitoring import Monitored
from bench.utils.utils import to_snake_case

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
        """The metadata of the current gRPC call."""
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
        service_slug = to_snake_case(self.__class__.__name__)
        method_slug = to_snake_case(method.split("/")[-1])
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

    def to_loopback_stub(self) -> "ServiceStubT":
        """Create a stub that calls this service directly."""

        proxied_service: BenchServiceBase = self
        proxied_base_type = first(
            cls for cls in type(proxied_service).__mro__ if issubclass(cls, ServiceBase)
        )

        # make a new class that inherits from the stub class
        class _LoopbackStub:
            def __str__(self):
                return f"local loop to {proxied_service}"

            def __repr__(self):
                return f"<{proxied_service.__name__}Loopback {self}>"

        # implement the 'stub' methods with a loopback call ...
        def _loopback_stub_method(method_name: str):
            method = getattr(type(proxied_service), method_name)

            @functools.wraps(method)
            async def loopback_method(self, *args, **kwargs):
                return await getattr(proxied_service, method_name)(*args, **kwargs)

            return loopback_method

        # ... for every regular method in the proxied service
        for method_name in dir(proxied_base_type):
            if not method_name.startswith("_") and callable(
                getattr(proxied_base_type, method_name)
            ):
                setattr(_LoopbackStub, method_name, _loopback_stub_method(method_name))

        return cast(ServiceStubT, _LoopbackStub())


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

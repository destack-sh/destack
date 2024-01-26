import asyncio
import contextvars
import functools
from typing import TYPE_CHECKING, Callable, Collection, Mapping, TypeVar, final, Generic

import grpclib.server
from grpclib.testing import ChannelFor
import structlog
from betterproto import ServiceStub
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from grpclib._typing import IServable

from bench.language.auth import AccessError
from bench.language.const import BenchError
from bench.language.link import NoNodeFoundError
from bench.proto.wire import RpcMetadata
from bench.server.utils import parse_metadata
from bench.utils.casing import Casing, to_casing
from bench.utils.monitoring import Monitored
from bench.utils.utils import DEBUG, TEST, sentry_capture

ServiceStubT = TypeVar("ServiceStubT", bound=ServiceStub)

logger = structlog.get_logger(__name__)
StubT = TypeVar("StubT", bound=ServiceStub)


class BenchServiceBase((IServable, Generic[StubT]) if TYPE_CHECKING else Generic[StubT]):
    """gRPC service with some extra stuff for custom loops, auth, logging, metadata, ..."""

    def __init__(self, loopback_stub_to: type[StubT] | None = None):
        self._stream: contextvars.ContextVar[grpclib.server.Stream] = contextvars.ContextVar(
            "stream"
        )
        self._metadata: contextvars.ContextVar[RpcMetadata] = contextvars.ContextVar("metadata")
        self._loopback_stub: type[StubT] | None = None
        self._needs_loopback_stub = loopback_stub_to

    @property
    def stream(self) -> grpclib.server.Stream:
        """The current gRPC request stream."""
        return self._stream.get()

    @property
    def metadata(self) -> RpcMetadata:
        """The received metadata in the current gRPC request stream."""
        return self._metadata.get()

    @property
    def loopback(self) -> StubT:
        if self._loopback_stub is not None:
            return self._loopback_stub
        elif self._needs_loopback_stub is None:
            raise RuntimeError(f"loopback stub not configured for {self!r}")
        else:
            raise RuntimeError(f"loopback stub not ready for {self!r}")

    async def start_quick(self) -> None:
        """Start the service. Should be ready for service when returning."""
        if self._needs_loopback_stub:
            # create a loopback like the one used for testing
            channel = ChannelFor([self])
            await channel.__aenter__()
            self._loopback_stub = self._loopback_stub(channel)

    def close(self) -> None:
        """Close the service.."""
        pass

    async def wait_closed(self) -> None:
        """Wait for the service to be fully closed."""
        if self._needs_loopback_stub:
            await self.loopback.channel.__aexit__(None, None, None)

    def __mapping__(self) -> Mapping[str, grpclib.const.Handler]:
        # combine mappings from non-overlapping superclasses
        patched_mapping = {}
        for cls in self.__class__.__bases__:
            if cls is BenchServiceBase:
                continue
            for method, handler in cls.__mapping__(self).items():
                patched_mapping[method] = self._wrap_rpc(method, handler)
        assert len(patched_mapping) > 0, f"no RPCs found in {self!r}"
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
            """Managed RPC call with some instrumentation and error handling."""

            start = asyncio.get_running_loop().time()
            log = logger.bind(service=self, method=method)
            try:
                self._stream.set(stream)
                metadata = parse_metadata(stream.metadata)
                self._metadata.set(metadata)
                log.info(rpc_name, metadata=metadata)
                await func(stream)
                duration = asyncio.get_running_loop().time() - start
                log.info(f"{rpc_name}.done", duration=duration)
            except BenchError as e:  # wrap error
                duration = asyncio.get_running_loop().time() - start
                log.exception(f"{rpc_name}.error", duration=duration, error=e)
                status_map = {
                    NoNodeFoundError: GRPCStatus.NOT_FOUND,
                    AccessError: GRPCStatus.UNAUTHENTICATED,
                }
                status = status_map.get(e.__class__, GRPCStatus.INVALID_ARGUMENT)
                raise GRPCError(status, str(e)) from e
            except GRPCError as e:  # pass through GRPC errors
                duration = asyncio.get_running_loop().time() - start
                sentry_capture(e)
                log.exception(f"{rpc_name}.error", duration=duration, error=e)
                raise
            except Exception as e:  # internal error
                duration = asyncio.get_running_loop().time() - start
                sentry_capture(e)
                log.exception(f"{rpc_name}.internal_error", duration=duration, error=e)
                if DEBUG or TEST:
                    details = f"{e.__class__.__name__}: {e}"
                else:
                    details = e.__class__.__name__
                raise GRPCError(GRPCStatus.INTERNAL, details) from e

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
        self._logger = structlog.get_logger(self.__class__.__name__)

    def __str__(self):
        return f"services={self._custom_handlers}, host={self._host}, port={self._port}"

    def __repr__(self):
        return f"<BenchServer {self}>"

    @functools.wraps(grpclib.server.Server.start)
    async def start(self, host: str = None, port: int = None, **kwargs) -> None:
        self._host = host
        self._port = port
        self._logger.info("server.start", server=self)
        await asyncio.gather(*(h.start_quick() for h in self._custom_handlers))
        await super().start(host=host, port=port, **kwargs)
        self._logger.info("server.start.done", server=self)

    def close(self) -> None:
        self._logger.info("server.close", server=self)
        for task in self._custom_handlers:
            task.close()
        super().close()

    @functools.wraps(grpclib.server.Server.wait_closed)
    async def wait_closed(self) -> None:
        await super().wait_closed()
        await asyncio.gather(*(h.wait_closed() for h in self._custom_handlers))
        self._logger.info("server.closed", server=self)

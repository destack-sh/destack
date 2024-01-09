import asyncio
import contextvars
import functools
from typing import Collection, TYPE_CHECKING, Mapping

from grpclib._typing import IServable
import grpclib.server
from multidict import MultiDict
import structlog

from bench.proto.wire import RpcMetadata
from bench.utils.monitoring import Monitored

logger = structlog.get_logger(__name__)


def parse_metadata(metadata: MultiDict) -> RpcMetadata:
    raise NotImplementedError("nocheckin: parse_metadata")


class BenchServiceBase(IServable if TYPE_CHECKING else object):
    """gRPC service with some extra stuff for custom loops, auth, metadata, ..."""

    def __init__(self):
        self._stream: contextvars.ContextVar[grpclib.server.Stream] = contextvars.ContextVar(
            "stream"
        )
        self._metadata: contextvars.ContextVar[RpcMetadata] = contextvars.ContextVar("metadata")

    @property
    def stream(self) -> grpclib.server.Stream:
        return self._stream.get()

    @property
    def metadata(self):
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
            func, cardinality, request_type, reply_type = handler

            @functools.wraps(func)
            async def wrapped_method(stream: grpclib.server.Stream) -> None:
                self._stream.set(stream)
                self._metadata.set(parse_metadata(stream.metadata))
                await func(stream)

            patched_mapping[method] = (wrapped_method, cardinality, request_type, reply_type)
        return patched_mapping


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

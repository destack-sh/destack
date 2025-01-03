import abc
import asyncio
import functools
from typing import (
    Any,
    AsyncIterable,
    AsyncIterator,
    Callable,
    ClassVar,
    Collection,
    Mapping,
    Optional,
    TypeVar,
    Union,
    cast,
    final,
    override,
)
from urllib.parse import urlparse
from uuid import UUID

import cachetools
import grpclib.client
import grpclib.server
import structlog
from google.protobuf.message import Message as ProtoMessage
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from grpclib._typing import IServable
from grpclib.client import Channel, ServiceMethod
from opentelemetry import trace

from bench.language import (
    AccessError,
    BenchError,
    ClientType,
    NodeNotFoundError,
    Subject,
    ValidationError,
)
from bench.proto import wire
from bench.proto.wire import (
    HealthBase,
    HealthCheckRequest,
    HealthCheckResponse,
    RpcMetadata,
    ServiceKind,
)
from bench.proto.wiring import pack_rpc_headers
from bench.utils.env import IS_DEV, IS_TEST
from bench.utils.oracle import Oracle
from bench.utils.string import Casing, to_casing
from bench.utils.task import TaskManager
from bench.utils.telemetry import (
    attach_propagation_context,
    collect_propagation_context,
    export_now,
    set_baggage,
)

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

UnaryRpcCallable = Callable[[Subject, ProtoMessage], ProtoMessage]
StreamRpcCallable = Callable[[Subject, ProtoMessage], AsyncIterable[ProtoMessage]]
RpcCallable = Union[UnaryRpcCallable, StreamRpcCallable]

ServiceStubT = TypeVar("ServiceStubT")
StubT = TypeVar("StubT")

GRPC_STATUS_BY_BENCH_ERROR_CLASS: Mapping[type, GRPCStatus] = {
    NotImplementedError: GRPCStatus.UNIMPLEMENTED,
    NodeNotFoundError: GRPCStatus.NOT_FOUND,
    AccessError: GRPCStatus.PERMISSION_DENIED,
    ValidationError: GRPCStatus.INVALID_ARGUMENT,
}


def get_grpc_status_from_bench_error(e: BenchError) -> GRPCStatus:
    status = GRPC_STATUS_BY_BENCH_ERROR_CLASS.get(e.__class__, GRPCStatus.INVALID_ARGUMENT)
    return status


class ServiceBase(abc.ABC):
    """gRPC service with some extra stuff for custom loops, auth, logging, metadata, ..."""

    kind: ClassVar[ServiceKind]

    def __init__(self, *, logger: Any, tracer: trace.Tracer, oracle: Oracle):
        self.logger = logger
        self.tracer = tracer
        self._oracle = oracle
        self.tasks = TaskManager(owner=self, logger=logger, oracle=oracle)

    def __str__(self) -> str:
        return ""

    @final
    def __repr__(self) -> str:
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str}>"
        else:
            return f"<{self.__class__.__name__}>"

    @property
    def oracle(self) -> Oracle:
        # wrap as property to comply with HostSpec
        return self._oracle

    def get_service_baggage(self) -> dict[str, Any]:
        return {}

    async def start(self) -> None:  # noqa: B027
        """Start the service. Should be ready for service when returning."""
        pass

    def close(self) -> None:
        """Close the service.."""
        self.tasks.close()

    async def wait_closed(self) -> None:
        """Wait for the service to be fully closed."""
        await self.tasks.wait_closed()

    def __mapping__(self) -> Mapping[str, grpclib.const.Handler]:
        # combine mappings from non-overlapping superclasses
        patched_mapping = {}
        for cls in self.__class__.__bases__:
            if cls.__mapping__ == ServiceBase.__mapping__:  # type: ignore
                continue
            for method, handler in cls.__mapping__(self).items():  # type: ignore
                patched_mapping[method] = self._wrap_rpc(method, handler)
        assert len(patched_mapping) > 0, f"no RPCs found in {self!r}"
        return patched_mapping

    def _validate_request(self, request: ProtoMessage) -> None:  # noqa: B027
        """Validate a request message."""
        pass

    async def get_request[ReqT: ProtoMessage](
        self, stream: grpclib.server.Stream[ReqT, Any]
    ) -> ReqT:
        """Gets the request from the given stream."""
        request = await stream.recv_message()
        if request is None:
            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing request")
        self._validate_request(request)
        return request

    async def send_unary_response[RespT: ProtoMessage](
        self, stream: grpclib.server.Stream[Any, RespT], response: RespT
    ) -> None:
        """Sends the response to the given stream."""
        await stream.send_message(response)

    async def send_stream_response[RespT: ProtoMessage](
        self, stream: grpclib.server.Stream[Any, RespT], response: RespT
    ) -> None:
        """Sends the response to the given stream."""
        await stream.send_message(response)

    async def get_request_subject(self, request: ProtoMessage, metadata: RpcMetadata) -> Subject:
        return Subject(is_authenticated=False)

    def _wrap_rpc_func(
        self, func: RpcCallable, method_name: str, handler: grpclib.const.Handler
    ) -> Callable:
        return func

    @final
    def _wrap_rpc(self, method: str, handler: grpclib.const.Handler) -> grpclib.const.Handler:
        method = method[1:]  # skip initial slash
        _, cardinality, request_type, reply_type = handler
        service_slug = to_casing(self.__class__.__name__, Casing.SNAKE)
        method_slug = to_casing(method.split("/")[-1], Casing.SNAKE)
        rpc_name = f"{service_slug}.{method_slug}"
        func = self.__getattribute__(method_slug)  # bypass the generated __rpc wrapper methods
        func = self._wrap_rpc_func(func, method_slug, handler)  # custom wrap per service

        @functools.wraps(func)
        async def _managed_rpc(stream: grpclib.server.Stream) -> None:
            """Managed RPC call with some instrumentation and error handling."""

            # context
            log = self.logger.bind(service=self, method=method)
            set_baggage(service=service_slug)
            attach_propagation_context(cast(Mapping, stream.metadata or {}))

            with tracer.start_as_current_span(rpc_name) as span:
                try:
                    set_baggage(**self.get_service_baggage())
                    request = await stream.recv_message()
                    if request is None:
                        raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing request")
                    self._validate_request(request)
                    if handler.cardinality == grpclib.const.Cardinality.UNARY_UNARY:
                        response = await func(request, stream.metadata)
                        await stream.send_message(response)
                    elif handler.cardinality == grpclib.const.Cardinality.UNARY_STREAM:
                        span.end()  # end early (streaming, span shouldn't continue forever)
                        async for response in func(request, stream.metadata):
                            log.trace(f"{rpc_name}.update")
                            await stream.send_message(response)
                    else:
                        raise RuntimeError(f"unsuported cardinality {handler.cardinality}")
                    log.info(rpc_name, span="current")
                except GRPCError as e:
                    # pass through GRPC errors
                    log.info(f"{rpc_name}.error", exc_info=e, span="current")
                    raise
                except BenchError as e:
                    # wrap error
                    log.info(f"{rpc_name}.error", exc_info=e, span="current")
                    status = get_grpc_status_from_bench_error(e)
                    raise GRPCError(status, str(e)) from e
                except Exception as e:
                    # internal error
                    log.error(f"{rpc_name}.internal_error", exc_info=e, span="current")
                    if IS_DEV or IS_TEST:
                        details = f"{e.__class__.__name__}: {e}"
                    else:
                        details = e.__class__.__name__
                    raise GRPCError(GRPCStatus.INTERNAL, details) from e

        return grpclib.const.Handler(_managed_rpc, cardinality, request_type, reply_type)


class HealthService(ServiceBase, HealthBase):
    """Health check service."""

    def __init__(self, services: Collection[ServiceBase], oracle: Oracle):
        super().__init__(logger=logger, tracer=tracer, oracle=oracle)
        self._services = services

    @override
    async def check(self, request: HealthCheckRequest, headers: Mapping) -> HealthCheckResponse:
        # NOTE :Robustness :Monitoring: check health properly
        response = HealthCheckResponse(status=HealthCheckResponse.ServingStatus.SERVING)
        logger.trace("health.check", service=self, span="current")
        return response

    @override
    async def watch(
        self, request: HealthCheckRequest, headers: Mapping
    ) -> AsyncIterator[HealthCheckResponse]:
        raise GRPCError(GRPCStatus.UNIMPLEMENTED)
        yield HealthCheckResponse()


class GrpcServer(grpclib.server.Server):
    """gRPC server with extra bells and whistles."""

    def __init__(self, handlers: Collection["IServable"], oracle: Oracle, **kwargs):
        assert all(
            isinstance(h, ServiceBase) for h in handlers
        ), f"unexpected handlers: {handlers!r}"
        self._services: tuple[ServiceBase, ...] = cast(tuple[ServiceBase, ...], tuple(handlers))
        self._health_service = HealthService(self._services, oracle)
        super().__init__((*handlers, self._health_service), **kwargs)
        self._host: str | None = None
        self._port: int | None = None

    def __str__(self):
        return f"services={self._services}, host={self._host}, port={self._port}"

    def __repr__(self):
        return f"<BenchServer {self}>"

    async def start(self, host: str | None = None, port: int | None = None, **kwargs) -> None:
        self._host = host
        self._port = port
        await asyncio.gather(*(h.start() for h in self._services))
        logger.info("server.start", server=self)
        await super().start(host=host, port=port, **kwargs)

    def close(self) -> None:
        for task in self._services:
            task.close()
        super().close()
        export_now()
        logger.debug("server.close", server=self)

    async def wait_closed(self) -> None:
        await super().wait_closed()
        await asyncio.gather(*(h.wait_closed() for h in self._services))
        logger.debug("server.wait_closed", server=self)


@cachetools.cached(
    cachetools.TTLCache(maxsize=128, ttl=300), key=lambda connection_uri: connection_uri
)
def get_channel(connection_uri: str):
    connection_info = urlparse(connection_uri)
    assert isinstance(connection_info.netloc, str), f"invalid connection uri: {connection_uri}"
    assert connection_info.port is not None, f"invalid connection uri: {connection_uri}"
    netloc = connection_info.netloc.split(":", 1)[0]
    channel = Channel(host=netloc, port=connection_info.port, ssl=connection_info.scheme == "https")
    return channel


def get_rpc_metadata(
    *,
    client_type: ClientType,
    client_id: str | UUID,
    client_access_token: str | UUID,
    client_nonce: str | UUID | None = None,
):
    """Gets the gRPC metadata for a client."""
    rpc_metadata = RpcMetadata(
        client_type=cast(wire.ClientType, client_type),
        client_id=str(client_id),
        client_nonce=str(client_nonce) if client_nonce is not None else None,
        client_access_token=str(client_access_token),
    )
    return rpc_metadata


def get_rpc_headers(
    *,
    client_type: ClientType,
    client_id: str | UUID,
    client_access_token: str | UUID,
    client_nonce: str | UUID | None = None,
):
    """Gets the gRPC headers for a client."""
    rpc_metadata = get_rpc_metadata(
        client_type=client_type,
        client_id=client_id,
        client_access_token=client_access_token,
        client_nonce=client_nonce,
    )
    rpc_headers = pack_rpc_headers(rpc_metadata)
    return rpc_headers


async def unary_stream_rpc[ReqT, RepT](
    method: grpclib.client.UnaryStreamMethod[ReqT, RepT],
    request: ReqT,
    *,
    timeout: Optional[float] = None,
) -> AsyncIterator[RepT]:
    async with method.open(timeout=timeout, metadata=collect_propagation_context()) as stream:
        await stream.send_message(request, end=True)
        async for response in stream:
            yield response


# monkey-patch grpclib clients to inject propagation context

_Value = Union[str, bytes]
_MetadataLike = Union[Mapping[str, _Value], Collection[tuple[str, _Value]]]


class _PatchedServiceMethod(ServiceMethod):
    def open(
        self,
        *,
        timeout: Optional[float] = None,
        metadata: Optional[_MetadataLike] = None,
    ):
        # inject propagation context
        if metadata is None:
            metadata = {}
        elif isinstance(metadata, Mapping):
            metadata = {**metadata}  # type: ignore
        else:
            raise TypeError(f"unexpected metadata type: {metadata}")
        metadata.update(collect_propagation_context())

        # open channel (see ServiceMethod.open)
        return self.channel.request(
            self.name,
            self._cardinality,
            self.request_type,
            self.reply_type,
            timeout=timeout,
            metadata=metadata,
        )


ServiceMethod.open = _PatchedServiceMethod.open  # type: ignore

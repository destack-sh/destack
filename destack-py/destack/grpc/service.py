import abc
import functools
from collections.abc import AsyncIterable, Collection, Mapping
from typing import (
    TYPE_CHECKING,
    Any,
    Callable,
    ClassVar,
    Optional,
    TypeVar,
    Union,
    cast,
    final,
)

import grpclib.client
import grpclib.server
import structlog
from google.protobuf.message import Message as ProtoMessage
from grpclib import GRPCError
from grpclib import Status as GRPCStatus
from grpclib.client import ServiceMethod
from opentelemetry import trace

from destack.encoder.proto.wiring import unpack_rpc_headers
from destack.language import EMPTY_DICT, Client, DestackError, IsActor, Oracle, Session
from destack.proto import RpcMetadata, ServiceKind
from destack.utils.env import IS_DEV, IS_TEST
from destack.utils.string import Casing, to_casing
from destack.utils.task import TaskManager
from destack.utils.telemetry import (
    attach_propagation_context,
    collect_propagation_context,
    set_baggage,
)

if TYPE_CHECKING:
    from .network import Network

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

UnaryRpcCallable = Callable[
    [ProtoMessage, Session, IsActor | None, Client | None, RpcMetadata], ProtoMessage
]
StreamRpcCallable = Callable[
    [ProtoMessage, Session, IsActor | None, Client | None, RpcMetadata],
    AsyncIterable[ProtoMessage],
]
RpcCallable = Union[UnaryRpcCallable, StreamRpcCallable]

ServiceStubT = TypeVar("ServiceStubT")
StubT = TypeVar("StubT")


def get_grpc_status_from_destack_error(e: DestackError) -> GRPCStatus:
    raise NotImplementedError


class ServiceBase(abc.ABC):
    """gRPC service with some extra stuff for custom loops, auth, logging, metadata, ..."""

    kind: ClassVar[ServiceKind]
    name: ClassVar[str]

    def __init__(
        self,
        *,
        id: str,
        logger: Any,
        tracer: trace.Tracer,
        network: "Network",
        oracle: Oracle,
        on_error: Callable[[BaseException], None] | None,
    ):
        self.id = id
        self.logger = logger.bind(service=self)
        self.tracer = tracer
        self.network = network
        self.tasks = TaskManager(owner=self, logger=logger, on_error=on_error)
        self.oracle = oracle
        self.active_unary_requests_count = 0
        self._on_error = on_error

    def __str__(self) -> str:
        return ""

    @final
    def __repr__(self) -> str:
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str}>"
        else:
            return f"<{self.__class__.__name__}>"

    def on_error(self, exc: BaseException) -> None:
        """Handle an error."""
        if self._on_error:
            self._on_error(exc)
        else:
            logger.debug(f"{self.name}.on_error", service=self, exc_info=exc)

    def get_service_baggage(self) -> dict[str, Any]:
        return {}

    @property
    def is_idle(self) -> bool:
        """Check if the service is idle (no pending requests or processing)."""
        # streaming requests are not considered 'active' since they're open until close
        return self.active_unary_requests_count <= 0

    async def start(self) -> None:  # noqa: B027
        """Start the service. Should be ready for service when returning."""
        pass

    def stop(self) -> None:
        """Stop the service."""
        self.tasks.close()
        logger.debug(f"{self.name}.stopping", service=self)

    async def wait_stopped(self) -> None:
        """Wait for the service to be fully closed."""
        await self.tasks.wait_closed()
        logger.info(f"{self.name}.stop", service=self)

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

    def _wrap_rpc_func(
        self, func: RpcCallable, method_name: str, handler: grpclib.const.Handler
    ) -> Callable:
        return func

    async def make_session(self, metadata: RpcMetadata) -> Session:
        raise NotImplementedError

    async def resolve_client(
        self, request: ProtoMessage, metadata: RpcMetadata
    ) -> tuple[IsActor | None, Client | None]:
        return None, None

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
                    metadata = unpack_rpc_headers(stream.metadata or EMPTY_DICT)
                    session = await self.make_session(metadata)
                    async with session:
                        request = await stream.recv_message()
                        if request is None:
                            raise GRPCError(GRPCStatus.INVALID_ARGUMENT, "missing request")
                        actor, client = await self.resolve_client(request, metadata)

                        if handler.cardinality == grpclib.const.Cardinality.UNARY_UNARY:
                            self.active_unary_requests_count += 1
                            response = await func(request, session, actor, client, metadata)
                            await stream.send_message(response)
                        elif handler.cardinality == grpclib.const.Cardinality.UNARY_STREAM:
                            span.end()  # end early (streaming, span shouldn't continue forever?)
                            async for response in func(request, session, actor, client, metadata):
                                log.trace(f"{rpc_name}.update")
                                await stream.send_message(response)
                        else:
                            raise RuntimeError(f"unsuported cardinality {handler.cardinality}")
                    log.trace(rpc_name, span="current")
                except GRPCError as e:
                    # pass through GRPC errors
                    log.error(f"{rpc_name}.error", exc_info=e, span="current")
                    self.on_error(e)
                    raise
                except DestackError as e:
                    # wrap error
                    log.error(f"{rpc_name}.error", exc_info=e, span="current")
                    self.on_error(e)
                    status = get_grpc_status_from_destack_error(e)
                    raise GRPCError(status, str(e)) from e
                except Exception as e:
                    # internal error
                    log.error(f"{rpc_name}.internal_error", exc_info=e, span="current")
                    if IS_DEV or IS_TEST:
                        details = f"{e.__class__.__name__}: {e}"
                    else:
                        details = e.__class__.__name__
                    raise GRPCError(GRPCStatus.INTERNAL, details) from e
                finally:
                    if handler.cardinality == grpclib.const.Cardinality.UNARY_UNARY:
                        self.active_unary_requests_count -= 1

        return grpclib.const.Handler(_managed_rpc, cardinality, request_type, reply_type)


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
del _PatchedServiceMethod

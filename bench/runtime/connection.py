import abc
import asyncio
import contextlib
from typing import Any, cast, override

import structlog
from opentelemetry import trace

from bench.language.bench import Bench, Package
from bench.language.const import get_active_session
from bench.language.node import Node
from bench.language.query import QueryBuilder
from bench.language.session import Session
from bench.language.transaction import edit_graph
from bench.proto.monkey import _PatchedRpcMetadata
from bench.proto.wire import (
    AnyNodeData,
    BenchData,
    GraphIoStub,
    GraphScope,
    HostStub,
    PackageData,
    RpcMetadata,
    SupervisorStub,
    WatchGetRequest,
)
from bench.utils.tenacity import RETRY_GRPC, RetryOptions

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class QueryConnector(abc.ABC):
    """A factory for connected queries."""

    @abc.abstractmethod
    async def get[NodeT: Node, NodeDataT: AnyNodeData](
        self,
        query: QueryBuilder[NodeT, NodeDataT],
        tx_lock: asyncio.Lock,
        session: Session,
        owner: Any,
    ) -> "Connection[NodeT, NodeDataT, Any]":
        """Create a connected query."""
        ...


class Connection[NodeT: Node, NodeDataT: AnyNodeData, ResultT](abc.ABC):
    """A live query result from a graph connection."""

    def __init__(
        self,
        *,
        query: QueryBuilder[NodeT, NodeDataT],
        tx_lock: asyncio.Lock,
        session: Session,
        owner: Any,
    ):
        self._query = query
        self._tx_lock = tx_lock
        self._session = session
        self._owner = owner
        self._result: ResultT | None = None

    def __str__(self):
        if self.has_result:
            return f"{self._query} -> {self._result}"
        else:
            return f"{self._query} -> <no result>"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def has_result(self) -> bool:
        """Whether the query has a current result."""
        return self._result is not None

    @property
    def result(self) -> ResultT:
        assert self._result is not None, f"{self!r} has no result"
        return self._result

    @property
    @abc.abstractmethod
    def epoch(self) -> int:
        """The current epoch of the query (if any)."""
        ...

    @abc.abstractmethod
    async def start(self) -> None:
        """Start the connection. Returns as soon as the connection is established (has a result)."""
        ...

    @abc.abstractmethod
    def close(self):
        """Stop the live connection. The result (if any) will remain."""
        ...

    @abc.abstractmethod
    async def wait_closed(self):
        """Wait for the connection to be fully closed."""
        ...


class RemoteGetConnection[NodeT: Node, NodeDataT: AnyNodeData](Connection[NodeT, NodeDataT, NodeT]):
    """Connect to a remote graph for a live get query."""

    def __init__(
        self,
        *,
        query: QueryBuilder[NodeT, NodeDataT],
        tx_lock: asyncio.Lock,
        session: Session,
        owner: Any,
        remote: GraphIoStub | HostStub | SupervisorStub,
        scope: GraphScope,
        rpc_metadata: RpcMetadata,
        retry: RetryOptions = RETRY_GRPC,
    ):
        super().__init__(query=query, tx_lock=tx_lock, session=session, owner=owner)
        self._remote = remote
        self._scope = scope
        self._has_result: asyncio.Event = asyncio.Event()
        self._epoch: int | None = None
        self._is_closed: bool = False
        self._is_paused: bool = False
        self._rpc_metadata = rpc_metadata
        self._retry = retry
        self._connect_task: asyncio.Task[None] | None = None

    @property
    def epoch(self) -> int:
        assert self._epoch is not None, f"{self!r} has no result"
        return self._epoch

    @tracer.start_as_current_span("connect.start")
    async def start(self) -> None:
        trace.get_current_span().set_attribute("query", repr(self._query))
        assert self._connect_task is None, "already connected"
        assert get_active_session() is self._session, f"must be in context of {self._session!r}"
        self._connect_task = asyncio.create_task(self._do_connect())
        await self._has_result.wait()

    async def _do_connect(self) -> None:
        """Runs the core connection loop forever (or until closed)."""
        retry = self._retry.new()
        log = logger.bind(query=self._query, owner=self._owner, retry=retry)

        while not self._is_closed:
            if not retry.should_retry:
                raise retry.to_error(operation=self._query)
            retry.on_attempt()
            try:
                # get initial result
                with tracer.start_as_current_span("connect.get"):
                    self._result = await self._query.get()
                    graph = self._result._graph
                    read_info = self._result._read_info
                    assert read_info is not None, f"no read info for {self!r}"
                    assert read_info.epoch is not None, f"no epoch for {self!r}"
                    assert read_info.connection_token is not None, f"no token for {self!r}"
                    self._epoch = read_info.epoch
                    self._has_result.set()
                    log.debug(
                        "connect.get", duration=retry.duration, node=self._result, span="current"
                    )

                # subscribe forever (until error)
                watch_req = WatchGetRequest(
                    scope=self._scope,
                    connection_token=read_info.connection_token,
                    since_epoch=read_info.epoch,
                )
                rpc_headers = cast(_PatchedRpcMetadata, self._rpc_metadata).to_headers()
                async for rep in self._remote.watch_get(watch_req, metadata=rpc_headers):
                    # apply edits (should filter these :ConnectionFilter)
                    log.trace("connect.update", node=self._result, epoch=rep.epoch)
                    async with self._tx_lock:
                        edit_graph(graph, rep.edits, options=self._query._options, untracked=True)
                        self._epoch = rep.epoch
            except self._retry.retry_on as e:
                log.error("connect.error", node=self._result, exc_info=e)
                retry.on_error(e)
                await asyncio.sleep(retry.interval)

    def close(self):
        self._is_closed = True
        if self._connect_task is not None:
            self._connect_task.cancel()

    async def wait_closed(self):
        if self._connect_task is not None:
            with contextlib.suppress(asyncio.CancelledError):
                await self._connect_task


class RemoteConnector(QueryConnector):
    """Connect queries to a remote graph with gRPC."""

    def __init__(
        self,
        remote: GraphIoStub | HostStub | SupervisorStub,
        scope: GraphScope,
        rpc_metadata: RpcMetadata,
    ):
        self._remote = remote
        self._scope = scope
        self._rpc_metadata = rpc_metadata

    @override
    async def get[NodeT: Node, NodeDataT: AnyNodeData](
        self,
        query: QueryBuilder[NodeT, NodeDataT],
        tx_lock: asyncio.Lock,
        session: Session,
        owner: Any,
    ) -> Connection[NodeT, NodeDataT, NodeT]:
        connected_query = RemoteGetConnection(
            query=query,
            tx_lock=tx_lock,
            session=session,
            owner=owner,
            remote=self._remote,
            scope=self._scope,
            rpc_metadata=self._rpc_metadata,
        )
        await connected_query.start()
        return connected_query

    # NOTE :Incomplete: live search/aggregate connections for runtime


ConnectedBench = Connection[Bench, BenchData, Bench]
ConnectedPackage = Connection[Package, PackageData, Package]

import abc
import asyncio
import contextlib
from typing import Any, cast, override

import structlog
from opentelemetry import trace

from bench.language.bench import Bench, Package
from bench.language.const import NodeType, get_active_session
from bench.language.node import Node
from bench.language.query import QueryBuilder
from bench.language.session import Session
from bench.language.transaction import edit_graph
from bench.proto import wire
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
    WatchEditsRequest,
)
from bench.utils.tenacity import RETRY_GRPC, RetryOptions

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class QueryConnector(abc.ABC):
    """A factory for connected queries."""

    @abc.abstractmethod
    async def connect[NodeT: Node, NodeDataT: AnyNodeData](
        self,
        query: QueryBuilder[NodeT, NodeDataT],
        tx_lock: asyncio.Lock,
        session: Session,
        owner: Any,
    ) -> "ConnectedQuery[NodeT, NodeDataT]":
        """Create a connected query."""
        ...


class ConnectedQuery[NodeT: Node, NodeDataT: AnyNodeData](abc.ABC):
    """A live query result."""

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
        self._node: NodeT | None = None

    def __str__(self):
        if self.has_result:
            return f"{self._query} -> {self._node}"
        else:
            return f"{self._query} -> <no result>"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    @abc.abstractmethod
    def has_result(self) -> bool:
        """Whether the query has a current result."""
        ...

    @property
    @abc.abstractmethod
    def node(self) -> NodeT:
        """The current result of the query (if any)."""
        ...

    @property
    @abc.abstractmethod
    def epoch(self) -> int:
        """The current epoch of the query (if any)."""
        ...

    def migrate(self, session: Session):
        """Migrate the query to a new session."""
        if self._node is not None:
            self._node._untrack_rec()
            self._node._track_rec(session)
        self._session = session

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


class RemoteQuery[NodeT: Node, NodeDataT: AnyNodeData](ConnectedQuery[NodeT, NodeDataT]):
    """Connect to a remote graph for a live query."""

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
    def has_result(self) -> bool:
        return self._node is not None

    @property
    def node(self) -> NodeT:
        assert self._node is not None, f"{self!r} has no result"
        return self._node

    @property
    def epoch(self) -> int:
        assert self._epoch is not None, f"{self!r} has no result"
        return self._epoch

    @tracer.start_as_current_span("query.start")
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
                with tracer.start_as_current_span("query.connect"):
                    self._node = await self._query.get()
                    graph = self._node._graph
                    assert (
                        self._node._read_info is not None
                        and self._node._read_info.epoch is not None
                    ), f"need read info for {self._node!r} from {self._query!r}: {self._node._read_info!r}"
                    self._epoch = self._node._read_info.epoch
                    self._has_result.set()
                    log.debug(
                        "query.connect", duration=retry.duration, node=self._node, span="current"
                    )

                # subscribe forever (until error)
                node_types: list[NodeType] = [self._query._node_type]
                if self._query._options:
                    node_types.extend(self._query._options.ancestor_types)
                    node_types.extend(self._query._options.descendant_types)
                watch_req = WatchEditsRequest(
                    scope=self._scope,
                    node_types=cast(list[wire.NodeType], node_types),
                    since_epoch=self._node._read_info.epoch,
                )
                rpc_headers = cast(_PatchedRpcMetadata, self._rpc_metadata).to_headers()
                async for rep in self._remote.watch_edits(watch_req, metadata=rpc_headers):
                    # apply edits (should filter these :ConnectionFilter)
                    log.trace("query.update", node=self._node, epoch=rep.epoch)
                    async with self._tx_lock:
                        edit_graph(graph, rep.edits, options=self._query._options, untracked=True)
                        self._epoch = rep.epoch
            except self._retry.retry_on as e:
                log.error("query.error", node=self._node, exc_info=e)
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
    async def connect[NodeT: Node, NodeDataT: AnyNodeData](
        self,
        query: QueryBuilder[NodeT, NodeDataT],
        tx_lock: asyncio.Lock,
        session: Session,
        owner: Any,
    ) -> ConnectedQuery[NodeT, NodeDataT]:
        connected_query = RemoteQuery(
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


ConnectedBench = ConnectedQuery[Bench, BenchData]
ConnectedPackage = ConnectedQuery[Package, PackageData]

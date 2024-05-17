import asyncio
from typing import Self, cast

import structlog

from bench.language.const import NodeType
from bench.language.graph import edit_graph
from bench.language.node import Node
from bench.language.query import QueryBuilder
from bench.proto import wire
from bench.proto.wire import (
    AnyNodeData,
    GraphIoStub,
    GraphScope,
    HostStub,
    SupervisorStub,
    WatchEditsRequest,
)
from bench.utils.tenacity import RetryOptions

logger = structlog.get_logger(__name__)


class ConnectedQuery[NodeT: Node, NodeDataT: AnyNodeData]:
    """A live connected query result that auto-reconnects correctly on error."""

    def __init__(
        self,
        *,
        query: QueryBuilder[NodeT, NodeDataT],
        remote: GraphIoStub | HostStub | SupervisorStub,
        scope: GraphScope,
        retry: RetryOptions = RetryOptions(),
    ):
        self._query = query
        self._remote = remote
        self._scope = scope
        self._node: NodeT | None = None
        self._has_result: asyncio.Event = asyncio.Event()
        self._is_closed: bool = False
        self._is_paused: bool = False
        self._retry = retry
        self._connect_task: asyncio.Task[None] | None = None

    @property
    def node(self) -> NodeT:
        assert self._node is not None, "query has no current result"
        return self._node

    async def connect(self) -> Self:
        """Start the connection. Returns as soon as the connection is established (has a result)."""
        assert self._connect_task is None, "already connected"
        self._connect_task = asyncio.create_task(self._do_connect())
        await self._has_result.wait()
        return self

    async def _do_connect(self) -> None:
        """Runs the core connection loop forever (or until closed)."""
        attempt = 0
        interval = self._retry.retry_interval
        last_error = None

        while not self._is_closed:
            if self._retry.max_attempts > 0 and attempt >= self._retry.max_attempts:
                raise last_error or RuntimeError(
                    f"exceeded {attempt} attempts for {self._query!r} (options={self._retry!r})"
                )
            try:
                # get initial result
                self._node = await self._query.get()
                graph = self._node._graph
                assert (
                    self._node._read_info is not None and self._node._read_info.epoch is not None
                ), f"need read info for {self._node!r} from {self._query!r}: {self._node._read_info!r}"
                self._has_result.set()
                logger.debug("query.connected", query=self._query, node=self._node)

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
                async for rep in self._remote.watch_edits(watch_req):
                    # apply edits (should filter these :ConnectionFilter)
                    logger.trace(
                        "query.update", query=self._query, node=self._node, epoch=rep.epoch
                    )
                    edit_graph(graph, rep.edits, options=self._query._options)
            except self._retry.retry_on as e:
                logger.error("query.error", query=self._query, exc_info=e)
                last_error = e
                await asyncio.sleep(interval)
                interval = min(interval * self._retry.backoff, self._retry.max_retry_interval)
                await asyncio.sleep(interval)

    def close(self):
        self._is_closed = True
        if self._connect_task is not None:
            self._connect_task.cancel()

    async def wait_closed(self):
        if self._connect_task is not None:
            try:
                await self._connect_task
            except asyncio.CancelledError:
                pass

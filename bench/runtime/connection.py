import asyncio
from typing import Self

import structlog

from bench.language.graph import NodeGraph, edit_graph
from bench.language.node import Node
from bench.language.query import QueryBuilder
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
        """Start the connection. Returns as soon as the connection is established (valid result)."""
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
                ), f"expected full read info from {self._query!r}"
                assert isinstance(
                    graph, NodeGraph
                ), f"expected full node graph for {self._node!r} but got {graph!r}"
                self._has_result.set()

                # subscribe forever (until error)
                watch_req = WatchEditsRequest(
                    scope=self._scope, since_epoch=self._node._read_info.epoch
                )
                async for rep in self._remote.watch_edits(watch_req):
                    if self._is_closed:
                        break
                    # apply edits (should filter these :ConnectionOverlapFilter)
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

import abc
import asyncio
import contextlib
from datetime import timedelta
from typing import TYPE_CHECKING, Any, AsyncIterator, Callable, Self, cast, final, override
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.core import DEFAULT_WAIT_TIMEOUT, EditType, Node, patch_graph
from bench.pb2 import AnyNodeData, GraphScopeData
from bench.utils.task import create_task
from bench.utils.tenacity import RetryOptions

from .capture import capture
from .engine import (
    AggregateOptions,
    AggregateResult,
    AggregateResultData,
    ConnectionOptions,
    Connector,
    EngineIncapableError,
    EngineUnavailableError,
    GetOptions,
    GetResult,
    GetResultData,
    Result,
    ResultData,
    SearchOptions,
    SearchResult,
    SearchResultData,
    Update,
    UpdateData,
    WatchAggregateUpdate,
    WatchAggregateUpdateData,
    WatchGetUpdate,
    WatchGetUpdateData,
    WatchSearchUpdate,
    WatchSearchUpdateData,
    origin_matches,
)

if TYPE_CHECKING:
    from bench.language import AggregationResult, Query, Session

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class Connection[
    ConnectorT: Connector,
    OptionsT: ConnectionOptions,
    ResultT: Result,
    ResultDataT: ResultData,
    UpdateDataT: UpdateData,
    UpdateT: Update,
](abc.ABC):
    """A Connection to a Graph for some Query."""

    def __init__(
        self,
        connector: ConnectorT,
        scope: GraphScopeData,
        query: "Query",
        retry: RetryOptions,
        options: OptionsT,
    ):
        self.connector = connector
        self.scope = scope
        self.query = query
        self.type = query._type
        self.type_name = self.type.name.lower()
        self.retry = retry
        self.node_types = list(query.all_node_types)
        self.options = options
        self.mode = options.mode
        self.is_live = options.live
        self.log = logger.bind(connection=self)

        self._connect_task: asyncio.Task | None = None
        self._has_result: asyncio.Event = asyncio.Event()
        self._result: ResultT | None = None
        self._result_data: ResultDataT | None = None
        self._epoch: int | None = None
        self._is_closed = False
        self._update_subscribers: list[Callable[[Any, UpdateT], None]] = []

        # capture
        capture(self)

    def __str__(self):
        is_live_postfix = " (live)" if self.is_live else ""
        if self.has_result:
            return f"{self.query} -> {self._result}{is_live_postfix}"
        else:
            return f"{self.query} -> <no result>{is_live_postfix}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def session(self) -> "Session":
        return self.connector.session

    @property
    def is_open(self) -> bool:
        """Whether the connection is open."""
        return not self._is_closed

    @property
    def has_result(self) -> bool:
        """Whether the connection has a result for the query."""
        return self._has_result.is_set()

    @property
    def result(self) -> ResultT:
        assert self._result is not None, f"{self!r} has no result"
        return self._result

    @property
    def result_data(self) -> ResultDataT:
        assert self._result_data is not None, f"{self!r} has no result data"
        return self._result_data

    @property
    def epoch(self) -> int:
        assert self._epoch is not None, f"{self!r} has no epoch"
        return self._epoch

    @final
    def on_update(self, callback: Callable[[Self, UpdateT], None]) -> Callable[[], None]:
        """Register a callback for updates."""
        assert self.is_live, f"{self!r} is not live"
        self._update_subscribers.append(callback)
        return lambda: self._update_subscribers.remove(callback)

    @final
    async def wait_until(
        self, condition: Callable[[], bool], timeout: timedelta | None = None
    ) -> None:
        """Wait until the condition is met for this Connection."""
        assert self.is_live, f"{self!r} is not live"
        if condition():
            return
        if timeout is None:
            timeout = DEFAULT_WAIT_TIMEOUT

        log = self.log.bind(condition=condition, timeout=timeout)
        complete_signal = asyncio.Event()

        def _check():
            if condition():
                complete_signal.set()

        unsub = self.on_update(lambda _, __: _check())
        try:
            _check()
            log.trace("connection.wait_until")
            await asyncio.wait_for(complete_signal.wait(), timeout=timeout.total_seconds())
            log.debug("connection.wait_until.complete")
        except BaseException as e:
            log.error("connection.wait_until.error", exc_info=e)
            raise
        finally:
            unsub()

    @final
    async def connect(self) -> None:
        """
        Connect to the graph and start watching for updates (if live).
        Unpacks the result if needed.
        """
        if not self.is_live:
            # one-off connection: just read and unpack
            self.session._on_connection_begin(self)
            retry = self.retry.new(self.session._oracle)
            try:
                while retry.should_retry:
                    retry.on_attempt()
                    # try read
                    with tracer.start_as_current_span(f"connect.{self.type_name}.read"):
                        try:
                            result_data = await self._do_read(self.query)
                            self.log.trace(f"connect.{self.type_name}", span="current")
                            break  # success
                        except Exception as e:
                            interval = retry.get_wait_interval()
                            self.log.error(
                                f"connect.{self.type_name}.error", exc_info=e, interval=interval
                            )
                            if not retry.on_error(e):
                                raise
                            await self.session._oracle.sleep(interval)
                            if isinstance(e, EngineUnavailableError):
                                # try reconnecting the channel
                                await self.connector.reset()
                                self.log.debug("connect.reset")
                else:
                    raise retry.to_error(operation=self.query)
                # unpack (should not be retried)
                self._epoch = result_data.epoch
                if self.mode == "packed" or self.mode == "both":
                    self._result_data = result_data
                if self.mode == "unpacked" or self.mode == "both":
                    self._result = self._unpack_result(result_data)
                self._has_result.set()
            finally:
                self.session._on_connection_end(self)
                self.close()
                await self.wait_closed()
        else:
            # live connection: loop (in seperate task) and await first result
            self.session._on_connection_begin(self)
            self._connect_task = create_task(
                self._do_connect_live(),
                logger=logger,
                task_id=f"connect.{self.type_name}",
                owner=self,
            )
            await self._has_result.wait()

    async def _do_connect_live(self) -> None:
        """Runs the live connection loop until closed."""
        retry = self.retry.new(self.session._oracle)
        log = self.log.bind(retry=retry, options=self.retry)
        last_error: Exception | None = None
        try:
            while not self._is_closed:
                if not retry.should_retry:
                    raise retry.to_error(operation=self.query)
                retry.on_attempt()

                try:
                    # initial read
                    with tracer.start_as_current_span(f"connect.{self.type_name}"):
                        result_data = await self._do_read(self.query)
                        assert result_data.epoch is not None, f"{result_data!r} has no epoch"
                        if last_error is not None:
                            log.debug(f"connect.{self.type_name}.recover", span="current")
                        else:
                            log.trace(f"connect.{self.type_name}", span="current")
                        last_error = None
                    # unpack
                    self._epoch = result_data.epoch
                    if self.mode == "packed" or self.mode == "both":
                        self._result_data = result_data
                    if self.mode == "unpacked" or self.mode == "both":
                        old_result = self._result
                        new_result = self._unpack_result(result_data)
                        if old_result is not None:  # patch in place
                            self._result = self._patch_result(old_result, new_result)
                        else:
                            self._result = new_result
                    self._has_result.set()
                    retry.on_success()

                    # subscribe
                    async for update_data in self._do_subscribe(
                        self.query, token=result_data.connection_token, epoch=result_data.epoch
                    ):
                        self._epoch = update_data.epoch
                        log.trace(f"connect.{self.type_name}.update")
                        update = self._apply_update(
                            self._result_data,
                            self._result,
                            update_data,
                            unpack_update=len(self._update_subscribers) > 0,
                        )
                        if update is not None:
                            for callback in self._update_subscribers:
                                callback(self, update)
                except Exception as e:
                    last_error = e
                    interval = retry.get_wait_interval()
                    log.error(f"connect.{self.type_name}.error", exc_info=e, interval=interval)
                    if not retry.on_error(e):
                        raise  # re-raise immediately
                    await self.session._oracle.sleep(interval)
                    continue
        finally:
            self.session._on_connection_end(self)
            if not self._is_closed:
                self.close()
                await self.wait_closed()
            log.trace(f"connect.{self.type_name}.end")

    @abc.abstractmethod
    def _unpack_result(self, result_data: ResultDataT) -> ResultT:
        """Unpacks the result data into a result object."""
        ...

    @abc.abstractmethod
    def _patch_result(self, old_result: ResultT, new_result: ResultT) -> ResultT:
        """Patches the old result with the new result (may just return the new result)."""
        ...

    @abc.abstractmethod
    def _apply_update(
        self,
        result_data: ResultDataT | None,
        result: ResultT | None,
        update: UpdateDataT,
        unpack_update: bool,
    ) -> UpdateT | None:
        """Applies an update to the result (in place)."""
        ...

    @abc.abstractmethod
    async def _do_read(self, query: "Query") -> ResultDataT:
        """Fetches the result data for the connection."""
        ...

    def _do_subscribe(
        self, query: "Query", token: str | None, epoch: int
    ) -> AsyncIterator[UpdateDataT]:
        """Subscribes to updates for the connection."""
        raise EngineIncapableError(self, query, reason="live subscription not supported")

    @final
    def close(self, detach: bool = False):
        """Close the connection. Optionally release any acquired graphs."""
        if self._is_closed:
            return  # already closed
        self._is_closed = True
        if self._connect_task is not None:
            self._connect_task.cancel()
        if detach:
            self.detach()
        self.log.trace(f"connect.{self.type_name}.close")

    @final
    async def wait_closed(self):
        """Wait for any pending operations to complete."""
        if self._connect_task is not None:
            with contextlib.suppress(asyncio.CancelledError):
                await self._connect_task
            self._connect_task = None

    @final
    def detach(self):
        """Release any acquired graphs."""
        if self._result is not None:
            self._detach_result(self._result)
            self.log.trace(f"connect.{self.type_name}.release")
        self._result = None
        self._result_data = None

    @abc.abstractmethod
    def _detach_result(self, result: ResultT):
        """Release the result."""
        ...


class GetConnection[ConnectorT: Connector, T: Node](
    Connection[
        ConnectorT, GetOptions, GetResult[T], GetResultData, WatchGetUpdateData, WatchGetUpdate
    ]
):
    """Base for get connections (may be live)."""

    @override
    def _unpack_result(self, result_data: GetResultData) -> GetResult:
        from bench.proto import wiring

        roots, graph = wiring.unpack_node_roots(
            data_graph=result_data.graph,
            supergraph=self.session._supergraph,
            roots=result_data.roots_ptr,
            session=self.session,
            connection=self,
        )
        return GetResult(graph=graph, roots=list(roots))

    @override
    def _patch_result(self, old_result: GetResult, new_result: GetResult) -> GetResult:
        _ = patch_graph(old_graph=old_result.graph, new_graph=new_result.graph)
        old_result.roots = [old_result.graph.get(root.id) for root in new_result.roots]
        return old_result

    @override
    def _detach_result(self, result: GetResult):
        result.graph.supergraph.remove_graph(result.graph)

    @override
    def _apply_update(
        self,
        result_data: GetResultData | None,
        result: GetResult | None,
        update: WatchGetUpdateData,
        unpack_update: bool,
    ) -> WatchGetUpdate | None:
        from bench.language import edit_data_graph, edit_graph
        from bench.proto import wiring

        # filter edits
        if self.session._origin:
            new_edits = [
                edit
                for edit in update.edits
                if not edit.origin or not origin_matches(edit.origin, self.session._origin)
            ]
        else:
            new_edits = update.edits
        if not new_edits:
            return None
        # NOTE: we ignore missing nodes in GetConnection/SearchConnection updates as filter edits
        #  is imperfect (we don't know if the edit was already applied to this connection's graph)

        # apply
        if result_data is not None:
            edit_data_graph(
                result_data.graph,
                update.edits,
                include_removed=self.query.include_removed,
                ignore_missing=True,
            )
        if result is not None:
            if unpack_update:
                # unpack update
                added: dict[UUID, Node] = {}
                updated: dict[UUID, Node] = {}
                removed: dict[UUID, Node] = {}

                # collect pre-edit nodes (for remove)
                for edit in new_edits:
                    if edit.type in (
                        EditType.ARCHIVE,
                        EditType.UNARCHIVE,
                        EditType.DELETE,
                        EditType.ERASE,
                    ):
                        node = result.graph.get(UUID(edit.node_ptr.id))
                        assert (
                            node is not None
                        ), f"missing node for edit: {wiring.describe_edit(edit)}"
                        removed[node.id] = node

                # do edit
                edit_graph(
                    graph=result.graph,
                    supergraph=self.session._supergraph,
                    edits=new_edits,
                    include_removed=self.query.include_removed,
                    validate=False,
                    ignore_missing=True,
                )

                # collect post-edit nodes (for add/update)
                for edit in new_edits:
                    if edit.type in (EditType.CREATE, EditType.UPDATE, EditType.MOVE):
                        node = result.graph.get(UUID(edit.node_ptr.id))
                        assert (
                            node is not None
                        ), f"missing node for edit: {wiring.describe_edit(edit)}"
                        if edit.type == EditType.CREATE:
                            added[node.id] = node
                        else:
                            updated[node.id] = node
                    elif edit.type in (EditType.UNARCHIVE, EditType.RESTORE):
                        node = result.graph.get(UUID(edit.node_ptr.id))
                        assert (
                            node is not None
                        ), f"missing node for edit: {wiring.describe_edit(edit)}"
                        added[node.id] = node

                return WatchGetUpdate(added=added, updated=updated, removed=removed)
            else:
                # just edit directly
                edit_graph(
                    graph=result.graph,
                    supergraph=self.session._supergraph,
                    edits=new_edits,
                    include_removed=self.query.include_removed,
                    validate=False,
                )


class SearchConnection[ConnectorT: Connector, T: Node](
    Connection[
        ConnectorT,
        SearchOptions,
        SearchResult[T],
        SearchResultData,
        WatchSearchUpdateData,
        WatchSearchUpdate,
    ]
):
    """Base for search connections (may be live)."""

    @override
    def _unpack_result(self, result_data: SearchResultData) -> SearchResult:
        from bench.proto import wiring

        roots, graph = wiring.unpack_node_roots(
            data_graph=result_data.graph,
            supergraph=self.session._supergraph,
            roots=result_data.roots_ptr,
            session=self.session,
            connection=self,
        )
        return SearchResult(graph=graph, roots=list(roots), total=result_data.total)

    @override
    def _patch_result(self, old_result: SearchResult, new_result: SearchResult) -> SearchResult:
        patch_graph(old_graph=old_result.graph, new_graph=new_result.graph)
        old_result.roots = [old_result.graph.get(root.id) for root in new_result.roots]
        old_result.total = new_result.total
        return old_result

    @override
    def _detach_result(self, result: SearchResult):
        result.graph.supergraph.remove_graph(result.graph)

    @override
    def _apply_update(
        self,
        result_data: SearchResultData | None,
        result: SearchResult | None,
        update: WatchSearchUpdateData,
        unpack_update: bool,
    ) -> WatchSearchUpdate | None:
        from bench.language import edit_data_graph, edit_graph
        from bench.proto import wiring

        # :ConnectionUpdateOrdering
        assert result_data is not None, f"{self!r} does not work without packed result"

        # apply other added/removed nodes
        for node_data in update.added_nodes:
            result_data.graph.add(node_data)
        for node_ptr in update.removed_nodes_ptr:
            node_data = result_data.graph.get(cast(str, node_ptr.id))
            assert (
                node_data is not None
            ), f"missing node for update: {wiring.describe_node_ptr(node_ptr)}"
            result_data.graph.remove(node_data)

        if result is not None:
            # and update unpacked result
            added: dict[UUID, Node] = {}
            updated: dict[UUID, Node] = {}
            removed: dict[UUID, Node] = {}

            # collect pre-edit nodes (for remove)
            for edit in update.edits:
                if edit.type in (
                    EditType.ARCHIVE,
                    EditType.DELETE,
                    EditType.ERASE,
                ):
                    node = result.graph.get(UUID(edit.node_ptr.id))
                    assert node is not None, f"missing node for edit: {wiring.describe_edit(edit)}"
                    removed[node.id] = node

            # add new nodes
            for node_data in update.added_nodes:
                node = wiring.unpack_builtin_object(
                    node_data,
                    supergraph=self.session._supergraph,
                    session=self.session,
                    connection=self,
                    expect=Node,
                    graph=result.graph,
                )
                result.graph.add(node)
                added[node.id] = node

            # remove nodes
            for node_ptr in update.removed_nodes_ptr:
                node = result.graph.get(UUID(cast(str, node_ptr.id)))
                assert node is not None, f"missing node for update: {node!r}"
                result.graph.remove(node)
                removed[node.id] = node

            # apply edits
            edit_data_graph(
                result_data.graph,
                update.edits,
                include_removed=self.query.include_removed,
                ignore_missing=True,
            )
            edit_graph(
                graph=result.graph,
                supergraph=self.session._supergraph,
                edits=update.edits,
                include_removed=self.query.include_removed,
                validate=False,
                ignore_missing=True,
            )

            # collect post-edit nodes (for add/update)
            for edit in update.edits:
                if edit.type in (EditType.CREATE, EditType.UPDATE, EditType.MOVE):
                    node = result.graph.get(UUID(edit.node_ptr.id))
                    assert node is not None, f"missing node for edit: {wiring.describe_edit(edit)}"
                    if edit.type == EditType.CREATE:
                        added[node.id] = node
                    else:
                        updated[node.id] = node
                elif edit.type in (EditType.UNARCHIVE, EditType.RESTORE):
                    node = result.graph.get(UUID(edit.node_ptr.id))
                    assert node is not None, f"missing node for edit: {wiring.describe_edit(edit)}"
                    added[node.id] = node

            # update 'roots' list
            result_data.roots_ptr = update.roots_ptr
            new_roots_data: list[AnyNodeData] = []
            for root_ptr_data in result_data.roots_ptr:
                root = result_data.graph.get(cast(str, root_ptr_data.id))
                assert (
                    root is not None
                ), f"missing root data for update: {wiring.describe_node_ptr(root_ptr_data)}"
                new_roots_data.append(root)
            result_data.roots = new_roots_data
            new_roots: list[Node] = []
            for root_ptr_data in result_data.roots_ptr:
                root_id = UUID(root_ptr_data.id)
                root = result.graph.get(root_id)
                assert (
                    root is not None
                ), f"missing root for update: {wiring.describe_node_ptr(root_ptr_data)}"
                new_roots.append(root)
            result.roots = new_roots

            if unpack_update:
                return WatchSearchUpdate(added=added, updated=updated, removed=removed)
        else:
            # just apply edits to result_data
            edit_data_graph(
                result_data.graph, update.edits, include_removed=self.query.include_removed
            )

            # update roots list
            result_data.roots_ptr = update.roots_ptr
            new_roots_data: list[AnyNodeData] = []
            for root_ptr_data in result_data.roots_ptr:
                root = result_data.graph.get(cast(str, root_ptr_data.id))
                assert (
                    root is not None
                ), f"missing root for update: {wiring.describe_node_ptr(root_ptr_data)}"
                new_roots_data.append(root)
            result_data.roots = new_roots_data


class AggregateConnection[ConnectorT: Connector](
    Connection[
        ConnectorT,
        AggregateOptions,
        AggregateResult,
        AggregateResultData,
        WatchAggregateUpdateData,
        WatchAggregateUpdate,
    ]
):
    """Base for aggregate connections (may be live)."""

    @override
    def _unpack_result(self, result_data: AggregateResultData) -> AggregateResult:
        from bench.language import AggregationResult
        from bench.proto import wiring

        aggregation = wiring.unpack_builtin_object(
            result_data.aggregation, supergraph=self.session._supergraph, expect=AggregationResult
        )
        return AggregateResult(aggregation=aggregation)

    @override
    def _patch_result(
        self, old_result: AggregateResult, new_result: AggregateResult
    ) -> AggregateResult:
        return new_result  # nothing to patch

    @override
    def _detach_result(self, result: AggregateResult):
        pass  # nothing to release

    @override
    def _apply_update(
        self,
        result_data: AggregateResultData | None,
        result: AggregateResult | None,
        update: WatchAggregateUpdateData,
        unpack_update: bool,
    ) -> WatchAggregateUpdate | None:
        from bench.proto import wiring

        if result_data is not None:
            result_data.aggregation = update.aggregation
        if result is not None:
            result.aggregation = wiring.unpack_builtin_object(
                update.aggregation, supergraph=self.session._supergraph, expect=AggregationResult
            )

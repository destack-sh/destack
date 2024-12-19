#
# Queries
#
import abc
import asyncio
import contextlib
from dataclasses import dataclass
from functools import wraps
from typing import (
    TYPE_CHECKING,
    Any,
    AsyncIterator,
    Callable,
    Collection,
    Literal,
    Mapping,
    Optional,
    Union,
    cast,
    final,
    override,
)
from uuid import UUID

import structlog
from grpclib import GRPCError
from more_itertools import first
from opentelemetry import trace

from bench.language.const import (
    BenchError,
    ConditionalType,
    EditType,
    NodeType,
    QueryType,
)
from bench.language.expression import C
from bench.language.graph import NodeDataGraph, NodeGraph
from bench.language.node import Node, patch_graph, repr_scope
from bench.language.registry import CHILD_NODE_TYPES, DESCENDANT_NODE_TYPES, NODE_CLASS_BY_TYPE
from bench.proto.wire import (
    AggregationResultData,
    AnyNodeData,
    BenchData,
    ClientOriginData,
    EditData,
    GraphScopeData,
    NodeReferenceData,
)
from bench.proto.wire.common_pb2 import RpcMetadata
from bench.proto.wire.lang_pb2 import ExpressionData
from bench.proto.wire.system_grpc import GraphIOClient, HostClient, SupervisorClient
from bench.proto.wire.system_pb2 import WatchGetRequest, WatchSearchRequest
from bench.utils.func import bittuple, group_by, repr_enums
from bench.utils.task import create_task
from bench.utils.tenacity import RETRY_GRPC, RETRY_GRPC_FOREVER, RetryOptions

if TYPE_CHECKING:
    from bench.language import (
        AggregationResult,
        Expression,
        Field,
        Property,
        QueryBuilder,
        Session,
    )

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

FieldOrProperty = Union[
    Field if TYPE_CHECKING else "Field", Property if TYPE_CHECKING else "Property", Any
]
NodeTypeOrClass = Union[NodeType, type[Node]]


class ChannelError(BenchError):
    """An error. From a channel."""

    def __init__(
        self,
        medium: Union["Engine", "Channel", Any],
        query: Optional["QueryBuilder"] | Collection[EditData] = None,
        expression: Union["Expression", list["Expression"], None] = None,
        reason: str | None = None,
        cause: Exception | None = None,
    ):
        if query is not None:
            action = repr(query)
        elif expression is not None:
            action = repr(expression)
        else:
            action = "<unknown action>"
        super().__init__(f"{medium!r} cannot {action!r}: {reason or '<unknown error>'}")
        self.medium = medium
        self.query = query
        self.expression = expression
        self.reason = reason
        self.__cause__ = cause


class ChannelUnavailableError(ChannelError):
    """The channel is temporarily unavailable."""

    pass


class ChannelIncapableError(ChannelError):
    """The channel can't do this thing."""

    pass


ConnectMode = Literal["both", "packed", "unpacked"]


@dataclass(slots=True)
class _ConnectOptions:
    live: bool
    mode: ConnectMode


#
# Get
#


@dataclass(slots=True)
class GetOptions(_ConnectOptions):
    is_optional: bool = False


@dataclass(slots=True)
class GetResultData:
    graph: NodeDataGraph
    roots_ptr: list[NodeReferenceData]
    epoch: int | None
    connection_token: str | None


@dataclass(slots=True)
class GetResult[T: Node]:
    graph: NodeGraph
    roots: list[T]


@dataclass(slots=True)
class WatchGetUpdateData:
    edits: list[EditData]
    cascaded_edits: list[EditData]
    added_nodes: list[AnyNodeData]
    removed_nodes_ptr: list[NodeReferenceData]
    epoch: int


@dataclass(slots=True)
class WatchGetUpdate:
    added: Mapping[UUID, Node]
    updated: Mapping[UUID, Node]
    removed: Mapping[UUID, Node]


#
# Search
#


@dataclass(slots=True)
class SearchOptions(_ConnectOptions):
    count: bool


@dataclass(slots=True)
class SearchResultData:
    graph: NodeDataGraph
    roots: list[AnyNodeData]
    roots_ptr: list[NodeReferenceData]
    total: int | None
    epoch: int | None
    connection_token: str | None


@dataclass(slots=True)
class SearchResult[T: Node]:
    graph: NodeGraph
    roots: list[T]
    total: int | None


@dataclass(slots=True)
class WatchSearchUpdateData:
    edits: list[EditData]
    cascaded_edits: list[EditData]
    added_nodes: list[AnyNodeData]
    removed_nodes_ptr: list[NodeReferenceData]
    roots_ptr: list[NodeReferenceData]
    total: int | None
    epoch: int


@dataclass(slots=True)
class WatchSearchUpdate:
    added: Mapping[UUID, Node]
    updated: Mapping[UUID, Node]
    removed: Mapping[UUID, Node]


#
# Aggregate
#


@dataclass(slots=True)
class AggregateOptions(_ConnectOptions):
    pass


@dataclass(slots=True)
class AggregateResultData:
    aggregation: AggregationResultData
    epoch: int | None
    connection_token: str | None


@dataclass(slots=True)
class AggregateResult:
    aggregation: "AggregationResult"


@dataclass(slots=True)
class WatchAggregateUpdateData:
    aggregation: AggregationResultData
    epoch: int


@dataclass(slots=True)
class WatchAggregateUpdate:
    pass


ConnectionOptions = GetOptions | SearchOptions | AggregateOptions
ResultData = GetResultData | SearchResultData | AggregateResultData
Result = GetResult | SearchResult | AggregateResult
UpdateData = WatchGetUpdateData | WatchSearchUpdateData | WatchAggregateUpdateData
Update = WatchGetUpdate | WatchSearchUpdate | WatchAggregateUpdate


def scope_includes(scope: GraphScopeData, other: GraphScopeData) -> bool:
    return (not scope.bench_id or scope.bench_id == other.bench_id) and (
        not scope.package_id or scope.package_id == other.package_id
    )


def origin_matches(origin: ClientOriginData, other: ClientOriginData) -> bool:
    return origin.id == other.id and origin.nonce == other.nonce


# NOTE :Architecture :Cleanup: the whole Engine/Channel/Connection system seems convoluted


class Engine[C: "Channel"](abc.ABC):
    """A Graph IO service to perform IO on some subgraph."""

    def __init__(
        self,
        name: str,
        scope: GraphScopeData,
        node_types: bittuple[NodeType],
    ):
        self.name = name
        self.scope = scope
        self.node_types = node_types

    def __str__(self) -> str:
        return f"{self.name} [scope={repr_scope(self.scope)}, node_types={repr_enums(self.node_types)}]"

    def __repr__(self):
        self_str = str(self)
        if self_str:
            return f"<{self.__class__.__name__} {self_str}>"
        else:
            return f"<{self.__class__.__name__}>"

    @property
    @abc.abstractmethod
    def is_readonly(self) -> bool:
        """Whether this channel is read-only."""
        ...

    @property
    @abc.abstractmethod
    def include_deleted(self) -> bool:
        """Whether this channel includes deleted nodes."""
        ...

    @property
    def id(self) -> int | str | UUID:
        return id(self)

    @abc.abstractmethod
    async def channel(self, session: "Session") -> C:
        """Opens an IO channel on this subgraph in a Session."""
        ...


class NullEngine(Engine):
    """A null engine that does nothing."""

    async def channel(self, session: "Session"):
        raise ChannelIncapableError(self, reason="null engine")

    @property
    def include_deleted(self) -> bool:
        return False

    @property
    def is_readonly(self) -> bool:
        return True


class Channel[E: Engine](abc.ABC):
    """A channel to a specific Store to read from in a Session."""

    def __init__(self, engine: E, session: "Session"):
        self.engine = engine
        self.session = session

    def __str__(self):
        return f"session={self.session}"

    def __repr__(self):
        self_str = str(self)
        if self_str:
            return f"<{self.__class__.__name__} {self}>"
        else:
            return f"<{self.__class__.__name__}>"

    @property
    def read_retry(self) -> RetryOptions:
        return RetryOptions(retry_on=(ChannelUnavailableError,))

    @abc.abstractmethod
    async def reset(self):
        """Resets this channel to its initial state."""
        ...

    @abc.abstractmethod
    async def close(self):
        """Closes this channel to all further operations."""
        ...

    #
    # Read
    #

    @abc.abstractmethod
    def _get_connection_cls(
        self, query: "QueryBuilder", scope: GraphScopeData, options: ConnectionOptions
    ) -> "type[Connection]": ...

    @final
    async def get(self, query: "QueryBuilder", options: GetOptions) -> "GetConnection":
        """Read a single node given the query in the current transaction context (if any)."""
        assert query._type == QueryType.GET, f"{query!r} is not a get"
        assert query._roots is not None, f"{query!r} has no roots"
        scope = self.session._get_scope_for_query(query)
        connection_cls = self._get_connection_cls(query, scope, options)
        connection = connection_cls(self, scope, query, self.read_retry, options)
        assert isinstance(connection, GetConnection), f"{connection!r} is not a get"
        await connection.connect()
        return connection

    @final
    async def search(self, query: "QueryBuilder", options: SearchOptions) -> "SearchConnection":
        """Read the nodes given the search query in the current transaction context (if any)."""
        assert query._type == QueryType.SEARCH, f"{query!r} is not a search"
        scope = self.session._get_scope_for_query(query)
        connection_cls = self._get_connection_cls(query, scope, options)
        connection = connection_cls(self, scope, query, self.read_retry, options)
        assert isinstance(connection, SearchConnection), f"{connection!r} is not a search"
        await connection.connect()
        return connection

    @final
    async def aggregate(
        self, query: "QueryBuilder", options: AggregateOptions
    ) -> "AggregateConnection":
        """Read the nodes given the aggregate query in the current transaction context (if any)."""
        assert query._type == QueryType.AGGREGATE, f"{query!r} is not an aggregate"
        assert query._aggregation is not None, f"{query!r} has no aggregation"
        scope = self.session._get_scope_for_query(query)
        connection_cls = self._get_connection_cls(query, scope, options)
        connection = connection_cls(self, scope, query, self.read_retry, options)
        assert isinstance(connection, AggregateConnection), f"{connection!r} is not an aggregate"
        await connection.connect()
        return connection


@dataclass(slots=True)
class FlushResultData:
    cascaded_edits: list[EditData]


@dataclass(slots=True)
class CommitResultData:
    cascaded_edits: list[EditData]


class WritableChannel[E: Engine](Channel[E]):
    """A channel you can write to."""

    #
    # Transaction management
    #

    @abc.abstractmethod
    async def flush(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResultData:
        """
        Flushes edits in the current transaction context. If not in a transaction, begins one.
        """
        ...

    @abc.abstractmethod
    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> CommitResultData:
        """
        Commits the flushed pending and given edits in the current transaction context.
        """
        ...


class Connection[
    ChannelT: Channel,
    OptionsT: ConnectionOptions,
    ResultT: Result,
    ResultDataT: ResultData,
    UpdateDataT: UpdateData,
    UpdateT: Update,
](abc.ABC):
    """A Connection to a Graph for some Query."""

    def __init__(
        self,
        channel: ChannelT,
        scope: GraphScopeData,
        query: "QueryBuilder",
        retry: RetryOptions,
        options: OptionsT,
    ):
        self.channel = channel
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
        self._update_subscribers: list[Callable[[UpdateT], None]] = []

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
        return self.channel.session

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
    def on_update(self, callback: Callable[[UpdateT], None]) -> Callable[[], None]:
        """Register a callback for updates."""
        assert self.is_live, f"{self!r} is not live"
        self._update_subscribers.append(callback)
        return lambda: self._update_subscribers.remove(callback)

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
                            if isinstance(e, ChannelUnavailableError):
                                # try reconnecting the channel
                                await self.channel.reset()
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
                                callback(update)
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
    async def _do_read(self, query: "QueryBuilder") -> ResultDataT:
        """Fetches the result data for the connection."""
        ...

    def _do_subscribe(
        self, query: "QueryBuilder", token: str | None, epoch: int
    ) -> AsyncIterator[UpdateDataT]:
        """Subscribes to updates for the connection."""
        raise ChannelIncapableError(self, query, reason="live subscription not supported")

    @final
    def close(self):
        """Close the connection."""
        if self._is_closed:
            return  # already closed
        self._is_closed = True
        if self._connect_task is not None:
            self._connect_task.cancel()
        self.log.trace(f"connect.{self.type_name}.close")

    @final
    async def wait_closed(self):
        """Wait for any pending operations to complete."""
        if self._connect_task is not None:
            with contextlib.suppress(asyncio.CancelledError):
                await self._connect_task
            self._connect_task = None


class GetConnection[ChannelT: Channel, T: Node](
    Connection[
        ChannelT, GetOptions, GetResult[T], GetResultData, WatchGetUpdateData, WatchGetUpdate
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
    def _apply_update(
        self,
        result_data: GetResultData | None,
        result: GetResult | None,
        update: WatchGetUpdateData,
        unpack_update: bool,
    ) -> WatchGetUpdate | None:
        from bench.language.transaction import edit_data_graph, edit_graph

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

        # apply
        if result_data is not None:
            edit_data_graph(
                result_data.graph, update.edits, include_deleted=self.query.include_deleted
            )
        if result is not None:
            if unpack_update:
                # unpack update
                added: dict[UUID, Node] = {}
                updated: dict[UUID, Node] = {}
                removed: dict[UUID, Node] = {}

                # collect pre-edit nodes (for remove)
                for edit in new_edits:
                    if edit.type in (EditType.DELETE, EditType.ERASE):
                        node = result.graph.get(UUID(edit.node_ptr.id))
                        assert node is not None, f"missing node for edit: {edit!r}"
                        removed[node.id] = node

                # do edit
                edit_graph(
                    graph=result.graph,
                    supergraph=self.session._supergraph,
                    edits=new_edits,
                    include_deleted=self.query.include_deleted,
                    validate=False,
                )

                # collect post-edit nodes (for add/update)
                for edit in new_edits:
                    if edit.type in (EditType.CREATE, EditType.UPDATE, EditType.MOVE):
                        node = result.graph.get(UUID(edit.node_ptr.id))
                        assert node is not None, f"missing node for edit: {edit!r}"
                        if edit.type == EditType.CREATE:
                            added[node.id] = node
                        else:
                            updated[node.id] = node

                return WatchGetUpdate(added=added, updated=updated, removed=removed)
            else:
                # just edit directly
                edit_graph(
                    graph=result.graph,
                    supergraph=self.session._supergraph,
                    edits=new_edits,
                    include_deleted=self.query.include_deleted,
                    validate=False,
                )


class SearchConnection[ChannelT: Channel, T: Node](
    Connection[
        ChannelT,
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
    def _apply_update(
        self,
        result_data: SearchResultData | None,
        result: SearchResult | None,
        update: WatchSearchUpdateData,
        unpack_update: bool,
    ) -> WatchSearchUpdate | None:
        from bench.language.transaction import edit_data_graph, edit_graph
        from bench.proto import wiring

        # :ConnectionUpdateOrdering
        assert result_data is not None, f"{self!r} does not work without packed result"

        # filter edits
        if self.session._origin:
            new_edits = [
                edit
                for edit in update.edits
                if not edit.origin or not origin_matches(edit.origin, self.session._origin)
            ]
        else:
            new_edits = update.edits

        # apply other added/removed nodes
        for node_data in update.added_nodes:
            result_data.graph.add(node_data)
        for node_ptr in update.removed_nodes_ptr:
            node_data = result_data.graph.get(cast(str, node_ptr.id))
            assert node_data is not None, f"missing node for update: {node_ptr!r}"
            result_data.graph.remove(node_data)
        if result is not None:  # and update unpacked result
            for node_data in update.added_nodes:
                node = wiring.unpack_builtin_object(
                    node_data,
                    supergraph=self.session._supergraph,
                    session=self.session,
                    connection=self,
                    expect=Node,
                )
                result.graph.add(node)
            for node_ptr in update.removed_nodes_ptr:
                node = result.graph.get(UUID(cast(str, node_ptr.id)))
                assert node is not None, f"missing node for update: {node!r}"
                result.graph.remove(node)

        # apply
        edit_data_graph(result_data.graph, update.edits, include_deleted=self.query.include_deleted)
        for node_data in update.added_nodes:
            result_data.graph.add(node_data)
        if result is not None:
            edit_graph(
                graph=result.graph,
                supergraph=self.session._supergraph,
                edits=new_edits,
                include_deleted=self.query.include_deleted,
                validate=False,
            )

        # update 'roots' list
        result_data.roots_ptr = update.roots_ptr
        new_roots_data: list[AnyNodeData] = []
        for root_ptr in result_data.roots_ptr:
            root = result_data.graph.get(cast(str, root_ptr.id))
            assert root is not None, f"missing root for update: {root_ptr!r}"
            new_roots_data.append(root)
        result_data.roots = new_roots_data
        if result is not None:  # and update unpacked result
            new_roots: list[Node] = []
            for root_data in result_data.roots_ptr:
                root = result.graph.get(UUID(root_data.id))
                assert root is not None, f"missing root for update: {root_data!r}"
                new_roots.append(root)
            result.roots = new_roots


class AggregateConnection[ChannelT: Channel](
    Connection[
        ChannelT,
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


#
# Splits
#


class SplitChannel(Channel[NullEngine]):
    """A read-only channel splits queries across channels."""

    @override
    async def reset(self):
        pass  # nothing to do

    @override
    async def close(self):
        pass  # nothing to do

    @override
    def _get_connection_cls(
        self, query: "QueryBuilder", scope: GraphScopeData, options: ConnectionOptions
    ) -> type[Connection]:
        if query._type == QueryType.GET:
            return SplitGetConnection
        elif query._type == QueryType.SEARCH:
            return SplitSearchConnection
        else:
            raise RuntimeError(f"unsupported split read {query!r}")


class SplitConnection(Connection):
    def _get_best_match_engine(
        self,
        scope: GraphScopeData,
        required_types: set[NodeType],
        match_types: set[NodeType],
        *,
        is_readonly: bool,
        include_deleted: bool,
    ) -> tuple[Engine, set[NodeType]]:
        """Get the engine with best coverage of required node types from candidates."""
        candidate_engines = [
            engine
            for engine in self.session._engines
            if (
                (is_readonly or not engine.is_readonly)
                and (not include_deleted or engine.include_deleted)
                and scope_includes(engine.scope, scope)
                and any(t in engine.node_types for t in required_types)
            )
        ]

        if not candidate_engines:
            types_str = "|".join(t.bench_name for t in required_types)
            raise BenchError(
                f"no engine for [scope={repr_scope(scope)}, node_types={types_str}] in {self!r}"
            )

        # Find engine with most overlap between required types and candidate types
        best_engine = max(
            candidate_engines,
            key=lambda e: len(set(e.node_types) & match_types),
        )
        covered_types = set(best_engine.node_types) & match_types
        return best_engine, covered_types

    async def _read_descendants(
        self,
        scope: GraphScopeData,
        combined_graph: NodeDataGraph,
        remaining_types: set[NodeType],
        query: "QueryBuilder",
    ) -> set[NodeType]:
        """Read descendants for nodes in the graph, returns covered types."""
        from bench.language import QueryBuilder

        engine, covered_types = self._get_best_match_engine(
            scope,
            remaining_types,
            remaining_types,
            is_readonly=True,
            include_deleted=query.include_deleted,
        )
        channel = await self.session._get_channel(engine)

        # descend into potential parents (for potential children)
        nodes_by_type = group_by(combined_graph.nodes, lambda n: NodeType(n.metatype)).items()
        for parent_type, parents in nodes_by_type:
            # get valid child types for this parent from covered types
            child_types = set(CHILD_NODE_TYPES[parent_type]) & covered_types
            for child_type in child_types:
                # get parents
                parent_ids = [n.id for n in parents if n.id]
                if not parent_ids:
                    continue

                # get children
                child_cls = NODE_CLASS_BY_TYPE[child_type]
                descendant_query = QueryBuilder(
                    type=QueryType.SEARCH,
                    node_type=child_type,
                    filter=C(
                        ConditionalType.IN,
                        property=child_cls.get_property("parent_id"),
                        value=parent_ids,
                    ),
                    descendant_types=list(set(DESCENDANT_NODE_TYPES[child_type]) & covered_types),
                    include_deleted=query.include_deleted,
                    select=query._select,
                )
                descendant_connection = await channel.search(
                    descendant_query,
                    SearchOptions(live=False, mode="packed", count=False),
                )
                combined_graph.extend(descendant_connection.result_data.graph.nodes)

        return covered_types

    async def _read_ancestors(
        self,
        scope: GraphScopeData,
        combined_graph: NodeDataGraph,
        remaining_types: set[NodeType],
        query: "QueryBuilder",
    ) -> set[NodeType]:
        """Read ancestors for roots in the graph, returns covered types."""

        from bench.language import NodeReference, QueryBuilder
        from bench.proto import wiring

        # get parent references from roots
        inner_roots = combined_graph.find_roots()
        inner_roots_parents_by_id = {
            n.parent_ptr.id: n.parent_ptr for n in inner_roots if n.parent_ptr.id
        }
        inner_roots_types = {NodeType(n.node_type) for n in inner_roots_parents_by_id.values()}
        next_ancestor_types = inner_roots_types.intersection(remaining_types)
        if not inner_roots_parents_by_id or not next_ancestor_types:
            return set()

        # get best engine for remaining ancestors
        engine, covered_types = self._get_best_match_engine(
            scope,
            next_ancestor_types,
            next_ancestor_types,
            is_readonly=True,
            include_deleted=query.include_deleted,
        )
        channel = await self.session._get_channel(engine)

        # get parents by type
        inner_roots_parents_by_type = group_by(
            inner_roots_parents_by_id.values(), lambda n: NodeType(n.node_type)
        ).items()
        for parent_type, parents in inner_roots_parents_by_type:
            if parent_type not in covered_types:
                continue

            ancestor_query = QueryBuilder(
                type=QueryType.GET,
                node_type=parent_type,
                roots=[
                    wiring.unpack_builtin_object(p, supergraph=None, expect=NodeReference)
                    for p in parents
                ],
                ancestor_types=list(covered_types),
                include_deleted=query.include_deleted,
                select=query._select,
            )
            ancestor_connection = await channel.get(ancestor_query, self.options)
            combined_graph.extend(ancestor_connection.result_data.graph.nodes)

        return covered_types

    async def _do_read_remainder(
        self,
        query: "QueryBuilder",
        initial_result: GetResultData | SearchResultData,
        initial_types: Collection[NodeType],
    ) -> NodeDataGraph:
        """Fetch the surrounding ancestor/descendant nodes for a split Query."""

        combined_graph = NodeDataGraph(
            scope=self.scope,
            node_types=tuple(query.all_node_types),
            nodes=initial_result.graph.nodes,
        )
        remaining_ancestors = set(query._ancestor_types or ()) - set(initial_types)
        remaining_descendants = set(query._descendant_types or ()) - set(initial_types)

        # read until there is nothing more to read
        while remaining_ancestors or remaining_descendants:
            if not combined_graph.find_roots():
                break

            # get descendants
            if remaining_descendants:
                bench = first((n for n in combined_graph.nodes if isinstance(n, BenchData)), None)
                descendants_scope = GraphScopeData(bench_id=bench.id) if bench else self.scope
                covered = await self._read_descendants(
                    descendants_scope, combined_graph, remaining_descendants, query
                )
                remaining_descendants -= covered

            # get ancestors
            if remaining_ancestors:
                covered = await self._read_ancestors(
                    self.scope, combined_graph, remaining_ancestors, query
                )
                if not covered:
                    break  # no more parents to traverse
                remaining_ancestors -= covered

        return combined_graph


class SplitSearchConnection[T: Node](SearchConnection[SplitChannel, T], SplitConnection):
    """Search across multiple connections."""

    @override
    async def _do_read(self, query: "QueryBuilder") -> SearchResultData:
        # search engine for initial query
        engine, covered_types = self._get_best_match_engine(
            self.scope,
            {query._node_type},
            set(query.all_node_types),
            is_readonly=True,
            include_deleted=query.include_deleted,
        )
        channel = await self.session._get_channel(engine)
        connection = await channel.search(
            query.trim_to(covered_types),
            SearchOptions(live=False, mode="packed", count=self.options.count),
        )
        result = connection.result_data
        if not query._ancestor_types and not query._descendant_types:
            return result  # nothing more to read

        # combine (keeping the 'roots' from the initial result)
        combined_graph = await self._do_read_remainder(query, result, covered_types)
        combined_result = SearchResultData(
            graph=combined_graph,
            roots=result.roots,
            roots_ptr=result.roots_ptr,
            total=result.total,
            epoch=result.epoch,
            connection_token=result.connection_token,
        )
        return combined_result


class SplitGetConnection[T: Node](GetConnection[SplitChannel, T], SplitConnection):
    """Get across multiple connections."""

    @override
    async def _do_read(self, query: "QueryBuilder") -> GetResultData:
        # get engine for initial query
        engine, covered_types = self._get_best_match_engine(
            self.scope,
            {query._node_type},
            set(query.all_node_types),
            is_readonly=True,
            include_deleted=query.include_deleted,
        )
        channel = await self.session._get_channel(engine)
        connection = await channel.get(
            query.trim_to(covered_types), GetOptions(live=False, mode="packed")
        )
        result = connection.result_data
        if not query._ancestor_types and not query._descendant_types:
            return result  # nothing more to read

        # combine (keeping the 'roots' from the initial result)
        combined_graph = await self._do_read_remainder(query, result, covered_types)
        combined_result = GetResultData(
            graph=combined_graph,
            roots_ptr=result.roots_ptr,
            epoch=result.epoch,
            connection_token=result.connection_token,
        )
        return combined_result


#
# Memory
#


class MemoryEngine(Engine["MemoryChannel"]):
    """A read-only Engine that reads from an in-memory graph."""

    def __init__(
        self,
        name: str,
        scope: GraphScopeData,
        node_types: bittuple[NodeType],
        graph: "NodeDataGraph",
        include_deleted: bool,
    ):
        super().__init__(name, scope, node_types)
        self.graph = graph
        self._include_deleted = include_deleted

    def __str__(self):
        return f"'{self.name}' [scope={repr_scope(self.scope)}, node_types={repr_enums(self.node_types)}, graph={self.graph!r}]"

    @property
    def is_readonly(self) -> bool:
        return True

    @property
    def include_deleted(self) -> bool:
        return self._include_deleted

    async def channel(self, session: "Session"):
        return MemoryChannel(self, session)


class MemoryChannel(Channel[MemoryEngine]):
    """A read-only Channel to an in-memory graph."""

    def __init__(self, engine: "MemoryEngine", session: "Session"):
        super().__init__(engine, session)

    def __str__(self):
        return f"engine={self.engine!r}, session={self.session}"

    @override
    async def reset(self):
        pass  # nothing to do

    @override
    async def close(self):
        pass  # nothing to do

    @override
    def _get_connection_cls(
        self, query: "QueryBuilder", scope: GraphScopeData, options: ConnectionOptions
    ) -> type[Connection]:
        if query._type == QueryType.GET:
            return MemoryGetConnection
        else:
            raise RuntimeError(f"unsupported memory read {query!r}")


class MemoryGetConnection[T: Node](GetConnection[MemoryChannel, T]):
    """Search an in-memory Channel."""

    @override
    async def _do_read(self, query: "QueryBuilder") -> GetResultData:
        from bench.language import NodeDataGraph, NodeReference
        from bench.proto import wire

        loaded_graph = self.channel.engine.graph
        visited_graph = NodeDataGraph(
            scope=self.channel.engine.scope, node_types=self.channel.engine.node_types
        )

        # get roots
        assert query._roots is not None, f"{query!r} has no roots"
        roots: list[AnyNodeData] = []
        for root_ptr in query._roots:
            root_id = str(root_ptr.id)
            if root_id in visited_graph:
                continue  # dedupe
            root = loaded_graph.get(root_id)
            if root is not None:
                roots.append(root)
                visited_graph.add(root)

        # select ancestors
        ancestor_types = query._ancestor_types or ()
        if len(ancestor_types) > 0:
            with tracer.start_as_current_span("memory.fetch.get_ancestors"):
                current_parents = roots
                while current_parents:
                    next_parents = []
                    for node in current_parents:
                        if (
                            node.parent_ptr is not None
                            and node.parent_ptr.id is not None
                            and node.parent_ptr.id not in visited_graph
                            and node.parent_ptr.node_type in ancestor_types
                        ):
                            parent = loaded_graph.get(node.parent_ptr.id)
                            assert parent is not None, f"missing parent {node.parent_ptr!r}"
                            visited_graph.add(parent)
                            next_parents.append(parent)
                    current_parents = next_parents

        # select descendants
        descendant_types = query._descendant_types or ()
        if len(descendant_types) > 0:
            with tracer.start_as_current_span("memory.fetch.get_descendants"):
                child_types_by_parent: dict[wire.NodeType, tuple[NodeType, ...]] = {
                    cast(wire.NodeType, node_type): tuple(
                        t for t in CHILD_NODE_TYPES[node_type] if t in descendant_types
                    )
                    for node_type in query.all_node_types
                }
                current_parents = roots
                while current_parents:
                    next_parents: list[AnyNodeData] = []
                    for node in current_parents:
                        child_types = child_types_by_parent[cast(wire.NodeType, node.metatype)]
                        for child_type in child_types:
                            children = loaded_graph.get_descendants(node, child_type)
                            visited_graph.extend(children)
                            for child in children:
                                if loaded_graph.has_descendants(child):
                                    next_parents.append(child)
                    current_parents = next_parents

        return GetResultData(
            graph=visited_graph,
            roots_ptr=[NodeReference._ref_data_from_node_data(r) for r in roots],
            epoch=None,
            connection_token=None,
        )


#
# Remote
#


class RemoteEngine(Engine["RemoteChannel"]):
    """An engine that proxies to a remote graph store."""

    def __init__(
        self,
        name: str,
        scope: GraphScopeData,
        node_types: bittuple[NodeType],
        remote: GraphIOClient | HostClient | SupervisorClient,
        rpc_metadata: RpcMetadata,
        write_retry: RetryOptions = RETRY_GRPC,
    ):
        super().__init__(name, scope, node_types)
        from bench.proto.wiring import pack_rpc_headers

        self.remote = remote
        self.rpc_metadata = rpc_metadata
        self.rpc_headers = pack_rpc_headers(rpc_metadata)
        self.write_retry = write_retry

    def __str__(self):
        return f"{self.name} [scope={repr_scope(self.scope)}, node_types={repr_enums(self.node_types)}, remote={self.remote.__class__.__name__}]"

    @override
    async def channel(self, session: "Session") -> "RemoteChannel":
        return RemoteChannel(self, session)

    @property
    def include_deleted(self) -> bool:
        return True

    @property
    def is_readonly(self) -> bool:
        return False


class RemoteChannel(WritableChannel[RemoteEngine]):
    """A channel to a remote graph."""

    def __str__(self):
        return f"engine={self.engine!r}, session={self.session}"

    @override
    async def reset(self):
        pass  # remote channels use a shared client

    @override
    async def close(self):
        pass  # remote channels use a shared client

    def _get_connection_cls(
        self, query: "QueryBuilder", scope: GraphScopeData, options: ConnectionOptions
    ) -> type[Connection]:
        if query._type == QueryType.GET:
            return RemoteGetConnection
        elif query._type == QueryType.SEARCH:
            return RemoteSearchConnection
        elif query._type == QueryType.AGGREGATE:
            return RemoteAggregateConnection
        else:
            raise RuntimeError(f"unsupported read type {query._type}")

    @property
    @override
    def read_retry(self):
        return RETRY_GRPC_FOREVER

    @staticmethod
    def _rpc(func):
        """Wraps an RPC function with tracing & retries."""
        method_name = func.__name__

        @wraps(func)
        @tracer.start_as_current_span(f"remote.{method_name}")
        async def wrapper(self: "RemoteChannel", *args, **kwargs):
            retry = self.engine.write_retry.new(self.session._oracle)
            while retry.should_retry:
                retry.on_attempt()
                try:
                    return await func(self, *args, **kwargs)
                except Exception as e:
                    logger.error(f"remote.{method_name}.error", channel=self, exc_info=True)
                    retry.on_error(e)
                    if not isinstance(e, self.read_retry.retry_on):
                        raise
                    if retry.should_retry:
                        await self.session._oracle.sleep(retry.get_wait_interval())
            error = retry.to_error()
            if isinstance(error, (OSError,)):
                raise ChannelUnavailableError(
                    self, args[0] if args else None, reason=str(error)
                ) from error
            else:
                raise error

        return wrapper

    @override
    @_rpc
    async def flush(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResultData:
        raise ChannelIncapableError(self, edits, reason="flush not yet supported")

    @override
    @_rpc
    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> CommitResultData:
        from bench.proto import wire

        edits = list(edits)
        request = wire.CommitTransactionRequest(
            id=str(self.session.tx.id),
            edits=edits,
            scope=self.engine.scope,
            context=self.session._get_context(),
        )
        response = await self.engine.remote.commit_transaction(
            request, metadata=self.engine.rpc_headers
        )
        return CommitResultData(cascaded_edits=list(response.cascaded_edits))


def _grpc_wrap_error(query: "QueryBuilder", e: GRPCError):
    """Wraps a GRPCError in something more harmonized."""
    return e  # NOTE :UX: wrap remote grpc errors :BadRemoteErrors


class RemoteGetConnection[T: Node](GetConnection[RemoteChannel, T]):
    """Search a remote channel live."""

    @override
    async def _do_read(self, query: "QueryBuilder") -> GetResultData:
        from bench.proto import wire, wiring

        assert query._roots, f"{query!r} has no roots"
        engine = self.channel.engine
        roots_ptr = [r._to_data() for r in query._roots]
        request = wire.GetNodesRequest(
            scope=engine.scope,
            roots=roots_ptr,
            block_ptr=query._base_block._to_ref_data() if query._base_block else None,
            ancestor_types=[wiring.pack_enum(NodeType, t) for t in query._ancestor_types],
            descendant_types=[wiring.pack_enum(NodeType, t) for t in query._descendant_types],
            select=query._select._to_data() if query._select else None,
            include_deleted=query._include_deleted,
        )
        try:
            response = await self.channel.engine.remote.get_nodes(
                request, metadata=engine.rpc_headers
            )
        except GRPCError as e:
            raise _grpc_wrap_error(query, e) from e
        nodes = [wiring.unwrap_some_node(n) for n in response.nodes]
        graph = NodeDataGraph(scope=engine.scope, node_types=engine.node_types, nodes=nodes)
        return GetResultData(
            graph=graph,
            roots_ptr=roots_ptr,
            epoch=response.epoch,
            connection_token=response.connection_token,
        )

    @override
    async def _do_subscribe(
        self, query: "QueryBuilder", token: str | None, epoch: int
    ) -> AsyncIterator[WatchGetUpdateData]:
        from bench.proto import wiring
        from bench.proto.services import unary_stream_rpc

        assert token is not None, f"{self!r} has no token"
        assert epoch is not None, f"{self!r} has no epoch"
        watch_req = WatchGetRequest(scope=self.scope, connection_token=token, since_epoch=epoch)
        async for rep in unary_stream_rpc(self.channel.engine.remote.watch_get, watch_req):
            update = WatchGetUpdateData(
                edits=list(rep.edits),
                cascaded_edits=list(rep.cascaded_edits),
                added_nodes=[wiring.unwrap_some_node(n) for n in rep.added_nodes],
                removed_nodes_ptr=list(rep.removed_nodes_ptr),
                epoch=rep.epoch,
            )
            yield update


class RemoteSearchConnection[T: Node](SearchConnection[RemoteChannel, T]):
    """Search a remote channel live."""

    @override
    async def _do_read(self, query: "QueryBuilder") -> SearchResultData:
        from bench.proto import wire, wiring

        engine = self.channel.engine
        request = wire.SearchNodesRequest(
            scope=engine.scope,
            node_type=wiring.pack_enum(NodeType, query._node_type),
            block_ptr=query._base_block._to_ref_data() if query._base_block else None,
            filter=wiring.pack_builtin_object_maybe(query._filter, ExpressionData),
            sort=(
                [wiring.pack_builtin_object(s, ExpressionData) for s in query._sort]
                if query._sort
                else []
            ),
            ancestor_types=[wiring.pack_enum(NodeType, t) for t in query._ancestor_types],
            descendant_types=[wiring.pack_enum(NodeType, t) for t in query._descendant_types],
            count=self.options.count,
            select=query._select._to_data() if query._select else None,
        )
        if query._first:
            request.first = query._first
        if query._skip:
            request.skip = query._skip
        try:
            response = await self.channel.engine.remote.search_nodes(
                request, metadata=engine.rpc_headers
            )
        except GRPCError as e:
            raise _grpc_wrap_error(query, e) from e
        nodes = [wiring.unwrap_some_node(n) for n in response.nodes]
        graph = NodeDataGraph(scope=engine.scope, node_types=engine.node_types, nodes=nodes)
        roots = [graph[cast(str, r.id)] for r in response.roots_ptr]
        return SearchResultData(
            graph=graph,
            roots=roots,
            roots_ptr=list(response.roots_ptr),
            total=response.total,
            epoch=response.epoch,
            connection_token=response.connection_token,
        )

    @override
    async def _do_subscribe(
        self, query: "QueryBuilder", token: str | None, epoch: int
    ) -> AsyncIterator[WatchSearchUpdateData]:
        from bench.proto import wiring
        from bench.proto.services import unary_stream_rpc

        assert token is not None, f"{self!r} has no token"
        assert epoch is not None, f"{self!r} has no epoch"
        watch_req = WatchSearchRequest(scope=self.scope, connection_token=token, since_epoch=epoch)
        async for rep in unary_stream_rpc(self.channel.engine.remote.watch_search, watch_req):
            update = WatchSearchUpdateData(
                edits=list(rep.edits),
                cascaded_edits=list(rep.cascaded_edits),
                added_nodes=[wiring.unwrap_some_node(n) for n in rep.added_nodes],
                removed_nodes_ptr=list(rep.removed_nodes_ptr),
                roots_ptr=list(rep.roots_ptr),
                total=rep.total,
                epoch=rep.epoch,
            )
            yield update


class RemoteAggregateConnection(AggregateConnection[RemoteChannel]):
    """Aggregate a remote channel live."""

    @override
    async def _do_read(self, query: "QueryBuilder") -> AggregateResultData:
        from bench.proto import wire, wiring

        engine = self.channel.engine
        assert query._aggregation is not None, f"{query!r} has no aggregation"
        request = wire.AggregateNodesRequest(
            node_type=wiring.pack_enum(NodeType, query._node_type),
            filter=wiring.pack_builtin_object_maybe(query._filter, ExpressionData),
            aggregation=cast(ExpressionData, query._aggregation._to_data()),
            scope=engine.scope,
        )
        try:
            response = await engine.remote.aggregate_nodes(request, metadata=engine.rpc_headers)
        except GRPCError as e:
            raise _grpc_wrap_error(query, e) from e
        assert response.aggregation is not None, f"{response!r} has no aggregation"
        return AggregateResultData(
            aggregation=response.aggregation,
            epoch=response.epoch,
            connection_token=response.connection_token,
        )

    # NOTE :Incomplete: RemoteAggregateConnection subscription

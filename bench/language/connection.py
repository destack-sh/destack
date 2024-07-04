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
    Optional,
    TypeVar,
    Union,
    cast,
    final,
    override,
)
from uuid import UUID

import psycopg
import structlog
from more_itertools import first
from opentelemetry import trace

from bench.language.const import (
    AggregationOp,
    BenchError,
    ConditionalOp,
    NodeType,
    ReadType,
)
from bench.language.expression import C
from bench.language.graph import NodeDataGraph, NodeGraph
from bench.language.node import Node
from bench.language.setup import CHILD_NODE_TYPES, DESCENDANT_NODE_TYPES, NODE_CLASS_BY_TYPE
from bench.proto.wire import (
    AggregationData,
    AnyNodeData,
    BenchData,
    ClientOriginData,
    EditData,
    ExpressionData,
    GraphIoClient,
    GraphScopeData,
    HostClient,
    NodeReferenceData,
    ReadOptionsData,
    RpcMetadata,
    SupervisorClient,
    WatchGetRequest,
    WatchSearchRequest,
)
from bench.utils.func import bittuple, group_by, repr_enums
from bench.utils.task import create_task
from bench.utils.tenacity import RETRY_GRPC, RETRY_GRPC_FOREVER, RetryOptions

if TYPE_CHECKING:
    from bench.language import (
        Aggregation,
        Bench,
        Expression,
        Field,
        Property,
        QueryBuilder,
        Session,
        Store,
    )
    from bench.sql.client import PgStoreConnection

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

NodeT = TypeVar("NodeT", bound=Node)
NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)
FieldOrProperty = Union[
    Field if TYPE_CHECKING else "Field", Property if TYPE_CHECKING else "Property", Any
]
NodeTypeOrClass = Union[NodeType, type[Node]]


class ChannelError(BenchError):
    """An error. From a channel."""

    def __init__(
        self,
        medium: Union["GraphEngine", "Channel", Any],
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


@dataclass(slots=True)
class _ConnectOptions:
    live: bool
    unpack: bool


#
# Get
#


@dataclass(slots=True)
class GetOptions(_ConnectOptions):
    pass


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
class WatchGetUpdate:
    edits: list[EditData]
    cascaded_edits: list[EditData]
    added_nodes: list[AnyNodeData]
    removed_nodes_ptr: list[NodeReferenceData]
    epoch: int


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
class WatchSearchUpdate:
    edits: list[EditData]
    cascaded_edits: list[EditData]
    added_nodes: list[AnyNodeData]
    removed_nodes_ptr: list[NodeReferenceData]
    roots_ptr: list[NodeReferenceData]
    total: int | None
    epoch: int


#
# Aggregate
#


@dataclass(slots=True)
class AggregateOptions(_ConnectOptions):
    pass


@dataclass(slots=True)
class AggregateResultData:
    aggregation: AggregationData
    epoch: int | None
    connection_token: str | None


@dataclass(slots=True)
class AggregateResult:
    aggregation: "Aggregation"


@dataclass(slots=True)
class WatchAggregateUpdate:
    aggregation: AggregationData
    epoch: int


Options = GetOptions | SearchOptions | AggregateOptions
ResultData = GetResultData | SearchResultData | AggregateResultData
Result = GetResult | SearchResult | AggregateResult
UpdateData = WatchGetUpdate | WatchSearchUpdate | WatchAggregateUpdate


def scope_includes(scope: GraphScopeData, other: GraphScopeData) -> bool:
    return (scope.bench_id is None or scope.bench_id == other.bench_id) and (
        scope.package_id is None or scope.package_id == other.package_id
    )


def origin_matches(origin: ClientOriginData, other: ClientOriginData) -> bool:
    return origin.id == other.id and origin.nonce == other.nonce


class GraphEngine[C: Channel](abc.ABC):
    """A Graph IO service to perform IO on some subgraph."""

    def __init__(
        self,
        scope: GraphScopeData,
        node_types: bittuple[NodeType],
    ):
        self.scope = scope
        self.node_types = node_types

    def __str__(self) -> str:
        return ""

    def __repr__(self):
        self_str = str(self)
        if self_str:
            return f"<{self.__class__.__name__} {self_str}>"
        else:
            return f"<{self.__class__.__name__}>"

    @property
    def is_readonly(self) -> bool:
        return False

    @property
    def id(self) -> int | str | UUID:
        return id(self)

    @abc.abstractmethod
    async def connect(self, session: "Session") -> C:
        """Opens an IO channel on this subgraph in a session."""
        ...


class NullEngine(GraphEngine):
    """A null engine that does nothing."""

    async def connect(self, session: "Session"):
        raise ChannelIncapableError(self, reason="null engine")


class Channel[E: GraphEngine](abc.ABC):
    """A channel to a specific store to read from in a session."""

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

    async def close(self):  # noqa: B027
        """Closes this channel to all further operations."""
        pass  # nothing to do

    #
    # Read
    #

    @abc.abstractmethod
    def _get_connection_cls(
        self, query: "QueryBuilder", scope: GraphScopeData, options: Options
    ) -> "type[Connection]": ...

    @final
    async def get(self, query: "QueryBuilder", options: GetOptions) -> "GetConnection":
        """Read a single node given the query in the current transaction context (if any)."""
        assert query._read_type == ReadType.GET, f"{query!r} is not a get"
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
        assert query._read_type == ReadType.SEARCH, f"{query!r} is not a search"
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
        assert query._read_type == ReadType.AGGREGATE, f"{query!r} is not an aggregate"
        assert query._aggregation is not None, f"{query!r} has no aggregation"
        scope = self.session._get_scope_for_query(query)
        connection_cls = self._get_connection_cls(query, scope, options)
        connection = connection_cls(self, scope, query, self.read_retry, options)
        assert isinstance(connection, AggregateConnection), f"{connection!r} is not an aggregate"
        await connection.connect()
        return connection


@dataclass(slots=True)
class FlushResultData:
    revisions: list[int]
    cascaded_edits: list[EditData]


@dataclass(slots=True)
class CommitResultData:
    revisions: list[int]
    cascaded_edits: list[EditData]


class WritableChannel[E: GraphEngine](Channel[E]):
    """A channel you can write to."""

    #
    # Transaction management
    #

    @abc.abstractmethod
    async def flush(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResultData:
        """
        Flushes edits in the current transaction context. If not in a transaction, begins one.
        If this is a primary store, must return the accepted revisions for every edit (in order).
        """
        ...

    @abc.abstractmethod
    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> CommitResultData:
        """
        Commits the flushed pending and given edits in the current transaction context.
        If this is a primary store, must return the accepted revisions for every edit (in order).
        """
        ...


class Connection[
    ChannelT: Channel,
    OptionsT: Options,
    ResultT: Result,
    ResultDataT: ResultData,
    UpdateT: UpdateData,
](abc.ABC):
    """A connection to a graph for some query."""

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
        self.type = query._read_type
        self.type_name = self.type.name.lower()
        self.retry = retry
        self.node_types = list(query.all_node_types)
        self.options = options
        self.is_live = options.live
        self.is_unpacked = options.unpack
        self.log = logger.bind(connection=self)

        self._has_result: asyncio.Event = asyncio.Event()
        self._result: ResultT | None = None
        self._result_data: ResultDataT | None = None
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
        return self._result_data is not None

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
        assert self._result_data is not None, f"{self!r} has no result data"
        assert self._result_data.epoch is not None, f"{self!r} has no epoch"
        return self._result_data.epoch

    @final
    def on_update(self, callback: Callable[[UpdateT], None]):
        """Register a callback for updates."""
        assert self.is_live, f"{self!r} is not live"
        self._update_subscribers.append(callback)

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
                            self._result_data = await self._do_read(self.query)
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
                else:
                    raise retry.to_error(operation=self.query)
                # unpack (should not be retried)
                if self.options.unpack:
                    self._result = self._unpack_result(self._result_data)
                self._has_result.set()
            finally:
                self.session._on_connection_end(self)
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
        try:
            while not self._is_closed:
                if not retry.should_retry:
                    raise retry.to_error(operation=self.query)
                retry.on_attempt()

                try:
                    # initial read
                    with tracer.start_as_current_span(f"connect.{self.type_name}"):
                        self._result_data = await self._do_read(self.query)
                        self.log.trace(f"connect.{self.type_name}", span="current")
                    # unpack
                    if self.options.unpack:
                        self._result = self._unpack_result(self._result_data)
                    self._has_result.set()
                    retry.on_success()
                    # subscribe
                    async for update in self._do_subscribe(self.query, self._result_data):
                        self.log.trace(f"connect.{self.type_name}.update", update=update)
                        self._apply_update(self._result_data, self._result, update)
                        for callback in self._update_subscribers:
                            callback(update)
                except Exception as e:
                    interval = retry.get_wait_interval()
                    self.log.error(f"connect.{self.type_name}.error", exc_info=e, interval=interval)
                    if not retry.on_error(e):
                        raise
                    await self.session._oracle.sleep(interval)
                    continue
        except Exception as e:
            if self.session._on_error:
                self.session._on_error(e)
        finally:
            self.session._on_connection_end(self)

    @abc.abstractmethod
    def _unpack_result(self, result_data: ResultDataT) -> ResultT:
        """Unpacks the result data into a result object."""
        ...

    @abc.abstractmethod
    def _apply_update(self, result_data: ResultDataT, result: ResultT | None, update: UpdateT):
        """Applies an update to the result (in place)."""
        ...

    @abc.abstractmethod
    async def _do_read(self, query: "QueryBuilder") -> ResultDataT:
        """Fetches the result data for the connection."""
        ...

    def _do_subscribe(self, query: "QueryBuilder", result: ResultDataT) -> AsyncIterator[UpdateT]:
        """Subscribes to updates for the connection."""
        raise ChannelIncapableError(self, query, reason="live subscription not supported")

    @final
    def close(self):
        """Close the connection."""
        self._is_closed = True
        if self._connect_task is not None:
            self._connect_task.cancel()

    @final
    async def wait_closed(self):
        """Wait for any pending operations to complete."""
        if self._connect_task is not None:
            with contextlib.suppress(asyncio.CancelledError):
                await self._connect_task
            self._connect_task = None


class GetConnection[ChannelT: Channel, T: Node](
    Connection[ChannelT, GetOptions, GetResult[T], GetResultData, WatchGetUpdate]
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
    def _apply_update(
        self, result_data: GetResultData, result: GetResult | None, update: WatchGetUpdate
    ):
        from bench.language.transaction import edit_data_graph, edit_graph

        if self.session._origin:
            new_edits = [
                edit
                for edit in update.edits
                if not edit.origin or not origin_matches(edit.origin, self.session._origin)
            ]
        else:
            new_edits = update.edits
        edit_data_graph(result_data.graph, update.edits, self.query._options)
        if result is not None:
            edit_graph(
                graph=result.graph,
                supergraph=self.session._supergraph,
                edits=new_edits,
                options=self.query._options,
                track=False,
                validate=False,
            )


class SearchConnection[ChannelT: Channel, T: Node](
    Connection[ChannelT, SearchOptions, SearchResult[T], SearchResultData, WatchSearchUpdate]
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
    def _apply_update(
        self, result_data: SearchResultData, result: SearchResult | None, update: WatchSearchUpdate
    ):
        from bench.language.transaction import edit_data_graph, edit_graph
        from bench.proto import wiring

        # apply edits
        if self.session._origin:
            new_edits = [
                edit
                for edit in update.edits
                if not edit.origin or not origin_matches(edit.origin, self.session._origin)
            ]
        else:
            new_edits = update.edits
        edit_data_graph(result_data.graph, update.edits, self.query._options)
        for node_data in update.added_nodes:
            result_data.graph.add(node_data)
        if result is not None:
            edit_graph(
                graph=result.graph,
                supergraph=self.session._supergraph,
                edits=new_edits,
                options=self.query._options,
                track=False,
                validate=False,
            )

        # apply other added/removed nodes
        for node_data in update.added_nodes:
            result_data.graph.add(node_data)
        for node_ptr in update.removed_nodes_ptr:
            node_data = result_data.graph.get(cast(str, node_ptr.id))
            assert node_data is not None, f"missing node for update: {node_ptr!r}"
            result_data.graph.remove(node_data)
        if result is not None:  # and update unpacked result
            for node_data in update.added_nodes:
                node = wiring.unpack_object(
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
        ChannelT, AggregateOptions, AggregateResult, AggregateResultData, WatchAggregateUpdate
    ]
):
    """Base for aggregate connections (may be live)."""

    @override
    def _unpack_result(self, result_data: AggregateResultData) -> AggregateResult:
        from bench.language import Aggregation
        from bench.proto import wiring

        aggregation = wiring.unpack_object(
            result_data.aggregation, supergraph=self.session._supergraph, expect=Aggregation
        )
        return AggregateResult(aggregation=aggregation)

    @override
    def _apply_update(
        self,
        result_data: AggregateResultData,
        result: AggregateResult | None,
        update: WatchAggregateUpdate,
    ):
        from bench.proto import wiring

        result_data.aggregation = update.aggregation
        if result is not None:
            result.aggregation = wiring.unpack_object(
                update.aggregation, supergraph=self.session._supergraph, expect=Aggregation
            )


class MemoryEngine(GraphEngine["MemoryChannel"]):
    """A read-only engine that reads from an in-memory graph."""

    def __init__(
        self, scope: GraphScopeData, node_types: bittuple[NodeType], graph: "NodeDataGraph"
    ):
        super().__init__(scope, node_types)
        self.graph = graph

    def __str__(self):
        return (
            f"scope={self.scope!r}, node_types={repr_enums(self.node_types)}, graph={self.graph!r}"
        )

    @property
    def is_readonly(self) -> bool:
        return True

    async def connect(self, session: "Session"):
        return MemoryChannel(self, session)


class MemoryChannel(Channel[MemoryEngine]):
    """A read-only channel to an in-memory graph."""

    def __init__(self, engine: "MemoryEngine", session: "Session"):
        super().__init__(engine, session)

    def __str__(self):
        return f"engine={self.engine!r}, session={self.session}"

    @override
    def _get_connection_cls(
        self, query: "QueryBuilder", scope: GraphScopeData, options: Options
    ) -> type[Connection]:
        if query._read_type == ReadType.GET:
            return MemoryGetConnection
        else:
            raise RuntimeError(f"unsupported memory read {query!r}")


class MemoryGetConnection[T: Node](GetConnection[MemoryChannel, T]):
    """Search an in-memory channel."""

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
        ancestor_types = query._options.ancestor_types if query._options else ()
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
                            and node.parent_ptr.type in ancestor_types
                        ):
                            parent = loaded_graph.get(node.parent_ptr.id)
                            assert parent is not None, f"missing parent {node.parent_ptr!r}"
                            visited_graph.add(parent)
                            next_parents.append(parent)
                    current_parents = next_parents

        # select descendants
        descendant_types = query._options.descendant_types if query._options else ()
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
            roots_ptr=[NodeReference.from_node_data(r) for r in roots],
            epoch=None,
            connection_token=None,
        )


class SplitChannel(Channel[NullEngine]):
    """A read-only channel splits queries across channels."""

    @override
    def _get_connection_cls(
        self, query: "QueryBuilder", scope: GraphScopeData, options: Options
    ) -> type[Connection]:
        if query._read_type == ReadType.GET:
            return SplitGetConnection
        elif query._read_type == ReadType.SEARCH:
            return SplitSearchConnection
        else:
            raise RuntimeError(f"unsupported split read {query!r}")


class SplitConnection(Connection):
    # NOTE :Robustness: we handle splits by assuming the node type split is a 'clean' horizontal
    #  line in the ancestry tree (like the local/global split).

    async def _do_read_remainder(
        self,
        query: "QueryBuilder",
        initial_result: GetResultData | SearchResultData,
        initial_types: Collection[NodeType],
    ) -> NodeDataGraph:
        """Fetch the surrounding ancestor/descendant nodes for a split query."""
        from bench.language import NodeReference, QueryBuilder, ReadOptions
        from bench.proto import wiring

        assert query._options is not None, f"{query!r} has no options"
        remaining_ancestors_types = [
            t for t in query._options.ancestor_types if t not in initial_types
        ]
        remaining_descendants_types = [
            t for t in query._options.descendant_types if t not in initial_types
        ]
        if not remaining_ancestors_types and not remaining_descendants_types:
            return initial_result.graph  # nothing more to read (full result)

        combined_graph = NodeDataGraph(
            scope=self.scope,
            node_types=tuple(query.all_node_types),
            nodes=initial_result.graph.nodes,
        )
        inner_roots = combined_graph.find_roots()
        if not inner_roots:
            return initial_result.graph  # nothing more to read (empty graph)

        # select down for each potential parent in all nodes
        if remaining_descendants_types:
            remaining_descendants_types = bittuple(*remaining_descendants_types)
            bench = first((n for n in combined_graph.nodes if isinstance(n, BenchData)), None)
            descendants_scope = GraphScopeData(bench_id=bench.id) if bench else self.scope
            inner_nodes_by_type = group_by(combined_graph.nodes, lambda n: NodeType(n.metatype))
            descendants_engine = self.session._get_engine_for(
                descendants_scope, remaining_descendants_types, is_readonly=True
            )
            descendants_channel = await self.session._get_channel(descendants_engine)
            for parent_type, parents in inner_nodes_by_type.items():
                child_types = remaining_descendants_types & CHILD_NODE_TYPES[parent_type]
                for child_type in child_types:
                    child_cls = NODE_CLASS_BY_TYPE[child_type]
                    descendant_types = (
                        remaining_descendants_types & DESCENDANT_NODE_TYPES[child_type]
                    )
                    parent_ids = [n.id for n in parents if n.id]
                    descendant_query = QueryBuilder(
                        read_type=ReadType.SEARCH,
                        node_type=child_type,
                        filter=C(
                            ConditionalOp.IN,
                            property=child_cls._prop("parent_id"),
                            value=parent_ids,
                            value_packed={},
                        ),
                        options=ReadOptions(descendant_types=list(descendant_types)),
                    )
                    descendant_connection = await descendants_channel.search(
                        descendant_query, SearchOptions(live=False, unpack=False, count=False)
                    )
                    combined_graph.extend(descendant_connection.result_data.graph.nodes)

        # select up for each parent in current roots
        if remaining_ancestors_types:
            inner_roots_parents = tuple(n.parent_ptr for n in inner_roots if n.parent_ptr)
            inner_roots_parents_by_type = group_by(inner_roots_parents, lambda n: n.type)
            ancestor_engine = self.session._get_engine_for(
                self.scope, remaining_ancestors_types, is_readonly=True
            )
            ancestor_channel = await self.session._get_channel(ancestor_engine)
            for parent_type, parents in inner_roots_parents_by_type.items():
                if parent_type not in remaining_ancestors_types:
                    continue
                ancestor_query = QueryBuilder(
                    read_type=ReadType.GET,
                    node_type=parent_type,
                    roots=[
                        wiring.unpack_object(p, supergraph=None, expect=NodeReference)
                        for p in parents
                    ],
                    options=ReadOptions(ancestor_types=remaining_ancestors_types),
                )
                ancestor_connection = await ancestor_channel.get(ancestor_query, self.options)
                combined_graph.extend(ancestor_connection.result_data.graph.nodes)

        return combined_graph


class SplitSearchConnection[T: Node](SearchConnection[SplitChannel, T], SplitConnection):
    """Search across multiple connections."""

    @override
    async def _do_read(self, query: "QueryBuilder") -> SearchResultData:
        # first trim query to nucleus around core node type (use best match)
        engine = self.session._get_engine_for(
            self.scope, query._node_type, best_match=self.node_types, is_readonly=True
        )
        channel = await self.session._get_channel(engine)
        connection = await channel.search(
            query.trim_to(engine.node_types),
            SearchOptions(live=False, unpack=False, count=self.options.count),
        )
        result = connection.result_data
        if query._options is None:
            return result  # nothing more to read

        # combine (keeping the 'roots' from the initial result)
        combined_graph = await self._do_read_remainder(query, result, engine.node_types)
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
        # first trim query to nucleus around core node type (use best match)
        engine = self.session._get_engine_for(
            self.scope, query._node_type, best_match=self.node_types, is_readonly=True
        )
        channel = await self.session._get_channel(engine)
        connection = await channel.get(
            query.trim_to(engine.node_types), GetOptions(live=False, unpack=False)
        )
        result = connection.result_data
        if query._options is None:
            return result  # nothing more to read

        # combine (keeping the 'roots' from the initial result)
        combined_graph = await self._do_read_remainder(query, result, engine.node_types)
        combined_result = GetResultData(
            graph=combined_graph,
            roots_ptr=result.roots_ptr,
            epoch=result.epoch,
            connection_token=result.connection_token,
        )
        return combined_result


class RemoteEngine(GraphEngine["RemoteChannel"]):
    """An engine that proxies to a remote graph store."""

    def __init__(
        self,
        scope: GraphScopeData,
        node_types: bittuple[NodeType],
        remote: GraphIoClient | HostClient | SupervisorClient,
        rpc_metadata: RpcMetadata,
        write_retry: RetryOptions = RETRY_GRPC,
    ):
        super().__init__(scope, node_types)
        from bench.proto.wiring import pack_rpc_headers

        self.remote = remote
        self.rpc_metadata = rpc_metadata
        self.rpc_headers = pack_rpc_headers(rpc_metadata)
        self.write_retry = write_retry

    def __str__(self):
        return f"scope={self.scope!r}, node_types={repr_enums(self.node_types)}, remote={self.remote.__class__.__name__}"

    @override
    async def connect(self, session: "Session") -> "RemoteChannel":
        return RemoteChannel(self, session)


class RemoteChannel(WritableChannel[RemoteEngine]):
    """A channel to a remote graph."""

    def __str__(self):
        return f"engine={self.engine!r}, session={self.session}"

    def _get_connection_cls(
        self, query: "QueryBuilder", scope: GraphScopeData, options: Options
    ) -> type[Connection]:
        if query._read_type == ReadType.GET:
            return RemoteGetConnection
        elif query._read_type == ReadType.SEARCH:
            return RemoteSearchConnection
        elif query._read_type == ReadType.AGGREGATE:
            return RemoteAggregateConnection
        else:
            raise RuntimeError(f"unsupported read type {query._read_type}")

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
        return CommitResultData(
            revisions=response.revisions, cascaded_edits=response.cascaded_edits
        )


class RemoteGetConnection[T: Node](GetConnection[RemoteChannel, T]):
    """Search a remote channel live."""

    @override
    async def _do_read(self, query: "QueryBuilder") -> GetResultData:
        from bench.proto import wire, wiring

        assert query._roots, f"{query!r} has no roots"
        engine = self.channel.engine
        roots_ptr = [r._to_data() for r in query._roots]
        request = wire.GetNodesRequest(
            roots=roots_ptr,
            options=query._options._to_data() if query._options else None,
            scope=engine.scope,
        )
        response = await self.channel.engine.remote.get_nodes(request, metadata=engine.rpc_headers)
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
        self, query: "QueryBuilder", result: GetResultData
    ) -> AsyncIterator[WatchGetUpdate]:
        from bench.proto import wiring

        assert result.connection_token is not None, f"{result!r} has no token"
        assert result.epoch is not None, f"{result!r} has no epoch"
        watch_req = WatchGetRequest(
            scope=self.scope, connection_token=result.connection_token, since_epoch=result.epoch
        )
        async for rep in self.channel.engine.remote.watch_get(watch_req):
            update = WatchGetUpdate(
                edits=rep.edits,
                cascaded_edits=rep.cascaded_edits,
                added_nodes=[wiring.unwrap_some_node(n) for n in rep.added_nodes],
                removed_nodes_ptr=rep.removed_nodes_ptr,
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
            node_type=wiring.pack_enum(NodeType, query._node_type),
            filter=wiring.pack_object_maybe(query._filter, ExpressionData),
            sort=(
                [wiring.pack_object(s, ExpressionData) for s in query._sort] if query._sort else []
            ),
            first=query._first,
            options=wiring.pack_object_maybe(query._options, ReadOptionsData),
            count=self.options.count,
            scope=engine.scope,
        )
        response = await self.channel.engine.remote.search_nodes(
            request, metadata=engine.rpc_headers
        )
        nodes = [wiring.unwrap_some_node(n) for n in response.nodes]
        graph = NodeDataGraph(scope=engine.scope, node_types=engine.node_types, nodes=nodes)
        roots = [graph[cast(str, r.id)] for r in response.roots_ptr]
        return SearchResultData(
            graph=graph,
            roots=roots,
            roots_ptr=response.roots_ptr,
            total=response.total,
            epoch=response.epoch,
            connection_token=response.connection_token,
        )

    @override
    async def _do_subscribe(
        self, query: "QueryBuilder", result: SearchResultData
    ) -> AsyncIterator[WatchSearchUpdate]:
        from bench.proto import wiring

        assert result.connection_token is not None, f"{result!r} has no token"
        assert result.epoch is not None, f"{result!r} has no epoch"
        watch_req = WatchSearchRequest(
            scope=self.scope, connection_token=result.connection_token, since_epoch=result.epoch
        )
        async for rep in self.channel.engine.remote.watch_search(watch_req):
            update = WatchSearchUpdate(
                edits=rep.edits,
                cascaded_edits=rep.cascaded_edits,
                added_nodes=[wiring.unwrap_some_node(n) for n in rep.added_nodes],
                removed_nodes_ptr=rep.removed_nodes_ptr,
                roots_ptr=rep.roots_ptr,
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
            filter=wiring.pack_object_maybe(query._filter, expect=ExpressionData),
            aggregation=cast(ExpressionData, query._aggregation._to_data()),
            scope=engine.scope,
        )
        response = await engine.remote.aggregate_nodes(request, metadata=engine.rpc_headers)
        return AggregateResultData(
            aggregation=response.aggregation,
            epoch=response.epoch,
            connection_token=response.connection_token,
        )

    # NOTE :Incomplete: RemoteAggregateConnection subscription


class PostgresEngine(GraphEngine):
    """An engine that talks directly to a Postgres store."""

    def __init__(
        self,
        store: "Store",
        bench: "Bench",
        scope: GraphScopeData,
        node_types: bittuple[NodeType],
    ):
        super().__init__(scope, node_types)
        self.store = store
        self.bench = bench

    def __str__(self):
        return (
            f"scope={self.scope!r}, node_types={repr_enums(self.node_types)}, store={self.store!r}"
        )

    @override
    async def connect(self, session: "Session") -> "PostgresChannel":
        from bench.sql.client import pg_store_connection

        conn = pg_store_connection(self.store)
        cur = await conn.open()
        return PostgresChannel(self, session, conn, cur)


class PostgresChannel(WritableChannel[PostgresEngine]):
    """A channel to a Postgres store (usually maps to a postgres connection)."""

    def __init__(
        self,
        engine: "PostgresEngine",
        session: "Session",
        conn: "PgStoreConnection",
        cur: psycopg.AsyncCursor,
    ):
        super().__init__(engine, session)
        self.conn = conn
        self.cur = cur

    def __str__(self):
        return f"engine={self.engine!r}, session={self.session}"

    @override
    def _get_connection_cls(
        self, query: "QueryBuilder", scope: GraphScopeData, options: Options
    ) -> type[Connection]:
        if query._read_type == ReadType.GET:
            return PostgresGetConnection
        elif query._read_type == ReadType.SEARCH:
            return PostgresSearchConnection
        elif query._read_type == ReadType.AGGREGATE:
            return PostgresAggregateConnection
        else:
            raise RuntimeError(f"unsupported read type {query._read_type}")

    @staticmethod
    def _pg_method(func):
        """Wraps a Postgres function with tracing & error wrapping."""
        method_name = func.__name__

        @wraps(func)
        @tracer.start_as_current_span(f"pg.{method_name}")
        async def wrapper(self: "PostgresChannel", *args, **kwargs):
            from bench.sql.engine import SqlConnectionError

            try:
                return await func(self, *args, **kwargs)
            except (SqlConnectionError, psycopg.OperationalError) as e:
                raise ChannelUnavailableError(self, args[0] if args else None, reason=str(e)) from e

        return wrapper

    @override
    @_pg_method
    async def flush(self, edits: list[EditData] | tuple[EditData, ...]) -> FlushResultData:
        from bench.sql.engine import pg_edit

        new_revisions, cascaded_edits = await pg_edit(cur=self.cur, edits=edits)
        return FlushResultData(revisions=new_revisions, cascaded_edits=cascaded_edits)

    @override
    @_pg_method
    async def commit(self, edits: list[EditData] | tuple[EditData, ...]) -> CommitResultData:
        from bench.sql.engine import pg_edit

        new_revisions, cascaded_edits = await pg_edit(cur=self.cur, edits=edits)
        await self.cur.connection.commit()
        return CommitResultData(revisions=new_revisions, cascaded_edits=cascaded_edits)

    @override
    @_pg_method
    async def close(self):
        if not self.cur.connection.broken:
            await self.cur.connection.rollback()
        await self.conn.close()


class PostgresGetConnection[T: Node](GetConnection[PostgresChannel, T]):
    """Get from a Postgres channel."""

    @override
    async def _do_read(self, query: "QueryBuilder") -> GetResultData:
        from bench.language import ReadOptions
        from bench.sql.engine import pg_get_node_graph

        assert query._roots, f"{query!r} has no roots"
        graph = NodeDataGraph(scope=self.scope, node_types=self.node_types)
        roots_ptr = [r._to_data() for r in query._roots]
        await pg_get_node_graph(
            cur=self.channel.cur,
            root_type=query._node_type,
            roots=roots_ptr,
            options=query._options or ReadOptions(),
            visited_graph=graph,
        )
        return GetResultData(
            graph=graph,
            roots_ptr=roots_ptr,
            epoch=None,
            connection_token=None,
        )


class PostgresSearchConnection[T: Node](SearchConnection[PostgresChannel, T]):
    """Search a Postgres channel."""

    @override
    async def _do_read(self, query: "QueryBuilder") -> SearchResultData:
        from bench.language import NodeReference, ReadOptions
        from bench.sql.engine import pg_compile_conditional_maybe, pg_count, pg_search_node_graph

        assert query._node_cls.__table__ is not None, f"{query._node_cls} has no table"
        roots, graph = await pg_search_node_graph(
            cur=self.channel.cur,
            scope=self.channel.engine.scope,
            node_type=query._node_type,
            options=query._options or ReadOptions(),
            filter=query._filter,
            sort=query._sort,
            first=query._first,
            skip=query._skip,
        )
        if self.options.count:
            total = await pg_count(
                cur=self.channel.cur,
                table=query._node_cls.__table__,
                where=pg_compile_conditional_maybe(query._node_cls, query._filter),
            )
        else:
            total = None
        return SearchResultData(
            graph=graph,
            roots=roots,
            roots_ptr=[NodeReference.from_node_data(r) for r in roots],
            total=total,
            epoch=None,
            connection_token=None,
        )


class PostgresAggregateConnection(AggregateConnection):
    """Aggregate a Postgres channel."""

    @override
    async def _do_read(self, query: "QueryBuilder") -> AggregateResultData:
        from bench.sql.engine import pg_compile_conditional_maybe, pg_count, pg_exists

        assert query._node_cls.__table__ is not None, f"{query._node_cls} has no table"
        assert query._aggregation is not None
        where = pg_compile_conditional_maybe(query._node_cls, query._filter)
        if query._aggregation.op == AggregationOp.EXISTS:
            exists = await pg_exists(
                cur=self.channel.cur, table=query._node_cls.__table__, where=where
            )
            return AggregateResultData(
                aggregation=AggregationData(exists=exists), epoch=None, connection_token=None
            )
        elif query._aggregation.op == AggregationOp.COUNT:
            count = await pg_count(
                cur=self.channel.cur, table=query._node_cls.__table__, where=where
            )
            return AggregateResultData(
                aggregation=AggregationData(count=count), epoch=None, connection_token=None
            )
        else:
            raise ChannelIncapableError(
                self, query, expression=query._aggregation, reason="unsupported"
            )

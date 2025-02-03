#
# Queries
#
import abc
from dataclasses import dataclass
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Collection,
    Literal,
    Mapping,
    Optional,
    Union,
    final,
)
from uuid import UUID

import structlog
from opentelemetry import trace

from bench.language.core import (
    BenchError,
    Node,
    NodeDataGraph,
    NodeGraph,
    NodeType,
    QueryType,
    bittuple,
    repr_enums,
    repr_scope,
)
from bench.pb2 import (
    AggregationResultData,
    AnyNodeData,
    ClientOriginData,
    EditData,
    GraphScopeData,
    NodeReferenceData,
)
from bench.utils.tenacity import RetryOptions

if TYPE_CHECKING:
    from bench.language import (
        AggregateConnection,
        AggregationResult,
        Connection,
        Expression,
        Field,
        GetConnection,
        Property,
        Query,
        SearchConnection,
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
        query: Optional["Query"] | Collection[EditData] = None,
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

    _engine_id: ClassVar[int] = 0

    def __init__(
        self,
        name: str,
        scope: GraphScopeData,
        node_types: bittuple[NodeType],
    ):
        self.id = self.__class__._engine_id
        self.__class__._engine_id += 1
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
        self, query: "Query", scope: GraphScopeData, options: ConnectionOptions
    ) -> "type[Connection]": ...

    @final
    async def get(self, query: "Query", options: GetOptions) -> "GetConnection":
        """Read a single node given the query in the current transaction context (if any)."""
        from .connection import GetConnection

        assert query._type == QueryType.GET, f"{query!r} is not a get"
        assert query._roots is not None, f"{query!r} has no roots"
        scope = self.session._get_scope_for_query(query)
        connection_cls = self._get_connection_cls(query, scope, options)
        connection = connection_cls(self, scope, query, self.read_retry, options)
        assert isinstance(connection, GetConnection), f"{connection!r} is not a get"
        await connection.connect()
        return connection

    @final
    async def search(self, query: "Query", options: SearchOptions) -> "SearchConnection":
        """Read the nodes given the search query in the current transaction context (if any)."""
        from .connection import SearchConnection

        assert query._type == QueryType.SEARCH, f"{query!r} is not a search"
        scope = self.session._get_scope_for_query(query)
        connection_cls = self._get_connection_cls(query, scope, options)
        connection = connection_cls(self, scope, query, self.read_retry, options)
        assert isinstance(connection, SearchConnection), f"{connection!r} is not a search"
        await connection.connect()
        return connection

    @final
    async def aggregate(self, query: "Query", options: AggregateOptions) -> "AggregateConnection":
        """Read the nodes given the aggregate query in the current transaction context (if any)."""
        from .connection import AggregateConnection

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

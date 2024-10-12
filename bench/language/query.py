#
# Queries
#
from itertools import chain
from typing import (
    TYPE_CHECKING,
    Any,
    Collection,
    Iterable,
    Optional,
    Sequence,
    TypeVar,
    Union,
    cast,
    overload,
)

import structlog
from opentelemetry import trace

from bench.language.connection import AggregateOptions, SearchConnection
from bench.language.const import (
    NODE_TYPES,
    AggregationOp,
    BenchError,
    ConditionalOp,
    ExpressionKind,
    NodeType,
    QueryType,
    StructType,
    active_session,
)
from bench.language.expression import C, Expression, coerce_conditional
from bench.language.node import (
    NODE_CLASS_BY_TYPE,
    BuiltinObject,
    Node,
    SomeNodeReference,
    SourceNode,
    Struct,
    local_node_,
    object_,
    struct_,
)
from bench.language.property import Property, p_node_parent, p_regular
from bench.language.setup import ANCESTOR_NODE_TYPES, NODE_CLASSES, _on_completing_setup
from bench.language.validation import NAME_CONSTRAINT
from bench.proto.wire import AnyNodeData, QueryData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import stable_hash

if TYPE_CHECKING:
    from bench.language import Block, Channel, Field, NodeReference, SelectOptions


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# default read options
FILTER_NOT_DELETED: Expression = C(ConditionalOp.AND, clauses=[])
SELECT_DEFAULT_PROPERTIES: dict[NodeType, tuple[Property, ...]] = {}
SELECT_ALL_PROPERTIES: dict[NodeType, tuple[Property, ...]] = {}


@_on_completing_setup
def _populate_default_query():
    FILTER_NOT_DELETED.clauses = [C(ConditionalOp.NOT_EXISTS, property=Node.deleted_at)]
    for node_t in NODE_CLASSES:
        SELECT_DEFAULT_PROPERTIES[node_t.metatype] = tuple(
            prop for prop in node_t.__stored_properties__.values() if not prop.is_deferred
        )
        SELECT_ALL_PROPERTIES[node_t.metatype] = tuple(node_t.__stored_properties__.values())


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false

NodeT = TypeVar("NodeT", bound=Node)
NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)
FieldOrProperty = Union[
    Field if TYPE_CHECKING else "Field", Property if TYPE_CHECKING else "Property", Any
]
NodeTypeOrClass = Union[NodeType, type[Node]]


@struct_(StructType.SELECT_OPTIONS)
class SelectOptions(Struct):
    """
    Fine-grained options to a read request specifying which properties/fields to load.
    """

    # properties (include/exclude relative to default OR select specific properties)
    select_all_properties: bool = p_regular(40, default=False)
    include_properties: list[Property] = p_regular(
        41, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    exclude_properties: list[Property] = p_regular(
        42, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    select_properties: list[Property] = p_regular(
        43, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )

    # fields
    select_all_fields: bool = p_regular(50, default=False)
    select_fields: list["Field"] = p_regular(
        51, require=True, array=True, references=NodeType.FIELD
    )

    def __content_str__(self) -> str:
        content_parts = []
        for key, prop in self.__declared_properties__.items():
            value = getattr(self, key)
            if value:
                if prop.is_enum:
                    value = "|".join(v.bench_name for v in value)
                content_parts.append(f"{prop.name}={value}")
        if content_parts:
            return ", ".join(content_parts)
        else:
            return "<default>"

    def get_properties(self, node_type: NodeType) -> list[Property] | tuple[Property, ...]:
        # NOTE :Performance: if len(exclude_properties) gets larger this will be pretty inefficient
        if self.select_all_properties:
            properties = SELECT_ALL_PROPERTIES[node_type]
            if self.exclude_properties:
                properties = tuple(
                    p
                    for p in properties
                    if not any(
                        e.component == p.component and e.id == p.id for e in self.exclude_properties
                    )
                )
            return properties
        elif self.select_properties:
            # select specific properties
            return tuple(p for p in self.select_properties if p.type == node_type)
        else:
            # select default properties +/- include/exclude
            properties = SELECT_DEFAULT_PROPERTIES[node_type]
            if self.include_properties:
                properties = properties + tuple(
                    p for p in self.include_properties if p.type == node_type
                )
            if self.exclude_properties:
                properties = tuple(
                    p for p in properties if not any(e.id == p.id for e in self.exclude_properties)
                )
            return properties

    @staticmethod
    def default():
        return SelectOptions()

    @staticmethod
    def all():
        return SelectOptions(select_all_properties=True, select_all_fields=True)


DEFAULT_SELECT_OPTIONS = SelectOptions.default()


class QueryError(BenchError, ValueError):
    def __init__(
        self, query: "QueryBuilder", result: Any | None = None, cause: Exception | None = None
    ):
        if result is None:
            super().__init__(repr(query))
        else:
            super().__init__(f"{query!r} -> {result}")
        self.query = query
        self.result = result
        self.cause = cause


class NodeNotFoundError(QueryError):
    pass


class MultipleNodesFoundError(QueryError):
    pass


class QueryBuilder[NodeT: Node, NodeDataT: AnyNodeData]:
    __slots__ = (
        "_after",
        "_aggregation",
        "_ancestor_types",
        "_block",
        "_descendant_types",
        "_filter",
        "_first",
        "_include_deleted",
        "_node_cls",
        "_node_type",
        "_roots",
        "_select",
        "_skip",
        "_sort",
        "_type",
    )

    def __init__(
        self,
        # root
        type: QueryType,
        node_type: NodeType,
        block: Optional["Block"] = None,
        roots: Optional[list["NodeReference"]] = None,
        filter: Optional["Expression"] = None,
        sort: list["Expression"] | None = None,
        aggregation: Optional["Expression"] = None,
        # joins
        ancestor_types: list[NodeType] | None = None,
        descendant_types: list[NodeType] | None = None,
        # options
        select: Optional["SelectOptions"] = None,
        include_deleted: bool = False,
        first: int | None = None,
        skip: int | None = None,
    ):
        from bench.language.node import NODE_CLASS_BY_TYPE, Node

        # root
        self._type = type
        self._node_type = node_type
        self._node_cls = NODE_CLASS_BY_TYPE[node_type] if node_type else Node
        self._block = block
        self._roots = roots
        self._filter = filter
        self._sort = sort
        self._aggregation = aggregation
        # joins
        self._ancestor_types = ancestor_types or []
        self._descendant_types = descendant_types or []
        # options
        self._select = select
        self._include_deleted = include_deleted
        self._first = first
        self._skip = skip

    def __str__(self):
        content_parts = []
        if self._block:
            content_parts.append(self._block.absolute_path)
        if self._roots is not None:
            content_parts.append(f"roots=[{', '.join(str(r) for r in self._roots)}]")
        for k in ("filter", "sort", "first", "skip", "aggregation"):
            v = getattr(self, f"_{k}", None)
            if k == "query":
                v = f"({v})" if v is not None else None
            if v is not None and not (type(v) is list and len(v) == 0):
                content_parts.append(f"{k}={v}")

        if self._ancestor_types:
            ancestors_str = "|".join(a.bench_name for a in self._ancestor_types)
            content_parts.append(f"ancestors={ancestors_str}")
        if self._descendant_types:
            descendants_str = "|".join(d.bench_name for d in self._descendant_types)
            content_parts.append(f"descendants={descendants_str}")

        return ", ".join(content_parts) if content_parts else "<empty>"

    def __repr__(self):
        return f"<{self._node_type.bench_name}Query.{self._type.bench_name} {self}>"

    def _stable_hash(self):
        return stable_hash(
            # self._type,
            # self._node_type,
            # self._block._stable_hash() if self._block is not None else None,
            # self._filter._stable_hash() if self._filter is not None else None,
            # tuple(r._stable_hash() for r in self._roots) if self._roots is not None else None,
            # tuple(s._stable_hash() for s in self._sort) if self._sort is not None else None,
            # self._first,
            # self._skip,
            # self._aggregation._stable_hash() if self._aggregation is not None else None,
            # self._select._stable_hash() if self._select is not None else None,
            # root
            self._type,
            self._node_type,
            self._block._stable_hash() if self._block is not None else None,
            tuple(r._stable_hash() for r in self._roots) if self._roots is not None else None,
            self._filter._stable_hash() if self._filter is not None else None,
            tuple(s._stable_hash() for s in self._sort) if self._sort is not None else None,
            self._aggregation._stable_hash() if self._aggregation is not None else None,
            # joins
            self._ancestor_types,
            self._descendant_types,
            # options
            self._select._stable_hash() if self._select is not None else None,
            self._include_deleted,
            self._first,
            self._skip,
        )

    @property
    def all_node_types(self) -> Iterable[NodeType]:
        return chain((self._node_type,), self._ancestor_types, self._descendant_types)

    @property
    def is_aggregation(self) -> bool:
        return self._aggregation is not None

    @property
    def include_deleted(self) -> bool:
        return self._include_deleted

    #
    # Builder
    #

    def clone(self):
        """Clones the query (the properties are immutable)."""
        return QueryBuilder(
            # root
            type=self._type,
            node_type=self._node_type,
            block=self._block,
            roots=self._roots,
            filter=self._filter,
            sort=self._sort,
            aggregation=self._aggregation,
            # joins
            ancestor_types=self._ancestor_types,
            descendant_types=self._descendant_types,
            # options
            select=self._select.clone() if self._select is not None else None,
            include_deleted=self._include_deleted,
            first=self._first,
            skip=self._skip,
        )

    def _clone_select(self) -> "SelectOptions":
        if self._select is None:
            return SelectOptions()
        else:
            return self._select.clone()

    def trim_to(self, node_types: Collection[NodeType]) -> "QueryBuilder[NodeT, NodeDataT]":
        assert self._node_type in node_types
        clone = self.clone()
        clone._ancestor_types = [a for a in self._ancestor_types if a in node_types]
        clone._descendant_types = [d for d in self._descendant_types if d in node_types]
        return clone

    def where(
        self, filter: Optional["Expression"] = None, **kwargs
    ) -> "QueryBuilder[NodeT, NodeDataT]":
        """Adds a filter clause to the query."""
        from bench.language.expression import coerce_conditional

        filter = coerce_conditional(self._node_cls, filter, kwargs)
        clone = self.clone()
        clone._filter = (
            filter & self._filter if filter is not None and self._filter is not None else filter
        )
        return clone

    def order_by(
        self,
        sort: Union[list[Union["Expression", str]], str, "Expression", None] = None,
        *args: str,
    ) -> "QueryBuilder[NodeT, NodeDataT]":
        """Sorts the query results by the given sort criteria."""
        from bench.language.expression import coerce_sort

        clone = self.clone()
        clone._sort = coerce_sort(self._node_cls, sort, *args)
        return clone

    def first(self, count: int) -> "QueryBuilder[NodeT, NodeDataT]":
        """Returns the first N results."""
        clone = self.clone()
        clone._first = count
        return clone

    limit = first

    def skip(self, count: int) -> "QueryBuilder[NodeT, NodeDataT]":
        """Skips the first N results."""
        clone = self.clone()
        clone._skip = count
        return clone

    def aggregate(self, aggregation: "Expression") -> "QueryBuilder[NodeT, NodeDataT]":
        """Aggregates the query results."""
        assert aggregation.kind == ExpressionKind.AGGREGATION, f"not an aggregation: {aggregation}"
        clone = self.clone()
        clone._aggregation = aggregation
        return clone

    def include(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        """Includes given default-excluded properties in the results."""
        clone = self.clone()
        clone._select = self._clone_select()
        clone._select.include_properties += self._to_properties(properties)
        return clone

    def select_all(self) -> "QueryBuilder[NodeT, NodeDataT]":
        """Includes all (non-relational) properties in the results."""
        clone = self.clone()
        clone._select = self._clone_select()
        clone._select.select_all_properties = True
        return clone

    def exclude(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        """Excludes given default-included properties from the results."""
        clone = self.clone()
        clone._select = self._clone_select()
        clone._select.exclude_properties += self._to_properties(properties)
        return clone

    def include_ancestors(self, *node_types: NodeTypeOrClass) -> "QueryBuilder[NodeT, NodeDataT]":
        """Includes all ancestors in the results."""
        # not quite happy with this API for getting a 'full' node yet, see :LoadOrphanNode
        clone = self.clone()
        for node_type in node_types or ANCESTOR_NODE_TYPES[self._node_type]:
            if not isinstance(node_type, NodeType):
                node_type = node_type.metatype
            if node_type not in clone._ancestor_types:
                clone._ancestor_types.append(node_type)
        return clone

    def include_descendants(self, *node_types: NodeTypeOrClass) -> "QueryBuilder[NodeT, NodeDataT]":
        """Joins the given descendants in the results."""
        clone = self.clone()
        for node_type in node_types:
            if not isinstance(node_type, NodeType):
                node_type = node_type.metatype
            if node_type not in clone._descendant_types:
                clone._descendant_types.append(node_type)
        return clone

    #
    # Read
    #

    def __getitem__(self, item: slice) -> Union["QueryBuilder[NodeT, NodeDataT]", NodeT]:  # type: ignore
        if isinstance(item, slice):
            if item.stop is None:
                return self.skip(item.start or 0)
            elif item.start is not None:
                return self.skip(item.start).first(item.stop - item.start)
            else:
                return self.first(item.stop)
        else:
            raise TypeError(f"expected slice into {self!r}, got {type(item)}: {item}")

    async def __aiter__(self):
        return iter(await self.search())

    async def _get_read_channel(self) -> "Channel":
        session = active_session()
        scope = session._get_scope_for_query(self)
        return await session._get_channel_for(
            scope,
            self.all_node_types,
            include_deleted=self.include_deleted,
            is_readonly=True,
        )

    @overload
    async def get(
        self,
        filter: Union["Expression", "SomeNodeReference", None] = None,
        live: bool = False,
        **kwargs,
    ) -> NodeT: ...
    @overload
    async def get(
        self, filter: Sequence["SomeNodeReference"], live: bool = False, **kwargs
    ) -> list[NodeT]: ...
    @tracer.start_as_current_span("query.get")
    async def get(
        self,
        filter: Union[
            "Expression", "SomeNodeReference", Sequence["SomeNodeReference"], None
        ] = None,
        live: bool = False,
        **kwargs,
    ) -> NodeT | list[NodeT]:
        """
        Returns the unique result matching the query (one or multiple nodes, errors otherwise).
        NOTE: this is only a true get query with specific node pointers as roots, otherwise it's a search.
        """
        from bench.language import GetOptions, NodeReferenceBase, coerce_conditional

        if isinstance(filter, (NodeReferenceBase, Sequence)):
            # true get request (with node pointers)
            assert self._filter is None, f"cannot combine filter and roots in {self!r}"
            query = self.clone()
            query._roots = [filter] if isinstance(filter, NodeReferenceBase) else list(filter)
            query._type = QueryType.GET
            channel = await query._get_read_channel()
            connection = await channel.get(query, GetOptions(unpack=True, live=live))

            # coerce to node/list of nodes
            if len(connection.result.roots) != len(query._roots):
                if len(connection.result.roots) < len(query._roots):
                    raise NodeNotFoundError(query=query)
                else:
                    raise MultipleNodesFoundError(query=query, result=connection.result.roots)
            if isinstance(filter, NodeReferenceBase):  # keep single node
                node = connection.result.roots[0]
                return cast(NodeT, node)
            else:
                return cast(list[NodeT], connection.result.roots)
        else:
            # search which should only have one result
            filter = coerce_conditional(self._node_cls, filter, kwargs)
            query = self.where(filter) if filter is not None else self.clone()
            results = await query.search()
            if len(results) != 1:
                if len(results) == 0:
                    raise NodeNotFoundError(query=query)
                else:
                    raise MultipleNodesFoundError(query=query, result=results)
            return results[0]  # success

    @tracer.start_as_current_span("query.search")
    async def search(self, filter: Optional["Expression"] = None, **kwargs) -> list[NodeT]:
        """Fetches the nodes matching the query."""
        from bench.language.connection import SearchOptions

        filter = coerce_conditional(self._node_cls, filter, kwargs)
        query = self.where(filter) if filter is not None else self
        channel = await query._get_read_channel()
        connection = await channel.search(
            query, SearchOptions(live=False, unpack=True, count=False)
        )
        return cast(list[NodeT], connection.result.roots)

    @tracer.start_as_current_span("query.search")
    async def search_live(
        self, filter: Optional["Expression"] = None, **kwargs
    ) -> "SearchConnection[Any, NodeT]":
        """Fetches the nodes matching the query (live)."""
        from bench.language.connection import SearchOptions

        filter = coerce_conditional(self._node_cls, filter, kwargs)
        query = self.where(filter) if filter is not None else self
        channel = await query._get_read_channel()
        connection = await channel.search(query, SearchOptions(live=True, unpack=True, count=False))
        return connection

    tolist = search  # type: ignore
    to_list = search  # type: ignore

    @tracer.start_as_current_span("query.count")
    async def count(self, filter: Optional["Expression"] = None, **kwargs) -> int:
        """Returns the number of results."""
        from bench.language.expression import A, coerce_conditional

        # prepare
        filter = coerce_conditional(self._node_cls, filter, kwargs)
        query = self.where(filter) if filter is not None else self
        query = query.aggregate(A(AggregationOp.COUNT))
        query._type = QueryType.AGGREGATE
        trace.get_current_span().set_attribute("query", repr(query))

        # query
        channel = await query._get_read_channel()
        connection = await channel.aggregate(query, AggregateOptions(live=False, unpack=False))
        aggregation = connection.result_data.aggregation
        assert aggregation.count is not None, f"missing count in {connection!r}"
        return aggregation.count

    @tracer.start_as_current_span("query.exists")
    async def exists(self, filter: Optional["Expression"] = None, **kwargs) -> bool:
        """Whether any nodes match the query."""
        from bench.language.expression import A, coerce_conditional

        # prepare
        filter = coerce_conditional(self._node_cls, filter, kwargs)
        query = self.where(filter) if filter is not None else self
        query = query.aggregate(A(AggregationOp.EXISTS))
        query._type = QueryType.AGGREGATE
        trace.get_current_span().set_attribute("query", repr(query))

        # query
        channel = await query._get_read_channel()
        connection = await channel.aggregate(query, AggregateOptions(live=False, unpack=False))
        aggregation = connection.result_data.aggregation
        assert aggregation.exists is not None, f"missing exists in {connection!r}"
        return aggregation.exists

    @tracer.start_as_current_span("query.scalar")
    async def scalar(self, *properties: "str | Property") -> Any:
        """Returns a single value from the single result (error if None)."""
        assert properties, "expected at least one property"
        properties_names = tuple(p.name if not isinstance(p, str) else p for p in properties)
        node = await self.get()
        if len(properties) == 1:
            return getattr(node, properties_names[0])
        else:
            return tuple(getattr(node, p) for p in properties_names)

    @tracer.start_as_current_span("query.scalar_maybe")
    async def scalar_maybe(self, *properties: "str | Property") -> Any | None:
        """Returns a single value from the single result, or None if no result."""
        assert properties, "expected at least one property"
        properties_names = tuple(p.name if not isinstance(p, str) else p for p in properties)
        results = await self.search()
        if len(results) == 1:
            node = results[0]
            if len(properties) == 1:
                return getattr(node, properties_names[0])
            else:
                return tuple(getattr(node, p) for p in properties_names)
        elif len(results) == 0:
            return None
        else:
            raise MultipleNodesFoundError(query=self, result=results)

    @tracer.start_as_current_span("query.scalar_list")
    async def scalar_list(self, *properties: "str | Property") -> list[Any]:
        """Returns a list of values from the results."""
        assert properties, "expected at least one property"
        properties_names = tuple(p.name if not isinstance(p, str) else p for p in properties)
        if len(properties) == 1:
            return [getattr(node, properties_names[0]) for node in await self.search()]
        else:
            return [
                tuple(getattr(node, p) for p in properties_names) for node in await self.search()
            ]

    def _to_properties(self, properties: tuple[FieldOrProperty, ...]) -> tuple["Property", ...]:
        if not all(isinstance(p, Property) for p in properties):
            raise NotImplementedError(f"fields not yet supported in {self!r}, got {properties}")
        return cast(tuple["Property"], properties)


@object_()
class QueryInfoBase(BuiltinObject):
    """An object we can build query from."""

    # root
    type: QueryType = p_regular(40)
    node_type: NodeType = p_regular(41)
    block: Optional["Block"] = p_regular(
        42, array=False, require=False, default=None, references=NodeType.BLOCK
    )
    roots: list[Node] = p_regular(43, require=True, array=True, references=NODE_TYPES.tuple)
    filter: Optional["Expression"] = p_regular(44, default=None, struct=StructType.EXPRESSION)
    sort: Optional[list["Expression"]] = p_regular(
        45, default=None, array=True, struct=StructType.EXPRESSION
    )
    aggregation: Optional["Expression"] = p_regular(46, default=None, struct=StructType.EXPRESSION)

    # joins
    ancestor_types: list[NodeType] = p_regular(50, require=True, array=True)
    descendant_types: list[NodeType] = p_regular(51, require=True, array=True)
    # joins, ...?

    # options
    select: Optional["SelectOptions"] = p_regular(
        60, default=None, struct=StructType.SELECT_OPTIONS
    )
    include_deleted: bool = p_regular(61, default=False)
    first: int | None = p_regular(62, default=None)
    skip: int | None = p_regular(63, default=None)


@struct_(StructType.QUERY_INFO)
class QueryInfo(Struct, QueryInfoBase):
    """A stored query."""

    pass


@local_node_(NodeType.QUERY)
class Query(SourceNode[QueryData], QueryInfoBase):
    """A stored query with identity."""

    # NOTE :Architecture: should Query be just a Struct or remain a Node?

    parent: "Block | None" = p_node_parent(4, NodeType.BLOCK)
    name: str = p_regular(30, constraint=NAME_CONSTRAINT)
    order_key: str = p_regular(31, default=INTEGER_ZERO)

    # ...QueryInfoBase[40-59]

    def __content_str__(self):
        return f"{self.node_type}[{self.filter}, {self.sort or '<default sort>'}]"

    @property
    def node_cls(self) -> type[Node]:
        return NODE_CLASS_BY_TYPE[self.node_type]

    def build(self) -> "QueryBuilder":
        return QueryBuilder(
            # root
            type=self.type,
            node_type=self.node_type,
            block=self.block,
            filter=self.filter,
            sort=self.sort,
            first=None,
            skip=None,
            aggregation=None,
            select=self.select,
        )

    # ... ReadQueryBase methods

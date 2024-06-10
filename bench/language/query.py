#
# Queries
#
import abc
from itertools import chain
from typing import (
    TYPE_CHECKING,
    Any,
    Collection,
    Generic,
    Iterable,
    Optional,
    Self,
    TypeVar,
    Union,
    cast,
    override,
)

import structlog
from opentelemetry import trace

from bench.language.const import (
    AggregationOp,
    BenchError,
    ConditionalOp,
    ExpressionKind,
    NodeType,
    StructType,
    active_tx,
)
from bench.language.expression import C, Expression
from bench.language.graph import NodeDataGraph
from bench.language.node import (
    NODE_CLASS_BY_TYPE,
    InlineStruct,
    Node,
    ReadInfo,
    SourceNode,
    node_,
    struct_,
)
from bench.language.property import Property, p_node_parent, p_regular
from bench.language.setup import ANCESTOR_NODE_TYPES, NODE_CLASSES, _on_completing_setup
from bench.language.validation import NAME_CONSTRAINT
from bench.proto.wire import AnyNodeData, QueryData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import Block, Field, ReadOptions


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# default read options
FILTER_VISIBLE: Expression = C(ConditionalOp.AND, clauses=[])
SELECT_DEFAULT_PROPERTIES: dict[NodeType, tuple[Property, ...]] = {}
SELECT_ALL_PROPERTIES: dict[NodeType, tuple[Property, ...]] = {}


@_on_completing_setup
def _populate_default_query():
    FILTER_VISIBLE.clauses = [
        C(ConditionalOp.NOT_EXISTS, property=Node.deleted_at),
        C(ConditionalOp.NOT_EXISTS, property=Node.archived_at),
    ]
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


@struct_(StructType.READ_OPTIONS, inline=True)
class ReadOptions(InlineStruct):
    """
    Fine-grained options to a read request.
    This is an addition to primary options (like the filter for a search or aggregation).
    """

    # relations
    ancestor_types: list[NodeType] = p_regular(31, array=True, require=False)
    descendant_types: list[NodeType] = p_regular(32, array=True, require=False)
    related_properties: list[Property] = p_regular(
        33, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )

    # properties (include/exclude relative to default OR select specific properties)
    include_properties: list[Property] = p_regular(
        40, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    exclude_properties: list[Property] = p_regular(
        41, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    select_properties: list[Property] = p_regular(
        42, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )
    select_all_properties: bool = p_regular(43, default=False)

    # filters (simplified for now)
    include_hidden: bool = p_regular(50, default=False)

    def __content_str__(self) -> str:
        content_parts = []
        for key, prop in self.__declared_properties__.items():
            value = getattr(self, key)
            if value:
                content_parts.append(f"{prop.name}={value}")
        if content_parts:
            return ", ".join(content_parts)
        else:
            return "<default>"

    def copy(self) -> "ReadOptions":
        return ReadOptions(
            ancestor_types=list(self.ancestor_types),
            descendant_types=list(self.descendant_types),
            related_properties=list(self.related_properties),
            include_properties=list(self.include_properties),
            exclude_properties=list(self.exclude_properties),
            select_properties=list(self.select_properties),
            select_all_properties=self.select_all_properties,
            include_hidden=self.include_hidden,
        )

    def trim_to(self, node_types: Collection[NodeType]) -> "ReadOptions":
        copy = self.copy()
        copy.ancestor_types = [t for t in self.ancestor_types if t in node_types]
        copy.descendant_types = [t for t in self.descendant_types if t in node_types]
        return copy

    def related(self, node_type: NodeType) -> list[Property] | tuple[Property, ...]:
        return tuple(p for p in self.related_properties if p.type == node_type)

    def select(self, node_type: NodeType) -> list[Property] | tuple[Property, ...]:
        # NOTE :Performance: if len(exclude_properties) gets larger this will be pretty inefficient
        if self.select_all_properties:
            properties = SELECT_ALL_PROPERTIES[node_type]
            if self.exclude_properties:
                properties = tuple(
                    p for p in properties if not any(e.id == p.id for e in self.exclude_properties)
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

    def filter(
        self, node_type: NodeType, custom_filter: Optional["Expression"] = None
    ) -> "Expression | None":
        if self.include_hidden:
            if custom_filter is None:
                return None
            else:
                return custom_filter
        else:
            if custom_filter is None:
                return FILTER_VISIBLE
            else:
                return FILTER_VISIBLE & custom_filter

    @staticmethod
    def default():
        """Read default: exclude soft delete & archived, select all non-deferred properties."""
        return ReadOptions()

    @staticmethod
    def all():
        """Read all: include everything, select all properties."""
        return ReadOptions(include_hidden=True, select_all_properties=True)


DEFAULT_READ_OPTIONS = ReadOptions.default()


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


class MakeQueryBase(abc.ABC, Generic[NodeT, NodeDataT]):
    """Build or modify a query. :ReadQueryBase"""

    @abc.abstractmethod
    def where(
        self, filter: Optional["Expression"] = None, **kwargs
    ) -> "QueryBuilder[NodeT, NodeDataT]":
        """Adds a filter clause to the query."""
        raise NotImplementedError

    @abc.abstractmethod
    def order_by(
        self,
        sort: Union[list[Union["Expression", str]], str, "Expression", None] = None,
        *args: str,
    ) -> "QueryBuilder[NodeT, NodeDataT]":
        """Sorts the query results by the given sort criteria."""
        raise NotImplementedError

    @abc.abstractmethod
    def first(self, count: int) -> "QueryBuilder[NodeT, NodeDataT]":
        """Returns the first N results."""
        raise NotImplementedError

    limit = first

    @abc.abstractmethod
    def skip(self, count: int) -> "QueryBuilder[NodeT, NodeDataT]":
        """Skips the first N results."""
        raise NotImplementedError

    @abc.abstractmethod
    def after(self, cursor: str) -> "QueryBuilder[NodeT, NodeDataT]":
        """Paginate using an opaque cursor."""
        raise NotImplementedError

    @abc.abstractmethod
    def aggregate(self, aggregation: "Expression") -> "QueryBuilder[NodeT, NodeDataT]":
        """Aggregates the query results."""
        raise NotImplementedError

    @abc.abstractmethod
    def include(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        """Includes given default-excluded properties in the results."""
        raise NotImplementedError

    @abc.abstractmethod
    def include_ancestors(self) -> "QueryBuilder[NodeT, NodeDataT]":
        """Includes all ancestors in the results."""
        raise NotImplementedError

    @abc.abstractmethod
    def select_all(self) -> "QueryBuilder[NodeT, NodeDataT]":
        """Includes all (non-relational) properties in the results."""
        raise NotImplementedError

    @abc.abstractmethod
    def exclude(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        """Excludes given default-included properties from the results."""
        raise NotImplementedError

    @abc.abstractmethod
    def related(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        """Joins the given related properties in the results."""
        raise NotImplementedError

    @abc.abstractmethod
    def ancestors(self, *node_types: NodeTypeOrClass) -> "QueryBuilder[NodeT, NodeDataT]":
        """Joins the given ancestors in the results."""
        raise NotImplementedError

    @abc.abstractmethod
    def descendants(self, *node_types: NodeTypeOrClass) -> "QueryBuilder[NodeT, NodeDataT]":
        """Joins the given descendants in the results."""
        raise NotImplementedError


class ReadQueryBase(abc.ABC, Generic[NodeT, NodeDataT]):
    """Fetch the nodes matching a query. :ReadQueryBase"""

    @abc.abstractmethod
    async def __aiter__(self):
        raise NotImplementedError

    @abc.abstractmethod
    def __len__(self):
        raise NotImplementedError

    @abc.abstractmethod
    def __getitem__(self, item: slice | int) -> Union[Self, NodeT]:
        raise NotImplementedError

    @abc.abstractmethod
    async def tolist(self) -> list[NodeT]:
        raise NotImplementedError

    @abc.abstractmethod
    async def scalar_list(self, *properties: str) -> list[Any]:
        """Returns a list of values from the results."""
        raise NotImplementedError

    @abc.abstractmethod
    async def get(self, filter: Optional["Expression"] = None, **kwargs) -> NodeT:
        """Returns the unique result matching the query (errors otherwise)."""
        raise NotImplementedError

    @abc.abstractmethod
    async def scalar(self, *properties: str) -> Any:
        """Returns a single value from the single result."""
        raise NotImplementedError

    @abc.abstractmethod
    async def scalar_maybe(self, *properties: str) -> Any | None:
        """Returns a single value from the single result, or None if no result."""
        raise NotImplementedError

    @abc.abstractmethod
    async def count(self, filter: Optional["Expression"] = None, **kwargs) -> int:
        """Returns the number of results."""
        raise NotImplementedError

    @abc.abstractmethod
    async def exists(self, filter: Optional["Expression"] = None, **kwargs) -> bool:
        raise NotImplementedError


class QueryBuilder(
    Generic[NodeT, NodeDataT], MakeQueryBase[NodeT, NodeDataT], ReadQueryBase[NodeT, NodeDataT]
):
    __slots__ = (
        "_after",
        "_aggregation",
        "_base",
        "_filter",
        "_first",
        "_node_cls",
        "_node_type",
        "_options",
        "_skip",
        "_sort",
    )

    def __init__(
        self,
        node_type: NodeType,
        base: Optional["Block"] = None,
        filter: Optional["Expression"] = None,
        sort: list["Expression"] | None = None,
        first: int | None = None,
        skip: int | None = None,
        after: str | None = None,
        aggregation: Optional["Expression"] = None,
        options: Optional["ReadOptions"] = None,
    ):
        from bench.language.node import NODE_CLASS_BY_TYPE, Node

        self._node_type = node_type
        self._node_cls = NODE_CLASS_BY_TYPE[node_type] if node_type else Node
        self._base = base
        self._filter = filter
        self._sort = sort
        self._first = first
        self._skip = skip
        self._after = after
        self._aggregation = aggregation
        self._options = options

    def __str__(self):
        args_strs = []
        if self._base:
            args_strs.append(self._base.absolute_path)
        for k in ("filter", "sort", "first", "skip", "aggregation"):
            v = getattr(self, f"_{k}", None)
            if k == "query":
                v = f"({v})" if v is not None else None
            if v is not None and not (type(v) is list and len(v) == 0):
                args_strs.append(f"{k}={v}")
        args_str = ", ".join(args_strs) if args_strs else "[*]"
        return args_str

    def __repr__(self):
        query_type = self._aggregation.op.bench_name if self._aggregation is not None else "Fetch"
        return f"<{self._node_type.bench_name}Query.{query_type} {self}>"

    @property
    def all_types(self) -> Iterable[NodeType]:
        if self._options is None:
            return (self._node_type,)
        else:
            return chain(
                (self._node_type,), self._options.ancestor_types, self._options.descendant_types
            )

    @property
    def is_aggregation(self) -> bool:
        return self._aggregation is not None

    #
    # Builder
    #

    def copy(self):
        """Clones the query (the properties are immutable)."""
        return QueryBuilder(
            node_type=self._node_type,
            base=self._base,
            filter=self._filter,
            sort=self._sort,
            first=self._first,
            skip=self._skip,
            aggregation=self._aggregation,
            options=self._options,
        )

    def _copy_options(self) -> "ReadOptions":
        if self._options is None:
            return ReadOptions()
        else:
            return self._options.copy()

    def trim_to(self, node_types: Collection[NodeType]) -> "QueryBuilder[NodeT, NodeDataT]":
        assert self._node_type in node_types
        copy = self.copy()
        if not self._options:
            return copy
        copy._options = self._options.trim_to(node_types)
        return copy

    @override
    def where(
        self, filter: Optional["Expression"] = None, **kwargs
    ) -> "QueryBuilder[NodeT, NodeDataT]":
        from bench.language.expression import coerce_conditional

        filter = coerce_conditional(self._node_cls, filter, kwargs)
        copy = self.copy()
        copy._filter = (
            filter & self._filter if filter is not None and self._filter is not None else filter
        )
        return copy

    @override
    def order_by(
        self,
        sort: Union[list[Union["Expression", str]], str, "Expression", None] = None,
        *args: str,
    ) -> "QueryBuilder[NodeT, NodeDataT]":
        from bench.language.expression import coerce_sort

        copy = self.copy()
        copy._sort = coerce_sort(self._node_cls, sort, *args)
        return copy

    @override
    def first(self, count: int) -> "QueryBuilder[NodeT, NodeDataT]":
        copy = self.copy()
        copy._first = count
        return copy

    limit = first

    @override
    def skip(self, count: int) -> "QueryBuilder[NodeT, NodeDataT]":
        copy = self.copy()
        copy._skip = count
        return copy

    @override
    def after(self, cursor: str) -> "QueryBuilder[NodeT, NodeDataT]":
        copy = self.copy()
        copy._after = cursor
        return copy

    @override
    def aggregate(self, aggregation: "Expression") -> "QueryBuilder[NodeT, NodeDataT]":
        assert aggregation.kind == ExpressionKind.AGGREGATION, f"not an aggregation: {aggregation}"
        copy = self.copy()
        copy._aggregation = aggregation
        return copy

    @staticmethod
    def _to_properties(properties: tuple[FieldOrProperty, ...]) -> tuple["Property", ...]:
        return cast(tuple["Property"], properties)

    @staticmethod
    def _to_node_types(node_types: tuple[NodeTypeOrClass, ...]) -> list[NodeType]:
        return [cast(type[Node], t).metatype if isinstance(t, type) else t for t in node_types]

    @override
    def include(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.include_properties.extend(self._to_properties(properties))
        return copy

    @override
    def select_all(self) -> "QueryBuilder[NodeT, NodeDataT]":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.select_all_properties = True
        return copy

    @override
    def exclude(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.exclude_properties.extend(self._to_properties(properties))
        return copy

    @override
    def related(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.related_properties.extend(self._to_properties(properties))
        return copy

    @override
    def include_ancestors(self) -> "QueryBuilder[NodeT, NodeDataT]":
        # not quite happy with this API for getting a 'full' node yet, see :LoadOrphanNode
        ancestors = ANCESTOR_NODE_TYPES[self._node_type]
        return self.ancestors(*ancestors)

    @override
    def ancestors(self, *node_types: NodeTypeOrClass) -> "QueryBuilder[NodeT, NodeDataT]":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.ancestor_types = self._to_node_types(node_types)
        return copy

    @override
    def descendants(self, *node_types: NodeTypeOrClass) -> "QueryBuilder[NodeT, NodeDataT]":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.descendant_types = self._to_node_types(node_types)
        return copy

    #
    # Fetch
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
        return iter(await self.fetch())

    def __len__(self):
        return self.count()

    @override
    @tracer.start_as_current_span("query.get")
    async def get(self, filter: Optional["Expression"] = None, **kwargs) -> NodeT:
        from bench.language.expression import coerce_conditional

        filter = coerce_conditional(self._node_cls, filter, kwargs)
        query = self.where(filter) if filter is not None else self
        trace.get_current_span().set_attribute("query", repr(query))
        results = await query.fetch()
        if len(results) == 1:
            return results[0]
        else:
            if len(results) == 0:
                raise NodeNotFoundError(query=query)
            else:
                raise MultipleNodesFoundError(query=query, result=results)

    @tracer.start_as_current_span("query.fetch")
    async def fetch(self) -> list[NodeT] | tuple[NodeT, ...]:
        from bench.language.connection import FetchOptions
        from bench.proto.wiring import unpack_node_roots

        tx = active_tx()
        result = await tx._read_connection.fetch(self, FetchOptions())
        data_graph = NodeDataGraph(result.nodes)
        read = ReadInfo(options=self._options, epoch=result.epoch, graph=data_graph)
        roots, _ = unpack_node_roots(
            data_graph, parent=self._base, session=tx.session, roots=result.roots, read=read
        )
        return cast(tuple[NodeT, ...], roots)

    tolist = fetch  # type: ignore
    to_list = fetch  # type: ignore

    @override
    @tracer.start_as_current_span("query.count")
    async def count(self, filter: Optional["Expression"] = None, **kwargs) -> int:
        from bench.language.expression import A, coerce_conditional

        tx = active_tx()
        filter = coerce_conditional(self._node_cls, filter, kwargs)
        query = self.where(filter) if filter is not None else self
        query = query.aggregate(A(AggregationOp.COUNT))
        trace.get_current_span().set_attribute("query", repr(query))
        result = await tx._read_connection.aggregate(query)
        assert result.aggregation.count is not None, f"missing count in {result!r}"
        return result.aggregation.count

    @override
    @tracer.start_as_current_span("query.exists")
    async def exists(self, filter: Optional["Expression"] = None, **kwargs) -> bool:
        from bench.language.expression import A, coerce_conditional

        tx = active_tx()
        filter = coerce_conditional(self._node_cls, filter, kwargs)
        query = self.where(filter) if filter is not None else self
        query = query.aggregate(A(AggregationOp.EXISTS))
        trace.get_current_span().set_attribute("query", repr(query))
        result = await tx._read_connection.aggregate(query)
        assert result.aggregation.exists is not None, f"missing exists in {result!r}"
        return result.aggregation.exists

    @override
    @tracer.start_as_current_span("query.scalar")
    async def scalar(self, *properties: "str | Property") -> Any:
        assert properties, "expected at least one property"
        properties_names = tuple(p.name if not isinstance(p, str) else p for p in properties)
        node = await self.get()
        if len(properties) == 1:
            return getattr(node, properties_names[0])
        else:
            return tuple(getattr(node, p) for p in properties_names)

    @override
    @tracer.start_as_current_span("query.scalar_maybe")
    async def scalar_maybe(self, *properties: "str | Property") -> Any | None:
        assert properties, "expected at least one property"
        properties_names = tuple(p.name if not isinstance(p, str) else p for p in properties)
        results = await self.fetch()
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

    @override
    @tracer.start_as_current_span("query.scalar_list")
    async def scalar_list(self, *properties: "str | Property") -> list[Any]:
        assert properties, "expected at least one property"
        properties_names = tuple(p.name if not isinstance(p, str) else p for p in properties)
        if len(properties) == 1:
            return [getattr(node, properties_names[0]) for node in await self.fetch()]
        else:
            return [
                tuple(getattr(node, p) for p in properties_names) for node in await self.fetch()
            ]


@node_(NodeType.QUERY)
class Query(SourceNode[QueryData]):
    """A stored query."""

    parent: "Block" = p_node_parent(4, NodeType.BLOCK)
    name: str = p_regular(30, constraint=NAME_CONSTRAINT)
    order_key: str = p_regular(31, default=INTEGER_ZERO)
    node_type: NodeType = p_regular(32)
    base: Optional["Block"] = p_regular(
        33, array=False, require=False, default=None, references=NodeType.BLOCK
    )
    filter: Optional["Expression"] = p_regular(34, default=None, struct=StructType.EXPRESSION)
    sort: Optional[list["Expression"]] = p_regular(
        35, default=None, array=True, struct=StructType.EXPRESSION
    )

    def __content_str__(self):
        return f"{self.node_type}[{self.filter}, {self.sort or '<default sort>'}]"

    @property
    def node_cls(self) -> type[Node]:
        return NODE_CLASS_BY_TYPE[self.node_type]

    def build(self) -> "QueryBuilder":
        return QueryBuilder(
            node_type=self.node_type,
            base=self.base,
            filter=self.filter,
            sort=self.sort,
            first=None,
            skip=None,
            aggregation=None,
            options=None,
        )

    # ... ReadQueryBase methods

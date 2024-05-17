#
# Queries
#
import abc
from typing import (
    TYPE_CHECKING,
    Any,
    Generic,
    Optional,
    Self,
    TypeVar,
    Union,
    cast,
)

from bench.language.const import (
    AccessKind,
    AggregationOp,
    BenchError,
    ExpressionKind,
    NodeType,
    StructType,
    active_tx,
)
from bench.language.graph import NodeDataGraph
from bench.language.node import NODE_CLASS_BY_TYPE, Node, ReadInfo, node
from bench.language.property import p_node_parent, p_regular
from bench.language.setup import ANCESTOR_NODE_TYPES
from bench.proto.wire import AnyNodeData, QueryData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import Block, Expression, Field, Property, ReadOptions

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false

NodeT = TypeVar("NodeT", bound=Node)
NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)
FieldOrProperty = Union[
    Field if TYPE_CHECKING else "Field", Property if TYPE_CHECKING else "Property", Any
]
NodeTypeOrClass = Union[NodeType, type[Node]]


@node(NodeType.QUERY)
class Query(Node[QueryData]):
    """A stored query."""

    parent: "Block" = p_node_parent(4, NodeType.BLOCK)
    name: str | None = p_regular(30, default=None)
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


class QueryError(BenchError, ValueError):
    def __init__(self, query: "QueryBuilder", cause: Exception | None = None):
        super().__init__(repr(query))
        self.query = query
        self.cause = cause


class NodeNotFoundError(QueryError):
    pass


class MultipleNodesFoundError(QueryError):
    pass


class MakeQueryBase(abc.ABC, Generic[NodeT, NodeDataT]):
    """Build or modify a query. :ReadQueryBase"""

    def where(
        self, filter: Optional["Expression"] = None, **kwargs
    ) -> "QueryBuilder[NodeT, NodeDataT]":
        raise NotImplementedError

    def order_by(
        self,
        sort: Union[list[Union["Expression", str]], str, "Expression", None] = None,
        *args: str,
    ) -> "QueryBuilder[NodeT, NodeDataT]":
        raise NotImplementedError

    def first(self, count: int) -> "QueryBuilder[NodeT, NodeDataT]":
        """Returns the first N results."""
        raise NotImplementedError

    limit = first

    def skip(self, count: int) -> "QueryBuilder[NodeT, NodeDataT]":
        """Skips the first N results."""
        raise NotImplementedError

    def after(self, cursor: str) -> "QueryBuilder[NodeT, NodeDataT]":
        """Paginate using an opaque cursor."""
        raise NotImplementedError

    def include(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        """Includes given default-excluded properties in the results."""
        raise NotImplementedError

    def select_all(self) -> "QueryBuilder[NodeT, NodeDataT]":
        """Includes all (non-relational) properties in the results."""
        raise NotImplementedError

    def exclude(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        """Excludes given default-included properties from the results."""
        raise NotImplementedError

    def related(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        """Joins the given related properties in the results."""
        raise NotImplementedError

    def ancestors(self, *node_types: NodeTypeOrClass) -> "QueryBuilder[NodeT, NodeDataT]":
        """Joins the given ancestors in the results."""
        raise NotImplementedError

    def descendants(self, *node_types: NodeTypeOrClass) -> "QueryBuilder[NodeT, NodeDataT]":
        """Joins the given descendants in the results."""
        raise NotImplementedError


class ReadQueryBase(abc.ABC, Generic[NodeT, NodeDataT]):
    """Fetch the nodes matching a query. :ReadQueryBase"""

    async def __aiter__(self):
        raise NotImplementedError

    async def tolist(self) -> list[NodeT]:
        raise NotImplementedError

    def __iter__(self):
        raise NotImplementedError

    def __len__(self):
        raise NotImplementedError

    def __getitem__(self, item: slice | int) -> Union[Self, NodeT]:
        raise NotImplementedError

    async def get(self, filter: Optional["Expression"] = None, **kwargs) -> NodeT:
        raise NotImplementedError

    async def exists(self, filter: Optional["Expression"] = None, **kwargs) -> bool:
        raise NotImplementedError

    async def count(self, filter: Optional["Expression"] = None, **kwargs) -> int:
        raise NotImplementedError


class QueryBuilder(
    Generic[NodeT, NodeDataT], MakeQueryBase[NodeT, NodeDataT], ReadQueryBase[NodeT, NodeDataT]
):
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
            if v is not None:
                args_strs.append(f"{k}={v}")
        if args_strs:
            args_str = ", ".join(args_strs)
        else:
            args_str = "[*]"
        return args_str

    def __repr__(self):
        if self._aggregation is not None:
            query_type = self._aggregation.op.bench_name
        else:
            query_type = "Fetch"
        return f"<{self._node_type.bench_name}Query.{query_type} {self}>"

    def query(self) -> Self:
        return self

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
            # cache is not copied on purpose as it shouldn't propagate
        )

    def _copy_options(self) -> "ReadOptions":
        from bench.language.access import ReadOptions

        if self._options is None:
            return ReadOptions()
        else:
            return self._options.copy()

    def where(
        self, filter: Optional["Expression"] = None, **kwargs
    ) -> "QueryBuilder[NodeT, NodeDataT]":
        """Adds a filter clause to the query."""
        from bench.language.expression import coerce_conditional

        filter = coerce_conditional(self._node_cls, filter, kwargs)
        copy = self.copy()
        copy._filter = (
            filter & self._filter if filter is not None and self._filter is not None else filter
        )
        return copy

    def order_by(
        self,
        sort: Union[list[Union["Expression", str]], str, "Expression", None] = None,
        *args: str,
    ) -> "QueryBuilder[NodeT, NodeDataT]":
        """Sorts the query results by the given sort criteria."""
        from bench.language.expression import coerce_sort

        copy = self.copy()
        copy._sort = coerce_sort(self._node_cls, sort, *args)
        return copy

    def first(self, count: int) -> "QueryBuilder[NodeT, NodeDataT]":
        """Returns the first N results."""
        copy = self.copy()
        copy._first = count
        return copy

    limit = first

    def skip(self, count: int) -> "QueryBuilder[NodeT, NodeDataT]":
        """Skips the first N results."""
        copy = self.copy()
        copy._skip = count
        return copy

    def after(self, cursor: str) -> "QueryBuilder[NodeT, NodeDataT]":
        copy = self.copy()
        copy._after = cursor
        return copy

    def aggregate(self, aggregation: "Expression") -> "QueryBuilder[NodeT, NodeDataT]":
        if aggregation.kind != ExpressionKind.AGGREGATION:
            raise ValueError(f"expected aggregation expression, got {aggregation!r}")
        copy = self.copy()
        copy._aggregation = aggregation
        return copy

    @staticmethod
    def _to_properties(properties: tuple[FieldOrProperty, ...]) -> tuple["Property", ...]:
        return cast(tuple["Property"], properties)

    @staticmethod
    def _to_node_types(node_types: tuple[NodeTypeOrClass, ...]) -> list[NodeType]:
        return [cast(type[Node], t).metatype if isinstance(t, type) else t for t in node_types]

    def include(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.include_properties.extend(self._to_properties(properties))
        return copy

    def select_all(self) -> "QueryBuilder[NodeT, NodeDataT]":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.select_all_properties = True
        return copy

    def exclude(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.exclude_properties.extend(self._to_properties(properties))
        return copy

    def related(self, *properties: FieldOrProperty) -> "QueryBuilder[NodeT, NodeDataT]":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.related_properties.extend(self._to_properties(properties))
        return copy

    def include_ancestors(self) -> "QueryBuilder[NodeT, NodeDataT]":
        # not quite happy with this API for getting a 'full' node yet, see :LoadOrphanNode
        ancestors = ANCESTOR_NODE_TYPES[self._node_type]
        return self.ancestors(*ancestors)

    def ancestors(self, *node_types: NodeTypeOrClass) -> "QueryBuilder[NodeT, NodeDataT]":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.ancestor_types = self._to_node_types(node_types)
        return copy

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

    async def get(self, filter: Optional["Expression"] = None, **kwargs) -> NodeT:
        """Returns the unique result matching the query (errors otherwise)."""
        from bench.language.expression import coerce_conditional

        filter = coerce_conditional(self._node_cls, filter, kwargs)
        combined_query = self.where(filter)
        results = await combined_query.fetch()
        if len(results) == 1:
            return results[0]
        else:
            if len(results) == 0:
                raise NodeNotFoundError(combined_query)
            else:
                raise MultipleNodesFoundError(combined_query)

    async def fetch(self) -> list[NodeT] | tuple[NodeT, ...]:
        from bench.language.connection import FetchOptions
        from bench.proto.wiring import unpack_roots

        tx = active_tx()
        connection = await tx.connect(
            base=self._base, node_type=self._node_type, access_kind=AccessKind.READ
        )
        result = await connection.fetch(self, FetchOptions())
        data_graph = NodeDataGraph(result.nodes)
        read = ReadInfo(options=self._options, epoch=result.epoch) if self._options else None
        roots = unpack_roots(
            data_graph, parent=self._base, session=tx.session, roots=result.roots, read=read
        )
        return cast(tuple[NodeT, ...], roots)

    tolist = fetch  # type: ignore
    to_list = fetch  # type: ignore

    async def count(self, filter: Optional["Expression"] = None, **kwargs) -> int:
        """Returns the number of results. May refine the query."""
        from bench.language.expression import A, coerce_conditional

        filter = coerce_conditional(self._node_cls, filter, kwargs, return_none_if_empty=True)
        query = self.where(filter) if filter is not None else self
        query = query.aggregate(A(AggregationOp.COUNT))
        connection = await active_tx().connect(
            base=query._base, node_type=query._node_type, access_kind=AccessKind.READ
        )
        result = await connection.aggregate(query)
        assert result.aggregation.count is not None, f"missing count in {result!r}"
        return result.aggregation.count

    async def exists(self, filter: Optional["Expression"] = None, **kwargs) -> bool:
        """Whether any results exist. May refine the query."""
        from bench.language.expression import A, coerce_conditional

        filter = coerce_conditional(self._node_cls, filter, kwargs, return_none_if_empty=True)
        query = self.where(filter) if filter is not None else self
        query = query.aggregate(A(AggregationOp.EXISTS))
        connection = await active_tx().connect(
            base=query._base, node_type=query._node_type, access_kind=AccessKind.READ
        )
        result = await connection.aggregate(query)
        assert result.aggregation.exists is not None, f"missing exists in {result!r}"
        return result.aggregation.exists

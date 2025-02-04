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

from bench.language.registry import ANCESTOR_NODE_TYPES, NODE_CLASSES, _on_completing_setup
from bench.pb2 import AnyNodeData
from bench.utils.func import stable_hash

from .const import (
    AggregationType,
    BenchError,
    ExpressionKind,
    FieldType,
    NodeType,
    QueryType,
    StructType,
    active_session,
)
from .expression import Expression, coerce_conditional
from .node import NODE_CLASS_BY_TYPE, Node, NodeReference
from .property import Property, p_regular
from .struct import Struct, struct_

if TYPE_CHECKING:
    from bench.language import (
        AggregateOptions,
        Block,
        ConnectMode,
        Connector,
        Field,
        GetConnection,
        NodeReference,
        PropertyReference,
        SearchConnection,
        SelectOptions,
    )

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# default read options
SELECT_DEFAULT_PROPERTIES: dict[NodeType, tuple[Property, ...]] = {}
SELECT_ALL_PROPERTIES: dict[NodeType, tuple[Property, ...]] = {}


@_on_completing_setup
def _init_default_query():
    # FILTER_NOT_DELETED.clauses = [C(ConditionalType.NOT_EXISTS, property=Node.deleted_at)]
    for node_t in NODE_CLASSES:
        SELECT_DEFAULT_PROPERTIES[node_t.metatype] = tuple(
            prop for prop in node_t.__stored_properties__.values() if not prop.is_deferred
        )
        SELECT_ALL_PROPERTIES[node_t.metatype] = tuple(node_t.__stored_properties__.values())


def get_default_query_filter():
    return Node.get_property("deleted_at").not_exists()


NodeT = TypeVar("NodeT", bound=Node)
NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)
FieldOrProperty = Union[
    Field if TYPE_CHECKING else "Field", Property if TYPE_CHECKING else "Property", Any
]
NodeTypeOrClass = Union[NodeType, type[Node]]


@struct_(StructType.SELECT_OPTIONS)
class SelectOptions(Struct):
    """
    Specify which Properties/Fields to load.
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
    select_fields: list["Field"] = p_regular(
        51, require=True, array=True, references=NodeType.FIELD
    )

    if TYPE_CHECKING:
        include_properties_ptr: list["PropertyReference"] = []
        exclude_properties_ptr: list["PropertyReference"] = []
        select_properties_ptr: list["PropertyReference"] = []
        select_fields_ptr: list["NodeReference"] = []

    def __content_str__(self) -> str:
        content_parts = []
        if self.select_all_properties:
            content_parts.append("<all properties>")
        else:
            if self.include_properties_ptr:
                content_parts.append(
                    f"include=[{','.join(p.name for p in self.include_properties)}]"
                )
        if self.exclude_properties_ptr:
            content_parts.append(f"exclude=[{','.join(p.name for p in self.exclude_properties)}]")
        if self.select_fields_ptr:
            content_parts.append(
                f"fields=[{','.join(f.code_name or '???' for f in self.select_fields)}]"
            )
        return ", ".join(content_parts) if content_parts else "<default>"

    def _stable_hash(self) -> int:
        # NOTE: we override stable_hash to ensure SelectOptions hash differs per field identities
        #  (otherwise when we cache per query and the field identity changes, we wouldn't re-query)
        return stable_hash(
            self.select_all_properties,
            tuple(r._stable_hash() for r in self.include_properties_ptr),
            tuple(r._stable_hash() for r in self.exclude_properties_ptr),
            tuple(r._stable_hash() for r in self.select_properties_ptr),
            tuple(r._stable_hash() for r in self.select_fields_ptr),
            tuple(r.identity_key for r in self.select_fields),
        )

    def get_selected_properties(self, node_type: NodeType) -> Sequence[Property]:
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

    def get_selected_fields(self, block: "Block") -> Sequence["Field"]:
        """Get the selected (member) fields for a block."""
        fields: list[Field] = []
        for field in self.select_fields:
            if field.base_ck == block.ck and field.type == FieldType.MEMBER:
                fields.append(field)
        return fields

    @staticmethod
    def default():
        return SelectOptions()

    @staticmethod
    def all():
        return SelectOptions(select_all_properties=True)


class QueryError(BenchError, ValueError):
    def __init__(
        self,
        query: "Query | NodeReference",
        result: Any | None = None,
        cause: Exception | None = None,
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


class Query[NodeT: Node, NodeDataT: AnyNodeData]:
    """Build a Query."""

    __slots__ = (
        "_after",
        "_aggregation",
        "_ancestor_types",
        "_base_block",
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
        base_block: Optional["Block"] = None,
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
        from .node import Node

        # root
        self._type = type
        self._node_type = node_type
        self._node_cls = NODE_CLASS_BY_TYPE[node_type] if node_type else Node
        self._base_block = base_block
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
        if self._base_block:
            content_parts.append(self._base_block.absolute_path)
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
        if self._base_block is not None:
            return f"<{self._base_block.code_name}Query {self}>"
        else:
            return f"<{self._node_type.bench_name}Query {self}>"

    def _stable_hash(self):
        return stable_hash(
            # root
            self._type,
            self._node_type,
            self._base_block._stable_hash() if self._base_block is not None else None,
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
    def block(self) -> "Block":
        assert self._base_block is not None, f"missing block in {self!r}"
        return self._base_block

    @property
    def include_deleted(self) -> bool:
        return self._include_deleted

    #
    # Builder
    #

    def clone(self):
        """Clones the query (the properties are immutable)."""
        return Query(
            # root
            type=self._type,
            node_type=self._node_type,
            base_block=self._base_block,
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

    def trim_to(self, node_types: Collection[NodeType]) -> "Query[NodeT, NodeDataT]":
        assert self._node_type in node_types
        clone = self.clone()
        clone._ancestor_types = [a for a in self._ancestor_types if a in node_types]
        clone._descendant_types = [d for d in self._descendant_types if d in node_types]
        return clone

    def where(self, filter: Optional["Expression"] = None, **kwargs) -> "Query[NodeT, NodeDataT]":
        """Adds a filter clause to the query."""
        from .expression import coerce_conditional

        filter = coerce_conditional(
            node_cls=self._node_cls, block=self._base_block, expr=filter, kwargs=kwargs
        )
        clone = self.clone()
        clone._filter = (
            filter & self._filter if filter is not None and self._filter is not None else filter
        )
        return clone

    def order_by(
        self,
        sort: Union[
            list[Union["Expression", str]], str, "Expression", "Field", "Property", None
        ] = None,
        *args: str,
    ) -> "Query[NodeT, NodeDataT]":
        """Sorts the query results by the given sort criteria."""
        from .expression import coerce_sort

        clone = self.clone()
        clone._sort = coerce_sort(
            node_cls=self._node_cls, block=self._base_block, expr=sort, args=args
        )
        return clone

    def first(self, count: int) -> "Query[NodeT, NodeDataT]":
        """Returns the first N results."""
        clone = self.clone()
        clone._first = count
        return clone

    limit = first

    def skip(self, count: int) -> "Query[NodeT, NodeDataT]":
        """Skips the first N results."""
        clone = self.clone()
        clone._skip = count
        return clone

    def aggregate(self, aggregation: "Expression") -> "Query[NodeT, NodeDataT]":
        """Aggregates the query results."""
        assert aggregation.kind == ExpressionKind.AGGREGATION, f"not an aggregation: {aggregation}"
        clone = self.clone()
        clone._aggregation = aggregation
        return clone

    def include(self, *properties: Property) -> "Query[NodeT, NodeDataT]":
        """Includes given default-excluded properties in the results."""
        clone = self.clone()
        clone._select = self._clone_select()
        clone._select.include_properties += self._to_properties(properties)
        return clone

    def select(self, *keys: FieldOrProperty) -> "Query[NodeT, NodeDataT]":
        """Selects only the given properties/fields in the results."""
        clone = self.clone()
        clone._select = self._clone_select()
        for key in keys:
            if isinstance(key, Property):
                clone._select.select_all_properties = False
                clone._select.select_properties_ptr.append(key.to_ref())
            else:
                clone._select.select_fields_ptr.append(key.to_ref())
        return clone

    def select_all(self) -> "Query[NodeT, NodeDataT]":
        """Includes all properties/fields in the results."""
        clone = self.clone()
        clone._select = self._clone_select()
        clone._select.select_all_properties = True
        if self._base_block is not None:
            clone._select.select_fields = list(self._base_block.fields)
        else:
            clone._select.select_fields = []
        return clone

    def deselect(self, *properties: FieldOrProperty) -> "Query[NodeT, NodeDataT]":
        """Excludes given default-included properties from the results."""
        clone = self.clone()
        clone._select = self._clone_select()
        clone._select.exclude_properties += self._to_properties(properties)
        return clone

    def include_ancestors(self, *node_types: NodeTypeOrClass) -> "Query[NodeT, NodeDataT]":
        """Includes all ancestors in the results."""
        # not quite happy with this API for getting a 'full' node yet, see :LoadOrphanNode
        clone = self.clone()
        for node_type in node_types or ANCESTOR_NODE_TYPES[self._node_type]:
            if not isinstance(node_type, NodeType):
                node_type = node_type.metatype
            if node_type not in clone._ancestor_types:
                clone._ancestor_types.append(node_type)
        return clone

    def include_descendants(self, *node_types: NodeTypeOrClass) -> "Query[NodeT, NodeDataT]":
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

    def __getitem__(self, item: slice) -> Union["Query[NodeT, NodeDataT]", NodeT]:  # type: ignore
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

    async def _get_read_connector(self) -> "Connector":
        session = active_session()
        scope = session._get_scope_for_query(self)
        return await session._get_connector_for(
            scope,
            self.all_node_types,
            include_deleted=self.include_deleted,
            is_readonly=True,
        )

    @overload
    async def get(
        self,
        filter: Union["Expression", "NodeReference", None] = None,
        live: bool = False,
        mode: "ConnectMode" = "unpacked",
        **kwargs,
    ) -> NodeT: ...
    @overload
    async def get(
        self,
        filter: Sequence["NodeReference"],
        live: bool = False,
        mode: "ConnectMode" = "unpacked",
        **kwargs,
    ) -> list[NodeT]: ...
    @tracer.start_as_current_span("query.get")
    async def get(
        self,
        filter: Union["Expression", "NodeReference", Sequence["NodeReference"], None] = None,
        live: bool = False,
        mode: "ConnectMode" = "unpacked",
        **kwargs,
    ) -> NodeT | list[NodeT]:
        """
        Returns the unique result matching the query (one or multiple nodes, errors otherwise).
        NOTE: this is only a true get query with specific node pointers as roots, otherwise it's a search.
        """
        result, _ = await self.get_connection(filter, live, mode, **kwargs)
        return result

    @overload
    async def get_connection(
        self,
        filter: Union["Expression", "NodeReference", None] = None,
        live: bool = False,
        mode: "ConnectMode" = "unpacked",
        **kwargs,
    ) -> tuple[NodeT, "GetConnection[Any, NodeT] | SearchConnection[Any, NodeT]"]: ...
    @overload
    async def get_connection(
        self,
        filter: Sequence["NodeReference"],
        live: bool = False,
        mode: "ConnectMode" = "unpacked",
        **kwargs,
    ) -> tuple[list[NodeT], "GetConnection[Any, NodeT] | SearchConnection[Any, NodeT]"]: ...
    @tracer.start_as_current_span("query.get_connection")
    async def get_connection(
        self,
        filter: Union["Expression", "NodeReference", Sequence["NodeReference"], None] = None,
        live: bool = False,
        mode: "ConnectMode" = "unpacked",
        **kwargs,
    ) -> tuple[NodeT | list[NodeT], "GetConnection[Any, NodeT] | SearchConnection[Any, NodeT]"]:
        """
        Returns the unique result matching the query and its connection.
        NOTE: this is only a true get query with specific node pointers as roots, otherwise it's a search.
        """
        from bench.language import GetOptions, NodeReference, coerce_conditional

        if isinstance(filter, (NodeReference, Sequence)):
            # true get request (with node pointers)
            assert self._filter is None, f"cannot combine filter and roots in {self!r}"
            query = self.clone()
            query._roots = [filter] if isinstance(filter, NodeReference) else list(filter)
            query._type = QueryType.GET
            connector = await query._get_read_connector()
            connection = await connector.get(query, GetOptions(mode=mode, live=live))

            # coerce to node/list of nodes
            if len(connection.result.roots) != len(query._roots):
                if len(connection.result.roots) < len(query._roots):
                    raise NodeNotFoundError(query=query)
                else:
                    raise MultipleNodesFoundError(query=query, result=connection.result.roots)
            if isinstance(filter, NodeReference):  # keep single node
                node = connection.result.roots[0]
                return cast(NodeT, node), connection
            else:
                return cast(list[NodeT], connection.result.roots), connection
        else:
            # search which should only have one result
            filter = coerce_conditional(
                node_cls=self._node_cls, block=self._base_block, expr=filter, kwargs=kwargs
            )
            query = self.where(filter) if filter is not None else self.clone()
            results, connection = await query.search_connection()
            if len(results) != 1:
                if len(results) == 0:
                    raise NodeNotFoundError(query=query)
                else:
                    raise MultipleNodesFoundError(query=query, result=results)
            return results[0], connection  # success

    @tracer.start_as_current_span("query.search")
    async def search(
        self,
        filter: Optional["Expression"] = None,
        live: bool = False,
        mode: "ConnectMode" = "both",
        **kwargs,
    ) -> list[NodeT]:
        """Fetches the nodes matching the query."""
        results, _ = await self.search_connection(filter, live, mode, **kwargs)
        return results

    @tracer.start_as_current_span("query.search_connection")
    async def search_connection(
        self,
        filter: Optional["Expression"] = None,
        live: bool = False,
        mode: "ConnectMode" = "both",
        **kwargs,
    ) -> tuple[list[NodeT], "SearchConnection"]:
        """Fetches the nodes matching the query and returns the connection."""
        from bench.language import SearchOptions

        filter = coerce_conditional(
            node_cls=self._node_cls, block=self._base_block, expr=filter, kwargs=kwargs
        )
        query = self.where(filter) if filter is not None else self
        connector = await query._get_read_connector()
        connection = await connector.search(query, SearchOptions(live=live, mode=mode, count=False))
        return cast(list[NodeT], connection.result.roots), connection

    tolist = search  # type: ignore
    to_list = search  # type: ignore
    to_list_connection = search_connection  # type: ignore

    @tracer.start_as_current_span("query.one_or_none")
    async def one_or_none(self) -> NodeT | None:
        """Returns the unique result matching the query (one or none, errors otherwise)."""
        result, _ = await self.one_or_none_connection()
        return result

    @tracer.start_as_current_span("query.one_or_none_connection")
    async def one_or_none_connection(self) -> tuple[NodeT | None, "SearchConnection"]:
        """Returns the unique result matching the query and its connection (one or none, errors otherwise)."""
        results, connection = await self.first(1).search_connection()
        if len(results) == 1:
            return results[0], connection
        elif len(results) == 0:
            return None, connection
        else:
            raise MultipleNodesFoundError(query=self, result=results)

    @tracer.start_as_current_span("query.count")
    async def count(
        self, filter: Optional["Expression"] = None, mode: "ConnectMode" = "unpacked", **kwargs
    ) -> int:
        """Returns the number of results."""
        from bench.language import AggregateOptions

        from .expression import A, coerce_conditional

        # prepare
        filter = coerce_conditional(
            node_cls=self._node_cls, block=self._base_block, expr=filter, kwargs=kwargs
        )
        query = self.where(filter) if filter is not None else self
        query = query.aggregate(A(AggregationType.COUNT))
        query._type = QueryType.AGGREGATE
        trace.get_current_span().set_attribute("query", repr(query))

        # query
        connector = await query._get_read_connector()
        connection = await connector.aggregate(query, AggregateOptions(live=False, mode=mode))
        aggregation = connection.result_data.aggregation
        assert aggregation.count is not None, f"missing count in {connection!r}"
        return aggregation.count

    @tracer.start_as_current_span("query.exists")
    async def exists(
        self, filter: Optional["Expression"] = None, mode: "ConnectMode" = "unpacked", **kwargs
    ) -> bool:
        """Whether any nodes match the query."""
        from .expression import A, coerce_conditional

        # prepare
        filter = coerce_conditional(
            node_cls=self._node_cls, block=self._base_block, expr=filter, kwargs=kwargs
        )
        query = self.where(filter) if filter is not None else self
        query = query.aggregate(A(AggregationType.EXISTENCE))
        query._type = QueryType.AGGREGATE
        trace.get_current_span().set_attribute("query", repr(query))

        # query
        connector = await query._get_read_connector()
        connection = await connector.aggregate(query, AggregateOptions(live=False, mode=mode))
        aggregation = connection.result_data.aggregation
        assert aggregation.exists is not None, f"missing exists in {connection!r}"
        return aggregation.exists

    @tracer.start_as_current_span("query.scalar")
    async def scalar(self, *properties: "str | Property") -> Any:
        """Returns a single value from the single result (error if None)."""
        assert properties, "expected at least one property"
        properties_names = tuple(p.name if not isinstance(p, str) else p for p in properties)
        node, connection = await self.get_connection()
        if len(properties) == 1:
            scalar = getattr(node, properties_names[0])
        else:
            scalar = tuple(getattr(node, p) for p in properties_names)
        connection.detach()
        return scalar

    @tracer.start_as_current_span("query.scalar_maybe")
    async def scalar_maybe(self, *properties: "str | Property") -> Any | None:
        """Returns a single value from the single result, or None if no result."""
        assert properties, "expected at least one property"
        properties_names = tuple(p.name if not isinstance(p, str) else p for p in properties)
        results, connection = await self.search_connection()
        if len(results) == 1:
            node = results[0]
            if len(properties) == 1:
                scalar = getattr(node, properties_names[0])
            else:
                scalar = tuple(getattr(node, p) for p in properties_names)
            connection.detach()
            return scalar
        elif len(results) == 0:
            return None
        else:
            raise MultipleNodesFoundError(query=self, result=results)

    @tracer.start_as_current_span("query.scalar_list")
    async def scalar_list(self, *properties: "str | Property") -> list[Any]:
        """Returns a list of values from the results."""
        assert properties, "expected at least one property"
        properties_names = tuple(p.name if not isinstance(p, str) else p for p in properties)
        nodes, connection = await self.search_connection()
        if len(properties) == 1:
            scalars = [getattr(node, properties_names[0]) for node in nodes]
        else:
            scalars = [tuple(getattr(node, p) for p in properties_names) for node in nodes]
        connection.detach()
        return scalars

    def _to_properties(self, properties: tuple[FieldOrProperty, ...]) -> tuple["Property", ...]:
        if not all(isinstance(p, Property) for p in properties):
            raise NotImplementedError(f"fields not yet supported in {self!r}, got {properties}")
        return cast(tuple["Property"], properties)

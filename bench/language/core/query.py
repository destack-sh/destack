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
from bench.utils.func import hash_stable

from .const import (
    BenchError,
    FieldType,
    NodeType,
    QueryType,
    StructType,
)
from .expression import Expression, coerce_conditional
from .node import NODE_CLASS_BY_TYPE, Node, NodeReference
from .property import Property, p_regular
from .struct import Struct, struct_

if TYPE_CHECKING:
    from bench.language import Field, NodeReference, PropertyReference, SelectOptions, Table

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
            prop for prop in node_t.__stored_properties__.values()
        )
        SELECT_ALL_PROPERTIES[node_t.metatype] = tuple(node_t.__stored_properties__.values())


def get_default_query_filter():
    return (
        Node.get_property("archived_at").not_exists() & Node.get_property("deleted_at").not_exists()
    )


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
    include_properties: list[Property] = p_regular(41)
    exclude_properties: list[Property] = p_regular(42)
    select_properties: list[Property] = p_regular(43)

    # fields
    select_fields: list["Field"] = p_regular(51)

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
        return hash_stable(
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

    def get_selected_fields(self, table: "Table") -> Sequence["Field"]:
        """Get the selected (member) fields for a table."""
        fields: list[Field] = []
        for field in self.select_fields:
            if field.parent_id == table.id and field.type == FieldType.VARIABLE:
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
        query: "LegacyQuery | NodeReference",
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


class LegacyQuery[NodeT: Node, NodeDataT: AnyNodeData]:
    """
    Build a (LEGACY) Query.
    Legacy because these queries are pretty bad and limited (together with Connections) :RichGraph.
    New Queries should look more like Query -> QueryResult
     (see ConvexDB, InstantDB, SpacetimeDB, Prisma, Firebase, ...)
    """

    __slots__ = (
        "_after",
        "_ancestor_types",
        "_base_type",
        "_descendant_types",
        "_filter",
        "_first",
        "_include_memory",
        "_include_removed",
        "_node_cls",
        "_node_type",
        "_roots",
        "_select",
        "_sort",
        "_type",
    )

    def __init__(
        self,
        # root
        type: QueryType,
        node_type: NodeType,
        base_type: Optional["Node"] = None,
        roots: Optional[list["NodeReference"]] = None,
        filter: Optional["Expression"] = None,
        sort: list["Expression"] | None = None,
        # "joins"
        ancestor_types: list[NodeType] | None = None,
        descendant_types: list[NodeType] | None = None,
        # options
        select: Optional["SelectOptions"] = None,
        include_removed: bool = False,
        include_memory: bool = True,  # NOTE :Cleanup: doesn't include_memory overlap with include_removed?
        first: int | None = None,
    ):
        from .node import Node

        # root
        self._type = type
        self._node_type = node_type
        self._node_cls = NODE_CLASS_BY_TYPE[node_type] if node_type else Node
        self._base_type = base_type
        self._roots = roots
        self._filter = filter
        self._sort = sort
        # "joins"
        self._ancestor_types = ancestor_types or []
        self._descendant_types = descendant_types or []
        # options
        self._select = select
        self._include_removed = include_removed
        self._include_memory = include_memory
        self._first = first

    def __str__(self):
        content_parts = []
        if self._base_type:
            content_parts.append(self._base_type.absolute_path)
        if self._roots is not None:
            content_parts.append(f"roots=[{', '.join(str(r) for r in self._roots)}]")
        for k in ("filter", "sort", "first", "include_memory", "include_removed"):
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
        if self._base_type is not None:
            return f"<{self._base_type.code_name}Query {self}>"
        else:
            return f"<{self._node_type.bench_name}Query {self}>"

    def _stable_hash(self):
        return hash_stable(
            # root
            self._type,
            self._node_type,
            self._base_type._stable_hash() if self._base_type is not None else None,
            tuple(r._stable_hash() for r in self._roots) if self._roots is not None else None,
            self._filter._stable_hash() if self._filter is not None else None,
            tuple(s._stable_hash() for s in self._sort) if self._sort is not None else None,
            # joins
            self._ancestor_types,
            self._descendant_types,
            # options
            self._select._stable_hash() if self._select is not None else None,
            self._include_removed,
            self._include_memory,
            self._first,
        )

    @property
    def all_node_types(self) -> Iterable[NodeType]:
        return chain((self._node_type,), self._ancestor_types, self._descendant_types)

    @property
    def table(self) -> "Table":
        from bench.language import Table

        assert isinstance(self._base_type, Table), f"{self!r} has Table: {self._base_type!r}"
        return self._base_type

    @property
    def include_removed(self) -> bool:
        return self._include_removed

    @property
    def include_memory(self) -> bool:
        return self._include_memory

    #
    # Builder
    #

    def clone(self):
        """Clones the query (the properties are immutable)."""
        return LegacyQuery(
            # root
            type=self._type,
            node_type=self._node_type,
            base_type=self._base_type,
            roots=self._roots,
            filter=self._filter,
            sort=self._sort,
            # "joins"
            ancestor_types=self._ancestor_types,
            descendant_types=self._descendant_types,
            # options
            select=self._select.clone() if self._select is not None else None,
            include_removed=self._include_removed,
            include_memory=self._include_memory,
            first=self._first,
        )

    def _clone_select(self) -> "SelectOptions":
        if self._select is None:
            return SelectOptions()
        else:
            return self._select.clone()

    def trim_to(self, node_types: Collection[NodeType]) -> "LegacyQuery[NodeT, NodeDataT]":
        assert self._node_type in node_types
        clone = self.clone()
        clone._ancestor_types = [a for a in self._ancestor_types if a in node_types]
        clone._descendant_types = [d for d in self._descendant_types if d in node_types]
        return clone

    def where(
        self, filter: Optional["Expression"] = None, **kwargs
    ) -> "LegacyQuery[NodeT, NodeDataT]":
        """Adds a filter clause to the query."""

        filter = coerce_conditional(
            node_cls=self._node_cls, base_type=self._base_type, expr=filter, kwargs=kwargs
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
    ) -> "LegacyQuery[NodeT, NodeDataT]":
        """Sorts the query results by the given sort criteria."""
        from .expression import coerce_sort

        clone = self.clone()
        clone._sort = coerce_sort(
            node_cls=self._node_cls, base_type=self._base_type, expr=sort, args=args
        )
        return clone

    def first(self, count: int) -> "LegacyQuery[NodeT, NodeDataT]":
        """Returns the first N results."""
        clone = self.clone()
        clone._first = count
        return clone

    limit = first

    def include(self, *properties: Property) -> "LegacyQuery[NodeT, NodeDataT]":
        """Includes given default-excluded properties in the results."""
        clone = self.clone()
        clone._select = self._clone_select()
        clone._select.include_properties += self._to_properties(properties)
        return clone

    def select(self, *keys: FieldOrProperty) -> "LegacyQuery[NodeT, NodeDataT]":
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

    def select_all(self) -> "LegacyQuery[NodeT, NodeDataT]":
        """Includes all properties/Fields in the results."""
        clone = self.clone()
        clone._select = self._clone_select()
        clone._select.select_all_properties = True
        if self._base_type is not None:
            clone._select.select_fields = cast(
                list[Field],
                self._base_type._graph.get_descendants(self._base_type, NodeType.FIELD),
            )
        else:
            clone._select.select_fields = []
        return clone

    def deselect(self, *properties: FieldOrProperty) -> "LegacyQuery[NodeT, NodeDataT]":
        """Excludes given default-included properties from the results."""
        clone = self.clone()
        clone._select = self._clone_select()
        clone._select.exclude_properties += self._to_properties(properties)
        return clone

    def include_ancestors(self, *node_types: NodeTypeOrClass) -> "LegacyQuery[NodeT, NodeDataT]":
        """Includes all ancestors in the results."""
        clone = self.clone()
        if node_types:
            # add node types
            for node_type in node_types:
                if not isinstance(node_type, NodeType):
                    node_type = node_type.metatype
                if node_type not in clone._ancestor_types:
                    clone._ancestor_types.append(node_type)
        else:
            # add all ancestors
            for ancestor_type in ANCESTOR_NODE_TYPES[self._node_type]:
                if ancestor_type not in clone._ancestor_types:
                    clone._ancestor_types.append(ancestor_type)
        return clone

    def include_descendants(self, *node_types: NodeTypeOrClass) -> "LegacyQuery[NodeT, NodeDataT]":
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

    async def __aiter__(self):
        return iter(await self.search())

    @overload
    async def get(
        self,
        filter: Union["Expression", "NodeReference", None] = None,
        live: bool = False,
        **kwargs,
    ) -> NodeT: ...
    @overload
    async def get(
        self,
        filter: Sequence["NodeReference"],
        live: bool = False,
        **kwargs,
    ) -> list[NodeT]: ...
    @tracer.start_as_current_span("query.get")
    async def get(
        self,
        filter: Union["Expression", "NodeReference", Sequence["NodeReference"], None] = None,
        live: bool = False,
        **kwargs,
    ) -> NodeT | list[NodeT]:
        raise NotImplementedError

    @tracer.start_as_current_span("query.search")
    async def search(
        self,
        filter: Optional["Expression"] = None,
        live: bool = False,
        **kwargs,
    ) -> list[NodeT]:
        """Fetches the nodes matching the query."""
        raise NotImplementedError

    tolist = search  # type: ignore
    to_list = search  # type: ignore

    @tracer.start_as_current_span("query.one_or_none")
    async def one_or_none(self) -> NodeT | None:
        """Returns the unique result matching the query (one or none, errors otherwise)."""
        raise NotImplementedError

    def _to_properties(self, properties: tuple[FieldOrProperty, ...]) -> tuple["Property", ...]:
        if not all(isinstance(p, Property) for p in properties):
            raise NotImplementedError(f"fields not yet supported in {self!r}, got {properties}")
        return cast(tuple["Property"], properties)

# ruff: noqa: RUF012

from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Optional, Union, assert_never

from fastuuid import UUID

from .const import BuiltinEnum, EnumType, NodeType, StructType, enum_
from .node import Node, NodeReference
from .object import Property
from .property import p_regular
from .struct import Struct, struct_
from .value import Value

if TYPE_CHECKING:
    from bench.language import Field, Table

# pyright: reportIncompatibleVariableOverride=false

#
# Relations/Attributes
#


@enum_(EnumType.RELATION_TYPE)
class RelationType(BuiltinEnum):
    BUILTIN_NODE = 1
    CUSTOM_NODE = 2


@struct_(StructType.RELATION_REFERENCE)
class RelationReference(Struct):
    """Reference to a Node "table" or base somewhere."""

    type: RelationType = p_regular(30)
    node_type: NodeType = p_regular(31)
    table: Optional["Table"] = p_regular(32)

    if TYPE_CHECKING:
        table_ptr: Optional[NodeReference] = None  # convenience only


def relation_ref(base: "NodeType | type[Node] | Table") -> RelationReference:
    from .node import Node

    if isinstance(base, NodeType):
        return RelationReference(type=RelationType.BUILTIN_NODE, node_type=base)
    elif isinstance(base, type):
        assert issubclass(base, Node), f"{base!r} is not a Node"
        return RelationReference(type=RelationType.BUILTIN_NODE, node_type=base.metatype)
    elif isinstance(base, Table):
        return RelationReference(type=RelationType.CUSTOM_NODE, node_type=base.metatype)
    else:
        assert_never(base)


@enum_(EnumType.ATTRIBUTE_TYPE)
class AttributeType(BuiltinEnum):
    PROPERTY = 1
    FIELD = 2
    QUERY = 3


@struct_(StructType.ATTRIBUTE_REFERENCE)
class AttributeReference(Struct):
    """Reference to a Field or Property."""

    type: AttributeType = p_regular(30)
    name: str | None = p_regular(31, description="Named attribute from another Query.")
    field: Optional["Field"] = p_regular(32)
    prop: Optional["Property"] = p_regular(33)
    table: Optional[RelationReference] = p_regular(34)


def attribute_ref(field: "str | Field | Property") -> AttributeReference:
    if isinstance(field, str):
        return AttributeReference(type=AttributeType.QUERY, name=field)
    elif isinstance(field, Field):
        return AttributeReference(type=AttributeType.FIELD, field=field)
    elif isinstance(field, Property):
        return AttributeReference(type=AttributeType.PROPERTY, prop=field)
    else:
        assert_never(field)


#
# Functions
#


@enum_(EnumType.FUNCTION_TYPE)
class FunctionType(BuiltinEnum):
    ADD = 1  # +
    SUBTRACT = 2  # -
    MULTIPLY = 3  # *
    DIVIDE = 4  # /
    MODULO = 5  # %
    POWER = 6  # ^ / **


@struct_(StructType.FUNCTION)
class Function(Struct):
    type: FunctionType = p_regular(30)
    left: "Expression" = p_regular(31)
    right: Optional["Expression"] = p_regular(32)


def function(
    type: FunctionType,
    left: "Expression",
    right: Optional["Expression"] = None,
) -> Function:
    return Function(type=type, left=left, right=right)


#
# Conditional
#


@enum_(EnumType.CONDITIONAL_TYPE)
class ConditionalType(BuiltinEnum):
    # logical
    NOT = 1
    AND = 2
    OR = 3
    # comparison
    EQUALS = 10
    NOT_EQUALS = 11
    GREATER_THAN = 12
    GREATER_THAN_OR_EQUALS = 13
    LESS_THAN = 14
    LESS_THAN_OR_EQUALS = 15
    # string
    MATCHES = 20
    STARTS_WITH = 21
    ENDS_WITH = 22
    # collections
    CONTAINS = 23
    NOT_CONTAINS = 24
    IN = 25
    NOT_IN = 26
    # existence
    EXISTS = 30
    NOT_EXISTS = 31


@struct_(StructType.CONDITION)
class Condition(Struct):
    """Boolean predicate (AND, =, <, etc.)."""

    type: ConditionalType = p_regular(30)
    left: "Expression" = p_regular(31)
    right: Optional["Expression"] = p_regular(32)


def condition(
    column: Union["Field", "Property"],
    type: ConditionalType = ConditionalType.EQUALS,
    value: Any = None,
) -> Condition:
    raise NotImplementedError


#
# Aggregation
#


@enum_(EnumType.AGGREGATION_TYPE)
class AggregationType(BuiltinEnum):
    EXISTS = 1
    COUNT = 2
    SUM = 3
    MIN = 4
    MAX = 5
    AVERAGE = 6
    MEDIAN = 7
    HISTOGRAM = 8


@struct_(StructType.AGGREGATION)
class Aggregation(Struct):
    """Aggregate expression like COUNT(col) DISTINCT OVER ( … ) AS total."""

    type: AggregationType = p_regular(30)
    operand: Optional["Expression"] = p_regular(31)  # COUNT(*) → arg=None
    alias: Optional[str] = p_regular(32)  # result key
    distinct: bool = p_regular(33)  # DISTINCT flag
    # over, ...


def aggregation(
    type: AggregationType,
    operand: Optional["Expression"] = None,
) -> Aggregation:
    return Aggregation(type=type, operand=operand)


#
# Expression
#


@enum_(EnumType.EXPRESSION_TYPE)
class ExpressionType(BuiltinEnum):
    LITERAL = 1
    COLUMN = 2
    CONDITION = 3
    FUNCTION = 4
    AGGREGATION = 5
    # SUBQUERY?


@struct_(StructType.EXPRESSION)
class Expression(Struct):
    """Wrapper to unify any scalar / boolean / aggregate sub-tree."""

    type: ExpressionType = p_regular(30)
    literal: Optional[Value] = p_regular(31)
    column: Optional[AttributeReference] = p_regular(32)
    condition: Optional[Condition] = p_regular(33)
    function: Optional[Function] = p_regular(34)
    aggregation: Optional[Aggregation] = p_regular(35)
    # subquery?


def expression(
    thing: "Value | AttributeReference | Condition | Function | Aggregation",
) -> Expression:
    if isinstance(thing, Value):
        return Expression(type=ExpressionType.LITERAL, literal=thing)
    elif isinstance(thing, AttributeReference):
        return Expression(type=ExpressionType.COLUMN, column=thing)
    elif isinstance(thing, Condition):
        return Expression(type=ExpressionType.CONDITION, condition=thing)
    elif isinstance(thing, Function):
        return Expression(type=ExpressionType.FUNCTION, function=thing)
    elif isinstance(thing, Aggregation):
        return Expression(type=ExpressionType.AGGREGATION, aggregation=thing)
    else:
        assert_never(thing)


#
# Sort
#


@enum_(EnumType.SORT_TYPE)
class SortType(BuiltinEnum):
    ASCENDING = 1
    DESCENDING = 2


@enum_(EnumType.SORT_MODE)
class SortMode(BuiltinEnum):
    MAX = 1
    MIN = 2
    AVERAGE = 3
    SUM = 4
    MEDIAN = 5


@struct_(StructType.SORT)
class Sort(Struct):
    """ORDER BY specification."""

    type: SortType = p_regular(30)
    by: Expression = p_regular(31)
    mode: Optional[SortMode] = p_regular(32)


SortIn = Union[Sort, "Expression", "Field", "Property"]


def sort(sort: SortIn, type: SortType = SortType.ASCENDING) -> Sort:
    if isinstance(sort, Sort):
        return sort
    elif isinstance(sort, Expression):
        return Sort(type=type, by=sort)
    else:
        return Sort(type=type, by=expression(attribute_ref(sort)))


#
# Join
#


@enum_(EnumType.JOIN_TYPE)
class JoinType(BuiltinEnum):
    LEFT = 1
    # RIGHT, INNER, OUTER, CROSS?
    PARENT = 10
    CHILD = 11


@struct_(StructType.JOIN)
class Join(Struct):
    """JOIN clause with ON expression."""

    type: JoinType = p_regular(30)
    table: Optional[RelationReference] = p_regular(31)
    on: Optional[Condition] = p_regular(32)
    recursive: bool = p_regular(33)  # for parent/child joins


JoinIn = Union[Join, "JoinType"]


def join(
    join: JoinIn,
    table: "NodeType | type[Node] | Table | None" = None,
    on: Optional[Condition] = None,
    recursive: bool = False,
) -> Join:
    if isinstance(join, Join):
        return join
    else:
        return Join(
            type=join,
            table=relation_ref(table) if table is not None else None,
            on=on,
            recursive=recursive,
        )


#
# Query
#


@enum_(EnumType.QUERY_TYPE)
class QueryType(BuiltinEnum):
    GET = 1
    SEARCH = 2
    AGGREGATE = 3


@struct_(StructType.QUERY)
class Query[T: "Node"](Struct):
    """A GraphQL-inspired Query node with subqueries."""

    id: UUID = p_regular(2)
    type: QueryType = p_regular(30)
    name: str | None = p_regular(
        31, description="Name for this subquery. Must be unique within the containing Query."
    )
    relation: RelationReference = p_regular(35)
    join: Optional[Join] = p_regular(36, description="Relative to parent Query.")
    subqueries: list["Query"] = p_regular(37)
    # fields, ...

    where: Optional[Condition] = p_regular(40)
    having: Optional[Condition] = p_regular(41)
    group_by: list[Expression] = p_regular(42)
    aggregation: Optional[Aggregation] = p_regular(43)
    sort: list[Sort] = p_regular(44)

    limit: Optional[int] = p_regular(50)
    offset: Optional[int] = p_regular(51)
    count: bool = p_regular(52)


def to_subqueries(subqueries: dict[str, "Query"]) -> list["Query"]:
    """Turn Queries into subqueries with default names & parent joins."""
    for name, subquery in subqueries.items():
        assert subquery.name is None, f"subquery {subquery!r} already has a name"
        if subquery.join is None:
            subquery.join = join(JoinType.PARENT)
        subquery.name = name
    return list(subqueries.values())


def get[T: Node](
    node_cls: type[T] | NodeType,
    name: str,
    join: Optional[Join] = None,
    where: Optional[Condition] = None,
    **subqueries: Query,
) -> Query[T]:
    """Create a Get Query."""
    metatype = node_cls if isinstance(node_cls, NodeType) else node_cls.metatype
    return Query(
        type=QueryType.GET,
        relation=relation_ref(metatype),
        name=name or metatype.bench_name,
        join=join,
        where=where,
        subqueries=to_subqueries(subqueries),
    )


def search[T: Node](
    node_cls: type[T] | NodeType,
    name: str,
    join: Optional[Join] = None,
    where: Optional[Condition] = None,
    having: Optional[Condition] = None,
    sort: Optional[list[Sort]] = None,
    group_by: Optional[list[Expression]] = None,
    aggregation: Optional[Aggregation] = None,
    limit: Optional[int] = None,
    offset: Optional[int] = None,
    count: bool = False,
    **subqueries: Query,
) -> Query[T]:
    """Create a Search Query."""
    metatype = node_cls if isinstance(node_cls, NodeType) else node_cls.metatype
    return Query(
        type=QueryType.SEARCH,
        relation=relation_ref(metatype),
        name=name or metatype.bench_name,
        join=join,
        where=where,
        having=having,
        group_by=group_by or [],
        aggregation=aggregation,
        sort=sort or [],
        limit=limit,
        offset=offset,
        count=count,
        subqueries=to_subqueries(subqueries),
    )


def aggregate[T: Node](
    node_cls: type[T] | NodeType,
    name: str,
    join: Optional[Join] = None,
    where: Optional[Condition] = None,
    group_by: Optional[list[Expression]] = None,
    aggregation: Optional[Aggregation] = None,
    sort: Optional[list[Sort]] = None,
    limit: Optional[int] = None,
    offset: Optional[int] = None,
    count: bool = False,
) -> Query[T]:
    """Create an Aggregate Query."""
    metatype = node_cls if isinstance(node_cls, NodeType) else node_cls.metatype
    return Query(
        type=QueryType.AGGREGATE,
        relation=relation_ref(metatype),
        name=name or metatype.bench_name,
        join=join,
        where=where,
        group_by=group_by or [],
        aggregation=aggregation,
        sort=sort or [],
        limit=limit,
        offset=offset,
        count=count,
    )


#
# Query result
# nocheckin: QueryResult
#  GraphData, ..
#  GetResult/SearchResult/AggregateResult, ...
#

# q = User.get(
#     where=User.property("id").eq(5),
#     Clients=Client.search(),
#     BenchMemberships=Membership.search(
#         where=Membership.property("parent").eq(NodeType.BENCH),
#         sort=[Membership.property("created_at").desc()],
#     ),
# )

# q = Bench.get(
#     "Bench",
#     where=Bench.property("id").eq(5),
#     Packages=Package.search(
#         Pages=Page.search(limit=10, count=True),
#         Memberships=Membership.search(
#             sort=[Membership.property("created_at").desc()],
#             limit=10,
#             count=True,
#         ),
#     ),
#     Pages=Page.search(count=True),
# )


# q = Thread.get(
#     where=Thread.property("id").eq(5),
#     Messages=Message.search(
#         sort=[Message.property("created_at").asc()],
#         limit=100,
#         count=True,
#     ),
# )

# q = Thread.search(
#     sort=[Thread.property("last_active_at").asc()],
#     limit=25,
#     count=True,
#     Cursor=Cursor.get(
#         on=Cursor.property("owned_by").eq(5),
#         UnreadCount=Message.aggregate(
#             where=Message.property("read_at").greater_than(attribute_ref("Cursor.last_read_at")),
#             aggregation=Aggregation(type=AggregationType.COUNT),
#         ),
#     ),
# )

# q = Space.get(
#     where=Space.property("id").eq(5),
#     Scenes=Scene.search(
#         Fields=Field.search(),
#         Themes=Theme.search(),
#         Views=ViewBase.search(join=join(JoinType.PARENT, recursive=True)),
#         Styles=IsStyle.search(join=join(JoinType.PARENT, recursive=True)),
#     ),
#     Route=Route.search(),
# )


@dataclass(slots=True)
class QueryResult:
    query: Query


#
# Queryable
#


class IsQueryable:
    def is_equal(self: Any, value: Any) -> "Condition":
        if value is None:
            return self.not_exists()
        return condition(self, ConditionalType.EQUALS, value=value)

    def not_equal(self: Any, value: Any) -> "Condition":
        return condition(self, ConditionalType.NOT_EQUALS, value=value)

    def greater_than(self: Any, value: Any) -> "Condition":
        return condition(self, ConditionalType.GREATER_THAN, value=value)

    def greater_than_or_equals(self: Any, value: Any) -> "Condition":
        return condition(self, ConditionalType.GREATER_THAN_OR_EQUALS, value=value)

    def less_than(self: Any, value: Any) -> "Condition":
        return condition(self, ConditionalType.LESS_THAN, value=value)

    def less_than_or_equals(self: Any, value: Any) -> "Condition":
        return condition(self, ConditionalType.LESS_THAN_OR_EQUALS, value=value)

    eq = is_equal
    neq = not_equal
    lt = less_than
    lte = less_than_or_equals
    gt = greater_than
    gte = greater_than_or_equals

    def starts_with(self: Any, value: str) -> "Condition":
        return condition(self, ConditionalType.STARTS_WITH, value=value)

    startswith = starts_with

    def ends_with(self: Any, value: str) -> "Condition":
        return condition(self, ConditionalType.ENDS_WITH, value=value)

    endswith = ends_with

    def in_(self: Any, *values: list[Any]) -> "Condition":
        return condition(self, ConditionalType.IN, value=values)

    def not_in(self: Any, *values: list[Any]) -> "Condition":
        return condition(self, ConditionalType.NOT_IN, value=values)

    def contains(self: Any, value: Any) -> "Condition":
        return condition(self, ConditionalType.CONTAINS, value=value)

    def not_contains(self: Any, value: Any) -> "Condition":
        return condition(self, ConditionalType.NOT_CONTAINS, value=value)

    def exists(self: Any) -> "Condition":
        return condition(self, ConditionalType.EXISTS)

    def is_not_none(self: Any) -> "Condition":
        return condition(self, ConditionalType.EXISTS)

    def not_exists(self: Any) -> "Condition":
        return condition(self, ConditionalType.NOT_EXISTS)

    def is_none(self: Any) -> "Condition":
        return condition(self, ConditionalType.NOT_EXISTS)

    def asc(self: Any) -> "Sort":
        return sort(self, SortType.ASCENDING)

    ascending = asc

    def desc(self: Any) -> "Sort":
        return sort(self, SortType.DESCENDING)

    descending = desc

    def sum(self: Any) -> "Aggregation":
        return aggregation(AggregationType.SUM, operand=self)

    def min(self: Any) -> "Aggregation":
        return aggregation(AggregationType.MIN, operand=self)

    def max(self: Any) -> "Aggregation":
        return aggregation(AggregationType.MAX, operand=self)

    def average(self: Any) -> "Aggregation":
        return aggregation(AggregationType.AVERAGE, operand=self)

    def median(self: Any) -> "Aggregation":
        return aggregation(AggregationType.MEDIAN, operand=self)


@struct_(StructType.SELECTION)
class Selection(Struct):
    """A selection of fields from a Node."""

    nodes: list[Node] = p_regular(40)

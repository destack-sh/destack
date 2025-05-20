# ruff: noqa: RUF012

from typing import TYPE_CHECKING, Any, Optional, Union, assert_never

from fastuuid import UUID

from .const import BuiltinEnum, EnumType, NodeType, StructType, enum_
from .node import Node, NodeReference
from .object import Property
from .property import p_regular
from .struct import Struct, struct_
from .value import Value

if TYPE_CHECKING:
    from bench.language import *

# pyright: reportIncompatibleVariableOverride=false
# nocheckin
# ruff: noqa: F405


@struct_(StructType.TABLE_REFERENCE)
class TableReference(Struct):
    """Reference to a Node "table"."""

    node_type: NodeType = p_regular(31)
    table: Optional["Table"] = p_regular(32)

    if TYPE_CHECKING:
        table_ptr: Optional[NodeReference] = None  # convenience only


def table_ref(base: "NodeType | type[Node] | Table") -> TableReference:
    from .node import Node

    if isinstance(base, NodeType):
        return TableReference(node_type=base)
    elif isinstance(base, type):
        assert issubclass(base, Node), f"{base!r} is not a Node"
        return TableReference(node_type=base.metatype)
    elif isinstance(base, Table):
        return TableReference(table=base)
    else:
        assert_never(base)


@struct_(StructType.FIELD_REFERENCE)
class FieldReference(Struct):
    """Reference to a Field or Property."""

    field: Optional["Field"] = p_regular(32)
    prop: Optional["Property"] = p_regular(33)
    table: Optional[TableReference] = p_regular(34)


def field_ref(field: "Field | Property") -> FieldReference:
    if isinstance(field, Field):
        return FieldReference(field=field)
    elif isinstance(field, Property):
        return FieldReference(prop=field)
    else:
        assert_never(field)


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
    column: Optional[FieldReference] = p_regular(32)
    condition: Optional[Condition] = p_regular(33)
    function: Optional[Function] = p_regular(34)
    aggregation: Optional[Aggregation] = p_regular(35)
    # subquery?


def expression(
    thing: "Value | FieldReference | Condition | Function | Aggregation",
) -> Expression:
    if isinstance(thing, Value):
        return Expression(type=ExpressionType.LITERAL, literal=thing)
    elif isinstance(thing, FieldReference):
        return Expression(type=ExpressionType.COLUMN, column=thing)
    elif isinstance(thing, Condition):
        return Expression(type=ExpressionType.CONDITION, condition=thing)
    elif isinstance(thing, Function):
        return Expression(type=ExpressionType.FUNCTION, function=thing)
    elif isinstance(thing, Aggregation):
        return Expression(type=ExpressionType.AGGREGATION, aggregation=thing)
    else:
        assert_never(thing)


@enum_(EnumType.SORT_TYPE)
class SortType(BuiltinEnum):
    ASCENDING = 500
    DESCENDING = 501


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


def sort(column: Union["Field", "Property"], type: SortType = SortType.ASCENDING) -> Sort:
    return Sort(
        type=type,
        by=Expression(type=ExpressionType.COLUMN, column=field_ref(column)),
    )


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
    table: Optional[TableReference] = p_regular(31)
    on: Optional[Condition] = p_regular(32)
    recursive: bool = p_regular(33)


def join(
    type: JoinType,
    table: "NodeType | type[Node] | Table | None" = None,
    on: Optional[Condition] = None,
    recursive: bool = False,
) -> Join:
    return Join(
        type=type,
        table=table_ref(table) if table is not None else None,
        on=on,
        recursive=recursive,
    )


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
    table: TableReference = p_regular(35)
    join: Optional[Join] = p_regular(36, description="Relative to parent Query.")
    subqueries: list["Query"] = p_regular(37)
    # fields, ...

    where: Optional[Condition] = p_regular(40)
    having: Optional[Condition] = p_regular(41)
    group_by: list[Expression] = p_regular(42)
    aggregation: Optional[Aggregation] = p_regular(43)

    sort: list[Sort] = p_regular(50)
    limit: Optional[int] = p_regular(51)
    offset: Optional[int] = p_regular(52)
    count: bool = p_regular(53)


def get[T: Node](
    node_cls: type[T] | NodeType,
    name: str,
    join: Optional[Join] = None,
    where: Optional[Condition] = None,
    **subqueries: Query,
) -> Query[T]:
    """Create a Get Query."""
    for name, subquery in subqueries.items():
        subquery.name = name
    return Query(
        type=QueryType.GET,
        table=table_ref(node_cls),
        name=name,
        join=join,
        where=where,
        subqueries=list(subqueries.values()),
    )


def search[T: Node](
    node_cls: type[T],
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
    for name, subquery in subqueries.items():
        subquery.name = name
    return Query(
        type=QueryType.SEARCH,
        table=table_ref(node_cls),
        name=name,
        join=join,
        where=where,
        having=having,
        group_by=group_by or [],
        aggregation=aggregation,
        sort=sort or [],
        limit=limit,
        offset=offset,
        count=count,
        subqueries=list(subqueries.values()),
    )


def aggregate[T: Node](
    node_cls: type[T],
    name: str,
    join: Optional[Join] = None,
    where: Optional[Condition] = None,
    group_by: Optional[list[Expression]] = None,
    aggregation: Optional[Aggregation] = None,
    sort: Optional[list[Sort]] = None,
    limit: Optional[int] = None,
    offset: Optional[int] = None,
    count: bool = False,
    **subqueries: Query,
) -> Query[T]:
    """Create an Aggregate Query."""
    for name, subquery in subqueries.items():
        subquery.name = name
    return Query(
        type=QueryType.AGGREGATE,
        table=table_ref(node_cls),
        name=name,
        join=join,
        where=where,
        group_by=group_by or [],
        aggregation=aggregation,
        sort=sort or [],
        limit=limit,
        offset=offset,
        count=count,
        subqueries=list(subqueries.values()),
    )


#
# Example Queries
#

q = Bench.get(
    "Bench",
    where=Bench.get_property("id").eq(5),
    Packages=Package.search(
        Pages=Page.search(limit=10, count=True),
        Memberships=Membership.search(
            sort=Membership.get_property("joined_at"),
            limit=10,
            count=True,
        ),
    ),
    Pages=Page.search(count=True),
)


q = Thread.get(
    where=Thread.get_property("id").eq(5),
    order_by=Thread.get_property("last_active_at"),
    Messages=Message.search(sort=Message.get_property("created_at"), limit=100, count=True),
)

q = Thread.search(
    sort=Thread.get_property("last_active_at"),
    limit=20,
    count=True,
    Cursor=Cursor.get(
        join=Cursor,
        on=Cursor.get_property("target") & Cursor.get_property("owner"),
        UnreadCount=Message.aggregate(),
    ),
)


q = Space.get(
    where=Space.get_property("id").eq(5),
    Scenes=Scene.search(
        Fields=Field.search(),
        Themes=Theme.search(),
        Views=ViewBase.search(join=join(JoinType.PARENT, recursive=True)),
        Styles=StyleBase.search(join=join(JoinType.PARENT, recursive=True)),
    ),
    Route=Route.search(),
)


class IsQueryable:
    @property
    def type_info(self) -> "TypeBase":
        raise NotImplementedError(f"{self!r} does not implement type")

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

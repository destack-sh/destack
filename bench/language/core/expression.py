# ruff: noqa: RUF012

from typing import TYPE_CHECKING, Any, Optional, Union

from fastuuid import UUID

from .const import BuiltinEnum, EnumType, NodeType, StructType, enum_
from .node import Node, NodeReference
from .object import Property
from .property import p_regular
from .struct import Struct, struct_
from .value import Value

if TYPE_CHECKING:
    from bench.language import Field, Node, Table, TypeBase

# pyright: reportIncompatibleVariableOverride=false


@struct_(StructType.TABLE_REFERENCE)
class TableReference(Struct):
    """Reference to a Node "table"."""

    node_type: NodeType = p_regular(31)
    table: Optional["Table"] = p_regular(32)

    if TYPE_CHECKING:
        table_ptr: Optional[NodeReference] = None  # convenience only


@struct_(StructType.FIELD_REFERENCE)
class FieldReference(Struct):
    """Reference to a Field or Property."""

    field: Optional["Field"] = p_regular(32)
    prop: Optional["Property"] = p_regular(33)
    table: Optional[TableReference] = p_regular(34)


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


@enum_(EnumType.JOIN_TYPE)
class JoinType(BuiltinEnum):
    INNER = 1
    LEFT = 2
    RIGHT = 3
    OUTER = 4
    CROSS = 5
    PARENT = 10
    CHILD = 11
    ANCESTOR = 12
    DESCENDANT = 13


@struct_(StructType.JOIN)
class Join(Struct):
    """JOIN clause with ON expression."""

    type: JoinType = p_regular(30)
    table: Optional[TableReference] = p_regular(31)
    on: Optional[Condition] = p_regular(32)


@enum_(EnumType.QUERY_TYPE)
class QueryType(BuiltinEnum):
    GET = 1
    LIST = 2
    AGGREGATE = 3


@struct_(StructType.QUERY)
class Query[T: "Node"](Struct):
    """A GraphQL-inspired Query node with subqueries."""

    id: UUID = p_regular(2)
    type: QueryType = p_regular(30)
    name: str = p_regular(
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

    order_by: list[Sort] = p_regular(50)
    limit: Optional[int] = p_regular(51)
    offset: Optional[int] = p_regular(52)
    count: bool = p_regular(53)


def condition(
    op: ConditionalType, column: Union["Field", "Property"], value: Any = None
) -> Condition:
    raise NotImplementedError


def sort(op: SortType, column: Union["Field", "Property"]) -> Sort:
    raise NotImplementedError


#
# Example Queries
#

# Bench->GET[id=...]
#    Package->LIST
#        Page->LIST[sort=last_edited_at, limit=10, count=True]
#        "Owner"->GET[join=User, on=Page.owned_by]
#    Membership->LIST[sort=joined_at, limit=10, count=True]
#        Member->GET[join=Subject]
#    Page->LIST[count=True]
# Q(
#     Bench,
#     subqueries=[
#         Q(Package)
#     ]
#     )

# Page->GET[id=...]
#    Block->LIST[recursive=true]
#    *PageNode->LIST

# Thread->GET[limit=20, count=True, order_by=last_active_at]
#    Message->LIST[sort=created_at, limit=100, count=True]
#        "Author"->GET[join=User, on=Message.created_by]

# Thread->LIST[sort=last_active_at, limit=20, count=True]
#    "Cursor"->GET[join=Cursor, on=Cursor.target & Cursor.owner=...]
#        "UnreadCount"->AGGREGATE[count(distinct Message where read_at >= "Cursor".last_read_at)]
#    "Banner"->GET[join=Banner]
#        File->GET[join=File]

# Record.Event->LIST[sort=date, count=True]
#    "RSVPCount"->AGGREGATE[count(distinct Record.EventResponse where response = 'yes')]
#    "CreatedBy"->GET[join=User, on=Record.Event.created_by]
#    "Speakers"->LIST[join=Record.EventSpeaker]
#    "Responses"->AGGREGATE[group_by=response, count(distinct Record.EventResponse)]
#        "Responder"->GET[join=User, on=Record.EventResponse.responded_by]
#    Record.EventResponse->LIST[sort=created_at, limit=3, count=True]

# Space->GET[id=...]
#     Scene->LIST
#         Field->LIST
#         Theme->LIST
#         *ViewBase->LIST[recursive=True]
#            Field->LIST
#         *StyleBase->LIST[recursive=True]
#            Field->LIST
#     Route->LIST

# Reaction->AGGREGATE[parent=..., sort=created_at, group_by=Reaction.type, limit=100, count=True]
#    "Reaction"->GET[join=Reaction]
#    "CreatedBy"->GET[join=User, on=Reaction.created_by]


class IsQueryable:
    @property
    def type_info(self) -> "TypeBase":
        raise NotImplementedError(f"{self!r} does not implement type")

    def is_equal(self: Any, value: Any) -> "Condition":
        if value is None:
            return self.not_exists()
        return condition(ConditionalType.EQUALS, self, value=value)

    def not_equal(self: Any, value: Any) -> "Condition":
        return condition(ConditionalType.NOT_EQUALS, self, value=value)

    def greater_than(self: Any, value: Any) -> "Condition":
        return condition(ConditionalType.GREATER_THAN, self, value=value)

    def greater_than_or_equals(self: Any, value: Any) -> "Condition":
        return condition(ConditionalType.GREATER_THAN_OR_EQUALS, self, value=value)

    def less_than(self: Any, value: Any) -> "Condition":
        return condition(ConditionalType.LESS_THAN, self, value=value)

    def less_than_or_equals(self: Any, value: Any) -> "Condition":
        return condition(ConditionalType.LESS_THAN_OR_EQUALS, self, value=value)

    eq = is_equal
    neq = not_equal
    lt = less_than
    lte = less_than_or_equals
    gt = greater_than
    gte = greater_than_or_equals

    def starts_with(self: Any, value: str) -> "Condition":
        return condition(ConditionalType.STARTS_WITH, self, value=value)

    startswith = starts_with

    def ends_with(self: Any, value: str) -> "Condition":
        return condition(ConditionalType.ENDS_WITH, self, value=value)

    endswith = ends_with

    def in_(self: Any, *values: list[Any]) -> "Condition":
        return condition(ConditionalType.IN, self, value=values)

    def not_in(self: Any, *values: list[Any]) -> "Condition":
        return condition(ConditionalType.NOT_IN, self, value=values)

    def contains(self: Any, value: Any) -> "Condition":
        return condition(ConditionalType.CONTAINS, self, value=value)

    def not_contains(self: Any, value: Any) -> "Condition":
        return condition(ConditionalType.NOT_CONTAINS, self, value=value)

    def exists(self: Any) -> "Condition":
        return condition(ConditionalType.EXISTS, self)

    def is_not_none(self: Any) -> "Condition":
        return condition(ConditionalType.EXISTS, self)

    def not_exists(self: Any) -> "Condition":
        return condition(ConditionalType.NOT_EXISTS, self)

    def is_none(self: Any) -> "Condition":
        return condition(ConditionalType.NOT_EXISTS, self)

    def asc(self: Any) -> "Sort":
        return sort(SortType.ASCENDING, self)

    ascending = asc

    def desc(self: Any) -> "Sort":
        return sort(SortType.DESCENDING, self)

    descending = desc

    #
    # Aggregation
    #

    ...


@struct_(StructType.SELECTION)
class Selection(Struct):
    """A selection of fields from a Node."""

    nodes: list[Node] = p_regular(40)

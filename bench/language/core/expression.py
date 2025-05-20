# ruff: noqa: RUF012

from typing import TYPE_CHECKING, Any, Optional, Union

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
    """Reference to a SQL table (built-in or custom)."""

    node_type: NodeType = p_regular(31)
    table: Optional["Table"] = p_regular(32)

    if TYPE_CHECKING:
        table_ptr: Optional[NodeReference] = None  # convenience only


@struct_(StructType.COLUMN_REFERENCE)
class ColumnReference(Struct):
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
    BINARY = 4
    AGGREGATION = 5
    SUBQUERY = 10


@struct_(StructType.EXPRESSION)
class Expression(Struct):
    """Wrapper to unify any scalar / boolean / aggregate sub-tree."""

    type: ExpressionType = p_regular(30)
    literal: Optional[Value] = p_regular(31)
    column: Optional[ColumnReference] = p_regular(32)
    function: Optional[Function] = p_regular(33)
    condition: Optional[Condition] = p_regular(34)
    aggregation: Optional[Aggregation] = p_regular(35)
    subquery: Optional["Query"] = p_regular(36)


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


@struct_(StructType.JOIN)
class Join(Struct):
    """JOIN clause with ON expression."""

    type: JoinType = p_regular(30)
    table: TableReference = p_regular(31)
    on: Optional[Condition] = p_regular(32)


@struct_(StructType.QUERY)
class Query[T: "Node"](Struct):
    """A SELECT-only query supporting joins, grouping, aggregation and paging."""

    table: TableReference = p_regular(31)
    columns: list[ColumnReference] = p_regular(32)
    joins: list[Join] = p_regular(33)
    where: Optional[Expression] = p_regular(34)
    group_by: list[Expression] = p_regular(35)
    having: Optional[Expression] = p_regular(36)
    order_by: list[Sort] = p_regular(37)
    limit: Optional[int] = p_regular(38)
    offset: Optional[int] = p_regular(39)


def condition(op: ConditionalType, column: Union["Field", "Property"], value: Any = None):
    raise NotImplementedError


def sort(op: SortType, column: Union["Field", "Property"]):
    raise NotImplementedError


class IsQueryable:
    """
    Base for field-like  on a field-like class.
    We define this here to use it for Property and Field.
    """

    @property
    def type_info(self) -> "TypeBase":
        raise NotImplementedError(f"{self!r} does not implement type")

    def is_equal(self: Any, value: Any) -> "Expression":
        if value is None:
            return self.not_exists()
        return condition(ConditionalType.EQUALS, self, value=value)

    def not_equal(self: Any, value: Any) -> "Expression":
        return condition(ConditionalType.NOT_EQUALS, self, value=value)

    def greater_than(self: Any, value: Any) -> "Expression":
        return condition(ConditionalType.GREATER_THAN, self, value=value)

    def greater_than_or_equals(self: Any, value: Any) -> "Expression":
        return condition(ConditionalType.GREATER_THAN_OR_EQUALS, self, value=value)

    def less_than(self: Any, value: Any) -> "Expression":
        return condition(ConditionalType.LESS_THAN, self, value=value)

    def less_than_or_equals(self: Any, value: Any) -> "Expression":
        return condition(ConditionalType.LESS_THAN_OR_EQUALS, self, value=value)

    eq = is_equal
    neq = not_equal
    lt = less_than
    lte = less_than_or_equals
    gt = greater_than
    gte = greater_than_or_equals

    def starts_with(self: Any, value: str) -> "Expression":
        return condition(ConditionalType.STARTS_WITH, self, value=value)

    startswith = starts_with

    def ends_with(self: Any, value: str) -> "Expression":
        return condition(ConditionalType.ENDS_WITH, self, value=value)

    endswith = ends_with

    def in_(self: Any, *values: list[Any]) -> "Expression":
        return condition(ConditionalType.IN, self, value=values)

    def not_in(self: Any, *values: list[Any]) -> "Expression":
        return condition(ConditionalType.NOT_IN, self, value=values)

    def contains(self: Any, value: Any) -> "Expression":
        return condition(ConditionalType.CONTAINS, self, value=value)

    def not_contains(self: Any, value: Any) -> "Expression":
        return condition(ConditionalType.NOT_CONTAINS, self, value=value)

    def exists(self: Any) -> "Expression":
        return condition(ConditionalType.EXISTS, self)

    def is_not_none(self: Any) -> "Expression":
        return condition(ConditionalType.EXISTS, self)

    def not_exists(self: Any) -> "Expression":
        return condition(ConditionalType.NOT_EXISTS, self)

    def is_none(self: Any) -> "Expression":
        return condition(ConditionalType.NOT_EXISTS, self)

    def asc(self: Any) -> "Expression":
        return sort(SortType.ASCENDING, self)

    ascending = asc

    def desc(self: Any) -> "Expression":
        return sort(SortType.DESCENDING, self)

    descending = desc

    #
    # Aggregation
    #

    ...


@struct_(StructType.SELECTION)
class Selection(Struct):
    """A selection of Nodes."""

    nodes: list[Node] = p_regular(50)
    fields: list["Field"] = p_regular(51)
    properties: list[Property] = p_regular(52)

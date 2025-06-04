from typing import TYPE_CHECKING, Any, Optional, Union, assert_never

from fastuuid import UUID

from .const import (
    BuiltinEnum,
    EnumType,
    NodeType,
    StructType,
    active_session,
    enum_,
)
from .node import Node
from .object import BuiltinObjectMutable, Property, object_
from .property import property_
from .struct import NodeReference, PropertyReference, StructFrozen, StructMutable, struct_
from .value import Value

if TYPE_CHECKING:
    from bench.language import CustomNodeDefinition, Field, QueryConnection

# pyright: reportIncompatibleVariableOverride=false

#
# Relations/Attributes
#


@enum_(EnumType.RELATION_TYPE)
class RelationType(BuiltinEnum):
    BUILTIN_NODE = 1
    CUSTOM_NODE = 2


@struct_(StructType.RELATION_REFERENCE, frozen=True)
class RelationReference(StructFrozen):
    """Reference to a Node (builtin or custom)."""

    type: RelationType = property_(30, is_repr=True)
    node_type: NodeType = property_(31, is_repr=True)
    definition: Optional["CustomNodeDefinition"] = property_(32, is_repr=True)
    if TYPE_CHECKING:
        definition_id: Optional[UUID] = None
        definition_ptr: Optional[NodeReference] = None


def relation_ref(base: "NodeType | type[Node] | CustomNodeDefinition") -> RelationReference:
    from .node import Node

    if isinstance(base, NodeType):
        return RelationReference(type=RelationType.BUILTIN_NODE, node_type=base)
    elif isinstance(base, type):
        assert issubclass(base, Node), f"{base!r} is not a Node"
        return RelationReference(type=RelationType.BUILTIN_NODE, node_type=base.metatype)
    elif isinstance(base, CustomNodeDefinition):
        return RelationReference(type=RelationType.CUSTOM_NODE, node_type=base.metatype)
    else:
        assert_never(base)


@enum_(EnumType.ATTRIBUTE_TYPE)
class AttributeType(BuiltinEnum):
    PROPERTY = 1
    FIELD = 2


@struct_(StructType.ATTRIBUTE_REFERENCE, frozen=True)
class AttributeReference(StructFrozen):
    """Reference to a Field or Property."""

    type: AttributeType = property_(30, is_repr=True)
    prop: Optional["Property"] = property_(31, is_repr=True)
    field: Optional["Field"] = property_(32, is_repr=True)
    relation: Optional[RelationReference] = property_(33, is_repr=True)
    if TYPE_CHECKING:
        prop_ptr: Optional[PropertyReference] = None
        field_id: Optional[UUID] = None
        field_ptr: Optional[NodeReference] = None


def attribute_ref(field: "Field | Property") -> AttributeReference:
    if isinstance(field, Property):
        return AttributeReference(type=AttributeType.PROPERTY, prop=field)
    elif isinstance(field, Node):
        return AttributeReference(type=AttributeType.FIELD, field=field)
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


@struct_(StructType.FUNCTION, frozen=True)
class Function(StructFrozen):
    type: FunctionType = property_(30, is_repr=True)
    left: "Expression" = property_(31, is_repr=True)
    right: Optional["Expression"] = property_(32, is_repr=True)


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
    IN = 30
    NOT_IN = 31
    # existence
    EXISTS = 40
    NOT_EXISTS = 41


@struct_(StructType.CONDITION, frozen=True)
class Condition(StructFrozen):
    """Boolean predicate (AND, =, <, etc.)."""

    type: ConditionalType = property_(30, is_repr=True)
    left: "Expression" = property_(31, is_repr=True)
    right: Optional["Expression"] = property_(32, is_repr=True)

    def __or__(self, other: "Condition") -> "Condition":
        return Condition(type=ConditionalType.OR, left=expression(self), right=expression(other))

    def __and__(self, other: "Condition") -> "Condition":
        return Condition(type=ConditionalType.AND, left=expression(self), right=expression(other))


def condition(
    attribute: Union["Field", "Property"],
    type: ConditionalType = ConditionalType.EQUALS,
    value: Any = None,
) -> Condition:
    from .value import to_value

    return Condition(
        type=type,
        left=expression(attribute_ref(attribute)),
        right=expression(to_value(value)),
    )


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


@struct_(StructType.AGGREGATION, frozen=True)
class Aggregation(StructFrozen):
    """Aggregation."""

    type: AggregationType = property_(30, is_repr=True)
    expression: Optional["Expression"] = property_(31, is_repr=True)
    # distinct, over, ...


def aggregation(
    type: AggregationType,
    operand: Optional["Expression"] = None,
) -> Aggregation:
    return Aggregation(type=type, expression=operand)


#
# Expression
#


@enum_(EnumType.EXPRESSION_TYPE)
class ExpressionType(BuiltinEnum):
    LITERAL = 1
    ATTRIBUTE = 2
    CONDITION = 3
    FUNCTION = 4
    AGGREGATION = 5
    # SUBQUERY?


@struct_(StructType.EXPRESSION, frozen=True)
class Expression(StructFrozen):
    """Wrapper to unify any scalar / boolean / aggregate sub-tree."""

    type: ExpressionType = property_(30, is_repr=True)
    literal: Optional[Value] = property_(31, is_repr=True)
    attribute: Optional[AttributeReference] = property_(32, is_repr=True)
    condition: Optional[Condition] = property_(33, is_repr=True)
    function: Optional[Function] = property_(34, is_repr=True)
    aggregation: Optional[Aggregation] = property_(35, is_repr=True)
    # subquery?


ExpressionIn = Union[
    "Value", "AttributeReference", "Condition", "Function", "Aggregation", "Expression"
]


def expression(
    thing: ExpressionIn,
) -> Expression:
    if isinstance(thing, Value):
        return Expression(type=ExpressionType.LITERAL, literal=thing)
    elif isinstance(thing, AttributeReference):
        return Expression(type=ExpressionType.ATTRIBUTE, attribute=thing)
    elif isinstance(thing, Condition):
        return Expression(type=ExpressionType.CONDITION, condition=thing)
    elif isinstance(thing, Function):
        return Expression(type=ExpressionType.FUNCTION, function=thing)
    elif isinstance(thing, Aggregation):
        return Expression(type=ExpressionType.AGGREGATION, aggregation=thing)
    elif isinstance(thing, Expression):
        return thing
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


@struct_(StructType.SORT, frozen=True)
class Sort(StructFrozen):
    """ORDER BY specification."""

    type: SortType = property_(30, is_repr=True)
    by: Expression = property_(31, is_repr=True)
    mode: Optional[SortMode] = property_(32, is_repr=True)


SortIn = Union[Sort, "Expression", "Field", "Property"]


def sort(sort: SortIn, type: SortType = SortType.ASCENDING) -> Sort:
    if isinstance(sort, Sort):
        return sort
    elif isinstance(sort, Expression):
        return Sort(type=type, by=sort)
    else:
        return Sort(type=type, by=expression(attribute_ref(sort)))


#
# Select
#


@struct_(StructType.SELECT, frozen=True)
class Select(StructFrozen):
    """Select specific Attributes."""

    properties: list["Property"] = property_(31, is_repr=True)
    fields: list["Field"] = property_(32, is_repr=True)


def select(*attributes: "Property | Field") -> Select:
    properties: list[Property] = []
    fields: list[Field] = []
    for attribute in attributes:
        if isinstance(attribute, Property):
            properties.append(attribute)
        elif isinstance(attribute, Field):
            fields.append(attribute)
    return Select(properties=properties, fields=fields)


#
# Join
#


@enum_(EnumType.JOIN_TYPE)
class JoinType(BuiltinEnum):
    LEFT = 1
    # RIGHT, INNER, OUTER, CROSS?
    PARENT = 10
    CHILD = 11


@struct_(StructType.JOIN, frozen=True)
class Join(StructFrozen):
    """JOIN clause with ON expression."""

    type: JoinType = property_(30, is_repr=True)
    relation: Optional[RelationReference] = property_(31, is_repr=True)
    on: Optional[Condition] = property_(32, is_repr=True)
    recursive: bool = property_(33, default=False, is_repr=True)  # for parent/child joins


JoinIn = Union[Join, "JoinType"]


def join(
    join: JoinIn,
    relation: "NodeType | type[Node] | CustomNodeDefinition | None" = None,
    on: Optional[Condition] = None,
    recursive: bool = False,
) -> Join:
    if isinstance(join, Join):
        return join
    else:
        return Join(
            type=join,
            relation=relation_ref(relation) if relation is not None else None,
            on=on,
            recursive=recursive,
        )


#
# Query
#


@enum_(EnumType.QUERY_TYPE)
class QueryType(BuiltinEnum):
    NODE = 1
    SCALAR = 2
    GROUPED_NODE = 10
    GROUPED_SCALAR = 11


@struct_(StructType.QUERY, frozen=True)
class Query[RootT: "Node"](StructFrozen):
    """A GraphQL-inspired Query node (with subqueries)."""

    # meta
    id: UUID = property_(2, default_factory="uuid")
    type: QueryType = property_(30, is_repr=True)
    name: str = property_(
        31,
        description="Name for this subquery. Must be unique within the parent Query.",
        is_repr=True,
    )
    relation: RelationReference = property_(32, is_repr=True)
    join: Optional[Join] = property_(
        33,
        description="Relative to parent Query.",
        default=Join(type=JoinType.CHILD),
        is_repr=True,
    )
    select: Optional[Select] = property_(34, is_repr=True)
    subqueries: list["Query"] = property_(35)
    is_live: bool = property_(39, default=True)

    # content
    where: Optional[Condition] = property_(40, is_repr=True)
    having: Optional[Condition] = property_(41, is_repr=True)
    group_by: list[Expression] = property_(42, is_repr=True)
    aggregation: Optional[Aggregation] = property_(43, is_repr=True)
    sort: list[Sort] = property_(44, is_repr=True)

    # pagination
    limit: Optional[int] = property_(50, is_repr=True)
    offset: Optional[int] = property_(51, is_repr=True)
    count: bool | None = property_(52, is_repr=True)

    async def execute(self) -> "QueryConnection[RootT]":
        """Execute the Query."""
        from .connection import QueryConnection

        session = active_session()
        store = session.store
        assert store is not None, f"no store in {session!r}"
        connection = QueryConnection(self, store, session)
        session.connections.append(connection)
        await connection.execute()
        return connection

    async def execute_one_or_none(self) -> Optional[RootT]:
        """Execute the Query and return the root (if any)."""
        connection = await self.execute()
        return connection.to_one_or_none()

    async def execute_one(self) -> RootT:
        """Execute the Query and return the root (error if none)."""
        connection = await self.execute()
        return connection.to_one()

    async def execute_list(self) -> list[RootT]:
        """Execute the Query and return the list of roots."""
        connection = await self.execute()
        return connection.to_list()

    async def execute_exists(self) -> bool:
        """Execute the Query and return whether any results exist."""
        connection = await self.execute()
        return connection.to_exists()

    async def execute_count(self) -> int:
        """Execute the Query and return the count."""
        connection = await self.execute()
        return connection.to_count()

    async def execute_scalar(self) -> Value:
        """Execute the Query and return the scalar value."""
        connection = await self.execute()
        return connection.to_scalar()


def to_subqueries(subqueries: dict[str, "Query"]) -> list["Query"]:
    """Turn Queries into subqueries with default names & parent joins."""
    for name, subquery in subqueries.items():
        if subquery.join is None:
            object.__setattr__(subquery, "join", join(JoinType.CHILD))
        object.__setattr__(subquery, "name", name)
        subquery._invalidate_frozen_cache()
    return list(subqueries.values())


@struct_(StructType.HISTOGRAM, frozen=True)
class Histogram(StructFrozen):
    """A histogram."""

    buckets: list[Value] = property_(40, is_repr=True)
    counts: list[int] = property_(41, is_repr=True)


@object_()
class QueryResultBase(BuiltinObjectMutable):
    """Common base for QueryResult and QueryResultGroup."""

    type: QueryType = property_(30, is_repr=True)

    nodes: list[Value] = property_(40, is_repr=True)
    count: Optional[int] = property_(41, is_repr=True)
    exists: Optional[bool] = property_(42, is_repr=True)
    scalar: Optional[Value] = property_(43, is_repr=True)


@struct_(StructType.QUERY_RESULT)
class QueryResult(QueryResultBase, StructMutable):
    """
    The result of a Query.
    For grouped queries, group results are in Query.groups.
    The subresults correspond to Query.subqueries.
    """

    id: UUID = property_(2, is_repr=True)
    groups: list["QueryResultGroup"] = property_(35, is_repr=True)
    subresults: list["QueryResult"] = property_(36, is_repr=True)


@struct_(StructType.QUERY_RESULT_GROUP)
class QueryResultGroup(QueryResultBase, StructMutable):
    """A group in a QueryResult."""

    discriminator: Optional[Value] = property_(31, is_repr=True)


@enum_(EnumType.QUERY_UPDATE_TYPE)
class QueryUpdateType(BuiltinEnum):
    FULL_RESULT = 1, "Full Result", "Full result tree"
    PARTIAL_RESULT = 2, "Partial Result", "Just this result"
    ...


@struct_(StructType.QUERY_UPDATE, frozen=True)
class QueryUpdate(StructFrozen):
    """An update to a QueryResult."""

    type: QueryUpdateType = property_(30, is_repr=True)
    result: Optional["QueryResult"] = property_(31, is_repr=True)


#
# Queryable
#


class IntoQuery:
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


@struct_(StructType.SELECTION, frozen=True)
class Selection(StructFrozen):
    """A selection of fields from a Node."""

    nodes: list[Node] = property_(40)

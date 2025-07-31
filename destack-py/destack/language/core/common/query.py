from typing import TYPE_CHECKING, Any, Optional, Union, assert_never, final

from destack.utils.uuid import UUID

from ..builtin import (
    Enum,
    EnumType,
    Node,
    ObjectDefinitionReference,
    PropertyDeclaration,
    PropertyReference,
    StructFrozen,
    StructType,
    TypeCardinality,
    UInt32,
    Value,
    ValueFactory,
    builtin_enum,
    builtin_property,
    builtin_struct,
)

if TYPE_CHECKING:
    from destack.language import (
        CustomPropertyDefinition,
        PropertyDefinition,
    )

# pyright: reportIncompatibleVariableOverride=false

type_ = type


#
# Conditional
#


@builtin_enum(EnumType.CONDITIONAL_TYPE)
class ConditionalType(Enum):
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


@builtin_struct(
    StructType.CONDITION,
    frozen=True,
    is_final=True,
)
@final
class Condition(StructFrozen):
    """Boolean predicate (AND, =, <, etc.)."""

    type: ConditionalType = builtin_property(100, is_repr=True)
    left: "Expression" = builtin_property(101, is_repr=True)
    right: Optional["Expression"] = builtin_property(102, is_repr=True)

    def __or__(self, right: "Condition") -> "Condition":
        """OR two Conditions."""
        return Condition(
            type=ConditionalType.OR, left=Expression.of(self), right=Expression.of(right)
        )

    def __and__(self, right: "Condition") -> "Condition":
        """AND two Conditions."""
        return Condition(
            type=ConditionalType.AND, left=Expression.of(self), right=Expression.of(right)
        )

    @classmethod
    def of(
        cls: type_["Condition"],
        attribute: Union["CustomPropertyDefinition", "PropertyDeclaration", "PropertyDefinition"],
        type: ConditionalType = ConditionalType.EQUALS,
        value: Any = None,
    ) -> "Condition":
        from ..builtin import PropertyDeclaration

        left = Expression.of(attribute)
        if value is not None:
            if isinstance(attribute, PropertyDeclaration):
                value_type = attribute.type.to_type()
            else:
                value_type = attribute.type
            if type in (ConditionalType.IN, ConditionalType.NOT_IN):
                value_type = value_type.clone(cardinality=TypeCardinality.LIST)
            right = Expression.of(Value.wrap(value, value_type))
            return Condition(type=type, left=left, right=right)
        else:
            return Condition(type=type, left=left)


#
# Aggregation
#


@builtin_enum(EnumType.AGGREGATION_TYPE)
class AggregationType(Enum):
    EXISTS = 1
    COUNT = 2
    SUM = 3
    MIN = 4
    MAX = 5
    AVERAGE = 6


@builtin_struct(StructType.AGGREGATION, frozen=True)
class Aggregation(StructFrozen):
    """Aggregation."""

    type: AggregationType = builtin_property(100, is_repr=True)
    expression: Optional["Expression"] = builtin_property(101, is_repr=True)
    # distinct, over, ...

    @classmethod
    def of(
        cls: type_["Aggregation"],
        type: AggregationType,
        operand: Optional["Expression"] = None,
    ) -> "Aggregation":
        return Aggregation(type=type, expression=operand)


#
# Expression
#


@builtin_enum(EnumType.EXPRESSION_TYPE)
class ExpressionType(Enum):
    LITERAL = 1
    ATTRIBUTE = 2
    CONDITION = 3
    FUNCTION = 4
    AGGREGATION = 5
    # SUBQUERY?


@builtin_struct(
    StructType.EXPRESSION,
    frozen=True,
    is_final=True,
)
@final
class Expression(StructFrozen):
    """Wrapper to unify any scalar / boolean / aggregate sub-tree."""

    type: ExpressionType = builtin_property(100, is_repr=True)
    literal: Optional[Value] = builtin_property(101, is_repr=True)
    attribute: Optional[PropertyReference] = builtin_property(102, is_repr=True)
    condition: Optional[Condition] = builtin_property(103, is_repr=True)
    aggregation: Optional[Aggregation] = builtin_property(105, is_repr=True)
    # subquery?

    @classmethod
    def of(cls, thing: "ExpressionIn") -> "Expression":
        from destack.language.core import PropertyDeclaration, PropertyDefinition

        if isinstance(thing, Value):
            return Expression(type=ExpressionType.LITERAL, literal=thing)
        elif isinstance(thing, PropertyReference):
            return Expression(type=ExpressionType.ATTRIBUTE, attribute=thing)
        elif isinstance(thing, (Node, PropertyDeclaration, PropertyDefinition)):
            return Expression(type=ExpressionType.ATTRIBUTE, attribute=PropertyReference.of(thing))
        elif isinstance(thing, Condition):
            return Expression(type=ExpressionType.CONDITION, condition=thing)
        elif isinstance(thing, Aggregation):
            return Expression(type=ExpressionType.AGGREGATION, aggregation=thing)
        elif isinstance(thing, Expression):
            return thing
        else:
            assert_never(thing)


ExpressionIn = Union[
    "Value",
    "PropertyReference",
    "PropertyDeclaration",
    "PropertyDefinition",
    "CustomPropertyDefinition",
    "Condition",
    "Aggregation",
    "Expression",
]


#
# Sort
#


@builtin_enum(EnumType.SORT_TYPE)
class SortType(Enum):
    ASCENDING = 1
    DESCENDING = 2


@builtin_enum(EnumType.SORT_MODE)
class SortMode(Enum):
    MAX = 1
    MIN = 2
    AVERAGE = 3
    SUM = 4
    MEDIAN = 5


SortIn = Union[
    "Sort",
    "Expression",
    "CustomPropertyDefinition",
    "PropertyDeclaration",
    "PropertyDefinition",
]


@builtin_struct(
    StructType.SORT,
    frozen=True,
    is_final=True,
)
@final
class Sort(StructFrozen):
    """ORDER BY specification."""

    type: SortType = builtin_property(100, is_repr=True)
    by: Expression = builtin_property(101, is_repr=True)
    mode: Optional[SortMode] = builtin_property(102, is_repr=True)

    @classmethod
    def of(
        cls, attribute: "SortIn", type: SortType = SortType.ASCENDING, mode: SortMode | None = None
    ) -> "Sort":
        if isinstance(attribute, Sort):
            return attribute
        elif isinstance(attribute, Expression):
            return Sort(type=type, by=attribute, mode=mode)
        else:
            return Sort(type=type, by=Expression.of(attribute), mode=mode)


#
# Select
#


@builtin_struct(
    StructType.SELECT,
    frozen=True,
    is_final=True,
)
@final
class Select(StructFrozen):
    """Select specific Attributes."""

    attributes: list[PropertyReference] = builtin_property(101, is_repr=True)

    @classmethod
    def of(cls, *attributes: "PropertyDeclaration | CustomPropertyDefinition") -> "Select":
        return Select(attributes=[PropertyReference.of(attribute) for attribute in attributes])


#
# Join
#


@builtin_enum(EnumType.JOIN_TYPE)
class JoinType(Enum):
    LEFT = 1
    # RIGHT, INNER, OUTER, CROSS?
    PARENT = 10
    CHILD = 11


@builtin_struct(
    StructType.JOIN,
    frozen=True,
    is_final=True,
)
@final
class Join(StructFrozen):
    """Join a Query with another Query."""

    type: JoinType = builtin_property(100, is_repr=True)
    # query_name?
    recursive: bool = builtin_property(102, default=False, is_repr=True)  # for tree joins
    on: Optional[Condition] = builtin_property(103, is_repr=True)

    @classmethod
    def of(
        cls,
        join: "JoinIn",
        on: Optional[Condition] = None,
        recursive: bool = False,
    ) -> "Join":
        if isinstance(join, Join):
            return join
        else:
            return Join(type=join, on=on, recursive=recursive)


JoinIn = Union[Join, "JoinType"]


#
# Query
#


@builtin_enum(EnumType.QUERY_TYPE)
class QueryType(Enum):
    NODE = 1, "Node", "Flat list of Nodes"
    SCALAR = 5, "Scalar", "Single scalar Value"
    GROUPED_NODE = 10, "Grouped Node", "Grouped list of Nodes"
    GROUPED_SCALAR = 15, "Grouped Scalar", "Grouped list of scalar Values"


@builtin_struct(
    StructType.QUERY,
    frozen=True,
    is_final=True,
)
@final
class Query(StructFrozen):
    """
    A Query into the supergraph about Nodes (node or scalar and potentially grouped).
    Queries may either be about Entities or Events.
    """

    # meta
    id: UUID = builtin_property(2, default_factory=ValueFactory.UUID4)
    type: QueryType = builtin_property(100, is_repr=True, description="The type of Query.")
    name: str = builtin_property(
        105,
        description="Name for this subquery. Should be unique within the parent Query.",
        is_repr=True,
    )
    definition: ObjectDefinitionReference = builtin_property(
        106,
        is_repr=True,
        description="The Node definition this Query is about.",
    )
    subqueries: list["Query"] = builtin_property(
        109,
        is_repr=True,
        description="Subqueries of this Query (if any).",
    )

    # content
    join: Optional[Join] = builtin_property(
        110,
        is_repr=True,
        description="How to join this Query to the parent Query (if any).",
    )
    select: Optional[Select] = builtin_property(
        111,
        is_repr=True,
        description="What to select from the Query.",
    )
    where: Optional[Condition] = builtin_property(
        112, is_repr=True, description="Filter the Query."
    )
    having: Optional[Condition] = builtin_property(
        113, is_repr=True, description="Filter the Query groups (for grouped Queries)."
    )
    group_by: list[Expression] | None = builtin_property(
        114, is_repr=True, description="Discriminator for grouped Queries."
    )
    aggregation: Optional[Aggregation] = builtin_property(
        115, is_repr=True, description="Aggregate the Query."
    )
    sort: list[Sort] | None = builtin_property(
        116,
        is_repr=True,
        description="How to sort the Query results.",
    )

    # pagination
    limit: Optional[UInt32] = builtin_property(
        120,
        is_repr=True,
        description="Limit the number of results.",
    )
    offset: Optional[UInt32] = builtin_property(
        121,
        is_repr=True,
        description="Offset the results.",
    )
    # count?


def to_subqueries(subqueries: dict[str, "Query"]) -> list["Query"]:
    """Turn Queries into subqueries with default names & parent joins."""
    for name, subquery in subqueries.items():
        if subquery.join is None:
            object.__setattr__(subquery, "join", Join.of(JoinType.CHILD))
        object.__setattr__(subquery, "name", name)
        subquery._invalidate_frozen_cache()
    return list(subqueries.values())

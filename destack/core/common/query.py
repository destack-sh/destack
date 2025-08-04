from typing import TYPE_CHECKING, Any, Optional, Union, assert_never, final

from ..builtin import (
    EnumType,
    Node,
    OptionEnum,
    PropertyDeclaration,
    StructFrozen,
    StructType,
    TypeCardinality,
    UInt32,
    ValueFactory,
    declare_enum,
    declare_option,
    declare_property,
    declare_struct,
)
from ..utils.uuid import UUID

if TYPE_CHECKING:
    from destack import (
        CustomPropertyDefinition,
        ObjectDefinitionReference,
        PropertyDefinition,
        PropertyReference,
        Value,
    )

# pyright: reportIncompatibleVariableOverride=false

type_ = type


#
# Conditional
#


@declare_enum(EnumType.CONDITIONAL_TYPE)
class ConditionalType(OptionEnum):
    # logical
    NOT = declare_option(1)
    AND = declare_option(2)
    OR = declare_option(3)
    # comparison
    EQUALS = declare_option(10)
    NOT_EQUALS = declare_option(11)
    GREATER_THAN = declare_option(12)
    GREATER_THAN_OR_EQUALS = declare_option(13)
    LESS_THAN = declare_option(14)
    LESS_THAN_OR_EQUALS = declare_option(15)
    # string
    MATCHES = declare_option(20)
    STARTS_WITH = declare_option(21)
    ENDS_WITH = declare_option(22)
    # collections
    IN = declare_option(30)
    NOT_IN = declare_option(31)
    # existence
    EXISTS = declare_option(40)
    NOT_EXISTS = declare_option(41)


@declare_struct(
    StructType.CONDITION,
    frozen=True,
    is_final=True,
)
@final
class Condition(StructFrozen):
    """Boolean predicate (AND, =, <, etc.)."""

    type: ConditionalType = declare_property(100, is_repr=True)
    left: "Expression" = declare_property(101, is_repr=True)
    right: Optional["Expression"] = declare_property(102, is_repr=True)

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
        from .value import Value

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


@declare_enum(EnumType.AGGREGATION_TYPE)
class AggregationType(OptionEnum):
    EXISTS = declare_option(1)
    COUNT = declare_option(2)
    SUM = declare_option(3)
    MIN = declare_option(4)
    MAX = declare_option(5)
    AVERAGE = declare_option(6)


@declare_struct(StructType.AGGREGATION, frozen=True)
class Aggregation(StructFrozen):
    """Aggregation."""

    type: AggregationType = declare_property(100, is_repr=True)
    expression: Optional["Expression"] = declare_property(101, is_repr=True)
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


@declare_enum(EnumType.EXPRESSION_TYPE)
class ExpressionType(OptionEnum):
    LITERAL = declare_option(1)
    ATTRIBUTE = declare_option(2)
    CONDITION = declare_option(3)
    FUNCTION = declare_option(4)
    AGGREGATION = declare_option(5)
    # SUBQUERY?


@declare_struct(
    StructType.EXPRESSION,
    frozen=True,
    is_final=True,
)
@final
class Expression(StructFrozen):
    """Wrapper to unify any scalar / boolean / aggregate sub-tree."""

    type: ExpressionType = declare_property(100, is_repr=True)
    literal: Optional[Value] = declare_property(101, is_repr=True)
    attribute: Optional[PropertyReference] = declare_property(102, is_repr=True)
    condition: Optional[Condition] = declare_property(103, is_repr=True)
    aggregation: Optional[Aggregation] = declare_property(105, is_repr=True)
    # subquery?

    @classmethod
    def of(cls, thing: "ExpressionIn") -> "Expression":
        from destack.core import PropertyDeclaration, PropertyDefinition

        from .value import Value

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


@declare_enum(EnumType.SORT_TYPE)
class SortType(OptionEnum):
    ASCENDING = declare_option(1)
    DESCENDING = declare_option(2)


@declare_enum(EnumType.SORT_MODE)
class SortMode(OptionEnum):
    MAX = declare_option(1)
    MIN = declare_option(2)
    AVERAGE = declare_option(3)
    SUM = declare_option(4)
    MEDIAN = declare_option(5)


SortIn = Union[
    "Sort",
    "Expression",
    "CustomPropertyDefinition",
    "PropertyDeclaration",
    "PropertyDefinition",
]


@declare_struct(
    StructType.SORT,
    frozen=True,
    is_final=True,
)
@final
class Sort(StructFrozen):
    """ORDER BY specification."""

    type: SortType = declare_property(100, is_repr=True)
    by: Expression = declare_property(101, is_repr=True)
    mode: Optional[SortMode] = declare_property(102, is_repr=True)

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


@declare_struct(
    StructType.SELECT,
    frozen=True,
    is_final=True,
)
@final
class Select(StructFrozen):
    """Select specific Attributes."""

    attributes: list[PropertyReference] = declare_property(101, is_repr=True)

    @classmethod
    def of(cls, *attributes: "PropertyDeclaration | CustomPropertyDefinition") -> "Select":
        return Select(attributes=[PropertyReference.of(attribute) for attribute in attributes])


#
# Join
#


@declare_enum(EnumType.JOIN_TYPE)
class JoinType(OptionEnum):
    LEFT = declare_option(1)
    # RIGHT, INNER, OUTER, CROSS?
    PARENT = declare_option(10)
    CHILD = declare_option(11)


@declare_struct(
    StructType.JOIN,
    frozen=True,
    is_final=True,
)
@final
class Join(StructFrozen):
    """Join a Query with another Query."""

    type: JoinType = declare_property(100, is_repr=True)
    # query_name?
    recursive: bool = declare_property(102, default=False, is_repr=True)  # for tree joins
    on: Optional[Condition] = declare_property(103, is_repr=True)

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


@declare_enum(EnumType.QUERY_TYPE)
class QueryType(OptionEnum):
    NODE = declare_option(1, description="Flat list of Nodes")
    SCALAR = declare_option(5, description="Single scalar Value")
    GROUPED_NODE = declare_option(10, description="Grouped list of Nodes")
    GROUPED_SCALAR = declare_option(15, description="Grouped list of scalar Values")


@declare_struct(
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
    id: UUID = declare_property(2, default_factory=ValueFactory.UUID4)
    type: QueryType = declare_property(100, is_repr=True, description="The type of Query.")
    name: str = declare_property(
        105,
        description="Name for this subquery. Should be unique within the parent Query.",
        is_repr=True,
    )
    definition: ObjectDefinitionReference = declare_property(
        106,
        is_repr=True,
        description="The Node definition this Query is about.",
    )
    subqueries: list["Query"] = declare_property(
        109,
        is_repr=True,
        description="Subqueries of this Query (if any).",
    )

    # content
    join: Optional[Join] = declare_property(
        110,
        is_repr=True,
        description="How to join this Query to the parent Query (if any).",
    )
    select: Optional[Select] = declare_property(
        111,
        is_repr=True,
        description="What to select from the Query.",
    )
    where: Optional[Condition] = declare_property(
        112, is_repr=True, description="Filter the Query."
    )
    having: Optional[Condition] = declare_property(
        113, is_repr=True, description="Filter the Query groups (for grouped Queries)."
    )
    group_by: list[Expression] | None = declare_property(
        114, is_repr=True, description="Discriminator for grouped Queries."
    )
    aggregation: Optional[Aggregation] = declare_property(
        115, is_repr=True, description="Aggregate the Query."
    )
    sort: list[Sort] | None = declare_property(
        116,
        is_repr=True,
        description="How to sort the Query results.",
    )

    # pagination
    limit: Optional[UInt32] = declare_property(
        120,
        is_repr=True,
        description="Limit the number of results.",
    )
    offset: Optional[UInt32] = declare_property(
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

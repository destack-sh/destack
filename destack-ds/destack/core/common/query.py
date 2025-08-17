from typing import TYPE_CHECKING, Optional, Union, final

from ..builtin import (
    UUID,
    EnumType,
    OptionEnum,
    PropertyDeclaration,
    Struct,
    StructType,
    UInt32,
    ValueFactory,
    declare_enum,
    declare_method,
    declare_option,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    from destack import (
        CustomPropertyDefinition,
        ObjectDefinitionReference,
        PropertyDefinition,
        PropertyReference,
        Value,
    )


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
    is_final=True,
)
@final
class Condition(Struct):
    """Boolean predicate (AND, =, <, etc.)."""

    type: ConditionalType = declare_property(100, is_repr=True, tag=None)
    left: "Expression" = declare_property(101, is_repr=True, tag=None)
    right: Optional["Expression"] = declare_property(102, is_repr=True, tag=None)

    @declare_method(100)
    def __or__(self, right: "Condition") -> "Condition":
        """OR two Conditions."""
        raise NotImplementedError

    @declare_method(101)
    def __and__(self, right: "Condition") -> "Condition":
        """AND two Conditions."""
        raise NotImplementedError


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


@declare_struct(StructType.AGGREGATION)
class Aggregation(Struct):
    """Aggregation."""

    type: AggregationType = declare_property(100, is_repr=True, tag=None)
    expression: Optional["Expression"] = declare_property(101, is_repr=True, tag=None)
    # distinct, over, ...


#
# Expression
#

# TODO :Incomplete: tagged Unions?
#  (and replace/remove current fake unions?)


@declare_enum(EnumType.EXPRESSION_TYPE)
class ExpressionType(OptionEnum):
    LITERAL = declare_option(1)
    ATTRIBUTE = declare_option(2)
    CONDITION = declare_option(3)
    AGGREGATION = declare_option(4)
    # SUBQUERY?


@declare_struct(
    StructType.EXPRESSION,
    is_final=True,
)
@final
class Expression(Struct):
    """Wrapper to unify any scalar / boolean / aggregate sub-tree."""

    type: ExpressionType = declare_property(100, is_repr=True, tag=None)
    literal: Optional["Value"] = declare_property(101, is_repr=True, tag=None)
    attribute: Optional["PropertyReference"] = declare_property(102, is_repr=True, tag=None)
    # condition: Optional[Condition] = declare_property(103, is_repr=True, tag=None)
    # aggregation: Optional[Aggregation] = declare_property(104, is_repr=True, tag=None)
    # subquery?


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
    is_final=True,
)
@final
class Sort(Struct):
    """ORDER BY specification."""

    type: SortType = declare_property(100, is_repr=True, tag=None)
    by: Expression = declare_property(101, is_repr=True, tag=None)
    mode: Optional[SortMode] = declare_property(102, is_repr=True, tag=None)


#
# Select
#


@declare_struct(
    StructType.SELECT,
    is_final=True,
)
@final
class Select(Struct):
    """Select specific Attributes."""

    attributes: list["PropertyReference"] = declare_property(101, is_repr=True, tag=None)

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
    is_final=True,
)
@final
class Join(Struct):
    """Join a Query with another Query."""

    type: JoinType = declare_property(100, is_repr=True, tag=None)
    # query_name?
    recursive: bool = declare_property(102, default=False, is_repr=True, tag=None)  # for tree joins
    on: Optional[Condition] = declare_property(103, is_repr=True, tag=None)

    @classmethod
    def of(
        cls,
        join: JoinType,
        on: Optional[Condition] = None,
        recursive: bool = False,
    ) -> "Join":
        if isinstance(join, Join):
            return join
        else:
            return Join(type=join, on=on, recursive=recursive)


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
    is_final=True,
)
@final
class Query(Struct):
    """
    A Query into the supergraph about Nodes (node or scalar and potentially grouped).
    Queries may either be about Entities or Events.
    """

    # meta
    id: UUID = declare_property(2, default_factory=ValueFactory.UUID7, tag=None)
    type: QueryType = declare_property(
        100, is_repr=True, description="The type of Query.", tag=None
    )
    name: str = declare_property(
        105,
        description="Name for this subquery. Should be unique within the parent Query.",
        is_repr=True,
        tag=None,
    )
    definition: "ObjectDefinitionReference" = declare_property(
        106,
        is_repr=True,
        description="The Node definition this Query is about.",
        tag=None,
    )
    subqueries: list["Query"] = declare_property(
        109,
        is_repr=True,
        description="Subqueries of this Query (if any).",
        tag=None,
    )

    # content
    join: Optional[Join] = declare_property(
        110,
        is_repr=True,
        description="How to join this Query to the parent Query (if any).",
        tag=None,
    )
    select: Optional[Select] = declare_property(
        111,
        is_repr=True,
        description="What to select from the Query.",
        tag=None,
    )
    where: Optional[Condition] = declare_property(
        112, is_repr=True, description="Filter the Query.", tag=None
    )
    having: Optional[Condition] = declare_property(
        113, is_repr=True, description="Filter the Query groups (for grouped Queries).", tag=None
    )
    group_by: list[Expression] | None = declare_property(
        114, is_repr=True, description="Discriminator for grouped Queries.", tag=None
    )
    aggregation: Optional[Aggregation] = declare_property(
        115, is_repr=True, description="Aggregate the Query.", tag=None
    )
    sort: list[Sort] | None = declare_property(
        116,
        is_repr=True,
        description="How to sort the Query results.",
        tag=None,
    )

    # pagination
    limit: Optional[UInt32] = declare_property(
        120,
        is_repr=True,
        description="Limit the number of results.",
        tag=None,
    )
    offset: Optional[UInt32] = declare_property(
        121,
        is_repr=True,
        description="Offset the results.",
        tag=None,
    )
    # count?

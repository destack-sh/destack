from typing import TYPE_CHECKING, Any, Optional

from ..builtin import (
    CascadeAction,
    EdgeType,
    Entity,
    NodeType,
    PropertyType,
    builtin_node,
    builtin_property,
)
from .query import Condition, ConditionalType, Sort, SortType

if TYPE_CHECKING:
    from destack.language import (
        Icon,
        Type,
    )


# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_node(NodeType.CUSTOM_PROPERTY)
class CustomProperty(Entity):
    """
    A CustomProperty is a custom attribute of an IsCustomizable or IsExtensible.
    """

    type: PropertyType = builtin_property(
        100,
        default=PropertyType.MEMBER,
        description="Where in the parent Entity this Property resides.",
    )
    icon: "Icon | None" = builtin_property(102)
    value_type: "Type" = builtin_property(
        110,
        description="The actual Type of this custom Property.",
    )

    # relationship
    edge_type: Optional[EdgeType] = builtin_property(140)
    cascade: Optional[CascadeAction] = builtin_property(141)

    # property flags
    is_unique: bool | None = builtin_property(
        201,
        description="Whether this property must have a unique value.",
    )
    is_readonly: bool | None = builtin_property(
        202,
        description="Whether this property is read-only.",
    )
    is_main: bool | None = builtin_property(
        203,
        description="Whether this property is the main property of the entity.",
    )

    def to_type(self) -> "Type":
        return self.value_type

    def eq(self, value: Any) -> Condition:
        if value is None:
            return self.not_exists()
        return Condition.of(self, ConditionalType.EQUALS, value=value)

    def neq(self, value: Any) -> Condition:
        if value is None:
            return self.exists()
        return Condition.of(self, ConditionalType.NOT_EQUALS, value=value)

    def gt(self, value: Any) -> Condition:
        return Condition.of(self, ConditionalType.GREATER_THAN, value=value)

    def gte(self, value: Any) -> Condition:
        return Condition.of(self, ConditionalType.GREATER_THAN_OR_EQUALS, value=value)

    def lt(self, value: Any) -> Condition:
        return Condition.of(self, ConditionalType.LESS_THAN, value=value)

    def lte(self, value: Any) -> Condition:
        return Condition.of(self, ConditionalType.LESS_THAN_OR_EQUALS, value=value)

    def starts_with(self, value: str) -> Condition:
        return Condition.of(self, ConditionalType.STARTS_WITH, value=value)

    def ends_with(self, value: str) -> Condition:
        return Condition.of(self, ConditionalType.ENDS_WITH, value=value)

    def in_(self, *values: Any) -> Condition:
        return Condition.of(self, ConditionalType.IN, value=values)

    def not_in(self, *values: Any) -> Condition:
        return Condition.of(self, ConditionalType.NOT_IN, value=values)

    def exists(self) -> Condition:
        return Condition.of(self, ConditionalType.EXISTS)

    def is_not_none(self) -> Condition:
        return Condition.of(self, ConditionalType.EXISTS)

    def not_exists(self) -> "Condition":
        return Condition.of(self, ConditionalType.NOT_EXISTS)

    def is_none(self) -> "Condition":
        return Condition.of(self, ConditionalType.NOT_EXISTS)

    def asc(self) -> "Sort":
        return Sort.of(self, SortType.ASCENDING)

    def desc(self) -> "Sort":
        return Sort.of(self, SortType.DESCENDING)

from typing import TYPE_CHECKING, Any, Optional, Union, final

from ..builtin import (
    CascadeAction,
    EdgeType,
    Entity,
    Event,
    Message,
    NodeType,
    Struct,
    StructType,
    Type,
    builtin_entity,
    builtin_event,
    builtin_message,
    builtin_property,
    builtin_property_parent,
    builtin_struct,
)
from .query import Condition, ConditionalType, Sort, SortType

if TYPE_CHECKING:
    from destack.language import (
        Condition,
        Icon,
        NodeDefinitionReference,
        NodeReference,
        Value,
    )

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(NodeType.CUSTOM_EVENT_DEFINITION)
class CustomEventDefinition(
    Entity,
):
    """A CustomEvent defines a custom Event with custom Properties."""

    icon: "Icon | None" = builtin_property(102)

    base_type: Optional["NodeDefinitionReference"] = builtin_property(110)
    self_traits: list["NodeDefinitionReference"] = builtin_property(111)
    is_abstract: bool = builtin_property(112, default=False)


@builtin_event(NodeType.CUSTOM_EVENT, is_abstract=True)
class CustomEvent(Event):
    """
    A CustomEvent is an instance of a CustomEventDefinition.
    """

    definition: "CustomEventDefinition" = builtin_property(
        11,
        is_internal=True,
        is_readonly=True,
        description="The CustomEvent this Signal is an instance of.",
    )
    if TYPE_CHECKING:
        definition_ptr: Optional[NodeReference] = None


@builtin_entity(NodeType.CUSTOM_STRUCT_DEFINITION)
class CustomStructDefinition(Entity):
    """A CustomStruct describes a custom Struct with custom Properties."""

    pass


@builtin_entity(NodeType.CUSTOM_MESSAGE_DEFINITION)
class CustomMessageDefinition(CustomStructDefinition):
    """A CustomMessage describes a custom Message with custom Properties."""

    pass


@builtin_struct(StructType.CUSTOM_STRUCT, is_final=True)
@final
class CustomStruct(Struct):
    """A CustomStruct is a generic instance of a custom Struct with custom Values."""

    definition: "CustomStructDefinition" = builtin_property(11, is_repr=True)
    custom_values: dict[str, "Value"] | None = builtin_property(
        45,
        description="The custom Values of this Struct, keyed by custom Property name.",
    )

    def __getitem__(self, key: str) -> "Value":
        val = None if self.custom_values is None else self.custom_values.get(key)
        if val is None:
            raise LookupError(f"{self!r} has no value for {key}")
        return val

    def __getattr__(self, name: str) -> "Value | None":
        val = None if self.custom_values is None else self.custom_values.get(name)
        return val


@builtin_message(StructType.CUSTOM_MESSAGE, is_final=True)
@final
class CustomMessage(Message):
    """A CustomMessage is an instance of a custom Message with custom Values."""

    definition: "CustomMessageDefinition" = builtin_property(11, is_repr=True)
    custom_values: dict[str, "Value"] | None = builtin_property(
        45,
        description="The custom Values of this Message, keyed by custom Property name.",
    )

    def __getitem__(self, key: str) -> "Value":
        val = None if self.custom_values is None else self.custom_values.get(key)
        if val is None:
            raise LookupError(f"{self!r} has no value for {key}")
        return val

    def __getattr__(self, name: str) -> "Value | None":
        val = None if self.custom_values is None else self.custom_values.get(name)
        return val


@builtin_entity(NodeType.CUSTOM_ENUM_DEFINITION)
class CustomEnumDefinition(Entity):
    """A CustomEnum describes a custom Enum with custom Options."""

    icon: "Icon | None" = builtin_property(102)


@builtin_entity(NodeType.CUSTOM_OPTION_DEFINITION)
class CustomOptionDefinition(Entity):
    parent: Union["CustomEnumDefinition", None] = builtin_property_parent()

    icon: "Icon | None" = builtin_property(102)

    value: "Value" = builtin_property(110)


@builtin_entity(NodeType.CUSTOM_PROPERTY_DEFINITION)
class CustomPropertyDefinition(Entity):
    """
    A CustomProperty is a custom attribute of an Entity.
    """

    type: "Type" = builtin_property(
        100,
        description="The actual Type of this custom Property.",
    )
    icon: "Icon | None" = builtin_property(102)

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

    def eq(self, value: Any) -> Condition:
        if value is None:
            return self.not_exists()
        return Condition.of(self, ConditionalType.EQUALS, value=value)

    def neq(self, value: Any) -> Condition:
        if value is None:
            return self.is_not_none()
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

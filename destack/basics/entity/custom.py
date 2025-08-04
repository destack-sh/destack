from typing import TYPE_CHECKING, Any, Optional, Union, final

from destack.core import (
    CascadeAction,
    Condition,
    ConditionalType,
    EdgeType,
    Entity,
    Event,
    Message,
    NodeReference,
    NodeType,
    Sort,
    SortType,
    Struct,
    StructType,
    Type,
    declare_entity,
    declare_event,
    declare_message,
    declare_method,
    declare_property,
    declare_property_parent,
    declare_struct,
)

if TYPE_CHECKING:
    from destack import Condition, Icon, Value


@declare_entity(NodeType.CUSTOM_EVENT_DEFINITION)
class CustomEventDefinition(
    Entity,
):
    """A CustomEvent defines a custom Event with custom Properties."""

    icon: "Icon | None" = declare_property(102)

    is_abstract: bool = declare_property(112, default=False)


@declare_event(NodeType.CUSTOM_EVENT, is_abstract=True)
class CustomEvent(Event):
    """
    A CustomEvent is an instance of a CustomEventDefinition.
    """

    definition: "CustomEventDefinition" = declare_property(
        11,
        is_internal=True,
        is_readonly=True,
        description="The CustomEvent this Signal is an instance of.",
    )

    @declare_method(2)
    def to_ref(self) -> "NodeReference":
        """Gets a reference to this Node."""
        ...


@declare_entity(NodeType.CUSTOM_STRUCT_DEFINITION)
class CustomStructDefinition(Entity):
    """A CustomStruct describes a custom Struct with custom Properties."""

    pass


@declare_entity(NodeType.CUSTOM_MESSAGE_DEFINITION)
class CustomMessageDefinition(CustomStructDefinition):
    """A CustomMessage describes a custom Message with custom Properties."""

    pass


@declare_struct(StructType.CUSTOM_STRUCT, is_final=True)
@final
class CustomStruct(Struct):
    """A CustomStruct is a generic instance of a custom Struct with custom Values."""

    definition: "CustomStructDefinition" = declare_property(11, is_repr=True)
    custom_values: dict[str, "Value"] | None = declare_property(
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


@declare_message(StructType.CUSTOM_MESSAGE, is_final=True)
@final
class CustomMessage(Message):
    """A CustomMessage is an instance of a custom Message with custom Values."""

    definition: "CustomMessageDefinition" = declare_property(11, is_repr=True)
    custom_values: dict[str, "Value"] | None = declare_property(
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


@declare_entity(NodeType.CUSTOM_ENUM_DEFINITION)
class CustomEnumDefinition(Entity):
    """A CustomEnum describes a custom Enum with custom Options."""

    icon: "Icon | None" = declare_property(102)


@declare_entity(NodeType.CUSTOM_OPTION_DEFINITION)
class CustomOptionDefinition(Entity):
    parent: Union["CustomEnumDefinition", None] = declare_property_parent()

    icon: "Icon | None" = declare_property(102)

    value: "Value" = declare_property(110)


@declare_entity(NodeType.CUSTOM_PROPERTY_DEFINITION)
class CustomPropertyDefinition(Entity):
    """
    A CustomProperty is a custom attribute of an Entity.
    """

    type: "Type" = declare_property(
        100,
        description="The actual Type of this custom Property.",
    )
    icon: "Icon | None" = declare_property(102)

    # relationship
    edge_type: Optional[EdgeType] = declare_property(140)
    cascade: Optional[CascadeAction] = declare_property(141)

    # property flags
    is_unique: bool | None = declare_property(
        201,
        description="Whether this property must have a unique value.",
    )
    is_readonly: bool | None = declare_property(
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

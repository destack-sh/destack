from typing import TYPE_CHECKING, Any, Optional, final

from destack.core import (
    Condition,
    Entity,
    Error,
    Event,
    Message,
    NodeReference,
    NodeType,
    ReferenceCascade,
    ReferenceType,
    Sort,
    Struct,
    StructType,
    Type,
    declare_entity,
    declare_error,
    declare_event,
    declare_message,
    declare_method,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    from destack import Condition, Value


@declare_entity(NodeType.CUSTOM_EVENT_DEFINITION)
class CustomEventDefinition(
    Entity,
):
    """A CustomEvent defines a custom Event with custom Properties."""

    is_abstract: bool = declare_property(112, default=False)


@declare_event(NodeType.CUSTOM_EVENT, is_abstract=True)
class CustomEvent(Event):
    """
    A CustomEvent is an instance of a CustomEventDefinition.
    """

    definition: "CustomEventDefinition" = declare_property(
        10,
        is_internal=True,
        is_readonly=True,
        description="The CustomEvent this Signal is an instance of.",
    )

    @declare_method(2)
    def to_ref(self) -> "NodeReference":
        """Gets a reference to this Node."""
        raise NotImplementedError


@declare_entity(NodeType.CUSTOM_STRUCT_DEFINITION)
class CustomStructDefinition(Entity):
    """A CustomStruct describes a custom Struct with custom Properties."""

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


@declare_entity(NodeType.CUSTOM_MESSAGE_DEFINITION)
class CustomMessageDefinition(CustomStructDefinition):
    """A CustomMessage describes a custom Message with custom Properties."""

    pass


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


@declare_entity(NodeType.CUSTOM_ERROR_DEFINITION)
class CustomErrorDefinition(CustomStructDefinition):
    """A CustomError describes a custom Error with custom Properties."""

    pass


@declare_error(StructType.CUSTOM_ERROR, is_final=True)
@final
class CustomError(Error):
    """A CustomError is an instance of a custom Error with custom Values."""

    definition: "CustomErrorDefinition" = declare_property(11, is_repr=True)
    custom_values: dict[str, "Value"] | None = declare_property(
        45,
        description="The custom Values of this Error, keyed by custom Property name.",
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


@declare_entity(NodeType.CUSTOM_OPTION_DEFINITION)
class CustomOptionDefinition(Entity):
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

    # relationship
    reference_type: Optional[ReferenceType] = declare_property(140)
    cascade: Optional[ReferenceCascade] = declare_property(141)

    # property flags
    is_unique: bool | None = declare_property(
        201,
        description="Whether this property must have a unique value.",
    )
    is_readonly: bool | None = declare_property(
        202,
        description="Whether this property is read-only.",
    )

    @declare_method(101)
    def eq(self, value: Any) -> Condition:
        """Compare this property to a value."""
        raise NotImplementedError

    @declare_method(102)
    def neq(self, value: Any) -> Condition:
        """Compare this property to a value."""
        raise NotImplementedError

    @declare_method(103)
    def gt(self, value: Any) -> Condition:
        """Compare this property to a value."""
        raise NotImplementedError

    @declare_method(104)
    def gte(self, value: Any) -> Condition:
        """Compare this property to a value."""
        raise NotImplementedError

    @declare_method(105)
    def lt(self, value: Any) -> Condition:
        """Compare this property to a value."""
        raise NotImplementedError

    @declare_method(106)
    def lte(self, value: Any) -> Condition:
        """Compare this property to a value."""
        raise NotImplementedError

    @declare_method(107)
    def starts_with(self, value: str) -> Condition:
        """Compare this property to a value."""
        raise NotImplementedError

    @declare_method(108)
    def ends_with(self, value: str) -> Condition:
        """Compare this property to a value."""
        raise NotImplementedError

    @declare_method(109)
    def in_(self, *values: Any) -> Condition:
        """Compare this property to a value."""
        raise NotImplementedError

    @declare_method(110)
    def not_in(self, *values: Any) -> Condition:
        """Compare this property to a value."""
        raise NotImplementedError

    @declare_method(111)
    def is_not_none(self) -> Condition:
        """Compare this property to a value."""
        raise NotImplementedError

    @declare_method(112)
    def is_none(self) -> Condition:
        """Compare this property to a value."""
        raise NotImplementedError

    @declare_method(113)
    def asc(self) -> Sort:
        """Compare this property to a value."""
        raise NotImplementedError

    @declare_method(114)
    def desc(self) -> Sort:
        """Compare this property to a value."""
        raise NotImplementedError

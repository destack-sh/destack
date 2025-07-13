from typing import TYPE_CHECKING, Optional

from destack.utils.uuid import UUID

from ..builtin import (
    Entity,
    IsCustomizable,
    IsDeletable,
    IsSourceable,
    IsTaggable,
    NodeType,
    StructMutable,
    StructType,
    builtin_node,
    builtin_property,
    builtin_struct,
)

if TYPE_CHECKING:
    from destack.language import Icon, StructDefinitionReference, Value

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CUSTOM_STRUCT_DEFINITION)
class CustomStructDefinition(
    IsTaggable,
    IsDeletable,
    IsSourceable,
    IsCustomizable,
    Entity,
):
    """A CustomStructDefinition describes a custom Struct with custom Properties."""

    prototype: Optional["CustomStruct"] = builtin_property(
        40,
        description="A custom Struct's prototype is the default template new CustomStruct instances are based on.",
    )
    base_type: Optional["StructDefinitionReference"] = builtin_property(41)
    is_frozen: bool = builtin_property(42, default=False)

    name: str = builtin_property(101, is_repr=True)
    icon: "Icon | None" = builtin_property(102)


@builtin_struct(StructType.CUSTOM_STRUCT)
class CustomStruct(StructMutable):
    """A CustomStruct is an instance of a CustomStructDefinition."""

    definition: "CustomStructDefinition" = builtin_property(6)
    custom_values: dict[UUID, "Value"] = builtin_property(26)

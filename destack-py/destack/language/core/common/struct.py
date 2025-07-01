from typing import TYPE_CHECKING, Optional

from destack.utils.uuid import UUID

from ..builtin import (
    Entity,
    HasIcon,
    HasName,
    IsCustomizable,
    IsDeletable,
    IsSourceable,
    IsSpatial,
    IsTaggable,
    NodeType,
    StructMutable,
    StructType,
    builtin_node,
    builtin_struct,
    property_,
)

if TYPE_CHECKING:
    from destack.language import StructDefinitionReference, Value

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CUSTOM_STRUCT_DEFINITION)
class CustomStructDefinition(
    IsSpatial,
    HasName,
    HasIcon,
    IsTaggable,
    IsDeletable,
    IsSourceable,
    IsCustomizable,
    Entity,
):
    """A CustomStructDefinition describes a custom Struct with custom Properties."""

    prototype: Optional["CustomStruct"] = property_(
        40,
        description="A custom Struct's prototype is the default template new CustomStruct instances are based on.",
    )
    base_type: Optional["StructDefinitionReference"] = property_(41)


@builtin_struct(StructType.CUSTOM_STRUCT)
class CustomStruct(StructMutable):
    """A CustomStruct is an instance of a CustomStructDefinition."""

    definition: "CustomStructDefinition" = property_(6)
    value: dict[UUID, "Value"] = property_(21)

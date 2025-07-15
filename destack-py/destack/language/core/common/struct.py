from typing import TYPE_CHECKING, Optional

from destack.utils.uuid import UUID

from ..builtin import (
    Entity,
    IsCustomizable,
    IsDeletable,
    IsSourceable,
    IsTaggable,
    NodeType,
    StructFrozen,
    StructMutable,
    StructType,
    builtin_node,
    builtin_property,
    builtin_struct,
)

if TYPE_CHECKING:
    from destack.language import Icon, StructDefinitionReference, Value

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.CUSTOM_STRUCT)
class CustomStruct(
    IsTaggable,
    IsDeletable,
    IsSourceable,
    IsCustomizable,
    Entity,
):
    """A CustomStruct describes a custom Struct with custom Properties."""

    base_type: Optional["StructDefinitionReference"] = builtin_property(41)
    is_frozen: bool = builtin_property(42, default=False)

    icon: "Icon | None" = builtin_property(102)


@builtin_struct(StructType.DATUM, frozen=True)
class Datum(StructFrozen):
    """A Datum is an immutable instance of a CustomStruct."""

    definition: "CustomStruct" = builtin_property(6)
    custom_values: dict[UUID, "Value"] = builtin_property(26)


@builtin_struct(StructType.DATUM_MUTABLE)
class DatumMutable(StructMutable):
    """A DatumMutable is a mutable instance of a CustomStruct."""

    definition: "CustomStruct" = builtin_property(6)
    custom_values: dict[UUID, "Value"] = builtin_property(26)

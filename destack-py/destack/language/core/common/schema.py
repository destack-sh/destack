from typing import TYPE_CHECKING

from destack.pb2 import SchemaData

from ..builtin import (
    Entity,
    Enum,
    EnumType,
    HasIcon,
    HasName,
    IsDeletable,
    IsExtensible,
    IsSourceable,
    IsTaggable,
    IsTemplatable,
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.SCHEMA_TYPE)
class SchemaType(Enum):
    """Built-in schema types."""

    STRUCT = 1
    ENUM = 2
    # NEWTYPE?


@builtin_node(NodeType.SCHEMA)
class Schema(
    Spatial,
    Entity,
    HasName,
    HasIcon,
    IsTaggable,
    IsTemplatable,
    IsDeletable,
    IsSourceable,
    IsExtensible,
    Node[SchemaData],
):
    """A Schema describes a custom Type with Fields."""

    pass

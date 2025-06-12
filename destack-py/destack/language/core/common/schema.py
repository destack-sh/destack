from typing import TYPE_CHECKING

from destack.pb2 import SchemaData

from ..builtin import (
    BuiltinEnum,
    Entity,
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
    enum_,
    node_,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.SCHEMA_TYPE)
class SchemaType(BuiltinEnum):
    """Built-in schema types."""

    STRUCT = 1
    ENUM = 2
    # NEWTYPE?


@node_(NodeType.SCHEMA)
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

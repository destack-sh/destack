from typing import TYPE_CHECKING, Union

from ..builtin import (
    Entity,
    Enum,
    EnumType,
    IsExtensible,
    IsTaggable,
    NodeType,
    StructType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
    builtin_struct,
)
from .definition import BuiltinDefinition

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_enum(EnumType.MIGRATION_TYPE)
class MigrationType(Enum):
    """Type of a builtin Migration."""

    CREATE = 1
    # UPDATE, DELETE, ...


@builtin_struct(StructType.MIGRATION_DEFINITION, frozen=True)
class MigrationDefinition(BuiltinDefinition):
    """Definition of a builtin Migration."""

    type: "MigrationType" = builtin_property(100, is_repr=True)


@builtin_node(NodeType.MIGRATION)
class Migration(IsTaggable, Entity):
    """Migration of an Entity."""

    parent: Union["IsExtensible", None] = builtin_property_parent()
    type: MigrationType = builtin_property(100, is_repr=True)


@builtin_struct(StructType.MIGRATION_OPERATION_DEFINITION, frozen=True)
class MigrationOperationDefinition(BuiltinDefinition):
    """Definition of a builtin MigrationOperation."""


@builtin_node(NodeType.MIGRATION_OPERATION)
class MigrationOperation(IsTaggable, Entity):
    """MigrationOperation of an Entity."""

    parent: Union["IsExtensible", None] = builtin_property_parent()

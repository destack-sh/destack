from typing import TYPE_CHECKING

from ..builtin import (
    Entity,
    Enum,
    EnumType,
    NodeType,
    StructType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_struct,
)
from ..builtin.definition import BuiltinDefinition

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
class Migration(Entity):
    """Migration of an Entity."""

    type: MigrationType = builtin_property(100, is_repr=True)


@builtin_struct(StructType.MIGRATION_OPERATION_DEFINITION, frozen=True)
class MigrationOperationDefinition(BuiltinDefinition):
    """Definition of a builtin MigrationOperation."""


@builtin_node(NodeType.MIGRATION_OPERATION)
class MigrationOperation(Entity):
    """MigrationOperation of an Entity."""

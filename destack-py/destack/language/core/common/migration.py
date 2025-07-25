from typing import TYPE_CHECKING

from ..builtin import (
    Entity,
    Enum,
    EnumType,
    NodeType,
    StructFrozen,
    StructType,
    UInt32,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_struct,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@builtin_enum(EnumType.MIGRATION_TYPE)
class MigrationType(Enum):
    """Type of a builtin Migration."""

    CREATE = 1
    # UPDATE, DELETE, ...


@builtin_struct(StructType.MIGRATION_DEFINITION, frozen=True)
class MigrationDefinition(StructFrozen):
    """Definition of a builtin Migration."""

    type: "MigrationType" = builtin_property(100, is_repr=True)
    name: str = builtin_property(101, is_repr=True)
    description: str | None = builtin_property(103, is_repr=True)


@builtin_node(NodeType.MIGRATION)
class Migration(Entity):
    """Migration of an Entity."""

    id: UInt32 = builtin_property(2, is_repr=True)
    type: MigrationType = builtin_property(100, is_repr=True)
    description: str | None = builtin_property(103, is_repr=True)


@builtin_struct(StructType.MIGRATION_OPERATION_DEFINITION, frozen=True)
class MigrationOperationDefinition(StructFrozen):
    """Definition of a builtin MigrationOperation."""

    id: UInt32 = builtin_property(2, is_repr=True)
    name: str = builtin_property(101, is_repr=True)
    description: str | None = builtin_property(103, is_repr=True)
    type: "MigrationType" = builtin_property(100, is_repr=True)


@builtin_node(NodeType.MIGRATION_OPERATION)
class MigrationOperation(Entity):
    """MigrationOperation of an Entity."""

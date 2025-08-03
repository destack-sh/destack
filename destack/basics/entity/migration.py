from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    EnumDeclaration,
    EnumType,
    NodeType,
    StructFrozen,
    StructType,
    UInt32,
    declare_entity,
    declare_enum,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@declare_enum(EnumType.MIGRATION_TYPE)
class MigrationType(EnumDeclaration):
    """Type of a builtin Migration."""

    CREATE = 1
    # UPDATE, DELETE, ...


@declare_struct(StructType.MIGRATION_DEFINITION, frozen=True)
class MigrationDefinition(StructFrozen):
    """Definition of a builtin Migration."""

    type: "MigrationType" = declare_property(100, is_repr=True)
    name: str = declare_property(101, is_repr=True)
    description: str | None = declare_property(103, is_repr=True)


@declare_entity(NodeType.MIGRATION)
class Migration(Entity):
    """Migration of an Entity."""

    type: MigrationType = declare_property(100, is_repr=True)
    description: str | None = declare_property(103, is_repr=True)


@declare_struct(StructType.MIGRATION_OPERATION_DEFINITION, frozen=True)
class MigrationOperationDefinition(StructFrozen):
    """Definition of a builtin MigrationOperation."""

    id: UInt32 = declare_property(2, is_repr=True)
    name: str = declare_property(101, is_repr=True)
    description: str | None = declare_property(103, is_repr=True)
    type: "MigrationType" = declare_property(100, is_repr=True)


@declare_entity(NodeType.MIGRATION_OPERATION)
class MigrationOperation(Entity):
    """MigrationOperation of an Entity."""

from typing import TYPE_CHECKING

from destack.core import (
    Entity,
    EnumType,
    NodeType,
    OptionEnum,
    Struct,
    StructType,
    UInt32,
    declare_entity,
    declare_enum,
    declare_option,
    declare_property,
    declare_struct,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false, reportIncompatibleMethodOverride=false


@declare_enum(EnumType.MIGRATION_TYPE)
class MigrationType(OptionEnum):
    """Type of a builtin Migration."""

    CREATE = declare_option(1, "Create", description="A Create Migration")
    # UPDATE, DELETE, ...


@declare_struct(StructType.MIGRATION_DEFINITION)
class MigrationDefinition(Struct):
    """Definition of a builtin Migration."""

    type: "MigrationType" = declare_property(100, is_repr=True)
    name: str = declare_property(101, is_repr=True)
    description: str | None = declare_property(103, is_repr=True)


@declare_entity(NodeType.MIGRATION)
class Migration(Entity):
    """Migration of an Entity."""

    type: MigrationType = declare_property(100, is_repr=True)
    description: str | None = declare_property(103, is_repr=True)


@declare_struct(StructType.MIGRATION_OPERATION_DEFINITION)
class MigrationOperationDefinition(Struct):
    """Definition of a builtin MigrationOperation."""

    id: UInt32 = declare_property(2, is_repr=True)
    name: str = declare_property(101, is_repr=True)
    description: str | None = declare_property(103, is_repr=True)
    type: "MigrationType" = declare_property(100, is_repr=True)


@declare_entity(NodeType.MIGRATION_OPERATION)
class MigrationOperation(Entity):
    """MigrationOperation of an Entity."""

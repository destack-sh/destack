from destack.language.core import (
    BuiltinObjectMutable,
    Region,
    RoleType,
    StructMutable,
    StructType,
    builtin_struct,
    object_,
    property_,
)

# pyright: reportIncompatibleVariableOverride=false


@object_()
class GalaxyBase(BuiltinObjectMutable):
    """A Galaxy is a logical grouping of hosts."""

    # infra
    region: Region = property_(50, can_write=RoleType.SYSTEM, is_repr=True)
    name: str = property_(51, can_write=RoleType.SYSTEM, is_repr=True)
    host: str = property_(52, can_write=RoleType.SYSTEM, is_repr=True)


@builtin_struct(StructType.GALAXY_INFO)
class GalaxyInfo(GalaxyBase, StructMutable):
    pass


# TODO :Infra: map Galaxys into actual Galaxy Nodes?

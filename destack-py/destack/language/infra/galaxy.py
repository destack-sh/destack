from destack.language.core import (
    Region,
    RoleType,
    StructFrozen,
    StructType,
    builtin_struct,
    property_,
)

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(StructType.GALAXY_INFO, frozen=True)
class GalaxyInfo(StructFrozen):
    region: Region = property_(50, can_write=RoleType.SYSTEM, is_repr=True)
    name: str = property_(51, can_write=RoleType.SYSTEM, is_repr=True)
    host: str = property_(52, can_write=RoleType.SYSTEM, is_repr=True)


# TODO :Infra: map Galaxys into actual Galaxy Nodes?

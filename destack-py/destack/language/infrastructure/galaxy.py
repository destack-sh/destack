from destack.language.core import (
    Region,
    RoleType,
    StructFrozen,
    StructType,
    builtin_property,
    builtin_struct,
)

# pyright: reportIncompatibleVariableOverride=false


@builtin_struct(StructType.GALAXY_INFO, frozen=True)
class GalaxyInfo(StructFrozen):
    region: Region = builtin_property(110, can_write=RoleType.SYSTEM, is_repr=True)
    name: str = builtin_property(111, can_write=RoleType.SYSTEM, is_repr=True)
    host: str = builtin_property(112, can_write=RoleType.SYSTEM, is_repr=True)


# TODO :Infra: map Galaxys into actual Galaxy Nodes?

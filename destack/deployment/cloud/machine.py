from typing import TYPE_CHECKING, Optional

from destack.core import (
    VERSION,
    Entity,
    EnumDeclaration,
    EnumType,
    Float32,
    NodeType,
    TraitType,
    UInt32,
    declare_entity,
    declare_enum,
    declare_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.MACHINE_TYPE)
class MachineType(EnumDeclaration):
    RUNTIME = 10, "Runtime", "The main Destack runtime"
    UBUNTU = 1000, "Ubuntu", "A Linux machine running Ubuntu"
    MAC = 1100, "Mac", "A Mac machine"
    WINDOWS = 1200, "Windows", "A Windows machine"
    CUSTOM = 9000, "Custom", "A custom Docker image"


@declare_entity(NodeType.MACHINE, traits=(TraitType.RESOURCE,))
class Machine(Entity):
    """
    A Machine provides physical compute.
    NOTE :RichComputing: Machines also need Deployments/Endpoints/...?
    """

    type: MachineType = declare_property(100, default=MachineType.RUNTIME)

    version: str = declare_property(120, default=VERSION)
    external_name: Optional[str] = declare_property(121)
    external_id: Optional[str] = declare_property(122)
    image_id: Optional[str] = declare_property(123)

    cpu: Float32 = declare_property(130, description="vCPU count", default=1.0)
    ram: Float32 = declare_property(131, description="GB", default=1.0)
    width: UInt32 = declare_property(132, default=1280)
    height: UInt32 = declare_property(133, default=960)
    is_headless: bool = declare_property(134, default=False)

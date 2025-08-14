from typing import TYPE_CHECKING, Optional

from destack.core import (
    VERSION,
    Entity,
    EnumType,
    Float32,
    NodeType,
    OptionEnum,
    TraitType,
    UInt32,
    declare_entity,
    declare_enum,
    declare_option,
    declare_property,
)

if TYPE_CHECKING:
    pass


@declare_enum(EnumType.MACHINE_TYPE)
class MachineType(OptionEnum):
    RUNTIME = declare_option(10, "Runtime", description="The main Destack runtime")
    UBUNTU = declare_option(1000, "Ubuntu", description="A Linux machine running Ubuntu")
    MAC = declare_option(1100, "Mac", description="A Mac machine")
    WINDOWS = declare_option(1200, "Windows", description="A Windows machine")
    CUSTOM = declare_option(9000, "Custom", description="A custom Docker image")


@declare_entity(NodeType.MACHINE, traits=(TraitType.RESOURCE,))
class Machine(Entity):
    """
    A Machine provides physical compute.
    NOTE :RichComputing: Machines also need Deployments/Endpoints/...?
    """

    type: MachineType = declare_property(
        100,
        default=MachineType.RUNTIME,
        tag=None,
    )

    version: str = declare_property(
        120,
        default=VERSION,
        tag=None,
    )
    external_name: Optional[str] = declare_property(
        121,
        tag=None,
    )
    external_id: Optional[str] = declare_property(
        122,
        tag=None,
    )
    image_id: Optional[str] = declare_property(
        123,
        tag=None,
    )

    cpu: Float32 = declare_property(
        130,
        description="vCPU count",
        default=1.0,
        tag=None,
    )
    ram: Float32 = declare_property(
        131,
        description="GB",
        default=1.0,
        tag=None,
    )
    width: UInt32 = declare_property(
        132,
        default=1280,
        tag=None,
    )
    height: UInt32 = declare_property(
        133,
        default=960,
        tag=None,
    )
    is_headless: bool = declare_property(
        134,
        default=False,
        tag=None,
    )

from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    VERSION,
    Enum,
    EnumType,
    IsSpatial,
    NodeReference,
    NodeType,
    Resource,
    RoleType,
    builtin_enum,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Client

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.MACHINE_TYPE)
class MachineType(Enum):
    RUNTIME = 10, "Runtime", "The main Destack runtime", "fas fa-machine-classic"
    UBUNTU = 1000, "Ubuntu", "A Linux machine running Ubuntu", "fab fa-ubuntu"
    MAC = 1100, "Mac", "A Mac machine", "fab fa-apple"
    WINDOWS = 1200, "Windows", "A Windows machine", "fab fa-windows"
    CUSTOM = 9000, "Custom", "A custom Docker image", "fas fa-whale"


@builtin_node(NodeType.MACHINE)
class Machine(IsSpatial, Resource):
    """
    A Machine provides physical compute.
    NOTE :RichComputing: Machines also need Deployments/Endpoints/...?
    """

    type: MachineType = builtin_property(30, default=MachineType.RUNTIME)

    version: str = builtin_property(60, default=VERSION)
    external_name: Optional[str] = builtin_property(
        62, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM
    )
    external_id: Optional[str] = builtin_property(
        63, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM
    )
    image_id: Optional[str] = builtin_property(
        64, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM
    )
    grpc_url: Optional[str] = builtin_property(
        65, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM
    )
    vnc_url: Optional[str] = builtin_property(
        66, can_read=RoleType.SYSTEM, can_write=RoleType.SYSTEM
    )
    client: Optional["Client"] = builtin_property(69, node_space_from="self")
    if TYPE_CHECKING:
        client_ptr: Optional[NodeReference] = None

    cpu: float = builtin_property(
        70, description="vCPU count", default=1.0, can_write=RoleType.SYSTEM
    )
    ram: float = builtin_property(71, description="GB", default=1.0, can_write=RoleType.SYSTEM)
    width: int = builtin_property(75, default=1280, can_write=RoleType.SYSTEM)
    height: int = builtin_property(76, default=960, can_write=RoleType.SYSTEM)
    is_headless: bool = builtin_property(77, default=False, can_write=RoleType.SYSTEM)

from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    VERSION,
    Enum,
    EnumType,
    Float32,
    NodeType,
    Resource,
    UInt32,
    builtin_entity,
    builtin_enum,
    builtin_property,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.MACHINE_TYPE)
class MachineType(Enum):
    RUNTIME = 10, "Runtime", "The main Destack runtime", "fas fa-machine-classic"
    UBUNTU = 1000, "Ubuntu", "A Linux machine running Ubuntu", "fab fa-ubuntu"
    MAC = 1100, "Mac", "A Mac machine", "fab fa-apple"
    WINDOWS = 1200, "Windows", "A Windows machine", "fab fa-windows"
    CUSTOM = 9000, "Custom", "A custom Docker image", "fas fa-whale"


@builtin_entity(NodeType.MACHINE)
class Machine(Resource):
    """
    A Machine provides physical compute.
    NOTE :RichComputing: Machines also need Deployments/Endpoints/...?
    """

    type: MachineType = builtin_property(100, default=MachineType.RUNTIME)

    version: str = builtin_property(120, default=VERSION)
    external_name: Optional[str] = builtin_property(121)
    external_id: Optional[str] = builtin_property(122)
    image_id: Optional[str] = builtin_property(123)

    cpu: Float32 = builtin_property(130, description="vCPU count", default=1.0)
    ram: Float32 = builtin_property(131, description="GB", default=1.0)
    width: UInt32 = builtin_property(132, default=1280)
    height: UInt32 = builtin_property(133, default=960)
    is_headless: bool = builtin_property(134, default=False)

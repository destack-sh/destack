from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    VERSION,
    BuiltinEnum,
    EnumType,
    IsInPackage,
    IsProvisionable,
    IsSubject,
    Node,
    NodeReference,
    NodeType,
    enum_,
    node_,
    property_,
)
from bench.pb2 import MachineData

if TYPE_CHECKING:
    from bench.language import Client

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.MACHINE_TYPE)
class MachineType(BuiltinEnum):
    RUNTIME = 10, "Runtime", "The main Bench runtime", "fas fa-machine-classic"
    UBUNTU = 1000, "Ubuntu", "A Linux machine running Ubuntu", "fab fa-ubuntu"
    MAC = 1100, "Mac", "A Mac machine", "fab fa-apple"
    WINDOWS = 1200, "Windows", "A Windows machine", "fab fa-windows"
    CUSTOM = 9000, "Custom", "A custom Docker image", "fas fa-whale"


@node_(NodeType.MACHINE)
class Machine(IsSubject, IsProvisionable, IsInPackage, Node[MachineData]):
    """
    A Machine provides physical compute.
    NOTE :RichComputing: Machines also need Deployments/Endpoints/...?
    """

    type: MachineType = property_(30, default=MachineType.RUNTIME)

    version: str = property_(60, default=VERSION)
    external_name: Optional[str] = property_(62, can_read="system", can_write="system")
    external_id: Optional[str] = property_(63, can_read="system", can_write="system")
    image_id: Optional[str] = property_(64, can_read="system", can_write="system")
    grpc_url: Optional[str] = property_(65, can_read="system", can_write="system")
    vnc_url: Optional[str] = property_(66, can_read="system", can_write="system")
    client: Optional["Client"] = property_(69, node_bench_from="self")
    if TYPE_CHECKING:
        client_ptr: Optional[NodeReference] = None
        client_id: Optional[UUID] = None

    cpu: float = property_(70, description="vCPU count", default=1.0, can_write="system")
    ram: float = property_(71, description="GB", default=1.0, can_write="system")
    width: int = property_(75, default=1280, can_write="system")
    height: int = property_(76, default=960, can_write="system")
    is_headless: bool = property_(77, default=False, can_write="system")

    @staticmethod
    def new(type: MachineType, name: str, *, is_headless: bool = False) -> "Machine":
        return Machine(type=type, name=name, is_headless=is_headless)

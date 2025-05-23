from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    VERSION,
    BuiltinEnum,
    EnumType,
    IsProvisionable,
    IsSubject,
    Node,
    NodeReference,
    NodeType,
    enum_,
    node_,
    property_,
)
from bench.pb2 import ComputerData

if TYPE_CHECKING:
    from bench.language import Client

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.COMPUTER_TYPE)
class ComputerType(BuiltinEnum):
    RUNTIME = 10, "Runtime", "The main Bench runtime", "fas fa-computer-classic"
    UBUNTU = 1000, "Ubuntu", "A Linux computer running Ubuntu", "fab fa-ubuntu"
    MAC = 1100, "Mac", "A Mac computer", "fab fa-apple"
    WINDOWS = 1200, "Windows", "A Windows computer", "fab fa-windows"
    CUSTOM = 9000, "Custom", "A custom Docker image", "fas fa-whale"


@node_(NodeType.COMPUTER)
class Computer(IsSubject, IsProvisionable, Node[ComputerData]):
    """
    A Computer provides physical compute.
    NOTE :RichComputing: Computers also need Deployments/Endpoints/...?
    """

    type: ComputerType = property_(30, default=ComputerType.RUNTIME)

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
    def new(type: ComputerType, name: str, *, is_headless: bool = False) -> "Computer":
        return Computer(type=type, name=name, is_headless=is_headless)

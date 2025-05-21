from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    VERSION,
    BuiltinEnum,
    EnumType,
    IsSubject,
    NodeReference,
    NodeType,
    ProvisionableResourceBase,
    enum_,
    node_,
    p_kernel,
    p_regular,
    p_system,
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
class Computer(IsSubject, ProvisionableResourceBase[ComputerData]):
    """
    A Computer provides physical compute.
    NOTE :RichComputing: Computers also need Deployments/Endpoints/...?
    """

    type: ComputerType = p_regular(30, default=ComputerType.RUNTIME)

    version: str = p_system(60, default=VERSION)
    external_name: Optional[str] = p_kernel(62, sensitive=True)
    external_id: Optional[str] = p_kernel(63, sensitive=True)
    image_id: Optional[str] = p_kernel(64, sensitive=True)
    # NOTE :Security: Computer.grpc_url/vnc_url should maybe be :RealSecrets
    grpc_url: Optional[str] = p_kernel(65, sensitive=True)
    vnc_url: Optional[str] = p_kernel(66, sensitive=True)
    client: Optional["Client"] = p_system(69, node_bench_from="self")
    if TYPE_CHECKING:
        client_ptr: Optional[NodeReference] = None
        client_id: Optional[UUID] = None

    cpu: float = p_system(70, description="vCPU count", default=1.0)
    ram: float = p_system(71, description="GB", default=1.0)
    width: int = p_system(75, default=1280)
    height: int = p_system(76, default=960)
    is_headless: bool = p_system(77, default=False)

    @staticmethod
    def new(type: ComputerType, name: str, *, is_headless: bool = False) -> "Computer":
        return Computer(type=type, name=name, is_headless=is_headless)

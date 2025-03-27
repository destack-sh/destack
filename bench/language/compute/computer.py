from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    CPU_CONSTRAINT,
    RAM_CONSTRAINT,
    VERSION,
    BuiltinEnum,
    EnumType,
    IsSubject,
    LocalNodeList,
    NodeType,
    Resource,
    enum_,
    node_,
    p_internal,
    p_kernel,
    p_node_children,
    p_regular,
    p_system,
)
from bench.pb2 import ComputerData

if TYPE_CHECKING:
    from bench.language import Application, Client

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.COMPUTER_TYPE)
class ComputerType(BuiltinEnum):
    RUNTIME = 10, "Runtime", "The main Bench runtime", "fas fa-computer-classic"
    UBUNTU = 1000, "Ubuntu", "A Linux computer running Ubuntu", "fab fa-ubuntu"
    MAC = 1100, "Mac", "A Mac computer", "fab fa-apple"
    WINDOWS = 1200, "Windows", "A Windows computer", "fab fa-windows"
    CUSTOM = 9000, "Custom", "A custom Docker image", "fas fa-whale"


@node_(NodeType.COMPUTER, has_subtypes=True)
class Computer(IsSubject, Resource[ComputerData]):
    """
    A Computer provides physical compute.
    """

    type: ComputerType = p_regular(30, default=ComputerType.RUNTIME)

    version: str = p_system(60, default=VERSION, default_sql=None)
    target_version: str = p_internal(61, default=VERSION, default_sql=None)
    external_name: Optional[str] = p_kernel(62, require=False, default=None, sensitive=True)
    external_id: Optional[str] = p_kernel(63, require=False, default=None, sensitive=True)
    connection_uri: Optional[str] = p_kernel(
        64, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )
    client: Optional["Client"] = p_system(
        65, require=False, array=False, references=NodeType.CLIENT, fk=True, same_bench=True
    )

    cpu: float = p_system(
        70, description="vCPU count", default=1.0, default_sql=None, constraint=CPU_CONSTRAINT
    )
    ram: float = p_system(
        71, description="GB", default=1.0, default_sql=None, constraint=RAM_CONSTRAINT
    )
    # is_headless?
    width: int = p_system(75, default=1280, default_sql=None)
    height: int = p_system(76, default=800, default_sql=None)

    applications: LocalNodeList["Application"] = p_node_children(NodeType.APPLICATION)

    @staticmethod
    def new(type: ComputerType, name: str, **kwargs) -> "Computer":
        return Computer(type=type, name=name, **kwargs)

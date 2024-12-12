from typing import TYPE_CHECKING, Optional

from bench.language.bench import (
    CPU_CONSTRAINT,
    RAM_CONSTRAINT,
    Bench,
    PhysicalResourceNode,
    Server,
)
from bench.language.const import VERSION, EnumType, NodeType, StructType, enum_
from bench.language.node import Struct, node_, struct_
from bench.language.property import p_internal, p_kernel, p_node_parent, p_regular, p_system
from bench.proto.wire.lang_pb2 import BrowserData, MachineData
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Client

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.MACHINE_TYPE)
class MachineType(IdEnum):
    RUNTIME = 1  # our own Bench runtime
    # IMAGE = 2  # custom Machine/Docker image


@struct_(StructType.MACHINE_IMAGE)
class MachineImage(Struct):
    pass


@node_(NodeType.MACHINE)
class Machine(PhysicalResourceNode[MachineData]):
    """
    A Machine provides physical compute.
    Machines may be tied to a Server for our own Runtime or may be manually provisioned.
    """

    parent: Server | Bench | None = p_node_parent(4, NodeType.SERVER, NodeType.BENCH)
    type: MachineType = p_regular(30, default=MachineType.RUNTIME)

    version: str = p_system(50, default=VERSION, default_sql=None)
    target_version: str = p_internal(51, default=VERSION, default_sql=None)
    external_name: Optional[str] = p_kernel(52, require=False, default=None, sensitive=True)
    external_id: Optional[str] = p_kernel(53, require=False, default=None, sensitive=True)
    connection_uri: Optional[str] = p_kernel(
        54, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )
    client: Optional["Client"] = p_system(
        55, require=False, array=False, references=NodeType.CLIENT, fk=True, same_bench=True
    )

    cpu: float = p_system(60, description="vCPU count", constraint=CPU_CONSTRAINT)
    target_cpu: Optional[float] = p_regular(61, default=None, description="vCPU count")
    ram: float = p_regular(62, description="GB", constraint=RAM_CONSTRAINT)
    target_ram: Optional[float] = p_regular(63, default=None, description="GB")


@enum_(EnumType.BROWSER_TYPE)
class BrowserType(IdEnum):
    CHROME = 1


@node_(NodeType.BROWSER)
class Browser(PhysicalResourceNode[BrowserData]):
    """A Browser instance for web browsing."""

    type: BrowserType = p_regular(30, default=BrowserType.CHROME)

    version: str | None = p_regular(50, default=None)
    target_version: Optional[str] = p_regular(51, default=None)
    external_name: Optional[str] = p_kernel(52, require=False, default=None, sensitive=True)

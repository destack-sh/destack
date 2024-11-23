from datetime import datetime
from typing import TYPE_CHECKING, Optional

from bench.language.bench import (
    CPU_CONSTRAINT,
    RAM_CONSTRAINT,
    AnonymousResourceNode,
    Bench,
    Server,
)
from bench.language.const import VERSION, EnumType, NodeType, StructType, enum_
from bench.language.node import Struct, node_, struct_
from bench.language.property import p_internal, p_kernel, p_node_parent, p_regular, p_system
from bench.language.validation import TITLE_CONSTRAINT
from bench.proto.wire.lang_pb2 import BrowserData, MachineData
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Client

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.MACHINE_TYPE)
class MachineType(IdEnum):
    RUNTIME = 1  # our own Bench runtime
    # IMAGE = 2  # custom Machine image


@struct_(StructType.MACHINE_IMAGE)
class MachineImage(Struct):
    pass


@node_(NodeType.MACHINE)
class Machine(AnonymousResourceNode[MachineData]):
    """
    A Machine provides physical compute.
    Machines may be tied to a Server for our own Runtime or may be manually provisioned.
    """

    parent: Server | Bench | None = p_node_parent(4, NodeType.SERVER, NodeType.BENCH)
    type: MachineType = p_regular(30, default=MachineType.RUNTIME)
    title: str = p_regular(32, constraint=TITLE_CONSTRAINT)

    version: str = p_system(40, default=VERSION, default_sql=None)
    current_version: Optional[str] = p_system(41, default=None)
    external_name: Optional[str] = p_kernel(42, require=False, default=None, sensitive=True)
    external_id: Optional[str] = p_kernel(43, require=False, default=None, sensitive=True)
    connection_uri: Optional[str] = p_kernel(
        44, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )
    client: Optional["Client"] = p_system(
        45, require=False, array=False, references=NodeType.CLIENT, fk=True, same_bench=True
    )

    cpu: float = p_regular(50, description="vCPU count", constraint=CPU_CONSTRAINT)
    current_cpu: Optional[float] = p_system(51, default=None, description="vCPU count")
    ram: float = p_regular(52, description="GB", constraint=RAM_CONSTRAINT)
    current_ram: Optional[float] = p_system(53, default=None, description="GB")

    started_at: Optional[datetime] = p_system(60, default=None)
    killed_at: Optional[datetime] = p_internal(61, default=None)
    terminated_at: Optional[datetime] = p_system(62, default=None)
    active_at: Optional[datetime] = p_system(63, default=None)
    restarted_at: Optional[datetime] = p_internal(64, default=None)


@node_(NodeType.BROWSER)
class Browser(AnonymousResourceNode[BrowserData]):
    """Browser instance for web browsing."""

    ...

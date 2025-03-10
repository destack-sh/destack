from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    CPU_CONSTRAINT,
    RAM_CONSTRAINT,
    VERSION,
    BuiltinEnum,
    EnumType,
    NodeType,
    Resource,
    Struct,
    StructType,
    enum_,
    node_,
    p_internal,
    p_kernel,
    p_regular,
    p_system,
    struct_,
)
from bench.pb2.lang_pb2 import MachineData

if TYPE_CHECKING:
    from bench.language import Client

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.MACHINE_TYPE)
class MachineType(BuiltinEnum):
    RUNTIME = 1  # our own Bench runtime
    # IMAGE = 2  # custom Machine/Docker image


@struct_(StructType.MACHINE_IMAGE)
class MachineImage(Struct):
    pass


@node_(NodeType.MACHINE, has_subtypes=True)
class Machine(Resource[MachineData]):
    """
    A Machine provides physical compute.
    Machines may be tied to a Server for our own Runtime or may be manually provisioned.
    """

    type: MachineType = p_regular(30, default=MachineType.RUNTIME)

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

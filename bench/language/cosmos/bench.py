from typing import TYPE_CHECKING, Optional
from uuid import UUID

from bench.language.core import (
    NAME_CONSTRAINT,
    REGION,
    SLUG_CONSTRAINT,
    BenchNode,
    BuiltinEnum,
    EnumType,
    IsOwnable,
    LocalNodeList,
    NodeType,
    Region,
    StructType,
    enum_,
    node_,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.pb2 import BenchData

if TYPE_CHECKING:
    from bench.language import (
        Database,
        Handle,
        Icon,
        Membership,
        NodeReference,
        Package,
        Region,
        TextLine,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.BENCH_STATUS)
class BenchStatus(BuiltinEnum):
    """The status of a Bench"""

    RESERVED = 20  # not yet initialized
    ACTIVATED = 50


@node_(NodeType.BENCH, roots=())
class Bench(IsOwnable, BenchNode[BenchData]):
    """
    A Bench is the OS for personal software.
    """

    parent: None = p_node_parent(4)
    handle: Optional["Handle"] = p_system(
        31,
        require=False,
        array=False,
        references=NodeType.HANDLE,
        fk=True,
        same_bench=True,
    )
    handles: LocalNodeList["Handle"] = p_node_children(NodeType.HANDLE)
    slug: str = p_system(32, unique=True, constraint=SLUG_CONSTRAINT)  # must match main handle
    name: str = p_regular(33, constraint=NAME_CONSTRAINT)
    line: Optional["TextLine"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.TEXT_LINE
    )
    icon: Optional["Icon"] = p_regular(35, default=None, struct=StructType.ICON)
    region: "Region" = p_system(37, require=True, default=REGION, default_sql=None)
    # TODO :Security: Bench.encryption_key (DEK) or put it into a Vault (Bench.main_vault) :RealSecrets

    # status
    status: BenchStatus = p_system(40, default=BenchStatus.RESERVED)

    # content
    database: Optional["Database"] = p_system(
        50, require=False, array=False, references=NodeType.DATABASE, fk=True, same_bench=True
    )
    package: Optional["Package"] = p_regular(
        51,
        require=False,
        array=False,
        references=NodeType.PACKAGE,
        fk=True,
        same_bench=True,
    )
    if TYPE_CHECKING:
        database_ptr: Optional[NodeReference] = None
        database_id: Optional[UUID] = None
        package_ptr: Optional[NodeReference] = None
        package_id: Optional[UUID] = None

    packages: LocalNodeList["Package"] = p_node_children(NodeType.PACKAGE)
    memberships: LocalNodeList["Membership"] = p_node_children(NodeType.MEMBERSHIP)

    @property
    def is_attached(self) -> bool:
        return True

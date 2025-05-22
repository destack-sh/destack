from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    REGION,
    BuiltinEnum,
    EnumType,
    IsDeletable,
    IsGlobal,
    IsInBench,
    IsInvite,
    IsMembership,
    IsOwnable,
    Node,
    NodeType,
    Region,
    StringFormat,
    enum_,
    node_,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.pb2 import BenchData, BenchInviteData, BenchMembershipData

if TYPE_CHECKING:
    from bench.language import (
        Database,
        Handle,
        Icon,
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


@node_(NodeType.BENCH)
class Bench(IsGlobal, IsOwnable, IsInBench, Node[BenchData]):
    """
    A Bench is the OS for personal software.
    """

    handle: Optional["Handle"] = p_system(31, node_bench_from="self")
    slug: str = p_system(32, unique=True, format=StringFormat.SLUG)  # must match main handle
    name: str = p_regular(33, format=StringFormat.NAME)
    line: Optional["TextLine"] = p_regular(34)
    icon: Optional["Icon"] = p_regular(35)
    region: "Region" = p_system(37, default=REGION)
    # TODO :Security: Bench.encryption_key (DEK) or put it into a Vault (Bench.vault) :RealSecrets

    # status
    status: BenchStatus = p_system(40, default=BenchStatus.RESERVED)

    # content
    database: Optional["Database"] = p_system(50, node_bench_from="self")
    package: Optional["Package"] = p_regular(51, node_bench_from="self")
    if TYPE_CHECKING:
        database_ptr: Optional[NodeReference] = None
        database_id: Optional[UUID] = None
        package_ptr: Optional[NodeReference] = None
        package_id: Optional[UUID] = None

    @property
    def is_attached(self) -> bool:
        return True


@node_(NodeType.BENCH_INVITE)
class BenchInvite(IsInvite, IsDeletable, IsInBench, Node[BenchInviteData]):
    """
    A BenchInvite is an invite to a Bench.
    """

    parent: "Bench" = p_node_parent()


@node_(NodeType.BENCH_MEMBERSHIP)
class BenchMembership(IsMembership, IsDeletable, IsInBench, Node[BenchMembershipData]):
    """
    A BenchMembership is a membership to a Bench.
    """

    parent: "Bench" = p_node_parent()

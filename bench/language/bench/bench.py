from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsDeletable,
    IsGlobal,
    IsIcon,
    IsInBench,
    IsInvite,
    IsMembership,
    IsNamed,
    IsOwnable,
    IsRegional,
    IsSlug,
    Node,
    NodeType,
    enum_,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import BenchData, BenchInviteData, BenchMembershipData

if TYPE_CHECKING:
    from bench.language import (
        Database,
        NodeReference,
        Package,
        TextLine,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.BENCH_STATUS)
class BenchStatus(BuiltinEnum):
    """The status of a Bench"""

    RESERVED = 20  # not yet initialized
    ACTIVATED = 50


@node_(NodeType.BENCH)
class Bench(
    IsGlobal,
    IsOwnable,
    IsInBench,
    IsNamed,
    IsSlug,
    IsIcon,
    IsRegional,
    Node[BenchData],
):
    """
    A Bench is the OS for personal software.
    """

    line: Optional["TextLine"] = property_(40)
    status: BenchStatus = property_(41, default=BenchStatus.RESERVED)

    # content
    database: Optional["Database"] = property_(
        50, node_bench_from="self", can_write="system", description="The Database."
    )
    package: Optional["Package"] = property_(
        51, node_bench_from="self", can_write="system", description="The Main Package."
    )
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

    parent: "Bench" = property_parent_()


@node_(NodeType.BENCH_MEMBERSHIP)
class BenchMembership(IsMembership, IsDeletable, IsInBench, Node[BenchMembershipData]):
    """
    A BenchMembership is a membership to a Bench.
    """

    parent: "Bench" = property_parent_()

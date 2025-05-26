from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    HasIcon,
    HasName,
    HasSlug,
    IsDeletable,
    IsGlobal,
    IsInBench,
    IsInvite,
    IsMembership,
    IsOwnable,
    IsRegional,
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
        Handle,
        NodeReference,
        Package,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.BENCH_STATUS)
class BenchStatus(BuiltinEnum):
    """The status of a Bench"""

    ACTIVE = 50


@node_(NodeType.BENCH)
class Bench(
    IsGlobal,
    IsOwnable,
    IsInBench,
    HasName,
    HasSlug,
    HasIcon,
    IsRegional,
    Node[BenchData],
):
    """
    A Bench is the OS for personal software.
    """

    status: BenchStatus = property_(41, is_repr=True)
    handle: Optional["Handle"] = property_(42, can_write="system")

    # content
    database: Optional["Database"] = property_(
        50, node_bench_from="self", can_write="system", description="The Database."
    )
    main_package: Optional["Package"] = property_(
        51, node_bench_from="self", can_write="system", description="The Main Package."
    )
    if TYPE_CHECKING:
        database_ptr: Optional[NodeReference] = None
        database_id: Optional[UUID] = None
        package_ptr: Optional[NodeReference] = None
        package_id: Optional[UUID] = None


@enum_(EnumType.BENCH_ROLE_TYPE)
class BenchRoleType(BuiltinEnum):
    """The role of a Bench"""

    ADMIN = 10
    MEMBER = 50


@node_(NodeType.BENCH_INVITE)
class BenchInvite(
    IsGlobal,
    IsInvite,
    IsDeletable,
    IsInBench,
    Node[BenchInviteData],
):
    """
    A BenchInvite is an invite to a Bench.
    """

    parent: Optional["Bench"] = property_parent_()
    role: BenchRoleType = property_(45, is_repr=True)


@node_(NodeType.BENCH_MEMBERSHIP)
class BenchMembership(
    IsGlobal,
    IsMembership,
    IsDeletable,
    IsInBench,
    Node[BenchMembershipData],
):
    """
    A BenchMembership is a membership to a Bench.
    """

    parent: Optional["Bench"] = property_parent_()
    role: BenchRoleType = property_(45, is_repr=True)

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
    IsTracked,
    Node,
    NodeType,
    Region,
    enum_,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import BenchData, BenchInviteData, BenchMembershipData

if TYPE_CHECKING:
    from bench.language import Database, Handle, NodeReference, Package

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.BENCH_STATUS)
class BenchStatus(BuiltinEnum):
    """The status of a Bench"""

    CREATING = 1
    QUEUED = 3
    RUNNING = 10
    PAUSED = 20


@node_(NodeType.BENCH, root_type=None)
class Bench(
    HasName,
    HasSlug,
    HasIcon,
    IsTracked,
    IsGlobal,
    IsOwnable,
    IsInBench,
    Node[BenchData],
):
    """
    A Bench is the OS for personal software.
    """

    # meta
    status: BenchStatus = property_(40, is_repr=True, can_write="system")
    handle: Optional["Handle"] = property_(41, node_bench_from="self", can_write="system")
    main_package: Optional["Package"] = property_(
        42, node_bench_from="self", can_write="system", description="The Main Package."
    )
    if TYPE_CHECKING:
        handle_ptr: Optional[NodeReference] = None
        handle_id: Optional[UUID] = None
        main_package_ptr: Optional[NodeReference] = None
        main_package_id: Optional[UUID] = None

    # infra
    region: Region = property_(50, can_write="system")
    cell_name: str | None = property_(51, can_write="system")  # -> Cell?
    database: Optional["Database"] = property_(55, node_bench_from="self", can_write="system")
    # search, analytics, vault, cache, ...
    if TYPE_CHECKING:
        database_ptr: Optional[NodeReference] = None
        database_id: Optional[UUID] = None


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

    role_type: BenchRoleType = property_(45, is_repr=True)


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

    role_type: BenchRoleType = property_(45, is_repr=True)

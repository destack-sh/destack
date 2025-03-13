from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    BenchNode,
    BuiltinEnum,
    EnumType,
    NodeReference,
    NodeType,
    enum_,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
)
from bench.pb2.lang_pb2 import MembershipData

if TYPE_CHECKING:
    from bench.language import Bench, Channel, Organization, Team, Thread, User

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.MEMBERSHIP_TYPE)
class MembershipType(BuiltinEnum):
    BENCH = 10
    ORGANIZATION = 20
    TEAM = 30
    CHANNEL = 100
    THREAD = 110


@node_(NodeType.MEMBERSHIP, roots=(NodeType.BENCH, NodeType.ORGANIZATION))
class Membership(BenchNode[MembershipData]):
    """
    A Membership to something for someone.
    """

    parent: Union["Bench", None] = p_node_parent(
        4, NodeType.BENCH, NodeType.ORGANIZATION, NodeType.TEAM
    )
    if TYPE_CHECKING:
        bench_id: Optional[UUID] = None
        bench_ptr: Optional[NodeReference] = None
        organization_id: Optional[UUID] = None
        organization_ptr: Optional[NodeReference] = None

    # meta
    type: MembershipType = p_regular(30, require=True)

    # content
    to: Union["Bench", "Organization", "Team", "Channel", "Thread"] = p_regular(
        40,
        require=True,
        array=False,
        baseless=True,
        ckless=True,
        references=(
            NodeType.BENCH,
            NodeType.ORGANIZATION,
            NodeType.TEAM,
            NodeType.CHANNEL,
            NodeType.THREAD,
        ),
    )
    member: "User" = p_internal(41, require=True, array=False, references=NodeType.USER)

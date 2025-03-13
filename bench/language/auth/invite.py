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
from bench.pb2.lang_pb2 import InviteData

if TYPE_CHECKING:
    from bench.language import Bench, Channel, Organization, Team, Thread, User

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.INVITE_TYPE)
class InviteType(BuiltinEnum):
    BENCH = 10
    ORGANIZATION = 20
    TEAM = 30
    CHANNEL = 100
    THREAD = 110


@node_(NodeType.INVITE, roots=(NodeType.BENCH, NodeType.ORGANIZATION))
class Invite(BenchNode[InviteData]):
    """
    An Invite to become a member of something.
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
    type: InviteType = p_regular(30, require=True)

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
    user: Optional["User"] = p_internal(41, require=False, array=False, references=NodeType.USER)
    user_email: Optional[str] = p_regular(42)
    is_owner: bool = p_regular(43, default=False)

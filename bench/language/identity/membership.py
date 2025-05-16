from typing import TYPE_CHECKING, Union, override
from uuid import UUID

from bench.language.core import (
    JOINABLE_NODE_TYPES,
    IsInstantiable,
    IsModal,
    Joinable,
    NodeReference,
    NodeType,
    Subject,
    node_,
    p_node_parent,
    p_regular,
)
from bench.language.core.node import PackageNode
from bench.pb2 import MembershipData

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.MEMBERSHIP)
class Membership(IsInstantiable, IsModal, PackageNode[MembershipData]):
    """
    A Membership of a Subject to a Joinable.
    """

    # meta
    parent: Union[Joinable, None] = p_node_parent(4, *JOINABLE_NODE_TYPES)

    # content
    member: Subject = p_regular(41)
    if TYPE_CHECKING:
        member_ptr: NodeReference | None = None
        member_id: UUID | None = None

    @override
    def __content_str__(self) -> str:
        if (member := self.member) is not None:
            return f"{member.metatype.name}, {member.absolute_path}"
        else:
            return f"{self.member_ptr}"

    @staticmethod
    def new(member: Subject, parent: Joinable | None = None, **kwargs) -> "Membership":
        membership = Membership(parent=parent, member=member, **kwargs)
        return membership

from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    IsInstantiable,
    IsJoinable,
    IsModal,
    IsNamed,
    LocalNodeList,
    NodeReference,
    NodeType,
    PageNode,
    node_,
    p_node_ancestor,
    p_node_children,
    p_node_parent,
)
from bench.pb2 import TeamData

if TYPE_CHECKING:
    from bench.language import Membership, Organization, Package, Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.TEAM, roots=(NodeType.ORGANIZATION,))
class Team(IsInstantiable, IsJoinable, IsModal, IsNamed, PageNode[TeamData]):
    """
    A Team of Users or Identities.
    """

    parent: Union["Package", "Page", "Team", None] = p_node_parent(
        4, NodeType.PACKAGE, NodeType.PAGE, NodeType.TEAM
    )
    organization: "Organization | None" = p_node_ancestor(
        7, NodeType.ORGANIZATION, require=False, store=True, wire=True
    )
    team: Optional["Team"] = p_node_ancestor(8, NodeType.TEAM, require=False, store=True, wire=True)
    if TYPE_CHECKING:
        team_id: Optional[UUID] = None
        team_ptr: Optional[NodeReference] = None
        organization_id: Optional[UUID] = None
        organization_ptr: Optional[NodeReference] = None

    memberships: LocalNodeList["Membership"] = p_node_children(NodeType.MEMBERSHIP)

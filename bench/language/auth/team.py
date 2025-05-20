from typing import TYPE_CHECKING, Optional, Union

from fastuuid import UUID

from bench.language.core import (
    IsInstantiable,
    IsJoinable,
    IsModal,
    IsNamed,
    NodeReference,
    NodeType,
    PageNode,
    node_,
    p_node_parent,
    p_regular,
)
from bench.pb2 import TeamData

if TYPE_CHECKING:
    from bench.language import Organization, Package, Page

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.TEAM)
class Team(IsInstantiable, IsJoinable, IsModal, IsNamed, PageNode[TeamData]):
    """
    A Team of Users or Identities.
    """

    parent: Union["Package", "Page", "Team", None] = p_node_parent(4)
    organization: "Organization | None" = p_regular(40)
    team: Optional["Team"] = p_regular(41)
    if TYPE_CHECKING:
        team_id: Optional[UUID] = None
        team_ptr: Optional[NodeReference] = None
        organization_id: Optional[UUID] = None
        organization_ptr: Optional[NodeReference] = None

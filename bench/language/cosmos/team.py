from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    NAME_CONSTRAINT,
    Node,
    NodeReference,
    NodeType,
    StructType,
    node_,
    p_node_ancestor,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.pb2.lang_pb2 import TeamData

if TYPE_CHECKING:
    from bench.language import Bench, Icon, Organization, Text

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.TEAM, roots=(NodeType.ORGANIZATION,))
class Team(Node[TeamData]):
    """
    A Team of Users.
    """

    parent: Union["Organization", "Team", None] = p_node_parent(
        4, NodeType.ORGANIZATION, NodeType.TEAM
    )
    team: Optional["Team"] = p_node_ancestor(5, NodeType.TEAM, require=False, store=True, wire=True)
    organization: "Organization | None" = p_node_ancestor(
        6, NodeType.ORGANIZATION, require=False, store=True, wire=True
    )
    if TYPE_CHECKING:
        organization_id: Optional[UUID] = None
        organization_ptr: Optional[NodeReference] = None

    slug: Optional[str] = p_system(32, unique=True)  # must match main handle
    name: str = p_regular(33, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(34, default=None, struct=StructType.TEXT)
    icon: Optional["Icon"] = p_regular(35, default=None, struct=StructType.ICON)

    main_bench: Optional["Bench"] = p_system(
        40, array=False, require=False, references=NodeType.BENCH, fk=True
    )
    if TYPE_CHECKING:
        main_bench_id: Optional[UUID] = None
        main_bench_ptr: Optional[NodeReference] = None

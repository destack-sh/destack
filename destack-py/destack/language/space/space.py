from typing import TYPE_CHECKING, Optional

from fastuuid import UUID

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    Global,
    HasIcon,
    HasName,
    HasSlug,
    IsFollowable,
    IsJoinable,
    IsOwnable,
    IsStarable,
    Node,
    NodeType,
    Region,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
)
from destack.pb2 import SpaceData

if TYPE_CHECKING:
    from destack.language import Database, Folder, Handle, NodeReference

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.SPACE_STATUS)
class SpaceStatus(Enum):
    """The status of a Space"""

    CREATING = 1
    QUEUED = 3
    RUNNING = 10
    PAUSED = 20


# @builtin_enum(EnumType.SPACE_PERMISSION)
# class SpacePermission(BuiltinEnum):
#     """The permission of a Space"""

#     MANAGE_ROLES = 1
#     MANAGE_MEMBERS = 2
#     BAN_MEMBERS = 3


@builtin_node(NodeType.SPACE, root_type=None)
class Space(
    HasName,
    HasSlug,
    HasIcon,
    IsFollowable,
    IsJoinable,
    Global,
    Entity,
    IsOwnable,
    IsStarable,
    Spatial,
    Node[SpaceData],
):
    """
    A Space is the home of your personal software studio.
    """

    # meta
    name: str = property_(31, is_repr=True)
    slug: str = property_(33, is_repr=True)
    status: SpaceStatus = property_(40, is_repr=True, can_write="system")
    handle: Optional["Handle"] = property_(41, node_space_from="self", can_write="system")
    root_folder: Optional["Folder"] = property_(
        42, node_space_from="self", can_write="system", description="The root Folder."
    )
    home_folder: Optional["Folder"] = property_(
        43, node_space_from="self", can_write="system", description="The home Folder."
    )
    if TYPE_CHECKING:
        handle_ptr: Optional[NodeReference] = None
        handle_id: Optional[UUID] = None
        root_folder_ptr: Optional[NodeReference] = None
        root_folder_id: Optional[UUID] = None
        home_folder_ptr: Optional[NodeReference] = None
        home_folder_id: Optional[UUID] = None

    # infra
    region: Region = property_(50, can_write="system")
    cell_name: str | None = property_(51, can_write="system")  # -> Cell?
    database: Optional["Database"] = property_(55, node_space_from="self", can_write="system")
    # search, analytics, vault, cache, ...
    if TYPE_CHECKING:
        database_ptr: Optional[NodeReference] = None
        database_id: Optional[UUID] = None

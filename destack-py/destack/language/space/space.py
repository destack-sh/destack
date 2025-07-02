from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    HasIcon,
    HasName,
    HasSlug,
    IsFollowable,
    IsGlobal,
    IsJoinable,
    IsOwnable,
    IsSpatial,
    IsStarable,
    NodeType,
    Region,
    RoleType,
    builtin_enum,
    builtin_node,
    builtin_property,
)

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


@builtin_node(NodeType.SPACE, root_type=None)
class Space(
    IsGlobal,
    HasName,
    HasSlug,
    HasIcon,
    IsFollowable,
    IsJoinable,
    IsOwnable,
    IsStarable,
    IsSpatial,
    Entity,
):
    """
    A Space is the home of your personal software studio.
    """

    # meta
    name: str = builtin_property(31, is_repr=True)
    slug: str = builtin_property(33, is_repr=True)
    status: SpaceStatus = builtin_property(40, is_repr=True, can_write=RoleType.SYSTEM)
    handle: Optional["Handle"] = builtin_property(
        41, node_space_from="self", can_write=RoleType.SYSTEM
    )
    system_folder: Optional["Folder"] = builtin_property(
        42, node_space_from="self", can_write=RoleType.SYSTEM, description="The system Folder."
    )
    home_folder: Optional["Folder"] = builtin_property(
        43, node_space_from="self", can_write=RoleType.SYSTEM, description="The home Folder."
    )
    if TYPE_CHECKING:
        handle_ptr: Optional[NodeReference] = None
        root_folder_ptr: Optional[NodeReference] = None
        home_folder_ptr: Optional[NodeReference] = None

    # infra
    region: Region = builtin_property(50, can_write=RoleType.SYSTEM)
    galaxy_name: str | None = builtin_property(51, can_write=RoleType.SYSTEM)  # -> Galaxy?
    database: Optional["Database"] = builtin_property(
        55, node_space_from="self", can_write=RoleType.SYSTEM
    )
    # search, analytics, vault, cache, ...
    if TYPE_CHECKING:
        database_ptr: Optional[NodeReference] = None

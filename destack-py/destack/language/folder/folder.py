from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    HasIcon,
    HasName,
    HasSlug,
    IndexIn,
    IsDeletable,
    IsFollowable,
    IsJoinable,
    IsOrdered,
    IsOwnable,
    IsSpatial,
    IsStarable,
    IsTaggable,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Scene, Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.FOLDER_TYPE)
class FolderType(Enum):
    SYSTEM = 1, "Root", "The root folder of a Space", "fas fa-home"
    HOME = 2, "Home", "The home folder of a Space", "fas fa-home"
    GENERAL = 3, "General", "A general folder", "fas fa-folder-open"
    MODULE = 4, "Module", "A module", "fas fa-box-open"
    APP = 5, "App", "An app folder", "fas fa-folder"
    # SERVICE, PLUGIN, WIDGET, TEMPLATE, LIBRARY, ...


@builtin_node(
    NodeType.FOLDER,
    index=(IndexIn(columns=("space_id", "slug"), is_unique=True),),
)
class Folder(
    IsSpatial,
    HasIcon,
    HasSlug,
    HasName,
    IsTaggable,
    IsOwnable,
    IsJoinable,
    IsOrdered,
    IsDeletable,
    IsStarable,
    IsFollowable,
    Entity,
):
    """A Folder is a sub-space of a Space."""

    parent: Union["Space", "Folder", None] = builtin_property_parent(node_is_extensible=False)
    type: FolderType = builtin_property(30, is_repr=True, default=FolderType.GENERAL)

    main_scene: Optional["Scene"] = builtin_property(41)

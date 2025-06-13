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
    IsStarable,
    IsTaggable,
    IsTemplatable,
    Node,
    NodeType,
    Spatial,
    builtin_enum,
    builtin_node,
    property_,
    property_parent_,
)
from destack.pb2 import FolderData

if TYPE_CHECKING:
    from destack.language import Scene, Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.FOLDER_TYPE)
class FolderType(Enum):
    ROOT = 1, "Root", "The root folder of a Space", "fas fa-home"
    HOME = 2, "Home", "The home folder of a Space", "fas fa-home"
    GENERIC = 3, "Generic", "A generic folder", "fas fa-folder-open"
    MODULE = 4, "Module", "A module", "fas fa-box-open"
    APP = 5, "App", "An app folder", "fas fa-folder"
    # SERVICE, PLUGIN, WIDGET, TEMPLATE, LIBRARY, ...


@builtin_node(
    NodeType.FOLDER,
    index=(IndexIn(columns=("space_id", "slug"), is_unique=True),),
)
class Folder(
    Spatial,
    Entity,
    HasIcon,
    HasSlug,
    HasName,
    IsTaggable,
    IsOwnable,
    IsJoinable,
    IsOrdered,
    IsTemplatable,
    IsDeletable,
    IsStarable,
    IsFollowable,
    Node[FolderData],
):
    """A Folder is a sub-space of a Space."""

    parent: Union["Space", "Folder", None] = property_parent_(node_is_customizable=False)
    type: FolderType = property_(30, is_repr=True, default=FolderType.GENERIC)

    main_scene: Optional["Scene"] = property_(41)

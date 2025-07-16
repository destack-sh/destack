from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    Entity,
    Enum,
    EnumType,
    IsFollowable,
    IsJoinable,
    IsOrdered,
    IsOwnable,
    IsReactable,
    IsStarable,
    IsTaggable,
    NodeType,
    builtin_enum,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Icon, Scene, Space

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.FOLDER_TYPE)
class FolderType(Enum):
    SYSTEM = 1, "Root", "The root folder of a Space", "fas fa-home"
    HOME = 2, "Home", "The home folder of a Space", "fas fa-home"
    GENERAL = 3, "General", "A general folder", "fas fa-folder-open"
    MODULE = 4, "Module", "A module", "fas fa-box-open"
    APP = 5, "App", "An app folder", "fas fa-folder"
    # SERVICE, PLUGIN, WIDGET, TEMPLATE, LIBRARY, ...


@builtin_node(NodeType.FOLDER)
class Folder(
    IsTaggable,
    IsOwnable,
    IsJoinable,
    IsOrdered,
    IsStarable,
    IsFollowable,
    IsReactable,
    Entity,
):
    """A Folder is a sub-space of a Space."""

    parent: Union["Space", "Folder", None] = builtin_property_parent()
    type: FolderType = builtin_property(100, is_repr=True, default=FolderType.GENERAL)
    icon: "Icon | None" = builtin_property(102)
    slug: str | None = builtin_property(103, is_repr=True)

    main_scene: Optional["Scene"] = builtin_property(110)

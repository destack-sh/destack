from typing import TYPE_CHECKING, Optional, Union

from destack.core import (
    Entity,
    Enum,
    EnumType,
    NodeType,
    TraitType,
    declare_entity,
    declare_enum,
    declare_property,
    declare_property_parent,
)

if TYPE_CHECKING:
    from destack import Icon, Scene, Space

# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.FOLDER_TYPE)
class FolderType(Enum):
    SYSTEM = 1, "Root", "The root folder of a Space", "fas fa-home"
    HOME = 2, "Home", "The home folder of a Space", "fas fa-home"
    GENERAL = 3, "General", "A general folder", "fas fa-folder-open"
    MODULE = 4, "Module", "A module", "fas fa-box-open"
    APP = 5, "App", "An app folder", "fas fa-folder"
    # SERVICE, PLUGIN, WIDGET, TEMPLATE, LIBRARY, ...


@declare_entity(
    NodeType.FOLDER,
    traits=(
        TraitType.OWNABLE,
        TraitType.ORDERED,
        TraitType.JOINABLE,
        TraitType.STARABLE,
        TraitType.FOLLOWABLE,
        TraitType.REACTABLE,
    ),
)
class Folder(Entity):
    """A Folder is a sub-space of a Space."""

    parent: Union["Space", "Folder", None] = declare_property_parent()
    type: FolderType = declare_property(100, is_repr=True, default=FolderType.GENERAL)
    icon: "Icon | None" = declare_property(102)
    slug: str | None = declare_property(103, is_repr=True)

    main_scene: Optional["Scene"] = declare_property(110)

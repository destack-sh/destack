from typing import TYPE_CHECKING, Union

from destack.core import (
    Entity,
    EnumDeclaration,
    EnumType,
    Icon,
    NodeType,
    Space,
    TraitType,
    declare_entity,
    declare_enum,
    declare_property,
    declare_property_parent,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@declare_enum(EnumType.FOLDER_TYPE)
class FolderType(EnumDeclaration):
    SYSTEM = 1, "Root", "The root folder of a Space"
    HOME = 2, "Home", "The home folder of a Space"
    GENERAL = 3, "General", "A general folder"
    MODULE = 4, "Module", "A module"
    APP = 5, "App", "An app folder"
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

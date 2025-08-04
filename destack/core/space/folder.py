from typing import TYPE_CHECKING, Union

from ..builtin import (
    Entity,
    EnumType,
    NodeType,
    OptionEnum,
    TraitType,
    declare_entity,
    declare_enum,
    declare_option,
    declare_property,
    declare_property_parent,
)

if TYPE_CHECKING:
    from destack import Icon, Space


@declare_enum(EnumType.FOLDER_TYPE)
class FolderType(OptionEnum):
    SYSTEM = declare_option(1, "Root", description="The root folder of a Space")
    HOME = declare_option(2, "Home", description="The home folder of a Space")
    GENERAL = declare_option(3, "General", description="A general folder")
    MODULE = declare_option(4, "Module", description="A module")
    APP = declare_option(5, "App", description="An app folder")
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

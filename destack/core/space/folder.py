from typing import TYPE_CHECKING

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
)

if TYPE_CHECKING:
    pass


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

    type: FolderType = declare_property(
        100,
        is_repr=True,
        default=FolderType.GENERAL,
        tag=None,
    )
    slug: str | None = declare_property(103, is_repr=True, tag=None)

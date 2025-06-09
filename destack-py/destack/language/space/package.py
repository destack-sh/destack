from typing import TYPE_CHECKING, Optional, Union

from destack.language.core import (
    BuiltinEnum,
    EnumType,
    HasIcon,
    HasName,
    HasSlug,
    IndexIn,
    IsArchivable,
    IsDeletable,
    IsEnvironmental,
    IsGlobal,
    IsInFolder,
    IsInvite,
    IsJoinable,
    IsMembership,
    IsOwnable,
    IsTemplatable,
    IsTracked,
    Node,
    NodeType,
    enum_,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import FolderData, FolderInviteData, FolderMembershipData

if TYPE_CHECKING:
    from destack.language import Scene, Space

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.FOLDER_TYPE)
class FolderType(BuiltinEnum):
    ROOT = 1, "Root", "The root folder of a Space", "fas fa-home"
    HOME = 2, "Home", "The home folder of a Space", "fas fa-home"
    GENERIC = 3, "Generic", "A generic folder", "fas fa-folder-open"
    APP = 4, "App", "An app folder", "fas fa-folder"
    MODULE = 5, "Module", "A module", "fas fa-box-open"
    # SERVICE, PLUGIN, WIDGET, TEMPLATE, LIBRARY, ...


@node_(
    NodeType.FOLDER,
    index=(IndexIn(columns=("space_id", "slug"), is_unique=True),),
)
class Folder(
    HasIcon,
    HasSlug,
    HasName,
    IsEnvironmental,
    IsOwnable,
    IsJoinable,
    IsTemplatable,
    IsInFolder,
    IsDeletable,
    IsArchivable,
    IsTracked,
    Node[FolderData],
):
    """A Folder is a sub-space of a Space."""

    parent: Union["Space", "Folder", None] = property_parent_(node_is_customizable=False)
    type: FolderType = property_(30, is_repr=True, default=FolderType.GENERIC)

    main_scene: Optional["Scene"] = property_(41)


@enum_(EnumType.FOLDER_ROLE_TYPE)
class FolderRoleType(BuiltinEnum):
    ADMIN = 1
    DEVELOPER = 3
    USER = 5
    SPECTATOR = 10


@node_(NodeType.FOLDER_MEMBERSHIP)
class FolderMembership(
    IsGlobal,
    IsMembership,
    IsDeletable,
    IsInFolder,
    Node[FolderMembershipData],
):
    """A FolderMembership is a membership to a Folder."""

    parent: Optional["Folder"] = property_parent_(node_is_customizable=False)

    role_type: FolderRoleType = property_(45)


@node_(NodeType.FOLDER_INVITE)
class FolderInvite(
    IsGlobal,
    IsInvite,
    IsDeletable,
    IsInFolder,
    Node[FolderInviteData],
):
    """A FolderInvite is an invite to a Folder."""

    parent: Optional["Folder"] = property_parent_(node_is_customizable=False)

    role_type: FolderRoleType = property_(45)

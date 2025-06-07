from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    HasEnvironment,
    HasIcon,
    HasName,
    HasSlug,
    IndexIn,
    IsArchivable,
    IsDeletable,
    IsGlobal,
    IsInPackage,
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
from bench.pb2 import PackageData, PackageInviteData, PackageMembershipData

if TYPE_CHECKING:
    from bench.language import Page, Scene, Space

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PACKAGE_TYPE)
class PackageType(BuiltinEnum):
    HOME = 10
    APPLICATION = 20
    # TEMPLATE, LIBRARY, ...


@node_(
    NodeType.PACKAGE,
    index=(IndexIn(columns=("space_id", "slug"), is_unique=True),),
)
class Package(
    HasIcon,
    HasSlug,
    HasEnvironment,
    HasName,
    IsGlobal,
    IsOwnable,
    IsJoinable,
    IsTemplatable,
    IsInPackage,
    IsDeletable,
    IsArchivable,
    IsTracked,
    Node[PackageData],
):
    """A Package is a semi-isolated area of a Bench."""

    parent: Optional["Space"] = property_parent_()
    type: PackageType = property_(30, is_repr=True)

    main_page: Optional["Page"] = property_(40)
    main_scene: Optional["Scene"] = property_(41)


@enum_(EnumType.PACKAGE_ROLE_TYPE)
class PackageRoleType(BuiltinEnum):
    MEMBER = 10
    ADMIN = 50


@node_(NodeType.PACKAGE_MEMBERSHIP)
class PackageMembership(
    IsGlobal,
    IsMembership,
    IsDeletable,
    IsInPackage,
    Node[PackageMembershipData],
):
    """A PackageMembership is a membership to a Package."""

    parent: Optional["Package"] = property_parent_()

    role_type: PackageRoleType = property_(45)


@node_(NodeType.PACKAGE_INVITE)
class PackageInvite(
    IsGlobal,
    IsInvite,
    IsDeletable,
    IsInPackage,
    Node[PackageInviteData],
):
    """A PackageInvite is an invite to a Package."""

    parent: Optional["Package"] = property_parent_()

    role_type: PackageRoleType = property_(45)

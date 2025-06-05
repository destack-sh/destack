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
    Node,
    NodeType,
    enum_,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import PackageData, PackageInviteData, PackageMembershipData

if TYPE_CHECKING:
    from bench.language import Bench, Page, Scene

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PACKAGE_TYPE)
class PackageType(BuiltinEnum):
    HOME = 10
    APPLICATION = 20
    # TEMPLATE, LIBRARY, ...


@node_(
    NodeType.PACKAGE,
    index=(IndexIn(columns=("bench_id", "slug"), is_unique=True),),
)
class Package(
    IsGlobal,
    IsOwnable,
    IsJoinable,
    IsTemplatable,
    HasIcon,
    HasSlug,
    HasEnvironment,
    HasName,
    IsInPackage,
    IsDeletable,
    IsArchivable,
    Node[PackageData],
):
    """A Package is a semi-isolated area of a Bench."""

    parent: Optional["Bench"] = property_parent_()
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

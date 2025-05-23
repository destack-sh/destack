from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IndexIn,
    IsArchivable,
    IsDeletable,
    IsIcon,
    IsInPackage,
    IsJoinable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsSlug,
    IsTemplatable,
    Node,
    NodeType,
    enum_,
    node_,
    property_,
    property_parent_,
)
from bench.pb2 import PackageData

if TYPE_CHECKING:
    from bench.language import Bench

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PACKAGE_TYPE)
class PackageType(BuiltinEnum):
    OPEN = 1, "Open"
    CLOSED = 2, "Closed"
    PRIVATE = 3, "Private"


@node_(NodeType.PACKAGE, index=(IndexIn(columns=("bench_id", "slug"), is_unique=True),))
class Package(
    IsOwnable,
    IsJoinable,
    IsTemplatable,
    IsIcon,
    IsSlug,
    IsModal,
    IsNamed,
    IsInPackage,
    IsDeletable,
    IsArchivable,
    Node[PackageData],
):
    """A Package is a semi-isolated area of a Bench."""

    # meta
    parent: Optional["Bench"] = property_parent_()
    type: PackageType = property_(30)

    @property
    def package(self):
        return self

    @property
    def absolute_path(self) -> str:
        bench = self.bench
        if bench is not None:
            return f"{bench.slug}:{self.slug}"
        else:
            return f"<detached>:{self.slug}"

    @property
    def package_id(self):
        return self.id

    @property
    def package_ptr(self):
        return self.to_ref()

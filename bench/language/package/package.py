from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    SLUG_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    IndexIn,
    IsJoinable,
    IsModal,
    IsNamed,
    IsOwnable,
    IsTemplatable,
    NodeType,
    PackageNode,
    enum_,
    node_,
    p_node_parent,
    p_regular,
)
from bench.pb2 import PackageData

if TYPE_CHECKING:
    from bench.language import Bench, Icon

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PACKAGE_TYPE)
class PackageType(BuiltinEnum):
    OPEN = 1, "Open"
    CLOSED = 2, "Closed"
    PRIVATE = 3, "Private"


@node_(NodeType.PACKAGE, index=(IndexIn(columns=("bench_id", "slug"), is_unique=True),))
class Package(IsOwnable, IsJoinable, IsTemplatable, IsModal, IsNamed, PackageNode[PackageData]):
    """A Package is a semi-isolated area of a Bench."""

    # meta
    parent: Optional["Bench"] = p_node_parent(4, NodeType.BENCH)
    type: PackageType = p_regular(30)
    slug: str | None = p_regular(34, constraint=SLUG_CONSTRAINT)
    icon: Optional["Icon"] = p_regular(35)

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

from typing import TYPE_CHECKING, Optional

from bench.language import SourceNode
from bench.language.core import (
    NAME_CONSTRAINT,
    OWNER_TYPES,
    SLUG_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    IndexIn,
    LocalNodeList,
    NodeType,
    Owner,
    StructType,
    enum_,
    node_,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.pb2 import (
    PackageData,
)

if TYPE_CHECKING:
    from bench.language import Bench, Channel, Dependency, Icon, Page, Space, Text

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PACKAGE_TYPE)
class PackageType(BuiltinEnum):
    MAIN = 1
    SIDE = 5
    SNAPSHOT = 10
    # ARCHIVE?


@node_(NodeType.PACKAGE, index=(IndexIn(columns=("bench_id", "slug"), is_unique=True),))
class Package(SourceNode[PackageData]):
    """A Package is an isolated segment of a Bench."""

    parent: "Bench | None" = p_node_parent(4, NodeType.BENCH)
    type: PackageType = p_regular(30, require=True)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    slug: str = p_regular(33, constraint=SLUG_CONSTRAINT)
    text: Optional["Text"] = p_regular(34, require=False, array=False, struct=StructType.TEXT)
    icon: Optional["Icon"] = p_regular(35, require=False, array=False, struct=StructType.ICON)
    owned_by: Optional[Owner] = p_regular(36, require=False, array=False, references=OWNER_TYPES)

    base: Optional["Package"] = p_system(
        40, require=False, array=False, references=NodeType.PACKAGE, fk=True, same_bench=True
    )

    pages: LocalNodeList["Page"] = p_node_children(NodeType.PAGE)
    channels: LocalNodeList["Channel"] = p_node_children(NodeType.CHANNEL)
    spaces: LocalNodeList["Space"] = p_node_children(NodeType.SPACE)
    dependencies: LocalNodeList["Dependency"] = p_node_children(NodeType.DEPENDENCY)

    @property
    def is_attached(self) -> bool:
        return True

    @property
    def package(self):
        return self

    @property
    def absolute_path(self) -> str:
        bench = self.bench
        if bench is not None:
            if bench.main_package_id == self.id:
                return f"@{bench.slug}"
            else:
                return f"{bench.slug}:{self.slug}"
        else:
            return "<detached>:{self.slug}"

    @property
    def package_id(self):
        return self.id

    @property
    def package_ptr(self):
        return self.to_ref()

    def __content_str__(self):
        parts = [
            f"pages={len(self.pages)}",
            f"channels={len(self.channels)}",
            f"spaces={len(self.spaces)}",
            f"dependencies={len(self.dependencies)}",
        ]
        return ", ".join(parts)

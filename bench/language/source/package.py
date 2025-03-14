from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    SLUG_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    IndexIn,
    IsModal,
    IsOwnable,
    IsTemplatable,
    LocalNodeList,
    NodeType,
    PackageNode,
    StructType,
    enum_,
    node_,
    p_node_children,
    p_node_parent,
    p_regular,
)
from bench.language.core.node import IsNamed
from bench.pb2 import PackageData

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        Channel,
        Dependency,
        Flow,
        Icon,
        Page,
        Scaler,
        Space,
        Store,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PACKAGE_TYPE)
class PackageType(BuiltinEnum):
    OPEN = 1, "Open"
    CLOSED = 2, "Closed"
    PRIVATE = 3, "Private"


@node_(NodeType.PACKAGE, index=(IndexIn(columns=("bench_id", "slug"), is_unique=True),))
class Package(IsOwnable, IsTemplatable, IsModal, IsNamed, PackageNode[PackageData]):
    """A Package is a semi-isolated area of a Bench."""

    # meta
    parent: "Bench | None" = p_node_parent(4, NodeType.BENCH)
    type: PackageType = p_regular(30, require=True)
    slug: str | None = p_regular(34, constraint=SLUG_CONSTRAINT)
    icon: Optional["Icon"] = p_regular(35, require=False, array=False, struct=StructType.ICON)

    # NOTE :Architecture: maybe we should factor out main_channel/main_flow/.. from Package
    #  (into something more general that we could also use in Page or Flow or such)
    main_channel: Optional["Channel"] = p_regular(
        50,
        require=False,
        array=False,
        references=NodeType.CHANNEL,
        same_bench=True,
        description="The default Channel to communicate with.",
    )
    main_flow: Optional["Flow"] = p_regular(
        51,
        require=False,
        array=False,
        references=NodeType.FLOW,
        description="The default Flow for dynamic behavior.",
    )

    stores: LocalNodeList["Store"] = p_node_children(NodeType.STORE)
    scalers: LocalNodeList["Scaler"] = p_node_children(NodeType.SCALER)
    pages: LocalNodeList["Page"] = p_node_children(NodeType.PAGE)
    channels: LocalNodeList["Channel"] = p_node_children(NodeType.CHANNEL)
    spaces: LocalNodeList["Space"] = p_node_children(NodeType.SPACE)
    dependencies: LocalNodeList["Dependency"] = p_node_children(NodeType.DEPENDENCY)

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

    def __content_str__(self):
        parts = [
            f"pages={len(self.pages)}",
            f"channels={len(self.channels)}",
            f"spaces={len(self.spaces)}",
            f"dependencies={len(self.dependencies)}",
        ]
        return ", ".join(parts)

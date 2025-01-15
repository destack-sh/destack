from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.pb2 import (
    BenchData,
    DependencyData,
    PackageData,
)
from bench.utils.func import IdEnum, generate_encryption_key

from .const import (
    REGION,
    EnumType,
    NodeType,
    Region,
    StructType,
    enum_,
)
from .list import LocalNodeList
from .node import (
    OWNER_TYPES,
    BenchNode,
    Owner,
    SourceNode,
    node_,
)
from .property import (
    p_kernel,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
)
from .validation import NAME_CONSTRAINT, SLUG_CONSTRAINT

if TYPE_CHECKING:
    from bench.language import (
        Block,
        Handle,
        Icon,
        NodeReference,
        Organization,
        Region,
        Scaler,
        Space,
        Store,
        Text,
        User,
    )

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.BENCH, roots=())
class Bench(BenchNode[BenchData]):
    """
    A Bench is an AI-native operating system for a new generation of fully integrated, fluid software.
    """

    parent: None = p_node_parent(4)
    main_handle: Optional["Handle"] = p_system(
        31, require=False, array=False, references=NodeType.HANDLE, fk=True
    )  # not actually optional but Handle.parent = Bench
    handles: LocalNodeList["Handle"] = p_node_children(NodeType.HANDLE)
    slug: str = p_system(32, unique=True, constraint=SLUG_CONSTRAINT)  # must match main handle
    name: str = p_regular(33, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(35, default=None, struct=StructType.ICON)
    owner: Union["User", "Organization", None] = p_system(
        36, require=False, array=False, references=(NodeType.USER, NodeType.ORGANIZATION)
    )
    if TYPE_CHECKING:
        owner_id: Optional[UUID] = None
        owner_type: Optional[NodeType] = None
    region: "Region" = p_system(37, require=True, default=REGION, default_sql=None)
    encryption_key: str = p_kernel(
        38,
        require=True,
        encrypt=True,
        defer=True,
        sensitive=True,
        default_factory=lambda: generate_encryption_key(32),
    )

    # resources
    main_store: Optional["Store"] = p_system(
        40, require=False, array=False, references=NodeType.STORE, fk=True, same_bench=True
    )

    # content
    main_package: Optional["Package"] = p_regular(
        50,
        require=False,
        array=False,
        references=NodeType.PACKAGE,
        fk=True,
        same_bench=True,
    )
    if TYPE_CHECKING:
        main_package_id: Optional[UUID] = None
        main_package_ptr: Optional[NodeReference] = None

    packages: LocalNodeList["Package"] = p_node_children(NodeType.PACKAGE)
    stores: LocalNodeList["Store"] = p_node_children(NodeType.STORE)
    scalers: LocalNodeList["Scaler"] = p_node_children(NodeType.SCALER)

    @property
    def is_attached(self) -> bool:
        return True


@enum_(EnumType.PACKAGE_TYPE)
class PackageType(IdEnum):
    ROOT = 1
    SIDE = 5
    SNAPSHOT = 10


@node_(NodeType.PACKAGE, unique=(("bench_id", "slug"),))
class Package(BenchNode[PackageData]):
    """A Package is an isolated part of a Bench."""

    parent: Bench | None = p_node_parent(4, NodeType.BENCH)
    type: PackageType = p_regular(30, require=True)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    slug: str = p_regular(33, constraint=SLUG_CONSTRAINT)
    text: Optional["Text"] = p_regular(34, require=False, array=False, struct=StructType.TEXT)
    icon: Optional["Icon"] = p_regular(35, require=False, array=False, struct=StructType.ICON)
    owned_by: Optional[Owner] = p_regular(36, require=False, array=False, references=OWNER_TYPES)

    base: Optional["Package"] = p_system(
        40, require=False, array=False, references=NodeType.PACKAGE, fk=True, same_bench=True
    )

    blocks: LocalNodeList["Block"] = p_node_children(NodeType.BLOCK)
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
            f"blocks={len(self.blocks)}",
            f"spaces={len(self.spaces)}",
            f"dependencies={len(self.dependencies)}",
        ]
        return ", ".join(parts)


@node_(NodeType.DEPENDENCY)
class Dependency(SourceNode[DependencyData]):
    """
    A dependency on another Bench (pointing to a specific Package).
    If scopes are given, only those blocks are included.
    NOTE :Incomplete: Dependency doesn't work yet :Dependencies
    """

    parent: Union[Package, None] = p_node_parent(4, NodeType.PACKAGE)

    depends_on_bench: Bench = p_regular(40, require=True, array=False, references=NodeType.BENCH)
    depends_on_packages: list[Package] = p_regular(
        41, require=True, array=True, references=NodeType.PACKAGE
    )

from datetime import datetime
from itertools import chain
from typing import TYPE_CHECKING, Collection, Optional, Union, Iterable

from bench.language.const import NodeType, StructType
from bench.language.graph import NodeList
from bench.language.node import Node, node
from bench.language.property import (
    p_internal,
    p_kernel,
    p_node_child,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.language.validation import validate_name, validate_slug
from bench.utils.casing import IdentifierType

if TYPE_CHECKING:
    from bench.language import (
        Block,
        Cache,
        Drive,
        Handle,
        Icon,
        Organization,
        Policy,
        Region,
        Server,
        Space,
        Store,
        Text,
        User,
        Resource,
    )

NodeT = Union[Node, "Node"]


@node(NodeType.BENCH, roots=(), identifier=IdentifierType.VARIABLE)
class Bench(Node):
    """
    A Bench is an AI-native operating system for a new generation of fully integrated, fluid software.
    """

    parent: None = p_node_parent(4)
    main_handle: Optional["Handle"] = p_system(
        31, require=False, array=False, references=NodeType.HANDLE
    )  # not actually optional but Handle.parent = Bench
    handles: NodeList["Handle"] = p_node_child(NodeType.HANDLE)
    slug: str = p_system(32, unique=True)  # must match main handle
    name: str = p_regular(33, validate=validate_name)
    text: Optional["Text"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.TEXT
    )
    icon: Optional["Icon"] = p_regular(35, default=None, struct=StructType.ICON)
    owner: Union["User", "Organization"] = p_system(
        36, require=False, array=False, references=(NodeType.USER, NodeType.ORGANIZATION)
    )
    region: "Region" = p_regular(37, require=True, array=False)
    encryption_key: str = p_kernel(38, require=True, encrypt=True, defer=True, sensitive=True)
    policies: list["Policy"] | None = p_regular(39, struct=StructType.POLICY, array=True)

    # source
    main_environment: Optional["Environment"] = p_regular(
        40, require=False, array=False, references=NodeType.ENVIRONMENT
    )
    main_branch: Optional["Branch"] = p_regular(
        42, require=False, array=False, references=NodeType.BRANCH
    )
    published_branch: Optional["Branch"] = p_regular(
        43, require=False, array=False, references=NodeType.BRANCH
    )
    packages: NodeList["Package"] = p_node_child(NodeType.PACKAGE)
    environments: NodeList["Environment"] = p_node_child(NodeType.ENVIRONMENT)
    branches: NodeList["Branch"] = p_node_child(NodeType.BRANCH)

    # resources
    servers: NodeList["Server"] = p_node_child(NodeType.SERVER)
    stores: NodeList["Store"] = p_node_child(NodeType.STORE)
    drives: NodeList["Drive"] = p_node_child(NodeType.DRIVE)
    caches: NodeList["Cache"] = p_node_child(NodeType.CACHE)

    @property
    def resources(self) -> Iterable["Resource"]:
        return chain(self.servers, self.stores, self.drives, self.caches)


@node(NodeType.ENVIRONMENT, identifier=IdentifierType.VARIABLE)
class Environment(Node):
    """An environment of resources for a Bench's packages."""

    parent: Bench = p_node_parent(4, NodeType.BENCH)
    name: Optional[str] = p_regular(32, validate=validate_name)
    text: Optional["Text"] = p_regular(34, require=False, array=False, struct=StructType.TEXT)
    icon: Optional["Icon"] = p_regular(35, require=False, array=False, struct=StructType.ICON)
    policies: list["Policy"] = p_regular(36, struct=StructType.POLICY, array=True)

    server: "Server" = p_system(40, require=True, array=False, references=NodeType.SERVER)
    store: "Store" = p_system(41, require=True, array=False, references=NodeType.STORE)
    search: "Store" = p_system(42, require=True, array=False, references=NodeType.STORE)
    analytics: Optional["Store"] = p_system(
        43, require=False, default=None, array=False, references=NodeType.STORE
    )
    drive: "Drive" = p_system(44, require=True, array=False, references=NodeType.DRIVE)
    cache: Optional["Cache"] = p_system(
        45, require=False, default=None, array=False, references=NodeType.CACHE
    )


@node(
    NodeType.BRANCH,
    identifier=IdentifierType.VARIABLE,
    unique_together=(("parent_bench_id", "slug"),),
)
class Branch(Node):
    """A branch is a Git-like pointer to the head of a lineage of packages."""

    parent: Bench = p_node_parent(4, NodeType.BENCH)
    name: Optional[str] = p_regular(32, validate=validate_name)
    slug: Optional[str] = p_regular(33, require=False, default=None, validate=validate_slug)
    text: Optional["Text"] = p_regular(34, require=False, array=False, struct=StructType.TEXT)
    icon: Optional["Icon"] = p_regular(35, require=False, array=False, struct=StructType.ICON)
    policies: list["Policy"] | None = p_regular(36, struct=StructType.POLICY, array=True)

    main_package: Optional["Package"] = p_system(
        40, require=False, array=False, references=NodeType.PACKAGE
    )


@node(
    NodeType.PACKAGE,
    identifier=IdentifierType.VARIABLE,
    unique_together=(("parent_bench_id", "slug"),),
)
class Package(Node):
    """A package is a semi-isolated version of a Bench."""

    parent: Bench = p_node_parent(4, NodeType.BENCH)
    slug: Optional[str] = p_regular(33, require=False, default=None, validate=validate_slug)
    text: Optional["Text"] = p_regular(34, require=False, array=False, struct=StructType.TEXT)
    icon: Optional["Icon"] = p_regular(35, require=False, array=False, struct=StructType.ICON)
    policies: list["Policy"] | None = p_regular(36, struct=StructType.POLICY, array=True)
    paused_at: datetime | None = p_internal(37, default=None)  # all activity is paused

    environment: Environment = p_system(
        40, require=True, array=False, references=NodeType.ENVIRONMENT
    )
    bases: list["Package"] = p_system(42, require=False, array=True, references=NodeType.PACKAGE)

    blocks: NodeList["Block"] = p_node_child(NodeType.BLOCK)
    spaces: NodeList["Space"] = p_node_child(NodeType.SPACE)
    dependencies: NodeList["Dependency"] = p_node_child(NodeType.DEPENDENCY)

    @property
    def name(self):
        return self.parent.name if self.parent is not None else None

    @property
    def is_paused(self) -> bool:
        return self.paused_at is not None

    @is_paused.setter
    def is_paused(self, value: bool) -> None:
        self.paused_at = datetime.utcnow() if value else None

    @property
    def _nodes(self) -> Collection[Node]:
        return self.root.nodes_by_ck.values()

    def __content_str__(self):
        return f"blocks={len(self.blocks)}, spaces={len(self.spaces)}"


@node(NodeType.DEPENDENCY, identifier=IdentifierType.VARIABLE)
class Dependency(Node):
    """
    A dependency on another Bench (pointing to a specific Package).
    If scopes are given, only those blocks are included.
    """

    # dependent
    parent: Union[Package, "Block"] = p_node_parent(4, NodeType.PACKAGE, NodeType.BLOCK)
    scopes: list["Block"] = p_regular(30, require=True, array=True, references=NodeType.BLOCK)

    # dependency
    dependency: Package = p_regular(40, require=True, array=False, references=NodeType.PACKAGE)
    dependency_scopes: list["Block"] = p_regular(
        41, require=True, array=True, references=NodeType.BLOCK
    )


@node(NodeType.UPGRADE, identifier=IdentifierType.VARIABLE)
class Upgrade(Node):
    """An 'upgrade' to a Package, marking changes made to the containing Package."""

    parent: Package = p_node_parent(4, NodeType.PACKAGE)
    name: Optional[str] = p_regular(32, validate=validate_name)
    title: Optional[str] = p_regular(34)
    text: Optional["Text"] = p_regular(35, require=False, array=False, struct=StructType.TEXT)

from typing import TYPE_CHECKING, Collection, Optional, Union

from bench.language.node import Node
from bench.language.const import NodeType, StructType
from bench.language.graph import NodeDataGraph, NodeList
from bench.language.node import node, p_child, p_kernel, p_parent, p_regular, p_runtime, p_system
from bench.utils.casing import IdentifierType

if TYPE_CHECKING:
    from bench.language import (
        Block,
        Cache,
        Drive,
        Environment,
        Handle,
        Organization,
        Package,
        Policy,
        RichText,
        Server,
        Space,
        Store,
        User,
    )

NodeT = Union[Node, "Node"]


@node(NodeType.BENCH, roots=(), identifier=IdentifierType.VARIABLE)
class Bench(Node):
    """
    A Bench is an AI-native operating system for a new generation of fully integrated, fluid software.
    """

    parent: None = p_parent(4)
    main_handle: Optional["Handle"] = p_system(
        31, require=False, array=False, references=NodeType.HANDLE
    )  # not actually optional but Handle.parent = Bench
    handles: NodeList["Handle"] = p_child(NodeType.HANDLE)
    slug: str = p_system(32, unique=True)  # must match main handle
    name: str = p_regular(33)
    text: Optional["RichText"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.RICH_TEXT
    )
    owner: Union["User", "Organization"] = p_system(
        35, require=False, array=False, references=(NodeType.USER, NodeType.ORGANIZATION)
    )
    encryption_key: str = p_kernel(36, require=True, encrypt=True, defer=True)
    policies: list["Policy"] | None = p_regular(
        37, default_factory=list, struct=StructType.POLICY, array=True
    )

    # source
    main_package: Optional["Package"] = p_system(
        40, require=False, array=False, references=NodeType.PACKAGE
    )
    packages: NodeList["Package"] = p_child(NodeType.PACKAGE)
    main_environment: Optional["Environment"] = p_system(
        41, require=False, array=False, references=NodeType.ENVIRONMENT
    )
    environments: NodeList["Environment"] = p_child(NodeType.ENVIRONMENT)
    main_branch: Optional["Branch"] = p_system(
        42, require=False, array=False, references=NodeType.BRANCH
    )
    branches: NodeList["Branch"] = p_child(NodeType.BRANCH)

    # resources
    servers: NodeList["Server"] = p_child(NodeType.SERVER)
    stores: NodeList["Store"] = p_child(NodeType.STORE)
    drives: NodeList["Drive"] = p_child(NodeType.DRIVE)
    caches: NodeList["Cache"] = p_child(NodeType.CACHE)


@node(NodeType.ENVIRONMENT, identifier=IdentifierType.VARIABLE)
class Environment(Node):
    """An environment isolates resources from the rest of a Bench."""

    parent: Bench = p_parent(4, NodeType.BENCH)
    name: Optional[str] = p_regular(32)
    text: Optional["RichText"] = p_regular(
        34, require=False, array=False, struct=StructType.RICH_TEXT
    )
    policies: list["Policy"] | None = p_regular(
        35, default_factory=list, struct=StructType.POLICY, array=True
    )

    store: "Store" = p_system(40, require=True, array=False, references=NodeType.STORE)
    search: "Store" = p_system(41, require=True, array=False, references=NodeType.STORE)
    analytics: Optional["Store"] = p_system(
        42, require=True, array=False, references=NodeType.STORE
    )
    drive: "Drive" = p_system(43, require=True, array=False, references=NodeType.DRIVE)
    cache: Optional["Cache"] = p_system(44, require=True, array=False, references=NodeType.CACHE)


@node(NodeType.BRANCH, identifier=IdentifierType.VARIABLE)
class Branch(Node):
    """A branch is a Git-like pointer to the head of a lineage of packages."""

    parent: Bench = p_parent(4, NodeType.BENCH)
    name: Optional[str] = p_regular(32)
    text: Optional["RichText"] = p_regular(
        34, require=False, array=False, struct=StructType.RICH_TEXT
    )
    main_package: Optional["Package"] = p_system(
        35, require=False, array=False, references=NodeType.PACKAGE
    )
    policies: list["Policy"] | None = p_regular(
        36, default_factory=list, struct=StructType.POLICY, array=True
    )


@node(
    NodeType.PACKAGE,
    identifier=IdentifierType.VARIABLE,
    unique_together=(("parent_bench_id", "slug"),),
)
class Package(Node):
    """A package is a semi-isolated version of a Bench containing all the source and data."""

    parent: Bench = p_parent(4, NodeType.BENCH)
    slug: Optional[str] = p_regular(33)
    text: Optional["RichText"] = p_regular(
        34, require=False, array=False, struct=StructType.RICH_TEXT
    )
    policies: list["Policy"] | None = p_regular(
        35, default_factory=list, struct=StructType.POLICY, array=True
    )
    is_paused: bool = p_system(36, default=False)
    is_partial: bool = p_system(37, default=False)
    base: Optional["Package"] = p_system(
        38, require=False, array=False, references=NodeType.PACKAGE
    )
    environment: Environment = p_system(
        39, require=True, array=False, references=NodeType.ENVIRONMENT
    )
    branch: Branch = p_system(40, require=True, array=False, references=NodeType.BRANCH)

    blocks: NodeList["Block"] = p_child(NodeType.BLOCK)
    spaces: NodeList["Space"] = p_child(NodeType.SPACE)
    dependencies: NodeList["Dependency"] = p_child(NodeType.DEPENDENCY)

    _source: Optional[NodeDataGraph] = p_runtime(default=None)

    @property
    def name(self):
        return self.parent.name if self.parent is not None else None

    @property
    def os_name(self) -> str:
        return self.environment.index.handle

    @property
    def _nodes(self) -> Collection[Node]:
        return self.root.nodes_by_ck.values()

    def __content_str__(self):
        return f"is_active={self.is_active}, blocks={len(self.blocks)}, spaces={len(self.spaces)}"


@node(NodeType.DEPENDENCY, identifier=IdentifierType.VARIABLE)
class Dependency(Node):
    """
    A dependency on another Bench (pointing to a specific Package).
    If scopes are given, only those blocks (and their
    """

    parent: Package = p_parent(4, NodeType.PACKAGE)

    dependency: Package = p_regular(30, require=True, array=False, references=NodeType.PACKAGE)
    scopes: list["Block"] = p_regular(31, require=True, array=True, references=NodeType.BLOCK)


@node(NodeType.UPGRADE, identifier=IdentifierType.VARIABLE)
class Upgrade(Node):
    """An 'upgrade' to a Package, marking changes made to the containing Package."""

    parent: Package = p_parent(4, NodeType.PACKAGE)

    name: Optional[str] = p_regular(32)
    text: Optional["RichText"] = p_regular(
        34, require=False, array=False, struct=StructType.RICH_TEXT
    )

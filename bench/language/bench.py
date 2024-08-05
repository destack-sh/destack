import abc
from datetime import datetime
from enum import Enum
from itertools import chain
from typing import TYPE_CHECKING, Generic, Iterable, Optional, TypeVar, Union
from uuid import UUID

from bench.language.const import (
    REGION,
    ClientType,
    EnumType,
    NodeType,
    ReferenceKind,
    Region,
    StructType,
    enum_,
)
from bench.language.graph import NodeList
from bench.language.node import (
    BenchNode,
    ClientOrigin,
    SourceNode,
    local_node_,
    node_,
    node_component,
)
from bench.language.property import (
    p_internal,
    p_kernel,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.language.validation import NAME_CONSTRAINT, SLUG_CONSTRAINT
from bench.proto.wire import (
    AnyNodeData,
    BenchData,
    BranchData,
    ClientData,
    DependencyData,
    DriveData,
    MachineData,
    PackageData,
    ServerData,
    StoreData,
)
from bench.utils.func import IdEnum, bittuple, generate_encryption_key

if TYPE_CHECKING:
    from bench.language import (
        Block,
        Drive,
        Handle,
        Icon,
        NodeReference,
        Organization,
        Policy,
        Region,
        ResourceNode,
        Server,
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
    handles: NodeList["Handle"] = p_node_children(NodeType.HANDLE)
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
    policies: list["Policy"] = p_regular(39, struct=StructType.POLICY, array=True)

    # resources
    main_store: Optional["Store"] = p_system(
        40, require=False, array=False, references=NodeType.STORE, fk=True, same_bench=True
    )
    main_server: Optional["Server"] = p_system(
        41, require=False, array=False, references=NodeType.SERVER, fk=True, same_bench=True
    )
    main_drive: Optional["Drive"] = p_system(
        42, require=False, array=False, references=NodeType.DRIVE, fk=True, same_bench=True
    )
    stores: NodeList["Store"] = p_node_children(NodeType.STORE)
    servers: NodeList["Server"] = p_node_children(NodeType.SERVER)
    drives: NodeList["Drive"] = p_node_children(NodeType.DRIVE)

    # source
    main_branch: Optional["Branch"] = p_regular(
        50,
        require=False,
        array=False,
        references=NodeType.BRANCH,
        fk=True,
        same_bench=True,
    )
    branches: NodeList["Branch"] = p_node_children(NodeType.BRANCH)

    @property
    def _is_attached(self) -> bool:
        return True

    @property
    def main_package(self) -> "Package":
        main_branch = self.main_branch
        assert main_branch is not None, f"{self!r} has no main branch"
        main_package = main_branch.main_package
        assert main_package is not None, f"{self!r} has no main package"
        return main_package

    @property
    def resources(self) -> Iterable["ResourceNode"]:
        return chain(
            self.servers,
            chain.from_iterable(server.machines for server in self.servers),
            self.stores,
            self.drives,
        )


@local_node_(NodeType.BRANCH, unique=(("bench_id", "slug"),))
class Branch(BenchNode[BranchData]):
    """A branch is a lineage of Bench history."""

    parent: Bench | None = p_node_parent(4, NodeType.BENCH)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    slug: Optional[str] = p_regular(33, require=False, default=None, constraint=SLUG_CONSTRAINT)
    text: Optional["Text"] = p_regular(34, require=False, array=False, struct=StructType.TEXT)
    icon: Optional["Icon"] = p_regular(35, require=False, array=False, struct=StructType.ICON)
    policies: list["Policy"] = p_regular(36, struct=StructType.POLICY, array=True)

    main_package: Optional["Package"] = p_system(
        40, require=False, array=False, references=NodeType.PACKAGE, fk=True, same_bench=True
    )
    if TYPE_CHECKING:
        main_package_id: Optional[UUID] = None
        main_package_ptr: Optional[NodeReference] = None
    base: Optional["Branch"] = p_system(
        41, require=False, array=False, references=NodeType.BRANCH, fk=True, same_bench=True
    )

    # flags
    is_overlay: bool = p_system(60, default=False)
    is_light: bool = p_system(61, default=False)

    packages: NodeList["Package"] = p_node_children(NodeType.PACKAGE)


@local_node_(NodeType.PACKAGE)
class Package(BenchNode[PackageData]):
    """A package is a version of a Bench in a Branch."""

    parent: Branch | None = p_node_parent(4, NodeType.BRANCH)
    text: Optional["Text"] = p_regular(34, require=False, array=False, struct=StructType.TEXT)
    icon: Optional["Icon"] = p_regular(35, require=False, array=False, struct=StructType.ICON)
    policies: list["Policy"] = p_regular(36, struct=StructType.POLICY, array=True)

    base: Optional["Package"] = p_system(
        40, require=False, array=False, references=NodeType.PACKAGE, fk=True, same_bench=True
    )

    # flags
    is_snapshot: bool = p_system(60, default=False)
    is_overlay: bool = p_system(61, default=False)
    is_paused: bool = p_system(65, default=False)

    blocks: NodeList["Block"] = p_node_children(NodeType.BLOCK)
    spaces: NodeList["Space"] = p_node_children(NodeType.SPACE)
    dependencies: NodeList["Dependency"] = p_node_children(NodeType.DEPENDENCY)

    @property
    def _is_attached(self) -> bool:
        return True

    @property
    def package(self):
        return self

    @property
    def absolute_path(self) -> str:
        bench = self.bench
        if bench is not None:
            return f"@{bench.slug}"
        else:
            return "<detached>"

    @property
    def package_id(self):
        return self.id

    @property
    def package_ptr(self):
        return self.to_ref()

    @property
    def name(self):
        bench = self.bench
        return bench.name if bench is not None else None

    @property
    def slug(self):
        bench = self.bench
        return bench.slug if bench is not None else None

    def __content_str__(self):
        parts = [
            f"blocks={len(self.blocks)}",
            f"spaces={len(self.spaces)}",
            f"dependencies={len(self.dependencies)}",
        ]
        for flag in ("is_paused",):
            if getattr(self, flag):
                parts.append(flag)
        return ", ".join(parts)


@local_node_(NodeType.DEPENDENCY)
class Dependency(SourceNode[DependencyData]):
    """
    A dependency on another Bench (pointing to a specific Package).
    If scopes are given, only those blocks are included.
    """

    # dependent
    parent: Union[Package, "Block", None] = p_node_parent(4, NodeType.PACKAGE, NodeType.BLOCK)
    scopes: list["Block"] = p_regular(30, require=True, array=True, references=NodeType.BLOCK)

    # dependency
    dependency: Package = p_regular(40, require=True, array=False, references=NodeType.PACKAGE)


@enum_(EnumType.RESOURCE_STATUS)
class ResourceStatus(IdEnum):
    """Generalized status of a Resource in its lifecycle."""

    # preparing
    DECLARED = 1
    PROVISIONING = 5
    # extant
    HEALTHY = 10
    UNHEALTHY = 15
    SLEEPING = 20
    # terminal
    DECOMMISSIONED = 30

    @property
    def is_extant(self) -> bool:
        return 10 <= self.value <= 20


EXTANT_RESOURCE_STATUSES = bittuple(*(s for s in ResourceStatus if 10 <= s.value <= 20))

NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)


@node_component()
class ResourceNode(BenchNode[NodeDataT], abc.ABC, Generic[NodeDataT]):
    """
    An external resource in a Bench.
    Resources generally work on the 'desired state' principle.
    The real 'current' state is stored in current_* properties.
    """

    parent: Bench | None = p_node_parent(4, NodeType.BENCH, is_system=True)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(34, default=None, struct=StructType.TEXT)
    region: Region = p_system(35, default=REGION, default_sql=None)
    status: ResourceStatus = p_system(36, default=ResourceStatus.DECLARED)
    current_status: Optional[ResourceStatus] = p_system(37, default=None)

    def __content_str__(self):
        value_strs: list[str] = []
        for prop in chain(
            ResourceNode.__declared_properties__.values(),
            self.__declared_properties__.values(),
        ):
            if (
                prop.name == "name"
                or prop.name == "parent"
                or prop.reference_kind == ReferenceKind.NODE_CHILDREN
                or prop.is_sensitive
            ):
                continue
            value = getattr(self, prop.name)
            if value:
                if isinstance(value, Enum):
                    value = value.name
                value_strs.append(f"{prop.name}={value}")
        return ", ".join(value_strs)


@node_(NodeType.SERVER)
class Server(ResourceNode[ServerData]):
    """
    A server provides some compute for a Bench's Runtime.
    Physical compute is materialized (on-demand) as Machines.
    """

    version: Optional[str] = p_system(40, default=None)
    current_version: Optional[str] = p_system(41, default=None)

    min_cpu: Optional[float] = p_system(50, default=None, description="vCPU count")
    max_cpu: Optional[float] = p_system(51, default=None, description="vCPU count")
    min_ram: Optional[float] = p_system(52, default=None, description="GB")
    max_ram: Optional[float] = p_system(53, default=None, description="GB")

    active_at: Optional[datetime] = p_internal(60, default=None)
    bumped_at: Optional[datetime] = p_internal(61, default=None)

    clients: NodeList["Client"] = p_node_children(NodeType.CLIENT)
    machines: NodeList["Machine"] = p_node_children(NodeType.MACHINE)


@node_(NodeType.MACHINE)
class Machine(ResourceNode[MachineData]):
    """
    A Machine provides some isolated compute, usually for a Server.
    A Machine may also be manually provisioned with specific image/profiles.
    """

    parent: Server | Bench | None = p_node_parent(4, NodeType.SERVER, NodeType.BENCH)

    version: Optional[str] = p_system(40, default=None)
    current_version: Optional[str] = p_system(41, default=None)
    external_name: Optional[str] = p_kernel(42, require=False, default=None, sensitive=True)
    external_id: Optional[str] = p_kernel(43, require=False, default=None, sensitive=True)
    connection_uri: Optional[str] = p_kernel(
        44, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )

    cpu: float = p_system(50, description="vCPU count")
    current_cpu: Optional[float] = p_system(51, default=None, description="vCPU count")
    ram: float = p_system(52, description="GB")
    current_ram: Optional[float] = p_system(53, default=None, description="GB")

    started_at: Optional[datetime] = p_internal(60, default=None)
    terminated_at: Optional[datetime] = p_internal(61, default=None)
    active_at: Optional[datetime] = p_internal(62, default=None)


@node_(NodeType.STORE)
class Store(ResourceNode[StoreData]):
    """A trusty Postgres-compatible database."""

    version: Optional[str] = p_system(40, default=None)
    current_version: Optional[str] = p_system(41, default=None)

    external_name: Optional[str] = p_kernel(50, require=False, default=None, sensitive=True)
    external_id: Optional[str] = p_kernel(51, require=False, default=None, sensitive=True)
    connection_uri: Optional[str] = p_kernel(
        52, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )


@node_(NodeType.DRIVE)
class Drive(ResourceNode[DriveData]):
    """Drive for file storage."""

    ...


@node_(NodeType.CLIENT, roots=(NodeType.USER, NodeType.BENCH))
class Client(BenchNode[ClientData]):
    """A client to a Bench."""

    parent: Union["User", "Server", None] = p_node_parent(4, NodeType.USER, NodeType.SERVER)
    type: ClientType = p_regular(30)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)

    device_type: Optional[str] = p_regular(40, default=None)
    device_name: Optional[str] = p_regular(41, default=None)
    operating_system: Optional[str] = p_regular(42, default=None)
    browser_name: Optional[str] = p_regular(43, default=None)
    browser_version: Optional[str] = p_regular(44, default=None)
    place_id: Optional[str] = p_regular(45, default=None)

    access_token: Optional[str] = p_kernel(
        50, default=None, defer=True, unique=True, sensitive=True
    )
    seen_at: datetime = p_system(51)
    logged_in_at: Optional[datetime] = p_system(52, default=None)

    space: Optional["Space"] = p_system(
        60, array=False, require=False, references=NodeType.SPACE, fk=True
    )
    machine: Optional[Machine] = p_system(
        61, array=False, require=False, references=NodeType.MACHINE, fk=True
    )

    @property
    def _is_attached(self) -> bool:
        parent = self.parent
        if parent is None:
            return False
        return parent._is_attached

    def __content_str__(self) -> str:
        value_parts = []
        for prop in (
            "device_type",
            "device_name",
            "operating_system",
            "browser_name",
            "browser_version",
        ):
            value = getattr(self, prop)
            if value is not None:
                value_parts.append(value)
        return ", ".join(value_parts)

    def to_origin(self, *, nonce: UUID | None) -> ClientOrigin:
        return ClientOrigin(type=self.type, id=self.id, nonce=nonce or self.id)

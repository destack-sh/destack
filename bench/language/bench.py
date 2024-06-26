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
    get_region,
)
from bench.language.graph import NodeList
from bench.language.node import BenchNode, ClientOrigin, Node, SourceNode, node_, node_component
from bench.language.property import (
    p_internal,
    p_kernel,
    p_node_children,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.language.validation import NAME_CONSTRAINT, SLUG_CONSTRAINT, TITLE_CONSTRAINT
from bench.proto.wire import (
    AnyNodeData,
    BenchData,
    BranchData,
    ClientData,
    DependencyData,
    DriveData,
    EnvironmentData,
    MachineData,
    PackageData,
    ServerData,
    StoreData,
    UpgradeData,
)
from bench.utils.casing import IdentifierType
from bench.utils.func import IdEnum, bittuple, generate_encryption_key

if TYPE_CHECKING:
    from bench.language import (
        BenchResourceNode,
        Block,
        Drive,
        Handle,
        Icon,
        NodeReference,
        Organization,
        Policy,
        Region,
        Server,
        Space,
        Store,
        Text,
        User,
    )

NodeT = Union[Node, "Node"]
# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.BENCH, roots=(), identifier=IdentifierType.VARIABLE)
class Bench(BenchNode[BenchData]):
    """
    A Bench is an AI-native operating system for a new generation of fully integrated, fluid software.
    """

    parent: None = p_node_parent(4)
    main_handle: Optional["Handle"] = p_system(
        31, require=False, array=False, references=NodeType.HANDLE, fk=True
    )  # not actually optional but Handle.parent = Bench
    handles: NodeList["Handle"] = p_node_children(NodeType.HANDLE)
    slug: str = p_system(32, unique=True)  # must match main handle
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
    region: "Region" = p_regular(37, require=True, default=REGION, default_sql=None)
    encryption_key: str = p_kernel(
        38,
        require=True,
        encrypt=True,
        defer=True,
        sensitive=True,
        default_factory=lambda: generate_encryption_key(32),
    )
    policies: list["Policy"] = p_regular(39, struct=StructType.POLICY, array=True)

    # source
    main_environment: Optional["Environment"] = p_regular(
        40,
        require=False,
        array=False,
        references=NodeType.ENVIRONMENT,
        fk=True,
        same_bench=True,
    )
    main_branch: Optional["Branch"] = p_regular(
        41,
        require=False,
        array=False,
        references=NodeType.BRANCH,
        fk=True,
        same_bench=True,
    )
    environments: NodeList["Environment"] = p_node_children(NodeType.ENVIRONMENT)
    branches: NodeList["Branch"] = p_node_children(NodeType.BRANCH)

    # resources
    servers: NodeList["Server"] = p_node_children(NodeType.SERVER)
    stores: NodeList["Store"] = p_node_children(NodeType.STORE)
    drives: NodeList["Drive"] = p_node_children(NodeType.DRIVE)

    @property
    def _is_attached(self) -> bool:
        return True

    @property
    def resources(self) -> Iterable["BenchResourceNode"]:
        return chain(
            self.servers,
            chain.from_iterable(server.machines for server in self.servers),
            self.stores,
            self.drives,
        )


@node_(NodeType.ENVIRONMENT, identifier=IdentifierType.VARIABLE)
class Environment(BenchNode[EnvironmentData]):
    """An environment of resources for a Bench's packages."""

    parent: Bench | None = p_node_parent(4, NodeType.BENCH)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(34, require=False, array=False, struct=StructType.TEXT)
    icon: Optional["Icon"] = p_regular(35, require=False, array=False, struct=StructType.ICON)
    policies: list["Policy"] = p_regular(36, struct=StructType.POLICY, array=True)

    server: "Server" = p_system(
        40, require=True, array=False, references=NodeType.SERVER, fk=True, same_bench=True
    )
    store: "Store" = p_system(
        41, require=True, array=False, references=NodeType.STORE, fk=True, same_bench=True
    )
    drive: "Drive" = p_system(
        42, require=True, array=False, references=NodeType.DRIVE, fk=True, same_bench=True
    )


@node_(
    NodeType.BRANCH,
    identifier=IdentifierType.VARIABLE,
    unique=(("bench_id", "slug"),),
)
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


@node_(
    NodeType.PACKAGE,
    identifier=IdentifierType.VARIABLE,
    unique=(("bench_id", "slug"),),
)
class Package(BenchNode[PackageData]):
    """A package is a version of a Bench in a Branch."""

    parent: Branch | None = p_node_parent(4, NodeType.BRANCH)
    slug: Optional[str] = p_regular(33, require=False, default=None, constraint=SLUG_CONSTRAINT)
    text: Optional["Text"] = p_regular(34, require=False, array=False, struct=StructType.TEXT)
    icon: Optional["Icon"] = p_regular(35, require=False, array=False, struct=StructType.ICON)
    policies: list["Policy"] = p_regular(36, struct=StructType.POLICY, array=True)

    environment: Environment = p_system(
        40,
        require=True,
        array=False,
        references=NodeType.ENVIRONMENT,
        fk=True,
        same_bench=True,
    )
    base: Optional["Package"] = p_system(
        41, require=False, array=False, references=NodeType.PACKAGE, fk=True, same_bench=True
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
    def package_id(self):
        return self.id

    @property
    def package_ptr(self):
        return self.to_ref()

    @property
    def name(self):
        bench = self.bench
        return bench.name if bench is not None else None

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


@node_(NodeType.DEPENDENCY, identifier=IdentifierType.VARIABLE)
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
    dependency_scopes: list["Block"] = p_regular(
        41, require=True, array=True, references=NodeType.BLOCK
    )


@node_(NodeType.UPGRADE, identifier=IdentifierType.VARIABLE)
class Upgrade(SourceNode[UpgradeData]):
    """An 'upgrade' to a Package, marking changes made to the containing Package."""

    parent: Package | None = p_node_parent(4, NodeType.PACKAGE)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    title: Optional[str] = p_regular(34, constraint=TITLE_CONSTRAINT)
    text: Optional["Text"] = p_regular(35, require=False, array=False, struct=StructType.TEXT)


@enum_(EnumType.TENANCY)
class Tenancy(IdEnum):
    """How a Resource is shared (if at all)."""

    SHARED = 3
    DEDICATED = 7


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
class BenchResourceNode(BenchNode[NodeDataT], abc.ABC, Generic[NodeDataT]):
    """
    An external resource in a Bench.
    Resources generally work on the 'desired state' principle (except for some system-only properties).
    If different, the real 'current' state is stored in current_* properties.
    """

    parent: Bench | None = p_node_parent(4, NodeType.BENCH, is_system=True)
    name: str = p_regular(32)
    text: Optional["Text"] = p_regular(34, default=None, struct=StructType.TEXT)
    region: Region = p_system(35, default_factory=get_region, default_sql=None)
    status: ResourceStatus = p_system(36, default=ResourceStatus.DECLARED)
    current_status: Optional[ResourceStatus] = p_system(37, default=None)

    def __content_str__(self):
        value_strs: list[str] = []
        for prop in chain(
            BenchResourceNode.__declared_properties__.values(),
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


# NOTE: ServerProfile/MachineProfile will be overhauled


@enum_(EnumType.SERVER_PROFILE)
class ServerProfile(IdEnum):
    TINY = 3
    SMALL = 5
    MEDIUM = 7


@enum_(EnumType.MACHINE_PROFILE)
class MachineProfile(IdEnum):
    TINY = 3
    SMALL = 5
    MEDIUM = 7


@node_(NodeType.SERVER)
class Server(BenchResourceNode[ServerData]):
    """
    A server provides some compute for a Bench's Runtime.
    Physical compute is materialized (on-demand) as Machines.
    """

    profile: ServerProfile = p_regular(40)
    current_profile: Optional[ServerProfile] = p_system(41, default=None)
    version: Optional[str] = p_system(42, default=None)
    current_version: Optional[str] = p_system(43, default=None)

    active_at: Optional[datetime] = p_internal(60, default=None)
    bumped_at: Optional[datetime] = p_internal(61, default=None)

    clients: NodeList["Client"] = p_node_children(NodeType.CLIENT)
    machines: NodeList["Machine"] = p_node_children(NodeType.MACHINE)


@node_(NodeType.MACHINE)
class Machine(BenchResourceNode[MachineData]):
    """
    A Machine provides some isolated compute for a Server.
    """

    parent: Server | None = p_node_parent(4, NodeType.SERVER)

    profile: MachineProfile = p_system(40)
    current_profile: Optional[MachineProfile] = p_system(41, default=None)
    version: Optional[str] = p_system(42, default=None)
    current_version: Optional[str] = p_system(43, default=None)

    external_name: Optional[str] = p_kernel(50, require=False, default=None, sensitive=True)
    external_id: Optional[str] = p_kernel(51, require=False, default=None, sensitive=True)
    connection_uri: Optional[str] = p_kernel(
        52, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )

    started_at: Optional[datetime] = p_internal(60, default=None)
    terminated_at: Optional[datetime] = p_internal(61, default=None)
    active_at: Optional[datetime] = p_internal(62, default=None)


@node_(NodeType.STORE)
class Store(BenchResourceNode[StoreData]):
    """A trusty Postgres database."""

    version: Optional[str] = p_system(40, default=None)
    current_version: Optional[str] = p_system(41, default=None)

    external_name: Optional[str] = p_kernel(50, require=False, default=None, sensitive=True)
    external_id: Optional[str] = p_kernel(51, require=False, default=None, sensitive=True)
    connection_uri: Optional[str] = p_kernel(
        52, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )


@node_(NodeType.DRIVE)
class Drive(BenchResourceNode[DriveData]):
    """Drive for file storage."""

    ...


@node_(NodeType.CLIENT, roots=(NodeType.USER, NodeType.BENCH), identifier=IdentifierType.VARIABLE)
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

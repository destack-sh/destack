import abc
from datetime import datetime
from enum import Enum
from itertools import chain
from typing import TYPE_CHECKING, Any, Optional, Self, TypeVar, Union
from uuid import UUID

from bench.language.const import (
    REGION,
    VERSION,
    ClientType,
    EnumType,
    NodeType,
    ReferenceKind,
    Region,
    StructType,
    active_session,
    enum_,
)
from bench.language.list import LocalNodeList
from bench.language.node import (
    OWNER_TYPES,
    BenchNode,
    ClientOrigin,
    HasTracingContext,
    Owner,
    SourceNode,
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
from bench.language.validation import (
    NAME_CONSTRAINT,
    SLUG_CONSTRAINT,
    TITLE_CONSTRAINT,
    constraint,
)
from bench.proto.wire import (
    AnyNodeData,
    BenchData,
    ClientData,
    DependencyData,
    DriveData,
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
        Machine,
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
    policies: list["Policy"] = p_regular(39, struct=StructType.POLICY, array=True)

    # (virtual) resources
    main_store: Optional["Store"] = p_system(
        40, require=False, array=False, references=NodeType.STORE, fk=True, same_bench=True
    )
    main_server: Optional["Server"] = p_system(
        41, require=False, array=False, references=NodeType.SERVER, fk=True, same_bench=True
    )
    main_drive: Optional["Drive"] = p_system(
        42, require=False, array=False, references=NodeType.DRIVE, fk=True, same_bench=True
    )
    main_vault: Optional["Vault"] = p_system(
        43, require=False, array=False, references=NodeType.VAULT, fk=True, same_bench=True
    )
    main_cache: Optional["Cache"] = p_system(
        44, require=False, array=False, references=NodeType.CACHE, fk=True, same_bench=True
    )
    stores: LocalNodeList["Store"] = p_node_children(NodeType.STORE)
    servers: LocalNodeList["Server"] = p_node_children(NodeType.SERVER)
    drives: LocalNodeList["Drive"] = p_node_children(NodeType.DRIVE)
    vaults: LocalNodeList["Vault"] = p_node_children(NodeType.VAULT)
    caches: LocalNodeList["Cache"] = p_node_children(NodeType.CACHE)

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


@enum_(EnumType.RESOURCE_STATUS)
class ResourceStatus(IdEnum):
    """Generalized status of a Resource in its lifecycle."""

    # definition
    DECLARED = 1
    # extant
    UP = 10
    SLEEPING = 11
    DOWN = 15
    DEGRADED = 16
    # terminal
    DECOMMISSIONED = 30

    @property
    def is_extant(self) -> bool:
        """Whether this resouce does/should exist."""
        return 10 <= self.value <= 20


@enum_(EnumType.RESOURCE_OCCUPANCY)
class ResourceOccupancy(IdEnum):
    """The occupancy of a resource (i.e. whether/how it's being used)."""

    AVAILABLE = 1
    RESERVED = 2
    OCCUPIED = 3
    DIRTY = 20


EXTANT_RESOURCE_STATUSES = bittuple(*(s for s in ResourceStatus if 10 <= s.value <= 20))

NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)


@node_component()
class Resource[NodeDataT: AnyNodeData](BenchNode[NodeDataT], HasTracingContext, abc.ABC):
    """
    A Resource in a Bench.
    Resources generally work on the 'desired state' principle.
    Where applicable, the target state is stored in target_* properties.
    """

    parent: Bench | None = p_node_parent(4, NodeType.BENCH, is_system=True)
    # ... space for type/title/...
    status: ResourceStatus = p_system(33, default=ResourceStatus.DECLARED, default_sql=None)
    text: Optional["Text"] = p_regular(34, default=None, struct=StructType.TEXT)
    region: Region = p_system(35, default=REGION, default_sql=None)

    # target status
    activated_at: Optional[datetime] = p_internal(40, default=None)
    deactivated_at: Optional[datetime] = p_internal(41, default=None)
    reset_at: Optional[datetime] = p_internal(42, default=None)
    suspended_at: Optional[datetime] = p_internal(43, default=None)
    decommissioned_at: Optional[datetime] = p_internal(44, default=None)
    # current status
    active_at: Optional[datetime] = p_system(45, default=None)

    def __content_str__(self):
        value_strs: list[str] = []
        for prop in chain(
            Resource.__declared_properties__.values(),
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

    def _get_target_diff(self, *keys: str) -> dict[str, Any]:
        """
        Checks whether specific properties <key> differ from their target_<key> values.
        Returns the target values of differing properties.
        """
        target_diff: dict[str, Any] = {}
        for key in keys:
            prop = self.__properties__.get(key)
            assert prop is not None, f"no property {key} in {self.__class__.__name__}"
            current_value = getattr(self, key)
            target_value = getattr(self, f"target_{key}")
            if current_value != target_value:
                target_diff[key] = target_value
        return target_diff

    @property
    def target_status(self) -> ResourceStatus:
        """The implied target status of this Resource."""
        if self.decommissioned_at is not None:
            return ResourceStatus.DECOMMISSIONED
        elif self.suspended_at is not None and not (
            self.activated_at is not None and self.activated_at > self.suspended_at
        ):
            return ResourceStatus.SLEEPING
        elif self.deactivated_at is not None and not (
            self.activated_at is not None and self.activated_at > self.deactivated_at
        ):
            return ResourceStatus.DOWN
        else:
            return ResourceStatus.UP


@node_component()
class VirtualResource[NodeDataT: AnyNodeData](Resource[NodeDataT]):
    """
    A 'virtual' Resource in a Bench.
    """

    name: str = p_regular(32, constraint=NAME_CONSTRAINT)

    @classmethod
    def new(cls, name: str, **kwargs: Any) -> Self:
        """Creates a new Resource of this type. Defaults to current Bench"""

        # parent
        if "parent" not in kwargs:
            if NodeType.BENCH not in cls.__parent_types__:
                raise ValueError(f"cannot create {cls!r} without parent")
            bench = active_session().bench
            assert bench is not None, "no active Bench"
            kwargs["parent"] = bench

        resource = cls(name=name, **kwargs)
        return resource


@node_component()
class PhysicalResource[NodeDataT: AnyNodeData](Resource[NodeDataT]):
    """
    A 'physical' Resource in a Bench.
    """

    title: str = p_regular(32, constraint=TITLE_CONSTRAINT)

    occupancy: ResourceOccupancy = p_system(
        36, default=ResourceOccupancy.RESERVED, default_sql=None
    )
    owned_by: Optional[Owner] = p_system(37, require=False, array=False, references=OWNER_TYPES)

    @classmethod
    def new(cls, *, title: str | None = None, **kwargs: Any) -> Self:
        """Creates a new Resource of this type. Defaults to current Bench"""
        session = active_session()

        # parent
        if "parent" not in kwargs:
            if NodeType.BENCH not in cls.__parent_types__:
                raise ValueError(f"cannot create {cls!r} without parent")
            bench = session.bench
            assert bench is not None, "no active Bench"
            kwargs["parent"] = bench

        # title
        if title is None:
            # fabricate title
            now = session._oracle.utc()
            title = cls.metatype.bench_name + now.strftime("%Y-%m-%d %H:%M:%S")

        resource = cls(title=title, **kwargs)
        return resource


CPU_CONSTRAINT = constraint(min_value=0.1, max_value=16.0, step_value=0.1)
RAM_CONSTRAINT = constraint(min_value=0.1, max_value=256.0, step_value=0.1)


@node_(NodeType.SERVER)
class Server(VirtualResource[ServerData]):
    """
    A Server provides virtual compute for a Bench's Runtime.
    Physical compute is materialized dynamically on Machines.
    """

    version: str = p_system(50, default=VERSION, default_sql=None)
    target_version: str = p_system(51, default=VERSION, default_sql=None)

    min_cpu: Optional[float] = p_system(
        60, default=None, description="vCPU count", constraint=CPU_CONSTRAINT
    )
    max_cpu: Optional[float] = p_system(
        61, default=None, description="vCPU count", constraint=CPU_CONSTRAINT
    )
    min_ram: Optional[float] = p_system(
        62, default=None, description="GB", constraint=RAM_CONSTRAINT
    )
    max_ram: Optional[float] = p_system(
        63, default=None, description="GB", constraint=RAM_CONSTRAINT
    )


@node_(NodeType.STORE)
class Store(VirtualResource[StoreData]):
    """A trusty Postgres-compatible database."""

    version: str = p_system(50, default=VERSION, default_sql=None)
    target_version: str = p_system(51, default=VERSION, default_sql=None)

    external_name: Optional[str] = p_kernel(60, require=False, default=None, sensitive=True)
    external_id: Optional[str] = p_kernel(61, require=False, default=None, sensitive=True)
    connection_uri: Optional[str] = p_kernel(
        62, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )


@node_(NodeType.DRIVE)
class Drive(VirtualResource[DriveData]):
    """Drive for file storage."""

    ...


@node_(NodeType.VAULT)
class Vault(VirtualResource[DriveData]):
    """Vault for secret storage."""

    ...


@node_(NodeType.CACHE)
class Cache(VirtualResource[DriveData]):
    """Cache for ephemeral key-value storage."""

    ...


@node_(NodeType.CLIENT, roots=(NodeType.USER, NodeType.BENCH))
class Client(BenchNode[ClientData]):
    """A client to a Bench."""

    parent: Union["User", "Bench", None] = p_node_parent(4, NodeType.USER, NodeType.BENCH)
    type: ClientType = p_regular(30)
    title: str = p_regular(32, constraint=TITLE_CONSTRAINT)

    device_type: Optional[str] = p_regular(40, default=None)
    device_name: Optional[str] = p_regular(41, default=None)
    operating_system: Optional[str] = p_regular(42, default=None)
    browser_name: Optional[str] = p_regular(43, default=None)
    browser_version: Optional[str] = p_regular(44, default=None)
    place_id: Optional[str] = p_regular(45, default=None)

    access_token: Optional[str] = p_kernel(
        50, default=None, defer=True, unique=True, sensitive=True
    )
    seen_at: Optional[datetime] = p_system(51, default=None)
    logged_in_at: Optional[datetime] = p_system(52, default=None)

    space: Optional["Space"] = p_system(
        60, array=False, require=False, references=NodeType.SPACE, fk=True
    )
    server: Optional["Server"] = p_system(
        61, array=False, require=False, references=NodeType.SERVER, fk=True
    )
    machine: Optional["Machine"] = p_system(
        62, array=False, require=False, references=NodeType.MACHINE, fk=True
    )

    @property
    def is_attached(self) -> bool:
        parent = self.parent
        if parent is None:
            return False
        return parent.is_attached

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

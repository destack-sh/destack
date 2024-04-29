from datetime import datetime
from typing import TYPE_CHECKING, Optional

from bench.language.const import (
    EnumType,
    FileStatus,
    NodeType,
    PrimitiveType,
    StoreEngineType,
    StoreKind,
    StructType,
    enum_,
)
from bench.language.graph import NodeList
from bench.language.node import Node, Struct, node, node_component, struct
from bench.language.property import (
    p_internal,
    p_kernel,
    p_node_child,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.language.text import Text
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Bench, Client

# pyright: reportIncompatibleVariableOverride=false,reportIncompatibleMethodOverride=false


@enum_(EnumType.REGION)
class Region(IdEnum):
    """Where a Resource is located (physically)."""

    GLOBAL = 1
    # europe
    EUROPE_CENTRAL = 100
    # americas
    ...


@enum_(EnumType.TENANCY)
class Tenancy(IdEnum):
    """How a Resource is shared (if at all)."""

    SHARED = 3
    DEDICATED = 7


@enum_(EnumType.RESOURCE_STATUS)
class ResourceStatus(IdEnum):
    """Generalized status of a Resource in its lifecycle."""

    PENDING = 1
    CREATING = 5
    UPGRADING = 10
    HEALTHY = 20
    UNHEALTHY = 25
    PAUSED = 30


@node_component()
class Resource(Node):
    """A resource owned by a Bench."""

    parent: "Bench" = p_node_parent(4, NodeType.BENCH, is_system=True)
    name: str = p_regular(32)
    text: Optional[Text] = p_regular(34, default=None, struct=StructType.TEXT)
    region: Region = p_system(35, default=Region.GLOBAL)
    tenancy: Tenancy = p_system(36, default=Tenancy.SHARED)
    status: ResourceStatus = p_system(37, default=ResourceStatus.PENDING)

    def __content_str__(self):
        return f"{self.status.bench_name}, {self.tenancy.bench_name}, {self.region.bench_name}"


@enum_(EnumType.SERVER_PROFILE)
class ServerProfile(IdEnum):
    TINY = 3
    SMALL = 5
    MEDIUM = 7
    LARGE = 9


@node(NodeType.SERVER)
class Server(Resource):
    """
    A server providing the Runtime for a Bench.
    Similar to other resources, a Server virtualizes a compute allocation that is
    materialized on demand on a set of physical machines.
    """

    profile: ServerProfile = p_regular(40)
    version: Optional[str] = p_system(42, default=None)
    is_paused: bool = p_regular(43, default=True)

    current_profile: Optional[ServerProfile] = p_system(51, default=None)
    current_version: Optional[str] = p_system(53, default=None)
    last_active_at: Optional[datetime] = p_internal(54, default=None)
    last_bumped_at: Optional[datetime] = p_internal(55, default=None)

    clients: NodeList["Client"] = p_node_child(NodeType.CLIENT)

    def __content_str__(self):
        return f"{self.profile.bench_name}, version={self.version}, {self.status.bench_name}, {self.tenancy.bench_name}, {self.region.bench_name}"


@struct(StructType.RESOURCE_CREDENTIAL, inline=True)
class ResourceCredential(Struct):
    username: str = p_regular(31, sensitive=True)
    password: str = p_regular(32, sensitive=True)


@node(NodeType.STORE)
class Store(Resource):
    """
    A store for database-like storage in a Bench.
    Virtualizes a physical database of that kind/engine (may be a sub-database/schema or such).
    """

    kind: StoreKind = p_system(40)
    engine: StoreEngineType = p_system(41)
    version: Optional[str] = p_system(42, default=None)

    host: Optional[str] = p_kernel(50, require=False, default=None, sensitive=True)
    database: Optional[str] = p_kernel(51, require=False, default=None, sensitive=True)
    schema: Optional[str] = p_kernel(52, require=False, default=None, sensitive=True)
    main_credential: Optional[ResourceCredential] = p_kernel(
        54,
        require=False,
        default=None,
        array=False,
        sensitive=True,
        encrypt=True,
        defer=True,
        struct=StructType.RESOURCE_CREDENTIAL,
    )

    def __content_str__(self) -> str:
        return f"{self.kind.bench_name}, {self.engine.bench_name}, {self.status.bench_name}, {self.tenancy.bench_name}, {self.region.bench_name}"


@node(NodeType.DRIVE)
class Drive(Resource):
    """
    A drive for file-like storage in a Bench.
    Virtualizes simple bucket-style access to some S3-like storage.
    """

    ...


@enum_(EnumType.FILE_RETENTION_MODE)
class FileRetentionMode(IdEnum):
    AUTOMATIC = 1  # garbage collected if no references
    MANUAL = 2  # never garbage collected
    TIMED = 3  # delete after a certain time


@node(NodeType.FILE_CONTENT, unique_together=(("parent_drive_id", "sha512"),))
class FileContent(Node):
    """(A pointer to) the actual file stored in a Drive. De-duped to 1 per sha512."""

    parent: Drive = p_node_parent(4, NodeType.DRIVE, is_system=True)
    sha512: str = p_internal(30)
    size: int = p_internal(31, primitive_type=PrimitiveType.INT64)
    type: str = p_internal(32)
    status: FileStatus = p_internal(33)
    retention: FileRetentionMode = p_regular(34)
    expires_at: Optional[datetime] = p_regular(35)


@node(NodeType.CACHE)
class Cache(Resource):
    """Cache for ephemeral data."""

    ...

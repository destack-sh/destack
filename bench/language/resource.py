from datetime import datetime
from typing import TYPE_CHECKING, Generic, Optional, TypeVar, Union, cast

from bench.language.const import (
    ClientType,
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
from bench.language.node import Node, node, node_component
from bench.language.property import (
    p_internal,
    p_kernel,
    p_node_child,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.language.text import Text
from bench.language.validation import NAME_CONSTRAINT
from bench.proto.wire import (
    AnyNodeData,
    ClientData,
    DriveData,
    FileContentData,
    ServerData,
    StoreData,
)
from bench.utils.casing import IdentifierType
from bench.utils.dt import utcnow_with_tz
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Bench, Space, User

# pyright: reportIncompatibleVariableOverride=false


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


NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)


@node_component()
class Resource(Node[NodeDataT], Generic[NodeDataT]):
    """
    A resource owned by a Bench.
    Certain resources may be branched into a Package.
    """

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
class Server(Resource[ServerData]):
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


@node(NodeType.CLIENT, roots=(NodeType.USER, NodeType.BENCH), identifier=IdentifierType.VARIABLE)
class Client(Node[ClientData]):
    """A client to a Bench."""

    parent: Union["User", "Server"] = p_node_parent(4, NodeType.USER, NodeType.SERVER)
    type: ClientType = p_regular(30)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)

    device_name: Optional[str] = p_regular(40, default=None)
    device_type: Optional[str] = p_regular(41, default=None)
    operating_system: Optional[str] = p_regular(42, default=None)
    browser_name: Optional[str] = p_regular(43, default=None)
    browser_version: Optional[str] = p_regular(44, default=None)
    place_id: Optional[str] = p_regular(45, default=None)

    access_token: Optional[str] = p_kernel(
        50, default=None, defer=True, unique=True, sensitive=True
    )
    last_seen_at: datetime = p_system(51, default_factory=utcnow_with_tz)
    logged_in_at: Optional[datetime] = p_system(52, default=None)

    # for user clients
    main_space: Optional["Space"] = p_system(
        60, array=False, require=False, references=NodeType.SPACE, fk=True
    )

    def __content_str__(self) -> str:
        if self.browser_name:
            return f"{self.device_name} {self.browser_name}"
        else:
            return self.device_name or "???"

    @property
    def user(self) -> "User":
        assert self.parent is not None, f"no parent for {self!r}"
        assert self.parent.metatype == NodeType.USER, f"{self!r} belongs to {self.parent!r}"
        return cast("User", self.parent)


@node(NodeType.STORE)
class Store(Resource[StoreData]):
    """Postgres database."""

    kind: StoreKind = p_system(40)
    engine: StoreEngineType = p_system(41)
    version: Optional[str] = p_system(42, default=None)

    external_name: Optional[str] = p_kernel(50, require=False, default=None, sensitive=True)
    external_id: Optional[str] = p_kernel(51, require=False, default=None, sensitive=True)
    connection_uri: Optional[str] = p_kernel(
        52, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )

    def __content_str__(self) -> str:
        return f"{self.kind.bench_name}, {self.engine.bench_name}, {self.status.bench_name}, {self.tenancy.bench_name}, {self.region.bench_name}"


@node(NodeType.DRIVE)
class Drive(Resource[DriveData]):
    """Drive for file storage."""

    ...


@enum_(EnumType.FILE_RETENTION_MODE)
class FileRetentionMode(IdEnum):
    AUTOMATIC = 1  # garbage collected if no references
    MANUAL = 2  # never garbage collected
    TIMED = 3  # delete after a certain time


@node(NodeType.FILE_CONTENT, unique_together=(("parent_drive_id", "sha512"),))
class FileContent(Node[FileContentData]):
    """(A pointer to) the actual file stored in a Drive. De-duped to 1 per sha512."""

    parent: Drive = p_node_parent(4, NodeType.DRIVE, is_system=True)
    sha512: str = p_internal(30)
    size: int = p_internal(31, primitive_type=PrimitiveType.INT64)
    type: str = p_internal(32)
    status: FileStatus = p_internal(33)
    retention: FileRetentionMode = p_regular(34)
    expires_at: Optional[datetime] = p_regular(35)

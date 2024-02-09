import contextvars
import enum
import typing
from typing import Optional
from uuid import UUID

from bench.utils.func import IdEnum, bytetuple, cyrb53a
from bench.utils.utils import frozendict

if typing.TYPE_CHECKING:
    from bench.language import Bench, Block, Node, Package, Session  # noqa: F401

# hard-coded, do not change ever :BenchUuidNamespace
UUID_NAMESPACE = UUID("d822dab7-41ad-4706-a9c8-4379e15b2ed0")
VERSION = "2024.02.09.3"
UNSET = object()
EMPTY_LIST: list = []
EMPTY_SET: frozenset = frozenset()
EMPTY_DICT: typing.Mapping = frozendict()


#
# Metatypes for Nodes/Structs
#


class NodeType(IdEnum):
    # root
    BENCH = 1
    # source containers
    # PLACE = 2
    ENVIRONMENT = 3
    BRANCH = 4
    # source
    PACKAGE = 20
    BLOCK = 21
    TRIGGER = 22
    TAG = 23
    FIELD = 24
    RECORD = 25  # (local)
    QUERY = 26
    VIEW = 27
    # STEP = 28
    # CONNECTION? (also for Flow)
    NOTICE = 29
    LINK = 35
    SKIP = 36
    # COMMENT = ...
    # REACTION = ...
    SPACE = 40
    # DEPENDENCY = ...
    # UPGRADE = ...
    # LOCK?

    # session
    SESSION = 50  # (local)
    RUN = 51  # (local)
    PAUSE = 52  # (local)
    SIGNAL = 53  # (local)

    # auth
    BADGE = 60
    ROLE = 61
    IDENTITY = 62

    # resources (compute/storage/external/etc.)
    SERVER = 160
    STORE = 161  # 'database' for Postgres/OpenSearch/ClickHouse
    DRIVE = 162  # 'bucket' for S3/MinIO
    CACHE = 163  # Redis/Memcached
    FILE_CONTENT = 170  # in a Drive
    # DOMAIN, EMAIL, ...

    # user
    HANDLE = 220
    USER = 221
    ORGANIZATION = 222
    CLIENT = 223
    NOTIFICATION = 224

    MEMBERSHIP = 240
    # INVITE = ...
    # FRIENDSHIP/FOLLOW/...?


NODE_TYPES: bytetuple[NodeType] = bytetuple(tuple(NodeType))
ROOT_NODE_TYPES: bytetuple[NodeType] = bytetuple(
    (NodeType.BENCH, NodeType.USER, NodeType.ORGANIZATION)
)
IN_PACKAGE_NODE_TYPES: bytetuple[NodeType] = bytetuple(
    tuple(nt for nt in NODE_TYPES if 20 <= nt.id < 100) + (NodeType.SPACE,)
)
SUB_PACKAGE_NODE_TYPES: bytetuple[NodeType] = bytetuple(
    tuple(nt for nt in IN_PACKAGE_NODE_TYPES if nt != NodeType.PACKAGE)
)
IN_BENCH_NODE_TYPES: bytetuple[NodeType] = bytetuple(
    tuple(nt for nt in NODE_TYPES if nt.id < 200)
    + (NodeType.MEMBERSHIP, NodeType.CLIENT, NodeType.SPACE, NodeType.HANDLE)
)
SUB_BENCH_NODE_TYPES: bytetuple[NodeType] = bytetuple(
    tuple(nt for nt in IN_BENCH_NODE_TYPES if nt != NodeType.BENCH)
)
PUBLIC_NODE_TYPES: bytetuple[NodeType] = bytetuple(
    (NodeType.BENCH, NodeType.USER, NodeType.ORGANIZATION, NodeType.MEMBERSHIP)
)
ABOVE_SOURCE_NODE_TYPES: bytetuple[NodeType] = bytetuple(
    tuple(nt for nt in NODE_TYPES if nt.id >= 200) + (NodeType.BENCH,)
)


class StructType(IdEnum):
    # starts at 500 to avoid collisions with NodeType (BenchType combines both in one metatype)
    BENCH_PATH = 500
    NODE_REFERENCE = 501
    PROPERTY_REFERENCE = 502
    PROPERTY_PATH = 503
    FIELD_PATH = 504
    FIELD_PATH_SEGMENT = 505
    VALUE_REFERENCE = 506
    VALUE_SELECTION = 507

    TYPE_INFO = 510
    CONTEXT = 511
    SCHEDULE = 512

    FILE = 520
    ICON = 521

    POLICY = 530
    POLICY_RULE = 531
    SUBJECT = 532
    ACCESS_ZONE = 534
    ACCESS_MATRIX = 535
    ACCESS = 537
    ACCESS_TRACE = 538
    REQUEST = 536
    ...
    READ_OPTIONS = 550

    EXPRESSION = 560
    AGGREGATION = 561
    AGGREGATION_BUCKET = 562

    CODE = 590
    CODE_SECTION = 591
    RUN_CODE_FRAME = 592
    RUN_ERROR = 593
    LOG_ENTRY = 600
    # CURSOR?

    SERVER_IMAGE = 630
    SERVER_IMAGE_REQUIREMENT = 631

    RICH_TEXT = 660
    RICH_TEXT_SPAN = 661

    # views
    SPACE_DOCK = 700
    SPACE_DOCK_ITEM = 701

    # ...

    # shapes
    # ...

    # workspace
    # ...


STRUCT_TYPES: bytetuple[StructType] = bytetuple(tuple(StructType))

if typing.TYPE_CHECKING:
    BenchType = NodeType | StructType
else:
    BenchType = IdEnum.combine("BenchType", NodeType, StructType)

BENCH_TYPES: bytetuple[BenchType] = bytetuple(tuple(BenchType))
INTERP_NODE_TYPES = (NodeType.NOTICE,)


class BlockType(IdEnum):
    PAGE = 1  # group of blocks
    BLANK = 2  # placeholder/spacer
    TEXT = 3  # define a 'paragraph' of text/comment/instruction/etc.
    ALIAS = 4  # refer to / 'redefine' an existing block or builtin (like a 'newtype')

    CLASS = 10  # define a class type with fields
    CHOICE = 11  # define a choice type with fields
    TAG = 12  # define a tag type with fields
    SIGNAL = 13  # define a signal type with fields
    PROTOCOL = 14  # define a 'protocol' for a block graph/template with fields
    # NOTICE = ...  # define a new notice type
    # NOTIFICATION = ...  # define a new notification type
    # BLOCK = ...  # define a new block type?

    SINGLE_VARIABLE = 20  # define a single-value variable
    MULTI_VARIABLE = 21  # define a variable with (multiple) fields

    TASK = 30  # define a task function with fields (incl. input/output)
    ROUTINE = 31  # define a code function with input/output fields (incl. input/output)
    SCRIPT = 32  # define a code script with fields
    FLOW = 33  # define a flow with steps and fields (optionally incl. input/output)

    QUERY = 40  # define a set of queries
    DATABASE = 41  # define a database with queries

    SCREEN = 50  # define a screen with views

    ROLE = 60  # define a role with policies
    IDENTITY = 61  # define an identity with roles & policies

    @property
    def is_type(self) -> bool:
        return self in BlockTypes.TYPES

    @property
    def is_runnable(self) -> bool:
        return self in BlockTypes.RUNNABLE

    @property
    def is_scriptable(self) -> bool:
        return self in BlockTypes.SCRIPTABLE

    @property
    def is_nestable(self) -> bool:
        return self not in BlockTypes.LEAVES


BLOCK_TYPES: tuple[BlockType, ...] = tuple(BlockType)


class BlockTypes:
    TYPES = tuple(t for t in BLOCK_TYPES if 10 <= t.id < 20)
    RUNNABLE = tuple(t for t in BLOCK_TYPES if 30 <= t.id < 40)
    SCRIPTABLE = tuple(t for t in BLOCK_TYPES if 10 <= t.id < 50) + (BlockType.ALIAS,)
    LEAVES = (BlockType.BLANK, BlockType.TEXT)
    NESTABLE = tuple(t for t in BLOCK_TYPES if t not in (BlockType.BLANK, BlockType.TEXT))


class BlockMode(IdEnum):
    INLINE = 1
    PAGE = 2
    MODULE = 3


class NodeSource(IdEnum):
    PERSISTED = 1
    INTERP = 2
    LOCAL = 3


class NodeVisibility(IdEnum):
    # ...?
    BLOCK = 2
    PAGE = 4
    MODULE = 6
    BENCH = 8
    ALL = 10


DYNAMIC_NODE_KEY_LENGTH = 8


def new_dynamic_node_key(ck_or_id: UUID) -> str:
    """
    Gets a 'random' alphabetic key as a persistent but self-directed identity key for a node.
    Used for storing dynamic field values, dynamic database identities (versioned/un-versioned).
    (short key length alphabetic characters)
    """
    hash_value = cyrb53a(str(ck_or_id))
    key_parts = []
    for _ in range(DYNAMIC_NODE_KEY_LENGTH):
        hash_value, remainder = divmod(hash_value, 52)
        if remainder < 26:
            key_parts.append(chr(ord("a") + remainder))
        else:
            key_parts.append(chr(ord("A") + remainder - 26))
    key = "".join(key_parts)
    return key


class NodeTrackingLevel(enum.IntEnum):
    NONE = 0
    ANONYMOUS = 1
    FULL = 2


NTL = NodeTrackingLevel


class NodeRelationType(enum.IntEnum):
    """Parent relation between node and descendants."""

    DEFAULT = 0  # default inline relation
    STORED_CUSTOM = 2**0  # not inline: Block->Record, ...
    CUMULATIVE = 2**1  # sum of descendants: Package->Issue, Block->Issue, ...
    NAMED = 2**2  # indexed by name: Package->Block, Block->Block, ...
    SCOPED = 2**3  # scoped by name: Package->Block, Block->Block, ...
    KEYED = 2**4  # indexed by key: Block->Tagging, Block->Tagging, ...
    ORDERED = 2**5  # ordered: Block->Block, Block->Field, ...


NRel = NodeRelationType


class NodeStatus(enum.IntEnum):
    SOURCE = 0  # just loaded
    INTERP = 1  # everything resolved & ready
    TRACKED = 2  # live in a session


#
# Access
# Access types are loosely ranked by access/destructiveness across and within types.
#


class ReadType(IdEnum):
    """A type of Read access on nodes."""

    GET = 1  # any direct read access
    AGGREGATE_SCALAR = 2  # count, sum, min, etc.
    AGGREGATE_BUCKET = 3  # histogram, etc.
    LIST = 4  # list, search, filter, etc.

    @property
    def kind(self) -> "AccessKind":
        return AccessKind.READ


class EditType(IdEnum):
    """A type of Edit access on nodes."""

    BUMP_CHANGED = 10
    BUMP_ACTIVE = 11
    CREATE = 12
    UPSERT = 13
    UPDATE = 14
    MOVE = 15
    ARCHIVE = 16
    UNARCHIVE = 17
    SOFT_DELETE = 18
    RESTORE = 19
    DELETE = 20

    @property
    def kind(self) -> "AccessKind":
        return AccessKind.EDIT


class RunType(IdEnum):
    """A type of Run access on nodes."""

    START = 30
    PAUSE = 31
    RESUME = 32
    KILL = 33

    @property
    def kind(self) -> "AccessKind":
        return AccessKind.RUN


class AccessKind(IdEnum):
    # NOTE: AccessType/AccessKind ids should match (used in properties masks).
    READ = 1
    EDIT = 10
    RUN = 30

    @property
    def from_id(self):
        return ACCESS_CLASS_BY_KIND[self].get_min_id()

    @property
    def to_id(self):
        return ACCESS_CLASS_BY_KIND[self].get_max_id()


if typing.TYPE_CHECKING:
    AccessType = ReadType | EditType | RunType
else:
    AccessType = IdEnum.combine("AccessType", ReadType, EditType, RunType)
    AccessType.kind = property(lambda self: ACCESS_KIND_BY_ACCESS[self])

READ_TYPES: bytetuple[ReadType] = bytetuple(tuple(ReadType))
EDIT_TYPES: bytetuple[EditType] = bytetuple(tuple(EditType))
RUN_TYPES: bytetuple[RunType] = bytetuple(tuple(RunType))
ACCESS_TYPES: bytetuple[AccessType] = bytetuple(tuple(AccessType))
ACCESS_CLASSES: tuple[type[AccessType], ...] = (ReadType, EditType, RunType, AccessType)
ACCESS_KINDS = bytetuple(tuple(AccessKind))
ACCESS_TYPES_BY_KIND: dict[AccessKind, bytetuple[AccessType]] = {
    AccessKind.READ: bytetuple(READ_TYPES),
    AccessKind.EDIT: bytetuple(EDIT_TYPES),
    AccessKind.RUN: bytetuple(RUN_TYPES),
}
ACCESS_CLASS_BY_KIND: dict[AccessKind, type[AccessType]] = {
    AccessKind.READ: ReadType,
    AccessKind.EDIT: EditType,
    AccessKind.RUN: RunType,
}
ACCESS_KIND_BY_ACCESS: dict[AccessType, AccessKind] = {
    access: kind for kind, access_types in ACCESS_TYPES_BY_KIND.items() for access in access_types
}


class AccessMode(IdEnum):
    ADAPTIVE = 1
    ATOMIC = 2


#
# Other stuff
#


class StoreKind(IdEnum):
    RELATIONAL = 1
    SEARCH = 2
    ANALYTICAL = 3  # would be nice to unify with SEARCH...


class StoreEngine(IdEnum):
    POSTGRES = 1
    OPENSEARCH = 20
    CLICKHOUSE = 30


class BadgeType(IdEnum):
    SHARING_LINK = 1
    ACCESS_KEY = 2


class PolicyEffect(IdEnum):
    ALLOW = 1
    DENY = 2
    # DEFER?, METER, LIMIT, ...


class ClientKind(IdEnum):
    USER = 1
    SERVER = 2


class PrimitiveType(IdEnum):
    """
    Fundamental column / storage types we support (subset of SQL types, used directly in sql/core).
    NOTE: the ids here are used in value pack/unpack keys, so any changes are breaking.
    """

    BOOLEAN = 1
    INT32 = 2  # range: -2147483648 to 2147483647
    INT64 = 3  # range: -9223372036854775808 to 9223372036854775807
    FLOAT32 = 4  # range: 1.175494351e-38 to 3.402823466e+38
    FLOAT64 = 5  # range: 2.2250738585072014e-308 to 1.7976931348623157e+308
    DECIMAL = 6  # numeric(precision, scale)
    STRING = 7
    DATETIME = 8
    INTERVAL = 9
    JSON = 10
    BYTES = 11
    VECTOR = 12
    UUID = 13


class FormatHint(IdEnum):
    """Extra semantic hint for types."""

    # string
    TITLE = 1
    EMAIL = 2
    URL = 3
    MARKDOWN = 4
    CODE = 5
    # number
    PHONE = 20
    RATING = 21
    SLIDER = 22
    # boolean
    TOGGLE = 40
    CHECKBOX = 41
    THUMBS = 42
    # files
    IMAGE = 60
    VIDEO = 61
    AUDIO = 62


class FileStatus(IdEnum):
    PENDING = 1
    UPLOADING = 2
    AVAILABLE = 3


class TriggerType(IdEnum):
    """Triggers for blocks (for both actual runs and pre-defined triggers)."""

    SCHEDULE = 1
    SIGNAL = 2


class ScheduleType(IdEnum):
    INTERVAL = 1
    CRON = 2


class NoticeKind(IdEnum):
    """Type of diagnostic in increasing severity."""

    HINT = 1
    INFORMATION = 2
    WARNING = 3
    ERROR = 4


class BenchRegion(IdEnum):
    EU_CENTRAL = 1
    US_WEST = 10


class ServerStatus(IdEnum):
    SLEEPING = 1
    PENDING = 2
    UPDATING = 3
    HEALTHY = 4
    UNHEALTHY = 5
    UNAVAILABLE = 6
    UNKNOWN = 7


class RunStatus(IdEnum):
    SCHEDULED = 1
    QUEUED = 2
    RUNNING = 3
    PAUSED = 4
    ABORTING = 5
    # terminal statuses
    CANCELLED = 6
    ABORTED = 7
    FAILED = 8
    COMPLETED = 9


TERMINAL_RUN_STATUSES = {
    RunStatus.CANCELLED,
    RunStatus.ABORTED,
    RunStatus.FAILED,
    RunStatus.COMPLETED,
}
PENDING_RUN_STATUSES = {
    RunStatus.SCHEDULED,
    RunStatus.QUEUED,
    RunStatus.RUNNING,
    RunStatus.PAUSED,
    RunStatus.ABORTING,
}
ACTIVE_RUN_STATUSES = {RunStatus.QUEUED, RunStatus.RUNNING, RunStatus.PAUSED, RunStatus.ABORTING}


class RunErrorKind(IdEnum):
    INTERNAL = 1
    PARSE = 2
    VALIDATION = 3
    RUNTIME = 4
    UNTRUSTED = 5


class ServerProfile(IdEnum):
    TINY = 1
    SMALL = 2
    MEDIUM = 3


class ExpressionKind(IdEnum):
    CONDITIONAL = 1
    SORT = 2
    AGGREGATION = 3


class ConditionalOp(IdEnum):
    # logical
    TRUE = 1
    FALSE = 2
    NOT = 3
    AND = 4
    OR = 5
    # comparison
    EQUALS = 10
    NOT_EQUALS = 11
    GREATER_THAN = 12
    GREATER_THAN_OR_EQUALS = 13
    LESS_THAN = 14
    LESS_THAN_OR_EQUALS = 15
    # string comparison
    MATCHES = 20
    STARTS_WITH = 21
    REGEX = 22
    # containment
    CONTAINS = 30
    NOT_CONTAINS = 31
    IN = 32
    NOT_IN = 33
    # existence
    EXISTS = 40
    NOT_EXISTS = 41
    # vector
    NEAR = 50


class AggregationOp(IdEnum):
    EXISTS = 100
    COUNT = 101
    SUM = 102
    AVERAGE = 103
    MIN = 104
    MAX = 105
    MEDIAN = 106
    HISTOGRAM = 107


class SortOp(IdEnum):
    ASCENDING = 200
    DESCENDING = 201


class QueryEngine(IdEnum):
    IN_MEMORY = 1
    GLOBAL_POSTGRES = 2
    LOCAL_POSTGRES = 4
    LOCAL_OPENSEARCH = 5


class SortMode(IdEnum):
    MAX = 1
    MIN = 2
    AVERAGE = 3
    SUM = 4
    MEDIAN = 5


if typing.TYPE_CHECKING:
    ExpressionOp = ConditionalOp | AggregationOp | SortOp
else:
    ExpressionOp = IdEnum.combine("ExpressionOp", ConditionalOp, AggregationOp, SortOp)


class BenchError(Exception):
    """Common base class for any regular errors."""

    pass


_active_session: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)
_no_validation: contextvars.ContextVar[bool] = contextvars.ContextVar(
    "_no_validation", default=False
)


def active_session() -> "Session":
    session = _active_session.get()
    assert session is not None, "no active session"
    return session

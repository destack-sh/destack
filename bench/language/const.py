import contextvars
import enum
import typing
from typing import Optional
from uuid import UUID

from bench.proto.wire import GraphScope
from bench.utils.func import IdEnum, bytetuple, cyrb53a
from bench.utils.utils import frozendict

if typing.TYPE_CHECKING:
    from bench.language import Session, Transaction

VERSION = "2024.02.22.0"
UNSET = object()
EMPTY_LIST: list = []
EMPTY_SET: frozenset = frozenset()
EMPTY_DICT: typing.Mapping = frozendict()
EMPTY_SCOPE = GraphScope()
REVISION_PENDING = -1


#
# Metatypes for Nodes/Structs
#


class NodeType(IdEnum):
    # root
    BENCH = 1
    PLACE = 2
    ENVIRONMENT = 3
    BRANCH = 4

    # source
    PACKAGE = 20
    DEPENDENCY = 21
    UPGRADE = 22
    SPACE = 23
    LINK = 24
    SKIP = 25
    NOTICE = 26
    BLOCK = 30
    TRIGGER = 31
    FIELD = 32
    RECORD = 33  # (local)
    QUERY = 34
    VIEW = 35
    # TAG?  (not sure what to do with tags yet)
    # STEP = ...
    # COMMENT = ...
    # REACTION = ...
    # LOCK?
    # BREAKPOINT?

    # auth
    BADGE = 60
    ROLE = 61
    IDENTITY = 62
    MEMBERSHIP = 63
    INVITE = 64

    # session/runtime
    SESSION = 80  # (local)
    RUN = 81  # (local)
    PAUSE = 82  # (local)
    SIGNAL = 83  # (local)
    LOG = 84  # (local, analytics only)
    NOTIFICATION = 85
    # METRIC = ...?

    # resources (compute/storage/external/etc.)
    SERVER = 160
    STORE = 161  # any 'database' (Postgres/OpenSearch/ClickHouse)
    DRIVE = 162  # 'bucket' like S3/MinIO, maybe block storage later
    CACHE = 163  # KV memory store (Redis/Memcached)
    # DOMAIN, EMAIL, ...
    FILE_CONTENT = 180  # in a Drive

    # user
    HANDLE = 220
    USER = 221
    ORGANIZATION = 222
    CLIENT = 223


NODE_TYPES: bytetuple[NodeType] = bytetuple(tuple(NodeType))
ROOT_NODE_TYPES: bytetuple[NodeType] = bytetuple(
    (NodeType.BENCH, NodeType.USER, NodeType.ORGANIZATION)
)
IN_PACKAGE_NODE_TYPES: bytetuple[NodeType] = bytetuple(
    tuple(nt for nt in NODE_TYPES if 20 <= nt.id < 100)
)
SUB_PACKAGE_NODE_TYPES: bytetuple[NodeType] = bytetuple(
    tuple(nt for nt in NODE_TYPES if 20 < nt.id < 100)
)
IN_BENCH_NODE_TYPES: bytetuple[NodeType] = bytetuple(
    tuple(nt for nt in NODE_TYPES if nt.id < 200) + (NodeType.CLIENT, NodeType.HANDLE)
)
SUB_BENCH_NODE_TYPES: bytetuple[NodeType] = bytetuple(
    tuple(nt for nt in IN_BENCH_NODE_TYPES if nt != NodeType.BENCH)
)
PUBLIC_NODE_TYPES: bytetuple[NodeType] = bytetuple((NodeType.USER, NodeType.ORGANIZATION))
USER_NODE_TYPES = bytetuple(tuple(nt for nt in NODE_TYPES if nt.id >= 200))
HIGH_VOLUME_NODE_TYPES = bytetuple(
    (NodeType.RUN, NodeType.LOG, NodeType.SIGNAL, NodeType.FILE_CONTENT)
)


class StructType(IdEnum):
    # core
    PATH = 500
    PATH_SEGMENT = 501
    PATH_TOKEN = 502
    NODE_REFERENCE = 503
    PROPERTY_REFERENCE = 504
    VALUE_REFERENCE = 505
    TYPE_INFO = 510
    CONTEXT = 511
    SCHEDULE = 512
    PROJECTION = 513

    # files
    FILE = 520
    ICON = 521

    # access
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

    # expressions
    EXPRESSION = 560
    AGGREGATION = 561
    AGGREGATION_BUCKET = 562

    # code
    CODE = 590
    CODE_LINE = 591
    RUN_CODE_FRAME = 600
    RUN_ERROR = 601
    # CURSOR?

    SERVER_IMAGE = 630
    SERVER_IMAGE_REQUIREMENT = 631
    STORE_CREDENTIAL = 632

    # text
    TEXT = 660
    TEXT_LINE = 661
    TEXT_SPAN = 662

    # views
    COLOR = 700
    ...

    # space
    SPACE_DOCK = 800
    SPACE_DOCK_ITEM = 801
    ...

    # shapes
    ...


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
    # TAG = 12  # define a tag type with fields
    SIGNAL = 13  # define a signal type with fields
    PROTOCOL = 14  # define a 'protocol' for a block graph/template with fields
    # NOTICE = ...  # define a new notice type
    # NOTIFICATION = ...  # define a new notification type
    # METRIC = ...  # define a new metric type
    # BLOCK = ...  # define a new block type?

    SINGLE_VARIABLE = 20  # define a single-value variable
    MULTI_VARIABLE = 21  # define a variable with (multiple) fields

    TASK = 30  # define a task function with fields (incl. input/output)
    ROUTINE = 31  # define a code function with input/output fields (incl. input/output)
    SCRIPT = 32  # define a code script with fields
    FLOW = 33  # define a flow with steps and fields (optionally incl. input/output)

    QUERY = 40  # define a set of queries
    DATABASE = 41  # define a database with records & queries

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


BLOCK_TYPES: tuple[BlockType, ...] = tuple(BlockType)


class BlockTypes:
    TYPES = bytetuple(tuple(t for t in BLOCK_TYPES if 10 <= t.id < 20))
    RUNNABLE = bytetuple(tuple(t for t in BLOCK_TYPES if 30 <= t.id < 40))
    SCRIPTABLE = bytetuple(tuple(t for t in BLOCK_TYPES if 10 <= t.id < 50) + (BlockType.ALIAS,))


class NodeSource(IdEnum):
    STORE = 1
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


class ReferenceKind(IdEnum):
    """A reference to a Node or Struct - usually both have an identity (except for inlined Structs)."""

    NODE_ANCESTOR_ROOT = 1
    NODE_ANCESTOR_FIRST = 2
    NODE_PARENT = 3
    NODE_CHILD = 4
    NODE_REGULAR = 5
    STRUCT_PARENT = 6
    STRUCT_CHILD = 7
    PROPERTY = 8

    @property
    def is_node_tree(self):
        return self.id <= 4

    @property
    def is_node(self):
        return self.id <= 5

    @property
    def is_struct_tree(self):
        return self.id > 5


class NodeRelationFlag(enum.IntFlag):
    """Parent relation between node and descendants."""

    DEFAULT = 0  # default inline relation
    STORED_CUSTOM = 2**0  # not inline: Block->Record, ...
    CUMULATIVE = 2**1  # sum of descendants: Package->Issue, Block->Issue, ...
    NAMED = 2**2  # indexed by name: Package->Block, Block->Block, ...
    SCOPED = 2**3  # scoped by name: Package->Block, Block->Block, ...
    KEYED = 2**4  # indexed by key: Block->Tagging, Block->Tagging, ...
    ORDERED = 2**5  # ordered: Block->Block, Block->Field, ...


NRel = NodeRelationFlag


class InterpStatus(IdEnum):
    SOURCE = 1  # just loaded
    INTERPED = 2  # everything resolved & ready
    TRACKED = 3  # live in a session


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


class UseType(IdEnum):
    """A type of Run access on nodes."""

    START = 30
    PAUSE = 31
    RESUME = 32
    STOP = 33
    SEND = 34
    RECEIVE = 35

    @property
    def kind(self) -> "AccessKind":
        return AccessKind.USE


class AccessKind(IdEnum):
    READ = 1
    EDIT = 10
    USE = 30

    @property
    def from_ord(self) -> int:
        return ACCESS_CLASS_BY_KIND[self].get_min_ord()

    @property
    def to_ord(self) -> int:
        return ACCESS_CLASS_BY_KIND[self].get_max_ord()


if typing.TYPE_CHECKING:
    AccessType = ReadType | EditType | UseType
else:
    AccessType = IdEnum.combine("AccessType", ReadType, EditType, UseType)
    AccessType.kind = property(lambda self: ACCESS_KIND_BY_ACCESS[self])

READ_TYPES: bytetuple[ReadType] = bytetuple(tuple(ReadType))
EDIT_TYPES: bytetuple[EditType] = bytetuple(tuple(EditType))
USE_TYPES: bytetuple[UseType] = bytetuple(tuple(UseType))
ACCESS_TYPES: bytetuple[AccessType] = bytetuple(tuple(AccessType))
ACCESS_CLASSES: tuple[type[AccessType], ...] = (ReadType, EditType, UseType, AccessType)
ACCESS_KINDS = bytetuple(tuple(AccessKind))
ACCESS_TYPES_BY_KIND: dict[AccessKind, bytetuple[AccessType]] = {
    AccessKind.READ: bytetuple(READ_TYPES),
    AccessKind.EDIT: bytetuple(EDIT_TYPES),
    AccessKind.USE: bytetuple(USE_TYPES),
}
ACCESS_CLASS_BY_KIND: dict[AccessKind, type[AccessType]] = {
    AccessKind.READ: ReadType,
    AccessKind.EDIT: EditType,
    AccessKind.USE: UseType,
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
    ANALYTICAL = 3


class StoreEngineType(IdEnum):
    INMEMORY = 1
    REMOTE = 2
    POSTGRES = 3
    OPENSEARCH = 4
    CLICKHOUSE = 5


class BadgeType(IdEnum):
    SHARING_LINK = 1
    ACCESS_KEY = 2


class PolicyEffect(IdEnum):
    ALLOW = 1
    DENY = 2
    # DEFER?, METER, LIMIT, ...


class PrimitiveType(IdEnum):
    """
    Fundamental column / storage types we support (subset of SQL types, used directly in sql/core).
    NOTE: the ids here are used in value pack/unpack keys, so any changes are breaking.
    """

    BOOLEAN = 1
    # ...
    INT16 = 4  # range: -32768 to 32767
    INT32 = 5  # range: -2147483648 to 2147483647
    INT64 = 6  # range: -9223372036854775808 to 9223372036854775807
    # ...
    FLOAT32 = 9  # range: 1.175494351e-38 to 3.402823466e+38
    FLOAT64 = 10  # range: 2.2250738585072014e-308 to 1.7976931348623157e+308
    # ...
    DECIMAL = 12  # numeric(precision, scale)
    # ...
    STRING = 15
    JSON = 16
    BYTES = 17
    VECTOR = 18
    UUID = 19
    DATETIME = 20
    INTERVAL = 21


class FormatHint(IdEnum):
    """Extra semantic hint for types."""

    # string
    TITLE = 1
    EMAIL = 2
    URL = 3
    MARKDOWN = 4
    CODE = 5
    EMOJI = 6
    # number
    PHONE = 20
    RATING = 21
    SLIDER = 22
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
    INFO = 2
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


TERMINAL_RUN_STATUSES: bytetuple[RunStatus] = bytetuple(
    RunStatus.CANCELLED,
    RunStatus.ABORTED,
    RunStatus.FAILED,
    RunStatus.COMPLETED,
)
ACTIVE_RUN_STATUSES = bytetuple(
    RunStatus.QUEUED, RunStatus.RUNNING, RunStatus.PAUSED, RunStatus.ABORTING
)


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
    # basic comparison
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


class UserStatus(IdEnum):
    INVITED = 1  # invited via email
    RESERVED = 2  # reserved a handle, unconfirmed
    REGISTERED = 3  # confirmed email
    ACTIVATED = 10  # has main bench


class OrganizationStatus(IdEnum):
    REGISTERED = 3  # created org
    ACTIVATED = 10  # has main bench


class NotificationKind(IdEnum):
    """
    The level of interaction required for a notification.
    """

    PASSIVE = 1  # no quick action required, not urgent
    ACTIVE = 2  # important action required / may want to know this as soon as possible
    URGENT = 3  # immediate action required


#
# Other common non-const stuff
#


class BenchError(Exception):
    """Common base class for any regular errors."""

    pass


_active_session: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)


def active_session() -> "Session":
    """Gets the currently active Session (error if none)."""
    session = _active_session.get()
    assert session is not None, "no active session"
    return session


def active_tx() -> "Transaction":
    """Gets the currently active Transaction (error if none)."""
    session = _active_session.get()
    assert session is not None, "no active session"
    return session._tx

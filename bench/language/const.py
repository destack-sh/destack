from __future__ import annotations

import contextvars
import enum
import typing
from itertools import chain
from typing import Optional
from uuid import UUID

from bench.proto.core import ProtoStrEnum
from bench.utils.casing import Casing, to_casing
from bench.utils.func import cyrb53a

if typing.TYPE_CHECKING:
    from bench.language import Node, Block, Session, Bench, Package  # noqa: F401

# hard-coded, do not change ever :BenchUuidNamespace
UUID_NAMESPACE = UUID("d822dab7-41ad-4706-a9c8-4379e15b2ed0")
VERSION = "2024.01.25.2"


#
# Metatypes
#


class NodeType(ProtoStrEnum):
    # root
    BENCH = "BENCH", 1
    # UNIVERSE = "UNIVERSE", 2
    # PLACE = "PLACE", 3
    # BRANCH = "BRANCH", 4
    # DEPENDENCY = "DEPENDENCY", 5
    # UPGRADE = "UPGRADE", 6

    # source
    PACKAGE = "PACKAGE", 20
    BLOCK = "BLOCK", 21
    TRIGGER = "TRIGGER", 22
    TAGGING = "TAGGING", 23
    FIELD = "FIELD", 24
    RECORD = "RECORD", 25  # (local)
    VIEW = "VIEW", 26
    # TILE = "TILE", 27
    # STEP = "STEP", 28
    ISSUE = "ISSUE", 29
    LINK = "LINK", 30

    # session (all local)
    SESSION = "SESSION", 50
    RUN = "RUN", 51
    PAUSE = "PAUSE", 52
    SIGNAL = "SIGNAL", 53

    # auth
    BADGE = "BADGE", 60
    # ROLE = "ROLE", 61
    # IDENTITY = "IDENTITY", 62

    # resources (compute/storage/etc.)
    WORKER_SET = "WORKER_SET", 80
    WORKER = "WORKER", 81
    # WORKER_PROCESS = "WORKER_PROCESS", 82
    BUCKET_OBJECT = "BUCKET_OBJECT", 83

    # user
    HANDLE = "HANDLE", 120
    USER = "USER", 121
    ORGANIZATION = "ORGANIZATION", 122
    CLIENT = "CLIENT", 123
    NOTIFICATION = "NOTIFICATION", 124

    # INVITE = "INVITE", 130
    # MEMBERSHIP = "MEMBERSHIP", 131
    # COMMENT = "COMMENT", 132
    # MESSAGE = "MESSAGE", 133

    @property
    def bench_name(self):
        return BENCH_TYPE_NAME[self]


NODE_TYPES: tuple[NodeType, ...] = tuple(NodeType)
# (we duplicate in-package/in-bench info here to access it while initialising the node classes,
#  but we check for consistency during finalization)
IN_PACKAGE_NODE_TYPES: tuple[NodeType, ...] = tuple(nt for nt in NODE_TYPES if 20 <= nt.id < 80)
IN_BENCH_NODE_TYPES: tuple[NodeType, ...] = tuple(nt for nt in NODE_TYPES if nt.id < 100)


class StructType(ProtoStrEnum):
    # starts at 100 to avoid collisions with NodeType (BenchType combines both in one metatype)
    BENCH_PATH = "BENCH_PATH", 200
    NODE_REFERENCE = "NODE_REFERENCE", 201
    PROPERTY_REFERENCE = "PROPERTY_REFERENCE", 202
    PROPERTY_PATH = "PROPERTY_PATH", 203
    FIELD_PATH = "FIELD_PATH", 204
    FIELD_PATH_SEGMENT = "FIELD_PATH_SEGMENT", 205
    VALUE_REFERENCE = "VALUE_REFERENCE", 206

    BLOB = "BLOB", 210  # nocheckin: rename Blob -> File
    TYPE_INFO = "TYPE_INFO", 211

    POLICY = "POLICY", 220
    POLICY_RULE = "POLICY_RULE", 221
    CONTEXT = "CONTEXT", 222

    EXPRESSION = "EXPRESSION", 240
    AGGREGATION = "AGGREGATION", 241
    AGGREGATION_BUCKET = "AGGREGATION_BUCKET", 242

    LOG_ENTRY = "LOG_ENTRY", 260
    RUN_CODE_FRAME = "RUN_CODE_FRAME", 261
    RUN_ERROR = "RUN_ERROR", 262
    # CURSOR = "CURSOR", 264

    WORKER_IMAGE = "WORKER_IMAGE", 280
    DEPENDENCY = "DEPENDENCY", 281

    RICH_TEXT = "RICH_TEXT", 300
    RICH_TEXT_SPAN = "RICH_TEXT_SPAN", 301

    # workspace
    # SPACE = "SPACE", 400 # should be a node?

    @property
    def bench_name(self):
        return BENCH_TYPE_NAME[self]


STRUCT_TYPES: tuple[StructType, ...] = tuple(StructType)

if typing.TYPE_CHECKING:
    BenchType = NodeType | StructType
else:
    BenchType = ProtoStrEnum(
        "BenchType",
        {bt.name: (bt.name, bt.id) for bt in chain(NodeType, StructType)},
    )
    BenchType.bench_name = NodeType.bench_name

BENCH_TYPES: tuple[BenchType, ...] = tuple(BenchType)


def to_bench_metatype(_type: typing.Union[BenchType, int]) -> BenchType:
    from bench.proto.wire import BenchType as WireBenchType

    if isinstance(_type, WireBenchType):
        assert _type.name != "UNSPECIFIED", f"cannot convert unspecified type {_type!r}"
        return BenchType[_type.name]
    return _type


BENCH_TYPE_NAME: dict[NodeType | StructType, str] = {
    _type: to_casing(_type, Casing.CAMEL) for _type in chain(NodeType, StructType)
}
INTERP_NODE_TYPES = {NodeType.ISSUE}


class BlockType(ProtoStrEnum):
    BOX = "box", 1  # group of blocks
    BLANK = "blank", 2  # placeholder/spacer
    TEXT = "text", 3  # define a 'paragraph' of text/comment/instruction/etc.
    SINGLE_VARIABLE = "single_variable", 4  # define a single-value variable
    MULTI_VARIABLE = "multi_variable", 5  # define a variable with (multiple) fields
    LINK = "link", 6  # an explicit link to another block/node
    ALIAS = "alias", 7  # extend an existing non-class block (kind of like a 'newtype')

    CLASS = "class", 10  # define a class type with fields
    CHOICE = "choice", 11  # define a choice type with fields
    TAG = "tag", 12  # define a tag with fields
    SIGNAL = "signal", 13  # define a signal type with fields

    TASK = "task", 20  # define a task with fields
    CODE = "code", 21  # define a code block / function with fields
    FLOW = "flow", 22  # define a flow with steps and fields
    MODEL = "model", 23  # define a model 'function' with fields

    VIEW = "view", 30  # define a set of views
    DATABASE = "database", 31  # define a database with views
    SCREEN = "screen", 32  # define a screen with tiles

    # ROLE = "role", 40  # define a role with policies
    # IDENTITY = "identity", 41  # define an identity with roles

    @property
    def bench_name(self):
        return to_casing(self.name, Casing.CAMEL)


RUNNABLE_BLOCK_TYPES = {BlockType.CODE, BlockType.MODEL, BlockType.TASK, BlockType.FLOW}


class NodeSource(ProtoStrEnum):
    PERSISTED = "PERSISTED", 1
    INTERP = "INTERP", 2
    LOCAL = "LOCAL", 3


class NodeVisibility(ProtoStrEnum):
    PRIVATE = "PRIVATE", 1
    INTERNAL = "INTERNAL", 2
    PUBLIC = "PUBLIC", 3


DYNAMIC_NODE_KEY_LENGTH = 8


def new_dynamic_node_key(ck_or_id: UUID) -> str:
    """
    Gets a 'random' alphabetic key as a persistent key for a node.
    Also used for dynamic database identities (versioned/un-versioned).
    (short key length alphabetic characters) :FieldKeys
    """
    hash_value = cyrb53a(str(ck_or_id))
    key = ""
    while len(key) < DYNAMIC_NODE_KEY_LENGTH:
        hash_value, remainder = divmod(hash_value, 52)
        if remainder < 26:
            key += chr(ord("a") + remainder)
        else:
            key += chr(ord("A") + remainder - 26)
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
    SOURCE = 0
    INTERP = 1
    ACTIVE = 2


NS = NodeStatus
UNSET = object()


#
# Edits
#


class ReadKind(ProtoStrEnum):
    READ = "READ", 1  # any read action
    LIST = "LIST", 2  # list, search, filter, etc.
    AGGREGATE = "AGGREGATE", 3  # count, sum, group, min, etc.


class EditKind(ProtoStrEnum):
    CREATE = "CREATE", 20  # start at 20, so we can have read 'actions' as well
    UPSERT = "UPSERT", 21
    UPDATE = "UPDATE", 22
    MOVE = "MOVE", 23
    BUMP = "BUMP", 24
    SOFT_DELETE = "SOFT_DELETE", 25
    RESTORE = "RESTORE", 26
    ARCHIVE = "ARCHIVE", 27
    UNARCHIVE = "UNARCHIVE", 28
    DELETE = "DELETE", 29


class RunKind(ProtoStrEnum):
    START = "START", 40
    PAUSE = "PAUSE", 41
    RESUME = "RESUME", 42
    KILL = "KILL", 43


if typing.TYPE_CHECKING:
    ActionKind = ReadKind | EditKind | RunKind
else:
    ActionKind = ProtoStrEnum(
        "ActionKind",
        {ak.name: (ak.name, ak.id) for ak in chain(ReadKind, EditKind, RunKind)},
    )


#
# Other stuff
#


class BenchStatus(ProtoStrEnum):
    RESERVED = "RESERVED", 1
    PREPARING = "PREPARING", 2
    AVAILABLE = "AVAILABLE", 3
    MIGRATING = "MIGRATING", 4


class BadgeType(ProtoStrEnum):
    SHARING_LINK = "SHARING_LINK", 1
    ACCESS_KEY = "ACCESS_KEY", 2


class PolicyEffect(ProtoStrEnum):
    ALLOW = "ALLOW", 1
    DENY = "DENY", 2


class NotificationKind(ProtoStrEnum):
    pass


class NotificationStatus(ProtoStrEnum):
    ACTIVE = "ACTIVE", 1
    READ = "READ", 2
    EXPIRED = "EXPIRED", 3


# ColumnType is pulled out from sql/core because it's also used in our type system
class ColumnType(ProtoStrEnum):
    """
    Fundamental column / storage types we support (subset of SQL types).
    NOTE: the ids here are used in encode/decode pipelines, take extra care.
    """

    STRING = "String", 1
    BOOLEAN = "Boolean", 2
    INT = "Int", 3  # range: -2147483648 to 2147483647
    BIGINT = "BigInt", 4  # range: -9223372036854775808 to 9223372036854775807
    FLOAT = "Float", 5
    DATETIME = "DateTime", 6
    INTERVAL = "Interval", 7
    JSON = "Json", 8
    BINARY = "Binary", 9
    VECTOR = "Vector", 10
    UUID = "UUID", 11
    BYTES = "Bytes", 12


class FormatHint(ProtoStrEnum):
    """Extra semantic hint for types."""

    # string
    NAME = "name", 1
    UUID = "uuid", 2
    EMAIL = "email", 3
    URL = "url", 4
    MARKDOWN = "markdown", 5
    CODE = "code", 6
    # number
    PHONE = "phone", 10
    RATING = "rating", 11
    SLIDER = "slider", 12
    # boolean
    TOGGLE = "toggle", 20
    CHECKBOX = "checkbox", 21
    THUMBS = "thumbs", 22
    # files
    IMAGE = "image", 30
    VIDEO = "video", 31
    AUDIO = "audio", 32


class RichTextFlag(enum.IntFlag):
    NONE = 0
    BOLD = 2**0
    ITALIC = 2**1
    UNDERLINE = 2**2
    STRIKETHROUGH = 2**3


class BlobStatus(ProtoStrEnum):
    PENDING = "PENDING", 1
    UPLOADING = "UPLOADING", 2
    AVAILABLE = "AVAILABLE", 3


class TriggerType(ProtoStrEnum):
    """Triggers for blocks (for both actual runs and pre-defined triggers)."""

    INVOKE = "invoke", 1
    TIME = "time", 2
    RUN = "run", 3
    EDIT = "edit", 4
    MESSAGE = "message", 5
    USER = "user", 6
    API = "api", 7


class ScheduleType(ProtoStrEnum):
    """Schedules for blocks."""

    INTERVAL = "interval", 1
    CRON = "cron", 2


class IssueKind(ProtoStrEnum):
    ERROR = "Error", 1
    WARNING = "Warning", 2
    NOTICE = "Notice", 3


class IssueType(ProtoStrEnum):
    # errors
    INTERNAL = "INTERNAL", 1
    MISSING_REFERENCE = "MISSING_REFERENCE", 2
    CIRCULAR_BASE = "CIRCULAR_BASE", 3
    MISMATCHED_BASE = "MISMATCHED_BASE", 4
    # warnings
    AMBIGUOUS_DEFINITION = "AMBIGUOUS_DEFINITION", 100
    TASK_MISSING_IO = "TASK_MISSING_IO", 101
    # notices
    BAD_NAME = "BAD_NAME", 200
    TASK_IS_STATIC = "TASK_IS_STATIC", 201


class BenchRegion(ProtoStrEnum):
    EU_CENTRAL = "EU_CENTRAL", 1
    US_WEST = "US_WEST", 10


class WorkerSetStatus(ProtoStrEnum):
    SLEEPING = "SLEEPING", 1
    PENDING = "PENDING", 2
    UPDATING = "UPDATING", 3
    HEALTHY = "HEALTHY", 4
    UNHEALTHY = "UNHEALTHY", 5
    UNAVAILABLE = "UNAVAILABLE", 6
    UNKNOWN = "UNKNOWN", 7


class SessionStatus(ProtoStrEnum):
    ACTIVE = "ACTIVE", 1
    SUSPENDED = "SUSPENDED", 2
    TERMINATED = "TERMINATED", 3


class RunStatus(ProtoStrEnum):
    SCHEDULED = "Scheduled", 1
    QUEUED = "Queued", 2
    RUNNING = "Running", 3
    HALTED = "Halted", 4
    ABORTING = "Aborting", 5
    # terminal statuses
    CANCELLED = "Cancelled", 6
    ABORTED = "Aborted", 7
    FAILED = "Failed", 8
    COMPLETED = "Completed", 9


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
    RunStatus.HALTED,
    RunStatus.ABORTING,
}
ACTIVE_RUN_STATUSES = {RunStatus.QUEUED, RunStatus.RUNNING, RunStatus.HALTED, RunStatus.ABORTING}


class RunErrorKind(ProtoStrEnum):
    Internal = "Internal", 1
    Parse = "Parse", 2
    Validation = "Validation", 3
    Runtime = "Runtime", 4
    Untrusted = "Untrusted", 5


class WorkerProfile(ProtoStrEnum):
    TINY = "TINY", 1
    SMALL = "SMALL", 2
    MEDIUM = "MEDIUM", 3


class ExpressionKind(ProtoStrEnum):
    CONDITIONAL = "CONDITIONAL", 1
    SORT = "SORT", 2
    AGGREGATION = "AGGREGATION", 3


class ConditionalOp(ProtoStrEnum):
    # logical
    TRUE = "TRUE", 1
    FALSE = "FALSE", 2
    NOT = "NOT", 3
    AND = "AND", 4
    OR = "OR", 5
    # comparison
    EQUALS = "EQUALS", 10
    NOT_EQUALS = "NOT_EQUALS", 11
    GREATER_THAN = "GREATER_THAN", 12
    GREATER_THAN_OR_EQUALS = "GREATER_THAN_OR_EQUALS", 13
    LESS_THAN = "LESS_THAN", 14
    LESS_THAN_OR_EQUALS = "LESS_THAN_OR_EQUALS", 15
    # string comparison
    MATCHES = "MATCHES", 20
    STARTS_WITH = "STARTS_WITH", 21
    REGEX = "REGEX", 22
    # containment
    CONTAINS = "CONTAINS", 30
    NOT_CONTAINS = "NOT_CONTAINS", 31
    IN = "IN", 32
    NOT_IN = "NOT_IN", 33
    # existence
    EXISTS = "EXISTS", 40
    NOT_EXISTS = "DOES_NOT_EXIST", 41
    # vector
    NEAR = "NEAR", 50


_CONDITIONAL_OP_SIGN: dict[ConditionalOp, str] = {
    # logical
    ConditionalOp.NOT: "~",
    ConditionalOp.AND: "&",
    ConditionalOp.OR: "|",
    # comparison
    ConditionalOp.EQUALS: "==",
    ConditionalOp.NOT_EQUALS: "!=",
    ConditionalOp.GREATER_THAN: ">",
    ConditionalOp.GREATER_THAN_OR_EQUALS: ">=",
    ConditionalOp.LESS_THAN: "<",
    ConditionalOp.LESS_THAN_OR_EQUALS: "<=",
    # string comparison
    ConditionalOp.MATCHES: "~=",
    ConditionalOp.STARTS_WITH: "^=",
    ConditionalOp.REGEX: "$re=",
    # containment
    ConditionalOp.CONTAINS: "∋",
    ConditionalOp.NOT_CONTAINS: "!∋",
    ConditionalOp.IN: "∈",
    ConditionalOp.NOT_IN: "!∈",
    # existence
    ConditionalOp.EXISTS: "?",
    ConditionalOp.NOT_EXISTS: "!?",
    # vector
    ConditionalOp.NEAR: "~=",
}


class AggregationOp(ProtoStrEnum):
    EXISTS = "EXISTS", 101
    COUNT = "COUNT", 102
    SUM = "SUM", 103
    AVERAGE = "AVERAGE", 104
    MIN = "MIN", 105
    MAX = "MAX", 106
    MEDIAN = "MEDIAN", 107
    HISTOGRAM = "HISTOGRAM", 108


class SortOp(ProtoStrEnum):
    ASCENDING = "ASCENDING", 201
    DESCENDING = "DESCENDING", 202


class QueryEngine(ProtoStrEnum):
    IN_MEMORY = "IN_MEMORY", 1
    GLOBAL_POSTGRES = "GLOBAL_PG", 2
    GLOBAL_OPENSEARCH = "GLOBAL_OS", 3
    LOCAL_POSTGRES = "LOCAL_PG", 4
    LOCAL_OPENSEARCH = "LOCAL_OS", 5


class SortMode(ProtoStrEnum):
    MAX = "MAX", 1
    MIN = "MIN", 2
    AVERAGE = "AVERAGE", 3
    SUM = "SUM", 4
    MEDIAN = "MEDIAN", 5


if typing.TYPE_CHECKING:
    ExpressionOp = ConditionalOp | AggregationOp | SortOp
else:
    ExpressionOp = ProtoStrEnum(
        "ExpressionOp",
        {op.name: (op.name, op.id) for op in chain(ConditionalOp, AggregationOp, SortOp)},
    )

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

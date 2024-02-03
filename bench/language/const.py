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
from bench.utils.utils import frozendict

if typing.TYPE_CHECKING:
    from bench.language import Bench, Block, Node, Package, Session  # noqa: F401

# hard-coded, do not change ever :BenchUuidNamespace
UUID_NAMESPACE = UUID("d822dab7-41ad-4706-a9c8-4379e15b2ed0")
VERSION = "2024.02.03.4"
UNSET = object()
EMPTY_LIST: list = []
EMPTY_SET: frozenset = frozenset()
EMPTY_DICT: typing.Mapping = frozendict()


#
# Metatypes
#


class NodeType(ProtoStrEnum):
    # root
    BENCH = "BENCH", 1
    # UNIVERSE = "UNIVERSE", 2
    # PLACE = "PLACE", 3
    # BRANCH = "BRANCH", 4

    # source
    PACKAGE = "PACKAGE", 20
    BLOCK = "BLOCK", 21
    TRIGGER = "TRIGGER", 22
    TAG = "TAG", 23
    FIELD = "FIELD", 24
    RECORD = "RECORD", 25  # (local)
    QUERY = "QUERY", 26
    VIEW = "VIEW", 27
    # STEP = "STEP", 28
    NOTICE = "NOTICE", 29
    LINK = "LINK", 30
    SKIP = "SKIP", 31
    # COMMENT = "COMMENT", ...
    # REACTION = "REACTION", ...
    SPACE = "SPACE", 40
    # DEPENDENCY = "DEPENDENCY", ...
    # UPGRADE = "UPGRADE", ...

    # session
    SESSION = "SESSION", 50  # (local)
    RUN = "RUN", 51  # (local)
    PAUSE = "PAUSE", 52  # (local)
    SIGNAL = "SIGNAL", 53  # (local)

    # auth
    BADGE = "BADGE", 60
    ROLE = "ROLE", 61
    IDENTITY = "IDENTITY", 62

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

    # INVITE = "INVITE", 140
    MEMBERSHIP = "MEMBERSHIP", 141

    @property
    def bench_name(self):
        return BENCH_TYPE_NAME[self]


NODE_TYPES: tuple[NodeType, ...] = tuple(NodeType)
# (we duplicate in-package/in-bench info here to access it while initialising the node classes,
#  but we check for consistency during finalization)
IN_PACKAGE_NODE_TYPES: tuple[NodeType, ...] = tuple(nt for nt in NODE_TYPES if 20 <= nt.id < 80) + (
    NodeType.SPACE,
)
SUB_PACKAGE_NODE_TYPES: tuple[NodeType, ...] = tuple(
    nt for nt in IN_PACKAGE_NODE_TYPES if nt != NodeType.PACKAGE
)
IN_BENCH_NODE_TYPES: tuple[NodeType, ...] = tuple(nt for nt in NODE_TYPES if nt.id < 100) + (
    NodeType.MEMBERSHIP,
    NodeType.CLIENT,
    NodeType.SPACE,
)
SUB_BENCH_NODE_TYPES: tuple[NodeType, ...] = tuple(
    nt for nt in IN_BENCH_NODE_TYPES if nt != NodeType.BENCH
)


class StructType(ProtoStrEnum):
    # starts at 100 to avoid collisions with NodeType (BenchType combines both in one metatype)
    BENCH_PATH = "BENCH_PATH", 200
    NODE_REFERENCE = "NODE_REFERENCE", 201
    PROPERTY_REFERENCE = "PROPERTY_REFERENCE", 202
    PROPERTY_PATH = "PROPERTY_PATH", 203
    FIELD_PATH = "FIELD_PATH", 204
    FIELD_PATH_SEGMENT = "FIELD_PATH_SEGMENT", 205
    VALUE_REFERENCE = "VALUE_REFERENCE", 206
    VALUE_SELECTION = "VALUE_SELECTION", 207

    TYPE_INFO = "TYPE_INFO", 210

    FILE = "FILE", 220
    ICON = "ICON", 221

    POLICY = "POLICY", 230
    POLICY_RULE = "POLICY_RULE", 231
    REQUEST_SUBJECT = "REQUEST_SUBJECT", 232
    REQUEST_OBJECT = "REQUEST_OBJECT", 233
    ACCESS_ZONE = "ACCESS_ZONE", 234
    ACCESS_MATRIX = "ACCESS_MATRIX", 235
    REQUEST = "REQUEST", 236
    ACTION = "ACTION", 237
    REQUEST_TRACE = "REQUEST_TRACE", 238
    ACTION_TRACE = "ACTION_TRACE", 239
    ...
    READ_OPTIONS = "READ_OPTIONS", 250

    EXPRESSION = "EXPRESSION", 260
    AGGREGATION = "AGGREGATION", 261
    AGGREGATION_BUCKET = "AGGREGATION_BUCKET", 262

    LOG_ENTRY = "LOG_ENTRY", 290
    RUN_CODE_FRAME = "RUN_CODE_FRAME", 291
    RUN_ERROR = "RUN_ERROR", 292
    # CURSOR = "CURSOR", 293

    WORKER_IMAGE = "WORKER_IMAGE", 320
    DEPENDENCY = "DEPENDENCY", 321

    RICH_TEXT = "RICH_TEXT", 350
    RICH_TEXT_SPAN = "RICH_TEXT_SPAN", 351

    # views
    SPACE_DOCK = "SPACE_DOCK", 500
    SPACE_DOCK_ITEM = "SPACE_DOCK_ITEM", 501

    # ...

    # shapes
    # ...

    # workspace
    # ...

    @property
    def bench_name(self):
        return BENCH_TYPE_NAME[self]


STRUCT_TYPES: tuple[StructType, ...] = tuple(StructType)

if typing.TYPE_CHECKING:
    BenchType = NodeType | StructType
else:
    BenchType = ProtoStrEnum(
        "BenchType",
        {bt.name: (bt.name, bt.id) for bt in chain(NODE_TYPES, STRUCT_TYPES)},
    )
    BenchType.bench_name = NodeType.bench_name

BENCH_TYPES: tuple[BenchType, ...] = tuple(BenchType)
BENCH_TYPE_NAME: dict[NodeType | StructType, str] = {
    _type: to_casing(_type, Casing.CAMEL) for _type in chain(NODE_TYPES, STRUCT_TYPES)
}
INTERP_NODE_TYPES = (NodeType.NOTICE,)


class BlockType(ProtoStrEnum):
    PAGE = "page", 1  # group of blocks
    BLANK = "blank", 2  # placeholder/spacer
    TEXT = "text", 3  # define a 'paragraph' of text/comment/instruction/etc.
    ALIAS = "alias", 4  # refer to / extend an existing block or builtin (like a 'newtype')

    CLASS = "class", 10  # define a class type with fields
    CHOICE = "choice", 11  # define a choice type with fields
    TAG = "tag", 12  # define a tag type with fields
    SIGNAL = "signal", 13  # define a signal type with fields
    # NOTICE = "notice", ...  # define a new notice type
    # NOTIFICATION = "notification", ...  # define a new notification type
    # BLOCK = "block", ...  # define a new block type?

    SINGLE_VARIABLE = "single_variable", 20  # define a single-value variable
    MULTI_VARIABLE = "multi_variable", 21  # define a variable with (multiple) fields

    TASK = "task", 30  # define a task function with fields (incl. input/output)
    ROUTINE = "routine", 31  # define a code function with input/output fields (incl. input/output)
    SCRIPT = "script", 32  # define a code script with fields
    FLOW = "flow", 33  # define a flow with steps and fields (optionally incl. input/output)
    MODEL = "model", 34  # define a model 'function' with input/output fields (incl. input/output)

    QUERY = "query", 40  # define a set of queries
    DATABASE = "database", 41  # define a database with queries
    SCREEN = "screen", 42  # define a screen with views

    ROLE = "role", 50  # define a role with policies
    IDENTITY = "identity", 51  # define an identity with roles & policies

    @property
    def bench_name(self):
        return to_casing(self.name, Casing.CAMEL)

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
    (short key length alphabetic characters)
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
    SOURCE = 0  # just loaded
    INTERP = 1  # everything resolved & ready
    TRACKED = 2  # live in a session


NS = NodeStatus


#
# Actions
# Action types are loosely ranked by access/destructiveness across and within types.
#


class ReadType(ProtoStrEnum):
    """A type of Read action on nodes."""

    GET = "GET", 1  # any direct read action
    AGGREGATE_SCALAR = "AGGREGATE", 2  # count, sum, min, etc.
    AGGREGATE_BUCKET = "AGGREGATE_BUCKET", 3  # histogram, etc.
    LIST = "LIST", 4  # list, search, filter, etc.

    @property
    def bench_name(self):
        return to_casing(self.name, Casing.CAMEL)

    @property
    def kind(self) -> ActionKind:
        return ActionKind.READ


class EditType(ProtoStrEnum):
    """A type of Edit action on nodes."""

    BUMP_CHANGED = "BUMP_CHANGED", 10
    BUMP_ACTIVE = "BUMP_ACTIVE", 11
    CREATE = "CREATE", 12
    UPSERT = "UPSERT", 13
    UPDATE = "UPDATE", 14
    MOVE = "MOVE", 15
    ARCHIVE = "ARCHIVE", 16
    UNARCHIVE = "UNARCHIVE", 17
    SOFT_DELETE = "SOFT_DELETE", 18
    RESTORE = "RESTORE", 19
    DELETE = "DELETE", 20

    @property
    def bench_name(self):
        return to_casing(self.name, Casing.CAMEL)

    @property
    def kind(self) -> ActionKind:
        return ActionKind.EDIT


class RunType(ProtoStrEnum):
    """A type of Run action on nodes."""

    START = "START", 25
    PAUSE = "PAUSE", 26
    RESUME = "RESUME", 37
    KILL = "KILL", 28
    # ideally <32 so we can bitpack into a single int

    @property
    def bench_name(self):
        return to_casing(self.name, Casing.CAMEL)

    @property
    def kind(self) -> ActionKind:
        return ActionKind.RUN


READ_TYPES: tuple[ReadType, ...] = tuple(ReadType)
EDIT_TYPES: tuple[EditType, ...] = tuple(EditType)
RUN_TYPES: tuple[RunType, ...] = tuple(RunType)

if typing.TYPE_CHECKING:
    ActionType = ReadType | EditType | RunType
else:
    ActionType = ProtoStrEnum(
        "ActionType",
        {ak.name: (ak.name, ak.id) for ak in chain(READ_TYPES, EDIT_TYPES, RUN_TYPES)},
    )
    ActionType.bench_name = ReadType.bench_name
    ActionType.kind = property(lambda self: KIND_BY_ACTION[self])

ACTION_TYPES: tuple[ActionType, ...] = tuple(ActionType)


class ActionKind(ProtoStrEnum):
    # NOTE: ActionType ids and their overlap with ActionKind is important for our masks.
    READ = "READ", 1
    EDIT = "EDIT", 10
    RUN = "RUN", 25

    @property
    def from_id(self):
        return ACTION_CLASS_BY_KIND[self].get_min_id()

    @property
    def to_id(self):
        return ACTION_CLASS_BY_KIND[self].get_max_id()

    @property
    def bench_name(self):
        return to_casing(self.name, Casing.CAMEL)


ACTION_KINDS = tuple(ActionKind)
ACTIONS_BY_KIND: dict[ActionKind, set[ActionType]] = {
    ActionKind.READ: set(READ_TYPES),
    ActionKind.EDIT: set(EDIT_TYPES),
    ActionKind.RUN: set(RUN_TYPES),
}
ACTION_CLASS_BY_KIND: dict[ActionKind, type[ActionType]] = {
    ActionKind.READ: ReadType,
    ActionKind.EDIT: EditType,
    ActionKind.RUN: RunType,
}
KIND_BY_ACTION: dict[ActionType, ActionKind] = {
    action: kind for kind, actions in ACTIONS_BY_KIND.items() for action in actions
}


#
# Other stuff
#


class BenchStatus(ProtoStrEnum):
    PREPARING = "PREPARING", 1
    MIGRATING = "MIGRATING", 2
    AVAILABLE = "AVAILABLE", 3


class BadgeType(ProtoStrEnum):
    SHARING_LINK = "SHARING_LINK", 1
    ACCESS_KEY = "ACCESS_KEY", 2


class PolicyEffect(ProtoStrEnum):
    ALLOW = "ALLOW", 1
    DENY = "DENY", 2


class ClientKind(ProtoStrEnum):
    USER = "USER", 1
    WORKER = "WORKER", 2


class NotificationKind(ProtoStrEnum):
    pass


class PrimitiveType(ProtoStrEnum):
    """
    Fundamental column / storage types we support (subset of SQL types, used directly in sql/core).
    NOTE: the ids here are used in encode/decode pipelines, take extra care.
    """

    BOOLEAN = "Boolean", 1
    INT32 = "Int32", 2  # range: -2147483648 to 2147483647
    INT64 = "Int64", 3  # range: -9223372036854775808 to 9223372036854775807
    FLOAT32 = "Float32", 4  # range: 1.175494351e-38 to 3.402823466e+38
    FLOAT64 = "Float64", 5  # range: 2.2250738585072014e-308 to 1.7976931348623157e+308
    DECIMAL = "Decimal", 6  # numeric(precision, scale)
    STRING = "String", 7
    DATETIME = "DateTime", 8
    INTERVAL = "Interval", 9
    JSON = "Json", 10
    BYTES = "Bytes", 11
    VECTOR = "Vector", 12
    UUID = "UUID", 13


class FormatHint(ProtoStrEnum):
    """Extra semantic hint for types."""

    # string
    TITLE = "title", 1
    EMAIL = "email", 2
    URL = "url", 3
    MARKDOWN = "markdown", 4
    CODE = "code", 5
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


class FileStatus(ProtoStrEnum):
    PENDING = "PENDING", 1
    UPLOADING = "UPLOADING", 2
    AVAILABLE = "AVAILABLE", 3


class TriggerType(ProtoStrEnum):
    """Triggers for blocks (for both actual runs and pre-defined triggers)."""

    INVOKE = "invoke", 1
    TIME = "time", 2
    SIGNAL = "signal", 3
    API = "api", 4


class ScheduleType(ProtoStrEnum):
    """Schedules for blocks."""

    INTERVAL = "interval", 1
    CRON = "cron", 2


class NoticeKind(ProtoStrEnum):
    """Type of diagnostic in increasing severity."""

    HINT = "HINT", 1
    INFORMATION = "NOTICE", 2
    WARNING = "WARNING", 3
    ERROR = "ERROR", 4

    @property
    def bench_name(self):
        return to_casing(self.name, Casing.CAMEL)


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

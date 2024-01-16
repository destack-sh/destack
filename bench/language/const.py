from __future__ import annotations

import enum
import re
import typing
from itertools import chain
from typing import NamedTuple
from uuid import UUID

from bench.proto.core import ProtoStrEnum
from bench.utils.func import cyrb53a
from bench.utils.utils import IdentifierType, to_pyidentifier

if typing.TYPE_CHECKING:
    from bench.language import Node, Statement  # noqa: F401

# hard-coded, do not change ever :BenchUuidNamespace
UUID_NAMESPACE = UUID("d822dab7-41ad-4706-a9c8-4379e15b2ed0")
VERSION = "2024.01.15.1"


#
# Metatypes
#


class NodeType(ProtoStrEnum):
    # root
    BENCH = "BENCH", 1
    # ENVIRONMENT = "ENVIRONMENT", 2
    # PLACE = "PLACE", 3
    # BRANCH = "BRANCH", 4

    # module/source
    MODULE = "MODULE", 10
    FILE = "FILE", 11
    STATEMENT = "STATEMENT", 12
    TRIGGER = "TRIGGER", 13
    TAGGING = "TAGGING", 14
    FIELD = "FIELD", 15
    RECORD = "RECORD", 16  # (local)
    VIEW = "VIEW", 17
    # TILE = "TILE", 18
    ISSUE = "ISSUE", 19
    LINK = "LINK", 20

    # bench-level
    BLOB = "BLOB", 40
    SECRET = "SECRET", 41

    # session (all local)
    SESSION = "SESSION", 50
    RUN = "RUN", 51
    HALT = "HALT", 52
    SIGNAL = "SIGNAL", 53

    # worker
    WORKER_SET = "WORKER_SET", 60
    WORKER = "WORKER", 61
    # WORKER_PROCESS = "WORKER_PROCESS", 62

    # user
    HANDLE = "HANDLE", 100
    USER = "USER", 101
    ORGANIZATION = "ORGANIZATION", 102
    CLIENT = "CLIENT", 103
    NOTIFICATION = "NOTIFICATION", 104
    BADGE = "BADGE", 105

    # INVITE = "INVITE", 110
    # MEMBERSHIP = "MEMBERSHIP", 111
    # ROLE = "ROLE", 112
    # COMMENT = "COMMENT", 130

    @property
    def camel_name(self):
        return BENCH_TYPE_CAMEL_CASE[self]


NODE_TYPES: tuple[NodeType, ...] = tuple(NodeType)
# (we duplicate in-module/in-bench info here to access it while initialising the node classes,
#  but we check for consistency during finalization)
IN_MODULE_NODE_TYPES: tuple[NodeType, ...] = tuple(
    nt
    for nt in NODE_TYPES
    if NodeType.MODULE.id <= nt.id < NodeType.WORKER_SET.id
    and nt not in (NodeType.BLOB, NodeType.SECRET)
)
IN_BENCH_NODE_TYPES: tuple[NodeType, ...] = tuple(
    nt
    for nt in NODE_TYPES
    if NodeType.BENCH.id <= nt.id < NodeType.HANDLE.id or nt in (NodeType.BADGE,)
)


class StructType(ProtoStrEnum):
    # starts at 100 to avoid collisions with NodeType (BenchType combines both in one metatype)
    POLICY = "POLICY", 200
    POLICY_RULE = "POLICY_RULE", 201
    EXPRESSION = "EXPRESSION", 210
    AGGREGATION = "AGGREGATION", 211
    AGGREGATION_BUCKET = "AGGREGATION_BUCKET", 212
    NODE_POINTER = "NODE_POINTER", 213
    PROPERTY_POINTER = "PROPERTY_POINTER", 214
    LOG_ENTRY = "LOG_ENTRY", 220
    RUN_CODE_FRAME = "RUN_CODE_FRAME", 221
    RUN_ERROR = "RUN_ERROR", 222
    MINI_RUN = "MINI_RUN", 223
    # CURSOR = "CURSOR", 224
    WORKER_IMAGE = "WORKER_IMAGE", 230
    DEPENDENCY = "DEPENDENCY", 231

    @property
    def camel_name(self):
        return BENCH_TYPE_CAMEL_CASE[self]


STRUCT_TYPES: tuple[StructType, ...] = tuple(StructType)

if typing.TYPE_CHECKING:
    BenchType = NodeType | StructType
else:
    BenchType = ProtoStrEnum(
        "BenchType",
        {bt.name: (bt.name, bt.id) for bt in chain(NodeType, StructType)},
    )
    BenchType.camel_name = NodeType.camel_name


def to_bench_metatype(_type: typing.Union[BenchType, int]) -> BenchType:
    from bench.proto.wire import BenchType as WireBenchType

    if isinstance(_type, WireBenchType):
        assert _type.name != "UNSPECIFIED", f"cannot convert unspecified type {_type!r}"
        return BenchType[_type.name]
    return _type


BENCH_TYPE_CAMEL_CASE: dict[NodeType | StructType, str] = {
    _type: to_pyidentifier(_type, IdentifierType.TYPE) for _type in chain(NodeType, StructType)
}
INTERP_NODE_TYPES = {NodeType.ISSUE}


class NodeSource(enum.IntEnum):
    PERSISTED = 1
    INTERP = 2
    LOCAL = 3


class NodeVisibility(enum.IntEnum):
    PUBLIC = 1
    INTERNAL = 2
    PRIVATE = 4


#
# Edits
#


class ReadKind(ProtoStrEnum):
    READ = "READ", 1  # any read action
    LIST = "LIST", 2  # list, search, filter, etc.
    AGGREGATE = "AGGREGATE", 3  # count, sum, group, min, etc.


class EditKind(ProtoStrEnum):
    CREATE = "CREATE", 10  # start at 10, so we can have read 'actions' as well
    UPSERT = "UPSERT", 11
    UPDATE = "UPDATE", 12
    MOVE = "MOVE", 13
    BUMP = "BUMP", 14
    SOFT_DELETE = "SOFT_DELETE", 15
    RESTORE = "RESTORE", 16
    ARCHIVE = "ARCHIVE", 17
    UNARCHIVE = "UNARCHIVE", 18
    DELETE = "DELETE", 19


class RunKind(ProtoStrEnum):
    START = "START", 20
    PAUSE = "PAUSE", 21
    RESUME = "RESUME", 22
    KILL = "KILL", 23


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


class BadgeType(ProtoStrEnum):
    SHARING_LINK = "SHARING_LINK", 1
    ACCESS_KEY = "ACCESS_KEY", 2


class PolicyEffect(ProtoStrEnum):
    ALLOW = "ALLOW", 1
    DENY = "DENY", 2


class NotificationType(ProtoStrEnum):
    EDIT = "EDIT", 1


class NotificationStatus(ProtoStrEnum):
    ACTIVE = "ACTIVE", 1
    READ = "READ", 2
    EXPIRED = "EXPIRED", 3
    ARCHIVED = "ARCHIVED", 4


class StatementType(ProtoStrEnum):
    TAG = "tag", 1
    TEXT = "text", 2
    BLANK = "blank", 3
    CLASS = "class", 4
    CHOICE = "choice", 5
    SIGNAL = "signal", 6
    TASK = "task", 7
    CODE = "code", 8
    FLOW = "flow", 9
    MODEL = "model", 10
    VARIABLE = "variable", 11
    DATABASE = "database", 12
    VIEW = "view", 13
    SCREEN = "screen", 14

    @property
    def camel_name(self):
        return to_pyidentifier(self.name, IdentifierType.TYPE)


RUNNABLE_STATEMENT_TYPES = {
    StatementType.CODE,
    StatementType.MODEL,
    StatementType.TASK,
    StatementType.FLOW,
}

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


class LookupBy(ProtoStrEnum):
    Name = "Name", 1
    PyIdent = "PyIdent", 2


class NodeRelationType(enum.IntEnum):
    """Parent relation between node and descendants."""

    Default = 0  # default inline relation
    Remote = 2**0  # not inline: Statement->Record, ...
    Shared = 2**1  # across versions: Statement->Comment, Statement[versioned=False]->Record, ...
    Flat = 2**2  # flattened inner hierarchy: Module->File, File->Statement, ...
    Cumulative = 2**3  # sum of descendants: Module->Issue, File->Issue, ...
    Named = 2**4  # indexed by name: Module->File, File->Statement, ...
    Scoped = 2**5  # scoped by name: Module->File, File->Statement, ...
    Keyed = 2**6  # indexed by key: File->Tagging, Statement->Tagging, ...
    Ordered = 2**7  # ordered: File->Statement, Statement->Field, ...


NRel = NodeRelationType


class NodeStatus(enum.IntEnum):
    SOURCE = 0
    INDEX = 1
    INTERP = 2
    ACTIVE = 3


NS = NodeStatus
UNSET = object()

ModuleReference = typing.NamedTuple(
    "ModuleReference", [("name", str), ("version", str), ("id", typing.Optional[UUID])]
)
NodePath = NamedTuple("NodePath", [("path", str), ("name", str)])
StatementReference = typing.Union["Statement", NodePath, UUID]
NodeReference = typing.Union["Node", NodePath, UUID]
TypedNodeReference = NamedTuple("TypedNodeReference", [("type", NodeType), ("ref", UUID)])
NODE_REFERENCE_REGEX = re.compile(
    r"^((?P<module_owner>[\w\- ]+)\.(?P<module_name>[\w\- ]+))?\.(?P<path>[\w.\- ]+)"
)


def parse_absolute_node_reference(path: str) -> tuple[str, str]:
    match = NODE_REFERENCE_REGEX.match(path)
    if not match:
        raise ValueError(f"invalid absolute node reference: {path}")
    module_name = match.group("module_owner") + "." + match.group("module_name")
    localized_path = "." + match.group("path")
    return module_name, localized_path


def parse_node_path(node_path: str) -> "NodePath":
    if "." not in node_path:
        return NodePath(".", node_path)
    path, name = node_path.rsplit(".", 1)
    return NodePath(path, name)


def node_path_as_str(node_path: "NodePath") -> str:
    return f"{node_path.path}:{node_path.name}"


class TextHeadingLevel(enum.IntEnum):
    """Classic headings big to small."""

    H1 = 1
    H2 = 2
    H3 = 3


# TODO @Architecture: :SimpleTypes ... TypeTag/TypeHint/TypeFlag/TypeStorageFormat mess
#  (should probably? just be type + flags as properties on Field + display hint Field)
#  IS_ARRAYABLE -> real unions of X | list[X] or whatever
#  IS_UNION_WITH -> inherit? from type (could remain flag, though that's not great)


class TypeTag(ProtoStrEnum):
    """The Bench primitive type of a field/type."""

    __RESERVED_IDS__ = {5}
    __RESERVED_NAMES__ = {"blob"}

    STRING = "string", 1
    NUMBER = "number", 2
    BOOLEAN = "boolean", 3
    VECTOR = "vector", 4
    JSON = "json", 7  # == ANY
    LITERAL = "literal", 10
    NODE = "node", 11
    # should retire these :SimpleTypes
    STRUCT = "struct", 6
    FUNCTION = "function", 8
    ENUM = "enum", 9
    TYPE_REFERENCE = "ref", 13


RESERVED_TYPE_TAGS = (TypeTag.FUNCTION, TypeTag.ENUM, TypeTag.STRUCT)


class TypeHint(ProtoStrEnum):
    """Extra representation/semantics of a field/type."""

    # string
    NAME = "name", 1
    UUID = "uuid", 2
    DATE = "date", 3
    DATETIME = "datetime", 4
    TIME = "time", 5
    DURATION = "duration", 6
    EMAIL = "email", 7
    URL = "url", 8
    MARKDOWN = "markdown", 9
    RICH_TEXT = "rich_text", 10
    HTML = "html", 11
    CODE = "code", 12
    KEY = "key", 13
    # number
    INTEGER = "integer", 14
    FLOAT = "float", 15
    SLIDER = "slider", 16
    PHONE = "phone", 17
    RATING = "rating", 18
    # boolean
    TOGGLE = "toggle", 19
    CHECKBOX = "checkbox", 20
    THUMBS = "thumbs", 21
    # vector
    EMBEDDING = "embedding", 22
    # node (relation)
    BENCH = "bench", 30  # not used yet
    # MODULE = "module", 31  # not used yet
    FILE = "file", 32
    STATEMENT = "statement", 33
    RECORD = "record", 34
    FIELD = "field", 35
    SECRET = "secret", 36
    BLOB = "blob", 37
    RUN = "run", 38  # not used yet
    # blob nodes
    IMAGE = "image", 50
    VIDEO = "video", 51
    AUDIO = "audio", 52


class TypeFlag(enum.IntFlag):
    """Extra information for fields"""

    # :TypeFlags
    ZERO = 0
    IS_OUTPUT = 2**0
    IS_ARRAY = 2**1
    IS_OPTIONAL = 2**2
    IS_UNION_WITH = 2**3  # nocheckin: move IS_UNION_WITH to Statement properties
    IS_SECRET = 2**4
    IS_STORE_ONLY = 2**5
    IS_ARRAYABLE = 2**6
    IS_META = 2**7
    IS_CONFIG = 2**8
    IS_HIDDEN = 2**9

    @property
    def short_name(self) -> str:
        return self.name.replace("Is", "")


class TypeStorageFormat(ProtoStrEnum):
    """
    The fundamental form of a field/type.:TypeStorageFormat
    """

    STRING = "str", 1
    DOUBLE = "f64", 2
    LONG = "s64", 3
    VECTOR = "vec", 4
    BINARY = "bin", 5
    BOOLEAN = "bool", 6
    DATE = "date", 7
    KEYWORD = "key", 8
    OBJECT = "obj", 9
    RELATION = "rel", 10


class BlobStatus(ProtoStrEnum):
    PENDING = "PENDING", 1
    UPLOADING = "UPLOADING", 2
    AVAILABLE = "AVAILABLE", 3


class TriggerType(ProtoStrEnum):
    """Triggers for statements (for both actual runs and pre-defined triggers)."""

    INVOKE = "invoke", 1
    TIME = "time", 2
    RUN = "run", 3
    EDIT = "edit", 4
    MESSAGE = "message", 5
    USER = "user", 6
    API = "api", 7


class ScheduleType(ProtoStrEnum):
    """Schedules for statements."""

    INTERVAL = "interval", 1
    CRON = "cron", 2


class IssueKind(ProtoStrEnum):
    ERROR = "Error", 1
    WARNING = "Warning", 2
    NOTICE = "Notice", 3


class IssueType(ProtoStrEnum):
    # errors
    INTERNAL = "INTERNAL", 1
    UNKNOWN_IMPORT_SOURCE = "UNKNOWN_IMPORT_SOURCE", 2
    MISSING_REFERENCE = "MISSING_REFERENCE", 3
    CIRCULAR_ANCESTRY = "CIRCULAR_ANCESTRY", 4
    CIRCULAR_UNION = "CIRCULAR_UNION", 5
    MISMATCHED_UNION = "MISMATCHED_UNION", 6
    INVALID_DATA = "INVALID_DATA", 7
    # warnings
    AMBIGUOUS_DEFINITION = "AMBIGUOUS_DEFINITION", 8
    CODE_NOT_EXPORTABLE = "CODE_NOT_EXPORTABLE", 9
    CODE_NOT_CACHEABLE = "CODE_NOT_CACHEABLE", 10
    CODE_REFERENCE_NOT_EXPORTED = "CODE_REFERENCE_NOT_EXPORTED", 11
    TASK_MISSING_IO = "TASK_MISSING_IO", 12
    # notices
    TASK_IS_STATIC = "TASK_IS_STATIC", 13


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
    EQUALS = "EQUALS", 6
    NOT_EQUALS = "NOT_EQUALS", 7
    GREATER_THAN = "GREATER_THAN", 8
    GREATER_THAN_OR_EQUALS = "GREATER_THAN_OR_EQUALS", 9
    LESS_THAN = "LESS_THAN", 10
    LESS_THAN_OR_EQUALS = "LESS_THAN_OR_EQUALS", 11
    # string comparison
    MATCHES = "MATCHES", 12
    STARTS_WITH = "STARTS_WITH", 13
    # containment
    CONTAINS = "CONTAINS", 14
    NOT_CONTAINS = "NOT_CONTAINS", 15
    IN = "IN", 16
    NOT_IN = "NOT_IN", 17
    # existence
    EXISTS = "EXISTS", 18
    NOT_EXISTS = "DOES_NOT_EXIST", 19
    # vector
    NEAR = "NEAR", 20


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
    EXISTS = "EXISTS", 1
    COUNT = "COUNT", 2
    SUM = "SUM", 3
    AVERAGE = "AVERAGE", 4
    MIN = "MIN", 5
    MAX = "MAX", 6
    MEDIAN = "MEDIAN", 7
    HISTOGRAM = "HISTOGRAM", 8


class SortOp(ProtoStrEnum):
    ASCENDING = "ASCENDING", 1
    DESCENDING = "DESCENDING", 2


class QueryEngine(ProtoStrEnum):
    MEMORY = "MEMORY", 1
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

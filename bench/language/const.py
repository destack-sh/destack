from __future__ import annotations

import enum
import re
import typing
from itertools import chain
from typing import NamedTuple
from uuid import UUID

from bench.proto.core import ProtoStrEnum
from bench.utils.func import cyrb53a
from bench.utils.utils import IdentifierType, to_all_caps, to_pyidentifier

if typing.TYPE_CHECKING:
    from bench.language import Node, Statement  # noqa: F401

# hard-coded, do not change ever :BenchUuidNamespace
BENCH_UUID_NAMESPACE = UUID("d822dab7-41ad-4706-a9c8-4379e15b2ed0")


class NodeType(ProtoStrEnum):
    # source
    MODULE = "MODULE", 1
    FILE = "FILE", 2
    STATEMENT = "STATEMENT", 3
    TRIGGER = "TRIGGER", 4
    TAGGING = "TAGGING", 5
    FIELD = "FIELD", 6
    RECORD = "RECORD", 7
    VIEW = "VIEW", 8
    # interp
    ISSUE = "ISSUE", 9
    RESOLVED_FIELD = "RESOLVED_FIELD", 10
    # remote
    BLOB = "BLOB", 11
    SECRET = "SECRET", 12
    # session
    SESSION = "SESSION", 13
    RUN = "RUN", 14

    # user (not used yet)
    # USER = "USER"
    # COMMENT = "COMMENT"
    # ACCESS = "ACCESS"

    @property
    def caps_name(self):
        return BENCH_TYPE_CAPS_CASE[self]

    @property
    def camel_name(self):
        return BENCH_TYPE_CAMEL_CASE[self]


class StructType(ProtoStrEnum):
    # starts at 100 to avoid collisions with NodeType (BenchType combines both in one metatype)
    EXPRESSION = "EXPRESSION", 101
    RUN_CODE_FRAME = "RUN_CODE_FRAME", 102
    RUN_ERROR = "RUN_ERROR", 103
    LOG_ENTRY = "LOG_ENTRY", 104
    WORKER_SET = "WORKER_SET", 105
    ENVIRONMENT = "ENVIRONMENT", 106

    @property
    def caps_name(self):
        return BENCH_TYPE_CAPS_CASE[self]

    @property
    def camel_name(self):
        return BENCH_TYPE_CAMEL_CASE[self]


if typing.TYPE_CHECKING:
    BenchType = NodeType | StructType
else:
    BenchType = ProtoStrEnum(
        "BenchType",
        {bt.name: (bt.name, bt.id) for bt in chain(NodeType, StructType)},
    )
    BenchType.caps_name = NodeType.caps_name
    BenchType.camel_name = NodeType.camel_name

# local = only stored in user Bench, not host
LOCAL_NODE_TYPES = (NodeType.RECORD,)
OUT_OF_LINE_NODE_TYPES = (
    NodeType.SESSION,
    NodeType.RUN,
    NodeType.RECORD,
    NodeType.BLOB,
    NodeType.SECRET,
)
INLINE_NODE_TYPES = (nt for nt in NodeType if nt not in OUT_OF_LINE_NODE_TYPES)
HOST_NODE_TYPES = (nt for nt in NodeType if nt not in LOCAL_NODE_TYPES)


class StatementType(ProtoStrEnum):
    TAG = "tag", 1
    TEXT = "text", 2
    BLANK = "blank", 3
    CLASS = "class", 4
    CHOICE = "choice", 5
    TASK = "task", 6
    CODE = "code", 7
    FLOW = "flow", 8
    MODEL = "model", 9
    VARIABLE = "variable", 10
    DATABASE = "database", 11
    VIEW = "view", 12
    GROUP = "group", 13

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


class SessionAccessLevel(enum.IntEnum):  # SessionAccessLevel
    Zero = 0
    Read = 1
    Create = 2
    Update = 3
    Delete = 4
    Full = Delete


NodeType = NodeType
BENCH_TYPE_CAPS_CASE: dict[NodeType | StructType, str] = {
    _type: to_all_caps(_type) for _type in chain(NodeType, StructType)
}
BENCH_TYPE_CAMEL_CASE: dict[NodeType | StructType, str] = {
    _type: to_pyidentifier(_type, IdentifierType.TYPE) for _type in chain(NodeType, StructType)
}
INTERP_NODE_TYPES = {NodeType.ISSUE, NodeType.RESOLVED_FIELD}

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


# TODO @Architecture: simplify the TypeTag/TypeHint/TypeFlag/TypeStorageFormat mess
#  (should probably? just be type + flags with display hints in metadata)
#  IS_ARRAYABLE -> real unions of X | list[X] or whatever
#  IS_UNION_WITH -> inherit? from type (could remain flag, though that's not great)


class TypeTag(ProtoStrEnum):
    """The Bench primitive type of a field/type."""

    STRING = "string", 1
    NUMBER = "number", 2
    BOOLEAN = "boolean", 3
    VECTOR = "vector", 4
    BLOB = "blob", 5
    STRUCT = "struct", 6
    JSON = "json", 7
    FUNCTION = "function", 8
    ENUM = "enum", 9
    LITERAL = "literal", 10
    TYPE_REFERENCE = "ref", 11
    NODE = "node", 12
    ANY = "any", 13


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
    # blob
    IMAGE = "image", 23
    VIDEO = "video", 24
    AUDIO = "audio", 25
    # node (relation)
    FILE = "file", 26
    STATEMENT = "statement", 27
    RECORD = "record", 28  # for relations
    FIELD = "field", 29
    RUN = "run", 30  # not used yet
    SECRET = "secret", 31  # not used as a node yet
    BLOB = "blob", 32  # not used as a node yet


class TypeFlag(enum.IntFlag):
    """Extra information for fields"""

    # :TypeFlags
    ZERO = 0
    IS_OUTPUT = 2**0
    IS_ARRAY = 2**1
    IS_OPTIONAL = 2**2
    IS_UNION_WITH = 2**3
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
    The fundamental form of a field/type.
    Since we're using OpenSearch for our user data backend, this
    needs to be compatible with OpenSearch's field types.
    However, be mindful of other future storage/indexing backends.
    :TypeStorageFormat
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
    PREPARED = "prepared", 1
    UPLOADING = "uploading", 2
    AVAILABLE = "available", 3


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


class ProjectRegion(ProtoStrEnum):
    US_WEST = "US_WEST", 1
    EU_CENTRAL = "EU_CENTRAL", 2


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
    SUSPENDED = "Suspended", 4
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
    RunStatus.SUSPENDED,
    RunStatus.ABORTING,
}
ACTIVE_RUN_STATUSES = {RunStatus.QUEUED, RunStatus.RUNNING, RunStatus.SUSPENDED, RunStatus.ABORTING}


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
    LARGE = "LARGE", 4
    XLARGE_CPU = "XLARGE_CPU", 5
    XLARGE_MEM = "XLARGE_MEM", 6


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
    # Single value
    COUNT = "COUNT", 1
    SUM = "SUM", 2
    AVERAGE = "AVERAGE", 3
    MIN = "MIN", 4
    MAX = "MAX", 5
    MEDIAN = "MEDIAN", 6
    # Bucket value
    HISTOGRAM = "HISTOGRAM", 7


class SortOp(ProtoStrEnum):
    ASCENDING = "ASCENDING", 1
    DESCENDING = "DESCENDING", 2


class QueryEngine(ProtoStrEnum):
    MODULE = "MODULE", 1
    HOST = "HOST", 2
    OPENSEARCH = "OS", 3
    POSTGRES = "PG", 4


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

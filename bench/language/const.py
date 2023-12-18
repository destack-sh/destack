from __future__ import annotations

import enum
import re
import typing
from typing import NamedTuple
from uuid import UUID

from bench.utils.func import cyrb53a
from bench.utils.utils import IdentifierType, to_all_caps, to_pyidentifier

if typing.TYPE_CHECKING:
    from bench.language import Node, Statement  # noqa: F401

# hard-coded, do not change ever :BenchUuidNamespace
BENCH_UUID_NAMESPACE = UUID("d822dab7-41ad-4706-a9c8-4379e15b2ed0")


class NodeType(enum.StrEnum):
    # source
    MODULE = "MODULE"
    FILE = "FILE"
    STATEMENT = "STATEMENT"
    TRIGGER = "TRIGGER"
    TAGGING = "TAGGING"
    FIELD = "FIELD"
    RECORD = "RECORD"
    VIEW = "VIEW"
    # interp
    ISSUE = "ISSUE"
    RESOLVED_FIELD = "RESOLVED_FIELD"
    # user
    USER = "USER"
    COMMENT = "COMMENT"
    ACCESS = "ACCESS"
    # remote
    BLOB = "BLOB"
    SECRET = "SECRET"
    # session
    SESSION = "SESSION"
    RUN = "RUN"

    @property
    def caps_name(self):
        return NODE_TYPE_CAPS_CASE[self]

    @property
    def camel_name(self):
        return NODE_TYPE_CAMEL_CASE[self]


class StructType(enum.StrEnum):
    EXPRESSION = "EXPRESSION"
    RUN_CODE_FRAME = "RUN_CODE_FRAME"
    RUN_ERROR = "RUN_ERROR"
    LOG_ENTRY = "LOG_ENTRY"
    PROJECTION = "PROJECTION"
    WORKER_SET = "WORKER_SET"


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


class StatementType(enum.StrEnum):
    TAG = "tag"
    TEXT = "text"
    BLANK = "blank"
    CLASS = "class"
    CHOICE = "choice"
    TASK = "task"
    CODE = "code"
    FLOW = "flow"
    MODEL = "model"
    VARIABLE = "variable"
    DATABASE = "database"
    VIEW = "view"
    GROUP = "group"

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
NODE_TYPE_CAPS_CASE: dict[NodeType, str] = {
    node_type: to_all_caps(node_type) for node_type in NodeType
}
NODE_TYPE_CAMEL_CASE: dict[NodeType, str] = {
    node_type: to_pyidentifier(node_type, IdentifierType.TYPE) for node_type in NodeType
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


class ViewLayout(enum.StrEnum):
    """The layout of a database view."""

    TABLE = "table"


# TODO @Architecture: simplify the TypeTag/TypeHint/TypeFlag/TypeStorageFormat mess
#  (should probably? just be type + flags with display hints in metadata)
#  IS_ARRAYABLE -> real unions of X | list[X] or whatever
#  IS_UNION_WITH -> inherit? from type (could remain flag, though that's not great)


class TypeTag(enum.StrEnum):
    """The Bench primitive type of a field/type."""

    STRING = "string"
    NUMBER = "number"
    BOOLEAN = "boolean"
    VECTOR = "vector"
    BLOB = "blob"
    STRUCT = "struct"
    JSON = "json"
    FUNCTION = "function"
    ENUM = "enum"
    LITERAL = "literal"
    TYPE_REFERENCE = "ref"
    NODE = "node"
    ANY = "any"


RESERVED_TYPE_TAGS = (TypeTag.FUNCTION, TypeTag.ENUM, TypeTag.STRUCT)


class TypeHint(enum.StrEnum):
    """Extra representation/semantics of a field/type."""

    # string
    NAME = "name"
    UUID = "uuid"
    DATE = "date"
    DATETIME = "datetime"
    TIME = "time"
    DURATION = "duration"
    EMAIL = "email"
    URL = "url"
    MARKDOWN = "markdown"
    RICH_TEXT = "rich_text"
    HTML = "html"
    CODE = "code"
    KEY = "key"
    # number
    INTEGER = "integer"
    FLOAT = "float"
    SLIDER = "slider"
    PHONE = "phone"
    RATING = "rating"
    # boolean
    TOGGLE = "toggle"
    CHECKBOX = "checkbox"
    THUMBS = "thumbs"
    # vector
    EMBEDDING = "embedding"
    # blob
    IMAGE = "image"
    VIDEO = "video"
    AUDIO = "audio"
    # node (relation)
    FILE = "file"
    STATEMENT = "statement"
    RECORD = "record"  # for relations
    FIELD = "field"
    RUN = "run"  # not used yet
    SECRET = "secret"  # not used as a node yet
    BLOB = "blob"  # not used as a node yet


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


class TypeStorageFormat(enum.StrEnum):
    """
    The fundamental form of a field/type.
    Since we're using OpenSearch for our user data backend, this
    needs to be compatible with OpenSearch's field types.
    However, be mindful of other future storage/indexing backends.
    :TypeStorageFormat
    """

    STRING = "str"
    DOUBLE = "f64"
    LONG = "s64"
    VECTOR = "vec"
    BINARY = "bin"
    BOOLEAN = "bool"
    DATE = "date"
    KEYWORD = "key"
    OBJECT = "obj"
    RELATION = "rel"


class BlobStatus(enum.StrEnum):
    PREPARED = "prepared"
    UPLOADING = "uploading"
    AVAILABLE = "available"


class TriggerType(enum.StrEnum):
    """Triggers for statements (for both actual runs and pre-defined triggers)."""

    INVOKE = "invoke"
    TIME = "time"
    RUN = "run"
    EDIT = "edit"
    MESSAGE = "message"
    USER = "user"
    API = "api"


class ScheduleType(enum.StrEnum):
    """Schedules for statements."""

    INTERVAL = "interval"
    CRON = "cron"


class IssueKind(enum.StrEnum):
    ERROR = "Error"
    WARNING = "Warning"
    NOTICE = "Notice"


class IssueType(enum.StrEnum):
    # errors
    INTERNAL = "INTERNAL"
    UNKNOWN_IMPORT_SOURCE = "UNKNOWN_IMPORT_SOURCE"
    MISSING_REFERENCE = "MISSING_REFERENCE"
    CIRCULAR_ANCESTRY = "CIRCULAR_ANCESTRY"
    CIRCULAR_UNION = "CIRCULAR_UNION"
    MISMATCHED_UNION = "MISMATCHED_UNION"
    INVALID_DATA = "INVALID_DATA"
    # warnings
    AMBIGUOUS_DEFINITION = "AMBIGUOUS_DEFINITION"
    CODE_NOT_EXPORTABLE = "CODE_NOT_EXPORTABLE"
    CODE_NOT_CACHEABLE = "CODE_NOT_CACHEABLE"
    CODE_REFERENCE_NOT_EXPORTED = "CODE_REFERENCE_NOT_EXPORTED"
    TASK_MISSING_IO = "TASK_MISSING_IO"
    # notices
    TASK_IS_STATIC = "TASK_IS_STATIC"


class ProjectRegion(enum.StrEnum):
    US_WEST = "US_WEST"
    EU_CENTRAL = "EU_CENTRAL"


class WorkerSetStatus(enum.StrEnum):
    SLEEPING = "SLEEPING"
    PENDING = "PENDING"
    UPDATING = "UPDATING"
    HEALTHY = "HEALTHY"
    UNHEALTHY = "UNHEALTHY"
    UNAVAILABLE = "UNAVAILABLE"
    UNKNOWN = "UNKNOWN"


class SessionStatus(enum.StrEnum):
    ACTIVE = "ACTIVE"
    SUSPENDED = "SUSPENDED"
    TERMINATED = "TERMINATED"


class RunStatus(enum.StrEnum):
    SCHEDULED = "Scheduled"
    QUEUED = "Queued"
    RUNNING = "Running"
    SUSPENDED = "Suspended"
    ABORTING = "Aborting"
    # terminal statuses
    CANCELLED = "Cancelled"
    ABORTED = "Aborted"
    FAILED = "Failed"
    COMPLETED = "Completed"


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


class RunErrorKind(enum.StrEnum):
    Internal = "Internal"
    Parse = "Parse"
    Validation = "Validation"
    Runtime = "Runtime"
    Untrusted = "Untrusted"


class WorkerProfile(enum.StrEnum):
    TINY = "TINY"
    SMALL = "SMALL"
    MEDIUM = "MEDIUM"
    LARGE = "LARGE"
    XLARGE_CPU = "XLARGE_CPU"
    XLARGE_MEM = "XLARGE_MEM"


class ExpressionKind(enum.StrEnum):
    CONDITIONAL = "CONDITIONAL"
    SORT = "SORT"
    AGGREGATION = "AGGREGATION"


class ConditionalOp(enum.StrEnum):
    # logical
    TRUE = "TRUE"
    FALSE = "FALSE"
    NOT = "NOT"
    AND = "AND"
    OR = "OR"
    # comparison
    EQUALS = "EQUALS"
    NOT_EQUALS = "NOT_EQUALS"
    GREATER_THAN = "GREATER_THAN"
    GREATER_THAN_OR_EQUALS = "GREATER_THAN_OR_EQUALS"
    LESS_THAN = "LESS_THAN"
    LESS_THAN_OR_EQUALS = "LESS_THAN_OR_EQUALS"
    # string comparison
    MATCHES = "MATCHES"
    STARTS_WITH = "STARTS_WITH"
    # containment
    CONTAINS = "CONTAINS"
    NOT_CONTAINS = "NOT_CONTAINS"
    IN = "IN"
    NOT_IN = "NOT_IN"
    # existence
    EXISTS = "EXISTS"
    NOT_EXISTS = "DOES_NOT_EXIST"
    # vector
    NEAR = "NEAR"


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


class AggregationOp(enum.StrEnum):
    # Single value
    COUNT = "COUNT"
    SUM = "SUM"
    AVERAGE = "AVERAGE"
    MIN = "MIN"
    MAX = "MAX"
    MEDIAN = "MEDIAN"
    # Bucket value
    HISTOGRAM = "HISTOGRAM"


class SortOp(enum.StrEnum):
    ASCENDING = "ASCENDING"
    DESCENDING = "DESCENDING"


class QueryEngine(enum.StrEnum):
    MODULE = "MODULE"
    HOST = "HOST"
    OPENSEARCH = "OS"
    POSTGRES = "PG"


class SortMode(enum.StrEnum):
    MAX = "MAX"
    MIN = "MIN"
    AVERAGE = "AVERAGE"
    SUM = "SUM"
    MEDIAN = "MEDIAN"


if typing.TYPE_CHECKING:
    ExpressionOp = ConditionalOp | AggregationOp | SortOp
else:
    ExpressionOp = enum.StrEnum(
        "ExpressionOp",
        {**ConditionalOp.__members__, **AggregationOp.__members__, **SortOp.__members__},
    )

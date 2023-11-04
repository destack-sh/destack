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


class ModuleNodeType(enum.StrEnum):
    # source
    MODULE = "Module"
    FILE = "File"
    STATEMENT = "Statement"
    TRIGGER = "Trigger"
    TAGGING = "Tagging"
    FIELD = "Field"
    RECORD = "Record"
    DATABASE_VIEW = "DatabaseView"
    DATABASE_VIEW_FIELD = "DatabaseViewField"
    # interp
    ISSUE = "Issue"
    RESOLVED_FIELD = "ResolvedField"
    # user
    COMMENT = "Comment"
    ACCESS = "Access"
    # remote
    BLOB = "Blob"
    SECRET = "Secret"
    # session
    RUN = "Run"

    @property
    def caps_name(self):
        return MNT_CAPS_CASE[self]


class StatementType(enum.StrEnum):
    """The type of Bench statement."""

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
    REFERENCE = "reference"
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

# :ModuleLimits
MAX_STATEMENTS_PER_FILE = 256
MAX_FILES = 64
MODULE_RECORD_LIMIT = 25_000
DATABASE_VERSIONED_RECORD_LIMIT = 2500
DATABASE_GENERAL_RECORD_LIMIT = 10_000_000


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


MNT = ModuleNodeType
MNT_CAPS_CASE: dict[MNT, str] = {mnt: to_all_caps(mnt) for mnt in MNT}
INTERP_NODE_TYPES = {MNT.ISSUE, MNT.RESOLVED_FIELD}

ModuleReference = typing.NamedTuple(
    "ModuleReference", [("name", str), ("version", str), ("id", typing.Optional[UUID])]
)
NodePath = NamedTuple("NodePath", [("path", str), ("name", str)])
StatementReference = typing.Union["Statement", NodePath, UUID]
NodeReference = typing.Union["Node", NodePath, UUID]
TypedNodeReference = NamedTuple("TypedNodeReference", [("type", MNT), ("ref", UUID)])
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


class DatabaseViewLayout(enum.StrEnum):
    """The layout of a database view."""

    TABLE = "table"


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
    SECRET = "secret"
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
    # file
    IMAGE = "image"
    VIDEO = "video"
    AUDIO = "audio"
    # node :NodesAsValues
    STATEMENT = "statement"
    FIELD = "field"
    RUN = "run"


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


class FieldReferenceMask(enum.IntFlag):
    """Per-key mask for field unions"""

    PICK = 2**0
    TO_INPUT = 2**2
    TO_OUTPUT = 2**3


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
    Error = "Error"
    Warning = "Warning"
    Notice = "Notice"


class IssueType(enum.StrEnum):
    # errors
    INTERNAL = "INTERNAL"
    UNKNOWN_IMPORT_SOURCE = "UNKNOWN_IMPORT_SOURCE"
    MISSING_REFERENCE = "MISSING_REFERENCE"
    CIRCULAR_ANCESTRY = "CIRCULAR_ANCESTRY"
    CIRCULAR_UNION = "CIRCULAR_UNION"
    MISMATCHED_UNION = "MISMATCHED_UNION"
    # warnings
    AMBIGUOUS_DEFINITION = "AMBIGUOUS_DEFINITION"
    CODE_NOT_EXPORTABLE = "CODE_NOT_EXPORTABLE"
    CODE_NOT_CACHEABLE = "CODE_NOT_CACHEABLE"
    CODE_REFERENCE_NOT_EXPORTED = "CODE_REFERENCE_NOT_EXPORTED"
    TASK_MISSING_IO = "TASK_MISSING_IO"
    # notices
    TASK_IS_STATIC = "TASK_IS_STATIC"


class WorkerRegion(enum.StrEnum):
    US_CENTRAL = "US_CENTRAL"
    EU_CENTRAL = "EU_CENTRAL"


class WorkerSetStatus(enum.StrEnum):
    SLEEPING = "SLEEPING"
    PENDING = "PENDING"
    UPDATING = "UPDATING"
    HEALTHY = "HEALTHY"
    UNHEALTHY = "UNHEALTHY"
    UNAVAILABLE = "UNAVAILABLE"
    UNKNOWN = "UNKNOWN"


class RunStatus(enum.StrEnum):
    Scheduled = "Scheduled"
    Queued = "Queued"
    Running = "Running"
    Suspended = "Suspended"
    Aborting = "Aborting"
    # terminal statuses
    Cancelled = "Cancelled"
    Aborted = "Aborted"
    Failed = "Failed"
    Completed = "Completed"


TERMINAL_RUN_STATUSES = {
    RunStatus.Cancelled,
    RunStatus.Aborted,
    RunStatus.Failed,
    RunStatus.Completed,
}
PENDING_RUN_STATUSES = {
    RunStatus.Scheduled,
    RunStatus.Queued,
    RunStatus.Running,
    RunStatus.Suspended,
    RunStatus.Aborting,
}
ACTIVE_RUN_STATUSES = {RunStatus.Queued, RunStatus.Running, RunStatus.Suspended, RunStatus.Aborting}


class WorkerProfile(enum.StrEnum):
    TINY = "TINY"
    SMALL = "SMALL"
    MEDIUM = "MEDIUM"
    LARGE = "LARGE"
    XLARGE_CPU = "XLARGE_CPU"
    XLARGE_MEM = "XLARGE_MEM"

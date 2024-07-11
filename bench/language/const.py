import contextvars
import secrets
import typing
from datetime import datetime, timedelta
from typing import Any, Mapping, Optional, cast
from uuid import UUID

from bench.utils.func import IdEnum, bittuple, cyrb53a
from bench.utils.utils import frozendict, get_from_env

if typing.TYPE_CHECKING:
    from bench.language import Bench, Session, Transaction


class _Unset:
    def __repr__(self):
        return "<UNSET!>"

    def __str__(self):
        return "<UNSET!>"


# forever constants
VERSION = "2024.07.11.0"  # auto change via version script
REVISION_PENDING = -1
TK_LENGTH_BYTES = 8
TK_LENGTH_B64 = 12  # 1.5 * TK_LENGTH_BYTES (must be integer)
FLOAT_EPSILON = 1e-6

# runtime constants
UNSET = cast(Any, _Unset())
EMPTY_LIST: list = []
EMPTY_SET: frozenset = frozenset()
EMPTY_DICT: dict[Any, Any] = frozendict()


def new_struct_id() -> int:
    id = secrets.randbits(31)
    if id < 0:
        id = -id
    return id


# NOTE: we have the enum registry here to avoid circular imports
_ENUM_CLASS_BY_TYPE: dict["EnumType", type[IdEnum]] = {}

IdEnumT = typing.TypeVar("IdEnumT", bound=IdEnum)


def enum_(enum_type: "EnumType"):
    """Register a Bench enum."""

    def register_enum(cls: type[IdEnumT]) -> type[IdEnumT]:
        if enum_type in _ENUM_CLASS_BY_TYPE:
            raise ValueError(f"enum {enum_type} duplicate: {_ENUM_CLASS_BY_TYPE[enum_type]}")
        _ENUM_CLASS_BY_TYPE[enum_type] = cls
        return cls

    return register_enum


#
# Enums
# NOTE: enum/struct id 'regions' should be roughly in sync with each other
#


class EnumType(IdEnum):
    # intrinsic
    ENUM_TYPE = 20001  # so meta
    NODE_TYPE = 20002
    STRUCT_TYPE = 20003
    OBJECT_TYPE = 20004  # NodeType | StructType
    BENCH_TYPE = 20005  # NodeType | StructType | EnumType
    VISIBILITY = 20010
    CHANGE_KIND = 20011

    # access
    ACCESS_MODE = 20030
    ACCESS_KIND = 20031
    READ_TYPE = 20032
    EDIT_TYPE = 20033
    USE_TYPE = 20034
    ACCESS_TYPE = 20035  # ReadType | EditType | UseType
    CHANGE_CATEGORY = 20036
    POLICY_EFFECT = 20040

    # bench
    REGION = 20050
    TENANCY = 20051
    SERVER_PROFILE = 20055
    MACHINE_PROFILE = 20056
    RESOURCE_STATUS = 20057
    FILE_RETENTION_MODE = 2060
    FILE_KIND = 2061
    CLIENT_TYPE = 20070

    # type
    PRIMITIVE_TYPE = 20080
    FORMAT_HINT = 20081
    FIELD_ZONE = 20082
    TYPE_KIND = 20083
    BLOCK_TYPE = 20384

    # basic
    SCHEDULE_TYPE = 20100
    TIME_INTERVAL = 20101
    DAY = 20102
    MONTH = 20103
    TEXT_LINE_TYPE = 20110
    ICON_KIND = 20111

    # expression
    EXPRESSION_KIND = 20200
    EXPRESSION_OP = 20201
    LITERAL_OP = 20202
    FUNCTIONAL_OP = 20203
    CONDITIONAL_OP = 20204
    AGGREGATION_OP = 20205
    SORT_MODE = 20206
    SORT_OP = 20207
    SELECTION_KIND = 20208
    PATH_TOKEN_TYPE = 20210

    # issue
    ISSUE_KIND = 20400
    ISSUE_TYPE = 20401

    # run
    STEP_TYPE = 20500
    PIPE_TYPE = 20501
    LOG_KIND = 20502
    LOG_LEVEL = 20503
    RUN_STATUS = 20504
    RUN_KIND = 20505
    RUN_ERROR_KIND = 20506
    RUN_ERROR_TYPE = 20507
    SESSION_STATUS = 20508
    TRIGGER_TYPE = 20509
    NOTIFICATION_KIND = 20510
    BREAKPOINT_KIND = 20511
    BREAKPOINT_ACTION = 20512
    CODE_KIND = 20513
    RUNNABLE_KIND = 20514

    # view
    SPACE_TYPE = 21000
    VIEW_TYPE = 21001
    VARIANT = 21002
    COLOR_TYPE = 21003
    COLOR_SHADE = 21004
    FONT_TYPE = 21005
    FONT_WEIGHT = 21006
    FONT_SIZE = 21007
    SPACING = 21008
    ANCHOR = 21009
    ORIENTATION = 21010
    ALIGNMENT = 21011
    # view states
    USER_WIZARD_STAGE = 21200
    TREE_VIEW_PRESET = 21201

    # user
    USER_STATUS = 29000
    ORGANIZATION_STATUS = 29001


enum_(EnumType.ENUM_TYPE)(EnumType)
ENUM_TYPES: bittuple[EnumType] = bittuple(*EnumType)
ENUM_TYPES_SET: frozenset[EnumType] = frozenset(ENUM_TYPES)

#
# Node metatypes
#


@enum_(EnumType.NODE_TYPE)
class NodeType(IdEnum):
    #
    # Global
    #

    # universe (global)
    BENCH = 1
    USER = 2
    ORGANIZATION = 3
    HANDLE = 4
    CLIENT = 5
    # CHALLENGE?

    # resource (global, later maybe regional, per Bench)
    SERVER = 500  # virtual infinitely scalable server
    STORE = 501  # our trusted postgres store
    MACHINE = 502  # actual machine providing compute and such
    DRIVE = 503  # object store like S3/MinIO, maybe block storage later
    BLOB = 504  # in a Drive (deferred)
    # CACHE, DOMAIN, EMAIL, PHONE, ...

    #
    # Local (per Bench)
    #

    # source (local)
    BRANCH = 1000
    PACKAGE = 1001
    DEPENDENCY = 1002
    SPACE = 1003
    LINK = 1004
    SKIP = 1005
    ISSUE = 1006
    BLOCK = 1010
    TRIGGER = 1011
    FIELD = 1012  # (based)
    QUERY = 1013
    VIEW = 1014
    STEP = 1015
    # TAG?
    # REACTION?
    # LOCK?
    BADGE = 1100
    MEMBERSHIP = 1101
    INVITE = 1102

    # runtime (local)
    SESSION = 1200  # (timed)
    RUN = 1201  # (based, timed)
    SIGNAL = 1202  # (based, timed)
    LOG = 1203  # (timed)
    NOTIFICATION = 1204  # (based, timed)
    MESSAGE = 1205  # (based, timed)
    RECORD = 1206  # (based)


# :NodeTypes
NODE_TYPES = bittuple(*NodeType)
NODE_TYPES_SET: frozenset[NodeType] = frozenset(NODE_TYPES)
ROOT_NODE_TYPES = bittuple(NodeType.BENCH, NodeType.USER, NodeType.ORGANIZATION)
UNIVERSE_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if nt.id < 100))
GLOBAL_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if nt.id < 1000))
LOCAL_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if nt.id >= 1000))
SOURCE_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if nt.id >= 1000 and nt.id < 1200))

BASED_NODE_TYPES = bittuple(  # :HasBase
    NodeType.FIELD,
    NodeType.RUN,
    NodeType.SIGNAL,
    NodeType.NOTIFICATION,
    NodeType.MESSAGE,
    NodeType.RECORD,
)
TIMED_NODE_TYPES = bittuple(
    NodeType.SESSION,
    NodeType.RUN,
    NodeType.SIGNAL,
    NodeType.LOG,
    NodeType.NOTIFICATION,
    NodeType.MESSAGE,
)
ETERNAL_NODE_TYPES: bittuple[NodeType] = bittuple(
    NodeType.SESSION, NodeType.RUN, NodeType.SIGNAL, NodeType.LOG
)
IN_PACKAGE_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if nt.id >= 1001))
SUB_PACKAGE_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if nt.id > 1001))
IN_BENCH_NODE_TYPES = bittuple(
    *(
        *tuple(nt for nt in NODE_TYPES if nt.id >= 500),
        NodeType.BENCH,
        NodeType.CLIENT,
        NodeType.HANDLE,
    )
)
IN_BENCH_GLOBAL_NODE_TYPES = bittuple(
    *tuple(nt for nt in IN_BENCH_NODE_TYPES if nt not in LOCAL_NODE_TYPES)
)
SUB_BENCH_NODE_TYPES = bittuple(*tuple(nt for nt in IN_BENCH_NODE_TYPES if nt != NodeType.BENCH))
RESOURCE_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if nt.id >= 500 and nt.id < 600))
DEFERRED_RESOURCE_NODE_TYPES = bittuple(NodeType.BLOB)
BENCH_NODE_TYPES = bittuple(
    NodeType.BENCH,
    NodeType.BRANCH,
    NodeType.PACKAGE,
    NodeType.HANDLE,
    NodeType.CLIENT,
    *RESOURCE_NODE_TYPES,
)
LOADED_BENCH_NODE_TYPES = bittuple(
    *(nt for nt in BENCH_NODE_TYPES if nt not in DEFERRED_RESOURCE_NODE_TYPES)
)
PUBLIC_NODE_TYPES = bittuple(NodeType.USER, NodeType.ORGANIZATION)
USER_NODE_TYPES = bittuple(NodeType.USER, NodeType.ORGANIZATION, NodeType.CLIENT, NodeType.HANDLE)

#
# Struct metatypes
# NOTE: enum/struct id 'regions' should be roughly in sync with each other
#


@enum_(EnumType.STRUCT_TYPE)
class StructType(IdEnum):
    # transaction
    SESSION_CONTEXT = 10001
    EDIT_CONTEXT = 10002
    EDIT = 10005
    CHANGE = 10006
    CHANGE_VIGNETTE = 10007

    # utility
    GRAPH_SCOPE = 10050
    CLIENT_ORIGIN = 10051
    NODE_REFERENCE = 10052
    PROPERTY_REFERENCE = 10053
    PATH = 10060
    PATH_TOKEN = 10061

    # access
    POLICY = 10100
    POLICY_RULE = 10101
    SUBJECT = 10102
    ACCESS_ZONE = 10104
    ACCESS_MATRIX = 10105
    ACCESS = 10107
    ...

    # text
    TEXT = 10150
    TEXT_LINE = 10151
    TEXT_SPAN = 10152

    # type
    TYPE_INFO = 10200
    TYPE_CONSTRAINT = 10201
    SCHEDULE = 10202
    PROJECTION = 10203
    FILE = 10204
    ICON = 10205
    TRIGGER_INFO = 10206

    # expressions
    EXPRESSION = 10300
    AGGREGATION = 10301
    SELECTION = 10302
    QUERY_INFO = 10303
    READ_OPTIONS = 10304
    VALUE = 10305
    COMPUTED_VALUE = 10306

    # run
    CODE = 10400
    CODE_LINE = 10401
    PIPE = 10450
    RUN_ERROR = 10500
    RUN_OPTIONS = 10501
    RUN_ATTEMPT = 10502
    RUN_TRACE = 10503
    RUN_FRAME = 10504
    BREAKPOINT = 10520
    LOG_INFO = 10550

    # space/views
    COLOR = 11000
    FONT = 11001
    BOX = 11002
    OFFSET = 11003
    TRANSFORM = 11004
    ...
    START_VIEW_STATE = 11200
    FEED_VIEW_STATE = 11201
    CHART_VIEW_STATE = 11202
    HISTORY_VIEW_STATE = 11203
    TIMELINE_VIEW_STATE = 11204
    USER_WIZARD_VIEW_STATE = 11205
    PAGE_VIEW_STATE = 11206
    TREE_VIEW_STATE = 11207

    # steps
    ...

    # signals
    ...

    # notifications
    ...


STRUCT_TYPES: bittuple[StructType] = bittuple(*StructType)
STRUCT_TYPES_SET: frozenset[StructType] = frozenset(STRUCT_TYPES)

if typing.TYPE_CHECKING:
    ObjectType = NodeType | StructType
    BenchType = NodeType | StructType | EnumType
else:
    ObjectType = IdEnum.combine("ObjectType", NodeType, StructType)
    enum_(EnumType.OBJECT_TYPE)(ObjectType)
    BenchType = IdEnum.combine("BenchType", NodeType, StructType, EnumType)
    enum_(EnumType.BENCH_TYPE)(BenchType)

OBJECT_TYPES: bittuple[ObjectType] = bittuple(*ObjectType)
OBJECT_TYPES_SET: frozenset[ObjectType] = frozenset(OBJECT_TYPES)
BENCH_TYPES: bittuple[BenchType] = bittuple(*BenchType)


def is_node_type(obj: IdEnum | int) -> bool:
    return obj in NODE_TYPES_SET


def is_struct_type(obj: IdEnum | int) -> bool:
    return obj in STRUCT_TYPES_SET


def is_object_type(obj: IdEnum | int) -> bool:
    return obj in OBJECT_TYPES_SET


def is_enum_type(obj: IdEnum | int) -> bool:
    return obj in ENUM_TYPES_SET


@enum_(EnumType.REGION)
class Region(IdEnum):
    """
    Where a Resource is located (physically).
    There are
      - 'continental' regions ([>1, <100]: Europe, North America, etc.).
      - 'area' regions ([%20=0]: Europe Central, US East, etc.).
      - 'city' regions (Frankfurt, Ohio, etc.).
    """

    GLOBAL = 1
    EUROPE = 2

    # europe
    EUROPE_CENTRAL = 100
    EUROPE_ZURICH = 101
    EUROPE_FRANKFURT = 102
    # americas
    ...


@enum_(EnumType.BLOCK_TYPE)
class BlockType(IdEnum):
    ALIAS = 1  # refer to / 'redefine' an existing block or builtin (like a 'newtype')
    PAGE = 2  # group of blocks
    MODULE = 3  # group of blocks with a 'namespace'
    BLANK = 4  # placeholder/spacer?

    # types
    CLASS = 10  # define a class type with fields
    CHOICE = 11  # define a choice type with fields (as literal options)
    # TAG = 12  # define a tag type with fields
    PROTOCOL = 14  # define a 'protocol' for a block graph/template with fields
    SIGNAL = 15  # define a signal type with fields
    # ISSUE = ...  # define a new issue type
    # NOTIFICATION = ...  # define a new notification type
    # METRIC = ...  # define a new metric type?
    # BLOCK = ...  # define a new block type?

    # runnable
    TEXT = 30  # define a 'paragraph' of text/prompt with fields (optionally incl. input/output)
    CODE = 31  # define a code function/script with fields (optionally incl. input/output)
    FLOW = 32  # define a flow with steps and fields (optionally incl. input/output)

    # state
    VARIABLE = 50  # define a single-value variable
    QUERY = 52  # define a set of queries
    DATABASE = 53  # define a database with records & queries

    # view
    VIEW = 70  # define a view (with fields and nested views)

    # auth
    ROLE = 90  # define a role with policies
    IDENTITY = 91  # define an identity with roles & policies

    @property
    def is_type(self) -> bool:
        return self in BlockTypes.TYPES

    @property
    def is_classy(self) -> bool:
        return self in BlockTypes.CLASSES

    @property
    def is_runnable(self) -> bool:
        return self in BlockTypes.RUNNABLE


BLOCK_TYPES: tuple[BlockType, ...] = tuple(BlockType)


class BlockTypes:
    TYPES = bittuple(*tuple(t for t in BLOCK_TYPES if 10 <= t.id < 20))
    RUNNABLE = bittuple(*tuple(t for t in BLOCK_TYPES if 30 <= t.id < 40))
    CLASSES = bittuple(
        BlockType.CLASS, BlockType.SIGNAL, *RUNNABLE, BlockType.VARIABLE, BlockType.DATABASE
    )


@enum_(EnumType.VISIBILITY)
class Visibility(IdEnum):
    # ...?
    # BLOCK = 2
    PAGE = 4
    MODULE = 6
    BENCH = 8
    PUBLIC = 10


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

    NODE_ANCESTOR = 1
    NODE_ANCESTOR_OR_SELF = 2
    NODE_PARENT = 3
    NODE_CHILDREN = 4
    NODE_REGULAR = 5
    NODE_TEMPLATE = 6
    STRUCT_PARENT = 10
    STRUCT_CHILD = 11
    PROPERTY = 20

    @property
    def is_node_tree(self):
        return self.id <= 4

    @property
    def is_node(self):
        return self.id < 10

    @property
    def is_struct_tree(self):
        return self.id >= 10 and self.id <= 20


#
# Access
# Access types are loosely ranked by access/destructiveness across and within types.
#


@enum_(EnumType.READ_TYPE)
class ReadType(IdEnum):
    """Ways to read nodes."""

    """Any direct read for specific nodes."""
    GET = 1
    """Search all nodes."""
    SEARCH = 2
    """Aggregate statistics."""
    AGGREGATE = 3

    @property
    def kind(self) -> "AccessKind":
        return AccessKind.READ


@enum_(EnumType.EDIT_TYPE)
class EditType(IdEnum):
    """Ways to edit nodes."""

    CREATE = 20
    UPSERT = 21
    UPDATE = 22
    MOVE = 23
    ARCHIVE = 24
    UNARCHIVE = 25
    DELETE = 26
    RESTORE = 27
    ERASE = 28

    @property
    def kind(self) -> "AccessKind":
        return AccessKind.EDIT


@enum_(EnumType.USE_TYPE)
class UseType(IdEnum):
    """Ways to use nodes."""

    START = 40
    PAUSE = 41
    RESUME = 42
    STOP = 43
    KILL = 44
    SEND = 45
    RECEIVE = 46

    @property
    def kind(self) -> "AccessKind":
        return AccessKind.USE


@enum_(EnumType.ACCESS_KIND)
class AccessKind(IdEnum):
    READ = 1
    EDIT = 20
    USE = 40

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
    enum_(EnumType.ACCESS_TYPE)(AccessType)

READ_TYPES: bittuple[ReadType] = bittuple(*ReadType)
EDIT_TYPES: bittuple[EditType] = bittuple(*EditType)
USE_TYPES: bittuple[UseType] = bittuple(*UseType)
ACCESS_TYPES: bittuple[AccessType] = bittuple(*AccessType)
ACCESS_CLASSES: tuple[type[AccessType], ...] = (ReadType, EditType, UseType, AccessType)
ACCESS_KINDS = bittuple(*AccessKind)
ACCESS_TYPES_BY_KIND: dict[AccessKind, bittuple[AccessType]] = {
    AccessKind.READ: bittuple(*READ_TYPES),
    AccessKind.EDIT: bittuple(*EDIT_TYPES),
    AccessKind.USE: bittuple(*USE_TYPES),
}
ACCESS_CLASS_BY_KIND: dict[AccessKind, type[AccessType]] = {
    AccessKind.READ: ReadType,
    AccessKind.EDIT: EditType,
    AccessKind.USE: UseType,
}
ACCESS_KIND_BY_ACCESS: dict[AccessType, AccessKind] = {
    access: kind for kind, access_types in ACCESS_TYPES_BY_KIND.items() for access in access_types
}
CASCADING_EDIT_TYPES: bittuple[EditType] = bittuple(
    EditType.ARCHIVE, EditType.UNARCHIVE, EditType.DELETE, EditType.RESTORE, EditType.ERASE
)


@enum_(EnumType.ACCESS_MODE)
class AccessMode(IdEnum):
    ADAPTIVE = 1
    ATOMIC = 2


#
# Other stuff
#


@enum_(EnumType.POLICY_EFFECT)
class PolicyEffect(IdEnum):
    ALLOW = 1
    DENY = 2
    # YIELD?, METER, LIMIT, ...


@enum_(EnumType.PRIMITIVE_TYPE)
class PrimitiveType(IdEnum):
    """
    Fundamental column / storage types we support (subset of SQL types, used directly in sql/core).
    NOTE: the ids here are used in type identity keys, so any change is breaking.
    """

    BOOLEAN = 1
    # ...
    INT16 = 5  # range: -32768 to 32767
    INT32 = 6  # range: -2147483648 to 2147483647
    INT64 = 7  # range: -9223372036854775808 to 9223372036854775807
    DECIMAL = 9  # numeric(precision, scale)
    # ...
    FLOAT32 = 12  # range: 1.175494351e-38 to 3.402823466e+38
    FLOAT64 = 13  # range: 2.2250738585072014e-308 to 1.7976931348623157e+308
    # ...
    STRING = 20
    UUID = 21
    JSON = 22
    BYTES = 25
    VECTOR = 26
    DATETIME = 30
    INTERVAL = 31

    @property
    def is_numeric(self) -> bool:
        return self.id >= 5 and self.id <= 15


PrimitiveValue = bool | int | float | str | bytes | UUID | datetime | timedelta

PY_TYPE_BY_PRIMITIVE_TYPE: dict[PrimitiveType, type] = {
    PrimitiveType.BOOLEAN: bool,
    PrimitiveType.INT16: int,
    PrimitiveType.INT32: int,
    PrimitiveType.INT64: int,
    PrimitiveType.FLOAT32: float,
    PrimitiveType.FLOAT64: float,
    PrimitiveType.STRING: str,
    PrimitiveType.BYTES: bytes,
    PrimitiveType.UUID: UUID,
    PrimitiveType.DATETIME: datetime,
    PrimitiveType.INTERVAL: timedelta,
}
PRIMITIVE_TYPE_BY_PY_TYPE: dict[type, PrimitiveType] = {
    bool: PrimitiveType.BOOLEAN,
    int: PrimitiveType.INT32,
    float: PrimitiveType.FLOAT32,
    str: PrimitiveType.STRING,
    bytes: PrimitiveType.BYTES,
    datetime: PrimitiveType.DATETIME,
    timedelta: PrimitiveType.INTERVAL,
    UUID: PrimitiveType.UUID,
}


@enum_(EnumType.FORMAT_HINT)
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


@enum_(EnumType.TYPE_KIND)
class TypeKind(IdEnum):
    """The 'kind' of a Type."""

    PRIMITIVE = 1
    STRUCT = 2
    NODE = 3
    ENUM = 4
    BASED_NODE = 5
    OBJECT = 6
    LITERAL = 7
    ALIAS = 10


@enum_(EnumType.FIELD_ZONE)
class FieldZone(IdEnum):
    """The 'zone' of a Field within its Block."""

    VARIABLE = 1
    MEMBER = 2
    INPUT = 3
    OUTPUT = 4
    OPTION = 5
    RUNTIME = 6


@enum_(EnumType.TRIGGER_TYPE)
class TriggerType(IdEnum):
    """Triggers for blocks (for both actual runs and pre-defined triggers)."""

    SCHEDULE = 1
    SIGNAL = 2


@enum_(EnumType.SCHEDULE_TYPE)
class ScheduleType(IdEnum):
    INTERVAL = 1
    CRON = 2


@enum_(EnumType.TIME_INTERVAL)
class TimeInterval(IdEnum):
    SECOND = 2
    MINUTE = 3
    HOUR = 4
    DAY = 5
    WEEK = 6
    MONTH = 7
    YEAR = 8


@enum_(EnumType.DAY)
class Day(IdEnum):
    MONDAY = 1
    TUESDAY = 2
    WEDNESDAY = 3
    THURSDAY = 4
    FRIDAY = 5
    SATURDAY = 6
    SUNDAY = 7


@enum_(EnumType.MONTH)
class Month(IdEnum):
    JANUARY = 1
    FEBRUARY = 2
    MARCH = 3
    APRIL = 4
    MAY = 5
    JUNE = 6
    JULY = 7
    AUGUST = 8
    SEPTEMBER = 9
    OCTOBER = 10
    NOVEMBER = 11
    DECEMBER = 12


@enum_(EnumType.ISSUE_KIND)
class IssueKind(IdEnum):
    """Type of diagnostic in increasing severity."""

    HINT = 1
    INFO = 2
    WARNING = 3
    ERROR = 4


@enum_(EnumType.RUN_KIND)
class RunKind(IdEnum):
    BLOCK = 1
    STEP = 2
    LAMBDA = 10


@enum_(EnumType.RUN_ERROR_KIND)
class RunErrorKind(IdEnum):
    INTERNAL = 1
    RUNTIME = 5


@enum_(EnumType.RUN_STATUS)
class RunStatus(IdEnum):
    SCHEDULED = 1
    QUEUED = 2
    # active
    RUNNING = 3
    # active+halted
    PAUSED = 4
    SUSPENDED = 5
    # terminal
    CANCELLED = 6
    ABORTED = 7
    FAILED = 8
    COMPLETED = 9

    @property
    def is_active(self) -> bool:
        return self in ACTIVE_RUN_STATUSES

    @property
    def is_halted(self) -> bool:
        return self in HALTED_RUN_STATUSES

    @property
    def is_terminal(self) -> bool:
        return self in TERMINAL_RUN_STATUSES


ACTIVE_RUN_STATUSES = bittuple(RunStatus.RUNNING, RunStatus.PAUSED, RunStatus.SUSPENDED)
HALTED_RUN_STATUSES = bittuple(RunStatus.PAUSED, RunStatus.SUSPENDED)
TERMINAL_RUN_STATUSES: bittuple[RunStatus] = bittuple(
    RunStatus.CANCELLED,
    RunStatus.ABORTED,
    RunStatus.FAILED,
    RunStatus.COMPLETED,
)


@enum_(EnumType.SESSION_STATUS)
class SessionStatus(IdEnum):
    PENDING = 1
    OPEN = 3
    CLOSED = 6


@enum_(EnumType.EXPRESSION_KIND)
class ExpressionKind(IdEnum):
    LITERAL = 1
    FUNCTIONAL = 2
    CONDITIONAL = 3
    SORT = 4
    AGGREGATION = 5


@enum_(EnumType.LITERAL_OP)
class LiteralOp(IdEnum):
    VALUE = 100  # any freeform value
    NONE = 101
    TRUE = 102
    FALSE = 103

    @property
    def kind(self) -> "ExpressionKind":
        return ExpressionKind.LITERAL


@enum_(EnumType.FUNCTIONAL_OP)
class FunctionalOp(IdEnum):
    # math
    ADD = 200
    SUBTRACT = 201
    MULTIPLY = 202
    DIVIDE = 203
    MODULO = 204
    POWER = 205
    # ...

    @property
    def kind(self) -> "ExpressionKind":
        return ExpressionKind.FUNCTIONAL


@enum_(EnumType.CONDITIONAL_OP)
class ConditionalOp(IdEnum):
    # logical
    NOT = 301
    AND = 302
    OR = 303
    # basic comparison
    EQUALS = 310
    NOT_EQUALS = 311
    GREATER_THAN = 312
    GREATER_THAN_OR_EQUALS = 313
    LESS_THAN = 314
    LESS_THAN_OR_EQUALS = 315
    # string comparison
    MATCHES_REGEX = 320
    STARTS_WITH = 321
    ENDS_WITH = 322
    # containment
    CONTAINS = 330
    NOT_CONTAINS = 331
    IN = 332
    NOT_IN = 333
    # existence
    EXISTS = 340
    NOT_EXISTS = 341
    # vector
    NEAR = 350

    @property
    def kind(self) -> "ExpressionKind":
        return ExpressionKind.CONDITIONAL


@enum_(EnumType.AGGREGATION_OP)
class AggregationOp(IdEnum):
    EXISTS = 400
    COUNT = 401
    SUM = 402
    MIN = 403
    MAX = 404
    AVERAGE = 405
    MEDIAN = 406
    HISTOGRAM = 407

    @property
    def kind(self) -> "ExpressionKind":
        return ExpressionKind.AGGREGATION


@enum_(EnumType.SORT_OP)
class SortOp(IdEnum):
    ASCENDING = 500
    DESCENDING = 501

    @property
    def kind(self) -> "ExpressionKind":
        return ExpressionKind.SORT


@enum_(EnumType.SORT_MODE)
class SortMode(IdEnum):
    MAX = 1
    MIN = 2
    AVERAGE = 3
    SUM = 4
    MEDIAN = 5


EXPRESSION_OPS_BY_KIND: Mapping[ExpressionKind, bittuple["ExpressionOp"]] = {  # type: ignore
    ExpressionKind.LITERAL: bittuple(*LiteralOp),
    ExpressionKind.FUNCTIONAL: bittuple(*FunctionalOp),
    ExpressionKind.CONDITIONAL: bittuple(*ConditionalOp),
    ExpressionKind.AGGREGATION: bittuple(*AggregationOp),
    ExpressionKind.SORT: bittuple(*SortOp),
}
EXPRESSION_KIND_BY_OP: Mapping["ExpressionOp", ExpressionKind] = {  # type: ignore
    op: kind
    for kind, ops in EXPRESSION_OPS_BY_KIND.items()  # type: ignore
    for op in ops
}

if typing.TYPE_CHECKING:
    ExpressionOp = LiteralOp | FunctionalOp | ConditionalOp | AggregationOp | SortOp
else:
    ExpressionOp = IdEnum.combine(
        "ExpressionOp", LiteralOp, FunctionalOp, ConditionalOp, AggregationOp, SortOp
    )
    ExpressionOp.kind = property(lambda self: EXPRESSION_KIND_BY_OP[self])
    enum_(EnumType.EXPRESSION_OP)(ExpressionOp)


@enum_(EnumType.CLIENT_TYPE)
class ClientType(IdEnum):
    # user
    BENCH_WEB = 1
    BENCH_BROWSER_PLUGIN = 2
    BENCH_DESKTOP = 3
    BENCH_MOBILE = 4

    # server
    BENCH_SERVER = 10


@enum_(EnumType.USER_STATUS)
class UserStatus(IdEnum):
    INVITED = 1  # invited via email
    RESERVED = 2  # reserved a handle, unconfirmed
    WAITLISTED = 3  # got handle, confirmed email, waiting
    REGISTERED = 4  # got handle, confirmed email, ready to activate
    ACTIVATED = 10  # has main bench, all ready to go


@enum_(EnumType.ORGANIZATION_STATUS)
class OrganizationStatus(IdEnum):
    # NOTE UserStatus/OrganizationStatus ids for same statuses should match
    REGISTERED = 4  # created org
    ACTIVATED = 10  # has main bench


@enum_(EnumType.NOTIFICATION_KIND)
class NotificationKind(IdEnum):
    """
    The level of interaction required for a notification.
    """

    PASSIVE = 1  # no quick action required, not urgent
    ACTIVE = 2  # important action required / may want to know this as soon as possible
    URGENT = 3  # immediate action required


#
# Other global stuff
#

REGION = get_from_env("REGION", typ=Region, description="Region we're running in")


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


def get_active_session() -> Optional["Session"]:
    """Gets the currently active Session (if any)."""
    return _active_session.get()


def active_bench() -> "Bench":
    """Gets the Bench of the currently active Session"""
    session = _active_session.get()
    assert session is not None, "no active session"
    assert session.parent is not None, f"no active bench in {session!r}"
    return session.bench


def get_active_bench() -> Optional["Bench"]:
    """Gets the Bench of the currently active Session (if any)."""
    session = _active_session.get()
    if session is None:
        return None
    return session.bench


def active_tx() -> "Transaction":
    """Gets the currently active Transaction (error if none)."""
    session = _active_session.get()
    assert session is not None, "no active session"
    assert session._tx is not None, f"no active transaction in {session!r}"
    return session._tx


def get_active_tx() -> Optional["Transaction"]:
    """Gets the currently active Transaction (if any)."""
    session = _active_session.get()
    if session is None:
        return None
    return session._tx


def get_region() -> Region:
    bench = get_active_bench()
    if bench is not None:
        return bench.region
    else:
        return REGION

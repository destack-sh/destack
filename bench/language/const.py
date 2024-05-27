import contextvars
import enum
import secrets
import typing
from datetime import datetime, timedelta
from typing import Any, Mapping, Optional, cast
from uuid import UUID, uuid4

from bench.proto.wire import GraphScope
from bench.utils.func import IdEnum, bittuple, cyrb53a
from bench.utils.utils import frozendict

if typing.TYPE_CHECKING:
    from bench.language import Bench, Run, Session, Transaction

VERSION = "2024.05.27.2"
REVISION_PENDING = -1
TK_LENGTH_BYTES = 8
TK_LENGTH_B64 = 12  # 1.5 * TK_LENGTH_BYTES (must be integer)
UNSET = cast(Any, object())
EMPTY_LIST: list = []
EMPTY_SET: frozenset = frozenset()
EMPTY_DICT: typing.Mapping = frozendict()
EMPTY_SCOPE = GraphScope()


def new_struct_id() -> int:
    id = secrets.randbits(32)
    if id < 0:
        id = -id
    return id


new_node_id = uuid4

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
#


class EnumType(IdEnum):
    ENUM_TYPE = 2001  # so meta
    NODE_TYPE = 2002
    STRUCT_TYPE = 2003
    OBJECT_TYPE = 2004  # NodeType | StructType
    BENCH_TYPE = 2005  # NodeType | StructType | EnumType
    VISIBILITY = 2010

    # access
    ACCESS_MODE = 2030
    ACCESS_KIND = 2033
    READ_TYPE = 2034
    EDIT_TYPE = 2035
    USE_TYPE = 2036
    ACCESS_TYPE = 2037  # ReadType | EditType | UseType
    POLICY_EFFECT = 2038

    # bench
    REGION = 2050
    TENANCY = 2051
    STORE_KIND = 2052
    STORE_ENGINE_TYPE = 2053
    SERVER_PROFILE = 2055
    MACHINE_PROFILE = 2056
    RESOURCE_STATUS = 2057
    FILE_RETENTION_MODE = 2058
    CLIENT_TYPE = 2060

    # block
    BLOCK_TYPE = 2070

    # type
    PRIMITIVE_TYPE = 2080
    FORMAT_HINT = 2081
    FIELD_ZONE = 2082
    TYPE_KIND = 2083

    # text
    TEXT_LINE_TYPE = 2090

    # time
    SCHEDULE_TYPE = 2100
    TIME_INTERVAL = 2101
    DAY = 2102
    MONTH = 2103

    # notice
    NOTICE_TYPE = 2170

    # flow
    STEP_TYPE = 2180

    # view
    SPACE_TYPE = 2200
    VIEW_TYPE = 2201
    VARIANT = 2202
    COLOR_TYPE = 2203
    COLOR_SHADE = 2204
    FONT_TYPE = 2205
    FONT_WEIGHT = 2206
    FONT_SIZE = 2207
    SPACING = 2208
    ANCHOR = 2209
    ORIENTATION = 2210
    ALIGNMENT = 2211
    ICON_KIND = 2212

    # session
    LOG_KIND = 2250
    LOG_LEVEL = 2251
    RUN_STATUS = 2260
    RUN_KIND = 2261
    RUN_ERROR_KIND = 2262
    RUN_ERROR_TYPE = 2263
    SESSION_STATUS = 2264
    TRIGGER_TYPE = 2270
    NOTICE_KIND = 2280
    NOTIFICATION_KIND = 2281

    # expression
    EXPRESSION_KIND = 2300
    EXPRESSION_OP = 2301
    CONDITIONAL_OP = 2302
    AGGREGATION_OP = 2303
    SORT_OP = 2304
    SORT_MODE = 2305
    SELECTION_KIND = 2306
    PATH_TOKEN_TYPE = 2310
    PATH_SEGMENT_TYPE = 2311

    # user
    USER_STATUS = 2500
    ORGANIZATION_STATUS = 2501


enum_(EnumType.ENUM_TYPE)(EnumType)
ENUM_TYPES: bittuple[EnumType] = bittuple(*EnumType)
ENUM_TYPES_SET: frozenset[EnumType] = frozenset(ENUM_TYPES)

#
# Struct/Node metatypes
#


@enum_(EnumType.NODE_TYPE)
class NodeType(IdEnum):
    # root
    BENCH = 1
    ENVIRONMENT = 2
    BRANCH = 3

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
    FIELD = 32  # (based)
    QUERY = 33
    VIEW = 34
    STEP = 35
    # TAG?
    # REACTION?
    # LOCK?
    # BREAKPOINT?

    # auth
    BADGE = 60
    ROLE = 61
    IDENTITY = 62
    MEMBERSHIP = 63
    INVITE = 64

    # runtime
    SESSION = 80  # (local)
    RUN = 81  # (local, based)
    SIGNAL = 82  # (local, based)
    LOG = 83  # (local)
    NOTIFICATION = 84  # (local, based)
    MESSAGE = 85  # (local, based)
    RECORD = 86  # (local, based)

    # resources (compute/storage/external/etc.)
    SERVER = 160
    STORE = 161  # any 'database' (Postgres/OpenSearch/ClickHouse)
    MACHINE = 162  # actual machine providing compute and such
    DRIVE = 163  # object store like S3/MinIO, maybe block storage later
    BLOB = 164  # in a Drive
    # CACHE = ...  # KV memory store (Redis/Memcached)
    # DOMAIN, EMAIL, ...

    # user
    HANDLE = 220
    USER = 221
    ORGANIZATION = 222
    CLIENT = 223


# :NodeTypes
NODE_TYPES = bittuple(*NodeType)
NODE_TYPES_SET: frozenset[NodeType] = frozenset(NODE_TYPES)
ROOT_NODE_TYPES = bittuple(NodeType.BENCH, NodeType.USER, NodeType.ORGANIZATION)
LOCAL_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if 80 <= nt.id < 100))
GLOBAL_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if nt not in LOCAL_NODE_TYPES))
BASED_NODE_TYPES = bittuple(  # :HasBase
    NodeType.FIELD,
    NodeType.RUN,
    NodeType.SIGNAL,
    NodeType.NOTIFICATION,
    NodeType.MESSAGE,
    NodeType.RECORD,
)
RUNTIME_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if 80 <= nt.id < 100))
TIMED_NODE_TYPES = bittuple(
    NodeType.SESSION,
    NodeType.RUN,
    NodeType.SIGNAL,
    NodeType.LOG,
    NodeType.NOTIFICATION,
    NodeType.MESSAGE,
)
IN_PACKAGE_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if 20 <= nt.id < 100))
SUB_PACKAGE_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if 20 < nt.id < 100))
IN_BENCH_NODE_TYPES = bittuple(
    *(*tuple(nt for nt in NODE_TYPES if nt.id < 200), NodeType.CLIENT, NodeType.HANDLE)
)
IN_BENCH_GLOBAL_NODE_TYPES = bittuple(
    *tuple(nt for nt in IN_BENCH_NODE_TYPES if nt not in LOCAL_NODE_TYPES)
)
SUB_BENCH_NODE_TYPES = bittuple(*tuple(nt for nt in IN_BENCH_NODE_TYPES if nt != NodeType.BENCH))
RESOURCE_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if 160 <= nt.id < 200))
BENCH_NODE_TYPES = bittuple(
    NodeType.BENCH,
    NodeType.ENVIRONMENT,
    NodeType.BRANCH,
    NodeType.PACKAGE,
    NodeType.HANDLE,
    NodeType.CLIENT,
    *RESOURCE_NODE_TYPES,
)
PUBLIC_NODE_TYPES = bittuple(NodeType.USER, NodeType.ORGANIZATION)
USER_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if nt.id >= 200))


@enum_(EnumType.STRUCT_TYPE)
class StructType(IdEnum):
    # core
    PATH = 1000
    PATH_SEGMENT = 1001
    PATH_TOKEN = 1002
    NODE_REFERENCE = 1003
    PROPERTY_REFERENCE = 1004
    VALUE_REFERENCE = 1005
    TYPE_INFO = 1010
    TYPE_CONSTRAINT = 1011
    CONTEXT = 1020
    SCHEDULE = 1012
    PROJECTION = 1013

    # access
    POLICY = 1030
    POLICY_RULE = 1031
    SUBJECT = 1032
    ACCESS_ZONE = 1034
    ACCESS_MATRIX = 1035
    ACCESS = 1037
    ...
    READ_OPTIONS = 1050

    # expressions
    EXPRESSION = 1060
    AGGREGATION = 1061
    AGGREGATION_BUCKET = 1062
    SELECTION = 1063

    # code
    CODE = 1090
    CODE_LINE = 1091
    # flow
    STEP_CONNECTION = 1100
    RUN_ERROR = 1110
    # CURSOR?

    # text
    TEXT = 1160
    TEXT_LINE = 1161
    TEXT_SPAN = 1162

    # views
    COLOR = 1200
    FONT = 1201
    BOX = 1202
    OFFSET = 1203
    ...

    # files
    FILE = 1250
    ICON = 1251

    # space
    ...

    # shapes
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
    SIGNAL = 13  # define a signal type with fields
    PROTOCOL = 14  # define a 'protocol' for a block graph/template with fields
    # NOTICE = ...  # define a new notice type
    # NOTIFICATION = ...  # define a new notification type
    # METRIC = ...  # define a new metric type
    # BLOCK = ...  # define a new block type?

    # runnable
    TEXT = 30  # define a 'paragraph' of text/comment/instruction with fields (incl. input/output)
    CODE = 31  # define a code function with fields (incl. input/output)
    SCRIPT = 32  # define a code script with exported code-level constructs
    FLOW = 33  # define a flow with steps and fields (optionally incl. input/output)

    # state
    VARIABLE = 50  # define a single-value variable
    QUERY = 52  # define a set of queries
    DATABASE = 53  # define a database with records & queries

    # view
    SCREEN = 70  # define a screen with views

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

    NODE_ANCESTOR_ROOT = 1
    NODE_ANCESTOR_FIRST = 2
    NODE_PARENT = 3
    NODE_CHILDREN = 4
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


NRel = NodeRelationFlag


class InterpStatus(IdEnum):
    # TODO :Cleanup :Architecture: clarify/simplify node lifecycle
    #  when interp? what does it do? can Node. _session while status != tracked?
    SOURCE = 1  # just loaded
    INTERPED = 2  # everything resolved & ready
    TRACKED = 3  # live in a session


#
# Access
# Access types are loosely ranked by access/destructiveness across and within types.
#


@enum_(EnumType.READ_TYPE)
class ReadType(IdEnum):
    """A type of Read access on nodes."""

    GET = 1  # any direct read access
    AGGREGATE = 3  # count, sum, min, etc.
    LIST = 5  # list, search, filter, etc.

    @property
    def kind(self) -> "AccessKind":
        return AccessKind.READ


@enum_(EnumType.EDIT_TYPE)
class EditType(IdEnum):
    """A type of Edit access on nodes."""

    CREATE = 20
    UPSERT = 21
    UPDATE = 22
    MOVE = 23
    ARCHIVE = 24
    UNARCHIVE = 25
    SOFT_DELETE = 26
    RESTORE = 27
    DELETE = 28

    @property
    def kind(self) -> "AccessKind":
        return AccessKind.EDIT


@enum_(EnumType.USE_TYPE)
class UseType(IdEnum):
    """A type of Run access on nodes."""

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


@enum_(EnumType.ACCESS_MODE)
class AccessMode(IdEnum):
    ADAPTIVE = 1
    ATOMIC = 2


#
# Other stuff
#


@enum_(EnumType.STORE_KIND)
class StoreKind(IdEnum):
    RELATIONAL = 1
    # SEARCH, ANALYTICAL, ...


@enum_(EnumType.STORE_ENGINE_TYPE)
class StoreEngineType(IdEnum):
    LOCAL = 1
    REMOTE = 2
    POSTGRES = 3
    # CLICKHOUSE, ...


@enum_(EnumType.POLICY_EFFECT)
class PolicyEffect(IdEnum):
    ALLOW = 1
    DENY = 2
    # DEFER?, METER, LIMIT, ...


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
    ALIAS = 10


@enum_(EnumType.FIELD_ZONE)
class FieldZone(IdEnum):
    """The 'zone' of a Field within its Block."""

    VARIABLE = 1
    MEMBER = 2
    INPUT = 3
    OUTPUT = 4
    OPTION = 5


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


@enum_(EnumType.NOTICE_KIND)
class NoticeKind(IdEnum):
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


@enum_(EnumType.RUN_STATUS)
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

    @property
    def is_active(self) -> bool:
        return self in ACTIVE_RUN_STATUSES

    @property
    def is_terminal(self) -> bool:
        return self in TERMINAL_RUN_STATUSES


TERMINAL_RUN_STATUSES: bittuple[RunStatus] = bittuple(
    RunStatus.CANCELLED,
    RunStatus.ABORTED,
    RunStatus.FAILED,
    RunStatus.COMPLETED,
)
ACTIVE_RUN_STATUSES = bittuple(RunStatus.RUNNING, RunStatus.PAUSED, RunStatus.ABORTING)


@enum_(EnumType.SESSION_STATUS)
class SessionStatus(IdEnum):
    PENDING = 1
    OPEN = 3
    CLOSED = 6


@enum_(EnumType.EXPRESSION_KIND)
class ExpressionKind(IdEnum):
    CONDITIONAL = 1
    SORT = 2
    AGGREGATION = 3


@enum_(EnumType.CONDITIONAL_OP)
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

    @property
    def kind(self) -> "ExpressionKind":
        return ExpressionKind.CONDITIONAL


@enum_(EnumType.AGGREGATION_OP)
class AggregationOp(IdEnum):
    EXISTS = 100
    COUNT = 101
    SUM = 102
    AVERAGE = 103
    MIN = 104
    MAX = 105
    MEDIAN = 106
    HISTOGRAM = 107

    @property
    def kind(self) -> "ExpressionKind":
        return ExpressionKind.AGGREGATION


@enum_(EnumType.SORT_OP)
class SortOp(IdEnum):
    ASCENDING = 200
    DESCENDING = 201

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
    ExpressionKind.CONDITIONAL: bittuple(*ConditionalOp),
    ExpressionKind.AGGREGATION: bittuple(*AggregationOp),
    ExpressionKind.SORT: bittuple(*SortOp),
}
EXPRESSION_KIND_BY_OP: Mapping["ExpressionOp", ExpressionKind] = {
    op: kind
    for kind, ops in EXPRESSION_OPS_BY_KIND.items()
    for op in ops  # type: ignore
}

if typing.TYPE_CHECKING:
    ExpressionOp = ConditionalOp | AggregationOp | SortOp
else:
    ExpressionOp = IdEnum.combine("ExpressionOp", ConditionalOp, AggregationOp, SortOp)
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


class BenchError(Exception):
    """Common base class for any regular errors."""

    pass


_active_session: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)
_active_run: contextvars.ContextVar[Optional["Run"]] = contextvars.ContextVar(
    "active_run", default=None
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


def active_run() -> "Run":
    """Gets the Run of the currently active Session"""
    run = _active_run.get()
    assert run is not None, "no active run"
    return run


def get_active_run() -> Optional["Run"]:
    """Gets the Run of the currently active Session (if any)."""
    return _active_run.get()


def active_root_run() -> "Run":
    """Gets the root Run of the currently active Session"""
    run = active_run()
    return run.root


def get_active_root_run() -> Optional["Run"]:
    """Gets the root Run of the currently active Session (if any)."""
    run = get_active_run()
    if run is None:
        return None
    return run.root

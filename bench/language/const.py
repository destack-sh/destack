import contextvars
import secrets
import typing
from datetime import datetime, timedelta
from typing import Any, Mapping, Optional, TypeGuard, cast
from uuid import UUID, uuid4

from bench.utils.func import IdEnum, bittuple, cyrb53a
from bench.utils.utils import frozendict, get_from_env

if typing.TYPE_CHECKING:
    from bench.language import Session, Transaction


class _Unset:
    def __repr__(self):
        return "<UNSET!>"

    def __str__(self):
        return "<UNSET!>"


# forever constants
VERSION = "2024.09.24.0"  # auto change via version script
REVISION_PENDING = -1
TK_LENGTH_BYTES = 8
TK_LENGTH_B64 = 12  # 1.5 * TK_LENGTH_BYTES (must be integer)
FLOAT_EPSILON = 1e-6

# runtime constants
NONCE = uuid4()
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
    CHANGE_KIND = 20011

    # access
    ACCESS_MODE = 20030
    ACCESS_KIND = 20031
    READ_TYPE = 20032
    EDIT_TYPE = 20033
    USE_TYPE = 20034
    ACCESS_TYPE = 20035  # ReadType | EditType | UseType
    CHANGE_CATEGORY = 20036
    EDIT_OPERATION_TYPE = 20037
    POLICY_EFFECT = 20040

    # bench
    CLOUD = 20050
    REGION = 20051
    REGION_ZONE = 20052
    REGION_AREA = 20053
    RESOURCE_STATUS = 20054
    FILE_RETENTION_MODE = 2060
    FILE_KIND = 2061
    FILE_TYPE = 2062
    FILE_FORMAT = 2063
    CLIENT_TYPE = 20070

    # type
    PRIMITIVE_TYPE = 20080
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
    SELECTION_TARGET = 20209
    PATH_TOKEN_TYPE = 20210

    # run
    LOG_KIND = 20500
    LOG_LEVEL = 20501
    RUN_STATUS = 20510
    RUN_KIND = 20511
    RUN_ERROR_KIND = 20512
    RUN_ERROR_TYPE = 20513
    RUN_SPAN_TYPE = 20514
    RUN_EVENT_TYPE = 20515
    SESSION_STATUS = 20516
    TRIGGER_TYPE = 20520
    BREAKPOINT_KIND = 20530
    BREAKPOINT_ACTION = 20531
    CACHE_BEHAVIOR = 20540
    CODE_TYPE = 20560
    MODEL_PROVIDER = 20570
    MODEL_TYPE = 20571
    STEP_TYPE = 20600
    PIPE_TYPE = 20601
    PIPE_FILTER_TYPE = 20602
    PORT_TYPE = 20603
    PORT_SIDE = 20604
    NOTIFICATION_LEVEL = 20610

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

    # universe
    BENCH = 1
    USER = 2
    ORGANIZATION = 3
    HANDLE = 4
    CLIENT = 5

    #
    # Regional
    #

    # resource (named)
    SERVER = 500  # infinite compute
    STORE = 501  # trusty ol' postgres
    MACHINE = 502  # actual machine providing compute
    DRIVE = 503  # object store like S3/MinIO, maybe block storage later
    VAULT = 504  # secret storage
    CACHE = 505  # ephemeral key-value store
    # BROWSER, DOMAIN, EMAIL, PHONE, ...
    # resource (anonymous)
    FILE = 550
    SECRET = 551

    # auth
    MEMBERSHIP = 600
    INVITE = 601
    # CHALLENGE?

    # synchronization
    # POOL, LOCK, BARRIER, ...?

    #
    # Local (per Bench)
    #

    # source (versioned, templatable)
    BRANCH = 1000
    PACKAGE = 1001
    DEPENDENCY = 1002
    SPACE = 1003
    BLOCK = 1010
    TRIGGER = 1011
    FIELD = 1012  # (based)
    QUERY = 1013
    VIEW = 1020
    STEP = 1030
    PIPE = 1031
    BADGE = 1040
    # TAG?
    # POLICY?

    # state (versioned)
    MESSAGE = 1100  # (based, timed)
    RECORD = 1101  # (based)

    # runtime
    SESSION = 1900  # (timed)
    RUN = 1901  # (based, timed)
    SIGNAL = 1902  # (based, timed)
    LOG = 1903  # (timed)
    NOTIFICATION = 1904  # (timed)

    #
    # Misc
    #

    SKIP = 9000


# :NodeTypes
NODE_TYPES = bittuple(*NodeType)
NODE_TYPES_SET: frozenset[NodeType] = frozenset(NODE_TYPES)
ROOT_NODE_TYPES = bittuple(NodeType.BENCH, NodeType.USER, NodeType.ORGANIZATION)
GLOBAL_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if nt.id < 1000))
RESOURCE_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if nt.id >= 500 and nt.id < 600))
ANONYMOUS_RESOURCE_NODE_TYPES = bittuple(*tuple(nt for nt in RESOURCE_NODE_TYPES if nt.id >= 550))
LOCAL_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if nt.id >= 1000))
SOURCE_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if nt.id >= 1000 and nt.id < 1100))
STATE_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if nt.id >= 1100 and nt.id < 1200))
RUNTIME_NODE_TYPES = bittuple(*tuple(nt for nt in NODE_TYPES if nt.id >= 1900 and nt.id < 2000))

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
IN_PACKAGE_NODE_TYPES = bittuple(
    *tuple(nt for nt in NODE_TYPES if nt.id >= 1001 and nt.id < 2000), NodeType.SKIP
)
SUB_PACKAGE_NODE_TYPES = bittuple(*tuple(nt for nt in IN_PACKAGE_NODE_TYPES if nt.id > 1001))
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
BENCH_NODE_TYPES = bittuple(
    NodeType.BENCH,
    NodeType.BRANCH,
    NodeType.PACKAGE,
    NodeType.HANDLE,
    NodeType.CLIENT,
    *(nt for nt in NODE_TYPES if nt.id >= 500 and nt.id < 700),
)
LOADED_BENCH_NODE_TYPES = bittuple(
    *(
        nt
        for nt in BENCH_NODE_TYPES
        if nt not in STATE_NODE_TYPES and nt not in ANONYMOUS_RESOURCE_NODE_TYPES
    )
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
    EDIT_INFO = 10006
    EDIT_OPERATION = 10007
    CHANGE = 10010
    CHANGE_VIGNETTE = 10011

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
    FILE_INFO = 10203
    FILE_REFERENCE = 10204
    ICON = 10205
    SECRET_REFERENCE = 10206
    TRIGGER_INFO = 10208

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
    PORT_KEY = 10450
    PORT = 10451
    RUN_ERROR = 10500
    RUN_OPTIONS = 10501
    RUN_ATTEMPT = 10502
    RUN_TRACE = 10503
    RUN_FRAME = 10504
    RUN_SPAN = 10505
    RUN_EVENT = 10506
    BREAKPOINT = 10520
    LOG_INFO = 10550

    # space/views
    COLOR = 11000
    FONT = 11001
    BOX = 11002
    OFFSET = 11003
    TRANSFORM = 11004
    VECTOR2 = 11020
    VECTOR3 = 11021
    VECTOR4 = 11022
    LINE = 11035
    ...
    START_VIEW_STATE = 11200
    FEED_VIEW_STATE = 11201
    USER_WIZARD_VIEW_STATE = 11205
    TREE_VIEW_STATE = 11207


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

OBJECT_TYPES: bittuple[ObjectType] = bittuple(*ObjectType)  # type: ignore
OBJECT_TYPES_SET: frozenset[ObjectType] = frozenset(OBJECT_TYPES)
BENCH_TYPES: bittuple[BenchType] = bittuple(*BenchType)  # type: ignore


def is_node_type(obj: IdEnum | int | Any) -> TypeGuard[NodeType]:
    return isinstance(obj, int) and obj in NODE_TYPES_SET


def is_struct_type(obj: IdEnum | int | Any) -> TypeGuard[StructType]:
    return isinstance(obj, int) and obj in STRUCT_TYPES_SET


def is_object_type(obj: IdEnum | int | Any) -> TypeGuard[ObjectType]:
    return isinstance(obj, int) and obj in OBJECT_TYPES_SET


def is_enum_type(obj: IdEnum | int | Any) -> TypeGuard[EnumType]:
    return isinstance(obj, int) and obj in ENUM_TYPES_SET


@enum_(EnumType.CLOUD)
class Cloud(IdEnum):
    """The cloud provider."""

    # own
    ...
    # big general
    AWS = 100
    AZURE = 101
    GCP = 102
    OCI = 103
    ALIBABA = 104
    # small general
    HETZNER = 200
    # small non-general
    NEON = 300
    # private
    PRIVATE = 900

    @property
    def slug(self) -> str:
        return self.name.lower().replace("_", "-")


@enum_(EnumType.REGION_AREA)
class RegionArea(IdEnum):
    """
    Rough 'contintents' of Regions.
    """

    EUROPE = 1000
    NORTH_AMERICA = 2000
    SOUTH_AMERICA = 3000
    MIDDLE_EAST = 4000
    AFRICA = 5000
    ASIA = 6000
    AUSTRALIA = 7000
    PRIVATE = 9000
    GLOBAL = 10090

    @property
    def slug(self) -> str:
        return REGION_AREA_SLUGS[self]

    @staticmethod
    def get_by_slug(slug: str) -> "RegionArea":
        return REGION_AREA_BY_SLUG[slug]


REGION_AREA_SLUGS: dict[RegionArea, str] = {
    RegionArea.EUROPE: "eu",
    RegionArea.NORTH_AMERICA: "na",
    RegionArea.SOUTH_AMERICA: "sa",
    RegionArea.MIDDLE_EAST: "me",
    RegionArea.AFRICA: "af",
    RegionArea.ASIA: "as",
    RegionArea.AUSTRALIA: "au",
}
REGION_AREA_BY_SLUG = {v: k for k, v in REGION_AREA_SLUGS.items()}


@enum_(EnumType.REGION_ZONE)
class RegionZone(IdEnum):
    """
    A larger zone of Regions within an Area.
    """

    EUROPE_CENTRAL = 1000
    NORTH_AMERICA_EAST = 2000
    NORTH_AMERICA_WEST = 2100
    SOUTH_AMERICA_EAST = 3000
    MIDDLE_EAST_CENTRAL = 4000
    MIDDLE_EAST_WEST = 4100
    AFRICA_SOUTH = 5000
    ASIA_WEST = 6000
    ASIA_SOUTH = 6100
    ASIA_EAST = 6200
    AUSTRALIA_SOUTH = 7000

    @property
    def area(self) -> RegionArea:
        return RegionArea((self.id // 1000) * 1000)

    @property
    def slug(self) -> str:
        zone = self.name.split("_")[-1]
        return REGION_AREA_SLUGS[self.area] + "-" + zone.replace("_", "-").lower()

    @staticmethod
    def get_by_slug(slug: str) -> "RegionZone":
        return REGION_ZONE_BY_SLUG[slug]


REGION_ZONE_SLUGS: dict[RegionZone, str] = {r: r.slug for r in RegionZone}
REGION_ZONE_BY_SLUG = {v: k for k, v in REGION_ZONE_SLUGS.items()}


@enum_(EnumType.REGION)
class Region(IdEnum):
    """Actual regions within a Zone in an Area."""

    # eu-central
    ZURICH = 1001
    FRANKFURT = 1002

    # na-east
    VIRGINIA = 2001
    OHIO = 2002

    # na-west
    OREGON = 2101

    # sa-east
    SAO_PAULO = 3001

    ...

    # af-south
    CAPE_TOWN = 5001

    # as-east
    MUMBAI = 6001

    # as-south
    SINGAPORE = 6101

    # as-east
    TOKYO = 6201

    # au-south
    SYDNEY = 7001

    @property
    def zone(self) -> RegionZone:
        return RegionZone((self.id // 100) * 100)

    @property
    def area(self) -> RegionArea:
        return RegionArea((self.id // 1000) * 1000)

    @property
    def slug(self) -> str:
        area = self.area
        return area.slug + "-" + self.name.replace("_", "-").lower()

    @staticmethod
    def get_by_slug(slug: str) -> "Region":
        return REGION_BY_SLUG[slug]


REGION_SLUGS: dict[Region, str] = {r: r.slug for r in Region}
REGION_BY_SLUG = {v: k for k, v in REGION_SLUGS.items()}


@enum_(EnumType.BLOCK_TYPE)
class BlockType(IdEnum):
    PAGE = 1  # group of blocks
    # ALIAS   # refer to / 'redefine' an existing block or builtin (like a 'newtype')

    # types
    CLASS = 10  # define a class type with fields
    CHOICE = 11  # define a choice type with fields (union of literal options or oneof fields)
    SIGNAL = 12  # define a signal type with fields
    NOTIFICATION = 13  # define a new notification type
    # PROTOCOL, TAG, ISSUE, METRIC, BLOCK, ...?

    # runnable
    TEXT = 20  # define a 'paragraph' of text/prompt with fields (optionally incl. input/output)
    CODE = 21  # define a code function/script with fields (optionally incl. input/output)
    FLOW = 22  # define a flow with steps and fields (optionally incl. input/output)

    # data
    VALUE = 30  # define a single-value variable
    DATABASE = 31  # define a database with records & queries
    QUERY = 32  # define a set of queries

    # view
    VIEW = 40  # define a view (with fields and nested views)

    # auth
    ROLE = 50  # define a role with policies
    IDENTITY = 51  # define an identity with roles & policies

    @property
    def is_page(self) -> bool:
        return self in BlockTypes.PAGES

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
    PAGES = bittuple(BlockType.PAGE, BlockType.FLOW, BlockType.DATABASE, BlockType.VIEW)
    TYPES = bittuple(*tuple(t for t in BLOCK_TYPES if 10 <= t.id < 20))
    RUNNABLE = bittuple(*tuple(t for t in BLOCK_TYPES if 20 <= t.id < 30))
    CLASSES = bittuple(
        BlockType.CLASS, BlockType.SIGNAL, *RUNNABLE, BlockType.VALUE, BlockType.DATABASE
    )


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
ACCESS_TYPES: bittuple[AccessType] = bittuple(*AccessType)  # type: ignore
ACCESS_CLASSES: tuple[type[AccessType], ...] = (ReadType, EditType, UseType, AccessType)  # type: ignore
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
    # INT8? UINTs?
    INT16 = 4  # range: -32768 to 32767
    INT32 = 6  # range: -2147483648 to 2147483647
    INT64 = 8  # range: -9223372036854775808 to 9223372036854775807
    DECIMAL = 10  # numeric(precision, scale)
    # ...
    # FLOAT16?
    FLOAT32 = 16  # range: 1.175494351e-38 to 3.402823466e+38
    FLOAT64 = 17  # range: 2.2250738585072014e-308 to 1.7976931348623157e+308
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
        return self.id >= 2 and self.id < 20

    @property
    def is_int(self) -> bool:
        return self.id >= 2 and self.id <= 10

    @property
    def is_float(self) -> bool:
        return self.id >= 15 and self.id < 20


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
    UNION = 9
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
    BENCH_MACHINE = 10


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


@enum_(EnumType.NOTIFICATION_LEVEL)
class NotificationLevel(IdEnum):
    """
    The level of interaction required for a notification.
    """

    PASSIVE = 1  # no quick action required, not urgent
    ACTIVE = 2  # important action required / may want to know this as soon as possible
    URGENT = 3  # immediate action required


#
# Other global stuff
#

CLOUD = get_from_env("CLOUD", typ=Cloud, description="Cloud we're running in")
REGION = get_from_env("REGION", typ=Region, description="Region we're running in")
REGION_ZONE = REGION.zone
REGION_AREA = REGION.area


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

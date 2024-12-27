import contextvars
import enum
import secrets
import typing
from datetime import date, datetime, time, timedelta
from typing import Any, Mapping, Optional, TypeGuard, cast
from uuid import UUID, uuid4, uuid5

from bench.utils.func import IdEnum, bittuple
from bench.utils.utils import frozendict, get_from_env

if typing.TYPE_CHECKING:
    from bench.language import Session, Transaction


class _Unset:
    def __repr__(self):
        return "<UNSET!>"

    def __str__(self):
        return "<UNSET!>"


# forever constants
BENCH_SLUG = "bench"
SYSTEM_SLUG = "system"
UUID_NAMESPACE = uuid5(UUID(int=0), b"bench")
VERSION = "2024.12.27.1"
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
    # intrinsic (20000-20499)
    ENUM_TYPE = 20001  # so meta
    NODE_TYPE = 20002
    STRUCT_TYPE = 20003
    OBJECT_TYPE = 20004  # NodeType | StructType

    # access (20500-20999)
    ACCESS_MODE = 20501
    ACCESS_KIND = 20502
    QUERY_TYPE = 20503
    EDIT_TYPE = 20504
    USE_TYPE = 20505
    ACCESS_TYPE = 20506  # ReadType | EditType | UseType
    CHANGE_CATEGORY = 20507
    EDIT_OPERATION_TYPE = 20508
    POLICY_EFFECT = 20509

    # bench (21000-21499)
    BENCH_TYPE = 21001
    PACKAGE_TYPE = 21002
    CLOUD = 21010
    REGION = 21020
    REGION_ZONE = 21021
    REGION_AREA = 21022
    RESOURCE_STATUS = 21030
    RESOURCE_OCCUPANCY = 21031
    FILE_RETENTION_MODE = 21040
    FILE_KIND = 21041
    FILE_TYPE = 21042
    FILE_FORMAT = 21043
    CLIENT_TYPE = 21050
    MACHINE_TYPE = 21060
    BROWSER_TYPE = 21070

    # type (21500-21999)
    PRIMITIVE_TYPE = 21501
    FIELD_ZONE = 21502
    TYPE_KIND = 21503
    TYPE_FORMAT = 21504
    OBJECT_KIND = 21505
    BLOCK_TYPE = 21506
    PARTIAL_OBJECT_SCOPE = 21508

    # basic (22000-22499)
    SCHEDULE_TYPE = 22001
    TIME_INTERVAL = 22002
    DAY = 22003
    MONTH = 22004
    TEXT_LINE_TYPE = 22005
    ICON_KIND = 22006
    MESSAGE_TYPE = 22007
    MESSAGE_STATUS = 22008

    # expression (22500-22999)
    EXPRESSION_KIND = 22501
    EXPRESSION_OP = 22502
    LITERAL_TYPE = 22503
    FUNCTIONAL_TYPE = 22504
    CONDITIONAL_TYPE = 22505
    AGGREGATION_TYPE = 22506
    SORT_MODE = 22507
    SORT_TYPE = 22508
    PATH_TOKEN_TYPE = 22509

    # run (23000-23499)
    LOG_KIND = 23001
    LOG_LEVEL = 23002
    RUN_STATUS = 23003
    RUN_TYPE = 23004
    RUN_ERROR_KIND = 23005
    RUN_ERROR_TYPE = 23006
    RUN_SPAN_TYPE = 23007
    RUN_EVENT_TYPE = 23008
    SESSION_STATUS = 23009
    TRIGGER_TYPE = 23010
    BREAKPOINT_SITE = 23011
    BREAKPOINT_ACTION = 23012
    BREAKPOINT_TARGET = 23013
    INTERRUPT_TYPE = 23014
    INTERRUPT_STATUS = 23015
    CACHE_MODE = 23016
    CODE_TYPE = 23017
    MODEL_PROVIDER = 23018
    MODEL_TYPE = 23019
    ACTION_TYPE = 23020
    PORT_SIDE = 23021
    PIPE_TYPE = 23022
    NODE_MODE = 23024

    # view (23500-23999)
    SPACE_TYPE = 23501
    VIEW_TYPE = 23502
    COLOR_TYPE = 23504
    COLOR_SHADE = 23505
    FONT_TYPE = 23506
    FONT_WEIGHT = 23507
    FONT_SIZE = 23508
    SPACING = 23509
    ANCHOR = 23510
    ORIENTATION = 23511
    ALIGNMENT = 23512
    USER_WIZARD_STAGE = 23513
    TREE_VIEW_PRESET = 23514
    HUB_ASPECT = 23515
    HELP_ASPECT = 23516
    BUTTON_VARIANT = 23517
    PICKER_VARIANT = 23518

    # user (24000-24499)
    USER_STATUS = 24001
    ORGANIZATION_STATUS = 24002


enum_(EnumType.ENUM_TYPE)(EnumType)
ENUM_TYPES: bittuple[EnumType] = bittuple(*EnumType)
ENUM_TYPES_SET: frozenset[EnumType] = frozenset(ENUM_TYPES)

#
# Node metatypes
#


class NodeArea(enum.StrEnum):
    GLOBAL = "global"
    REGIONAL = "regional"
    LOCAL = "local"


@enum_(EnumType.NODE_TYPE)
class NodeType(IdEnum):
    #
    # Global (1-1000)
    #

    # universe
    BENCH = 1
    USER = 2
    HANDLE = 4
    CLIENT = 5
    ORGANIZATION = 10
    # TEAM/GROUP?

    # auth
    MEMBERSHIP = 100
    INVITE = 101
    # CHALLENGE?

    #
    # Regional (2000-3000)
    #

    # resource (virtual)
    SERVER = 2000  # elastic compute
    STORE = 2001  # real database
    DRIVE = 2003  # object store like S3/MinIO, maybe block storage later
    VAULT = 2004  # secret storage
    CACHE = 2005  # ephemeral key-value store

    # resource (physical)
    MACHINE = 2100
    BROWSER = 2101
    FILE = 2110
    STREAM = 2111
    SECRET = 2112
    # ACCOUNT, DOMAIN, EMAIL, PHONE, APPLICATION, ...

    # synchronization
    # POOL, LOCK, BARRIER, CONDITION, ...?

    #
    # Local (3000-4000)
    #

    # source (named, versioned, templatable)
    PACKAGE = 3001
    DEPENDENCY = 3002
    SPACE = 3003
    BLOCK = 3010
    TRIGGER = 3011
    FIELD = 3012  # (based)
    QUERY = 3013
    VIEW = 3020
    ACTION = 3030
    PIPE = 3031
    # BADGE? POLICY?
    # TAG?

    # state
    MESSAGE = 3500  # (based, timed)
    RECORD = 3501  # (based)

    # runtime
    SESSION = 3600  # (timed)
    RUN = 3601  # (based, timed)
    INTERRUPT = 3602  # (timed)
    LOG = 3603  # (timed)

    #
    # Misc
    #

    SKIP = 9000

    @property
    def is_global(self) -> bool:
        return self.id < 1000

    @property
    def is_regional(self) -> bool:
        return self.id >= 2000 and self.id < 3000

    @property
    def is_local(self) -> bool:
        return self.id >= 3000

    @property
    def area(self) -> NodeArea:
        return AREA_BY_NODE_TYPE[self]

    @property
    def is_resource(self) -> bool:
        return self.id >= 2000 and self.id < 3000

    @property
    def is_source(self) -> bool:
        return self.id >= 3000 and self.id < 3100

    @property
    def is_state(self) -> bool:
        return self.id >= 3500 and self.id < 3600

    @property
    def is_runtime(self) -> bool:
        return self.id >= 3600 and self.id < 3700


# :NodeTypes
NODE_TYPES = bittuple(*NodeType)
NODE_TYPES_SET: frozenset[NodeType] = frozenset(NODE_TYPES)


def _get_node_types(
    start: int | None = None, end: int | None = None, *extra_node_types: NodeType
) -> bittuple[NodeType]:
    if start is None:
        start = 0
    if end is None:
        end = 10000
    node_types = bittuple(*tuple(nt for nt in NODE_TYPES if nt.id >= start and nt.id < end))
    if extra_node_types:
        node_types = node_types | bittuple(*extra_node_types)
    return node_types


GLOBAL_NODE_TYPES = _get_node_types(None, 1000)
REGIONAL_NODE_TYPES = _get_node_types(2000, 3000)
LOCAL_NODE_TYPES = _get_node_types(3000, None)
AREA_BY_NODE_TYPE = {
    **dict.fromkeys(GLOBAL_NODE_TYPES, NodeArea.GLOBAL),
    **dict.fromkeys(REGIONAL_NODE_TYPES, NodeArea.REGIONAL),
    **dict.fromkeys(LOCAL_NODE_TYPES, NodeArea.LOCAL),
}
NODE_TYPES_BY_AREA = {
    NodeArea.GLOBAL: GLOBAL_NODE_TYPES,
    NodeArea.REGIONAL: REGIONAL_NODE_TYPES,
    NodeArea.LOCAL: LOCAL_NODE_TYPES,
}

ROOT_NODE_TYPES = bittuple(NodeType.BENCH, NodeType.USER, NodeType.ORGANIZATION)
RESOURCE_NODE_TYPES = _get_node_types(2000, 2200)
VIRTUAL_RESOURCE_NODE_TYPES = _get_node_types(2000, 2100)
PHYSICAL_RESOURCE_NODE_TYPES = _get_node_types(2100, 2200)
SOURCE_NODE_TYPES = _get_node_types(3000, 3100)
STATE_NODE_TYPES = _get_node_types(3500, 3600)
RUNTIME_NODE_TYPES = _get_node_types(3600, 3700)
BASED_NODE_TYPES = bittuple(  # :HasBase
    NodeType.FIELD, NodeType.RUN, NodeType.MESSAGE, NodeType.RECORD
)
PACKAGE_NODE_TYPES = _get_node_types(3001, 3500, NodeType.SKIP)
BENCH_NODE_TYPES = _get_node_types(
    2000,
    10000,
    NodeType.BENCH,
    NodeType.PACKAGE,
    NodeType.HANDLE,
    NodeType.MEMBERSHIP,
    NodeType.INVITE,
    NodeType.CLIENT,
)
PUBLIC_NODE_TYPES = bittuple(NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH)
USER_NODE_TYPES = bittuple(NodeType.USER, NodeType.ORGANIZATION, NodeType.CLIENT, NodeType.HANDLE)

#
# Struct metatypes
# NOTE: enum/struct id 'regions' should be roughly in sync with each other
#


@enum_(EnumType.STRUCT_TYPE)
class StructType(IdEnum):
    # intrinsic (10000-10499)
    RUNTIME_CONTEXT = 10001
    EDIT_CONTEXT = 10002
    EDIT = 10003
    EDIT_INFO = 10004
    EDIT_OPERATION = 10005
    CHANGE = 10006
    CHANGE_VIGNETTE = 10007
    GRAPH_SCOPE = 10008
    CLIENT_ORIGIN = 10009
    NODE_REFERENCE = 10010
    PROPERTY_REFERENCE = 10011

    # access (10500-10999)
    POLICY = 10500
    POLICY_RULE = 10501
    SUBJECT = 10502
    ACCESS_ZONE = 10503
    ACCESS_MATRIX = 10504
    ACCESS = 10505

    # bench (11000-11499)
    MACHINE_IMAGE = 11000

    # type (11500-11999)
    TYPE_INFO = 11500
    TYPE_CONSTRAINT = 11501
    VARIABLE_OBJECT = 11502
    MEMBER_OBJECT = 11503
    INPUT_OBJECT = 11504
    OUTPUT_OBJECT = 11505
    SCHEDULE = 11506
    FILE_INFO = 11507
    ICON = 11509

    # text (12000-12099)
    TEXT = 12000
    TEXT_LINE = 12001
    TEXT_SPAN = 12002

    # code (12100-12199)
    CODE = 12100
    CODE_LINE = 12101

    # expression (12200-12699)
    PATH = 12200
    PATH_TOKEN = 12201
    EXPRESSION = 12202
    AGGREGATION_RESULT = 12203
    SELECTION = 12204
    SELECT_OPTIONS = 12205
    VALUE = 12206
    OBJECT_MAPPING = 12208
    FIELD_MAPPING = 12209

    # run (12700-13199)
    RUN_ERROR = 12700
    RUN_OPTIONS = 12701
    RUN_ATTEMPT = 12702
    RUN_TRACE = 12703
    RUN_FRAME = 12704
    RUN_SPAN = 12705
    RUN_EVENT = 12706
    TEXT_OPTIONS = 12707
    AUDIO_OPTIONS = 12708
    IMAGE_OPTIONS = 12709
    VIDEO_OPTIONS = 12710
    CONTINUE = 12711
    CALL = 12712
    CONTEXT = 12713
    BREAKPOINT = 12714
    LOG_INFO = 12715

    # space/views (13200-13699)
    COLOR = 13200
    FONT = 13201
    RECTANGLE = 13202
    OFFSET = 13203
    TRANSFORM = 13204
    VECTOR2 = 13205
    VECTOR3 = 13206
    VECTOR4 = 13207
    LINE = 13208
    RECTANGLE_CONSTRAINT = 13209

    # browser/application (13700-13999)
    DOM_NODE = 13700


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
    TEXT = 2  # define a 'paragraph' of text
    # ALIAS   # refer to / 'redefine' an existing block or builtin (like a 'newtype')

    # types
    # CLASS?
    CHOICE = 11  # define a choice type with fields (union of literal options or oneof fields)
    MESSAGE = 12  # define a signal type with fields
    # SECRET, RESOURCE, PROTOCOL, TAG, ISSUE, METRIC, BLOCK, ...?

    # runnable
    FLOW = 22  # define a flow of actions (actions connected with pipes)

    # data
    VARIABLE = 30  # define a single-value variable
    DATABASE = 31  # define a database with records & queries
    QUERY = 32  # define a set of queries

    # view
    VIEW = 40  # define a view (with fields and nested views)

    # auth
    ROLE = 50  # define a role with policies
    IDENTITY = 51  # define an identity with roles & policies

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
    RUNNABLE = bittuple(*tuple(t for t in BLOCK_TYPES if 20 <= t.id < 30))
    CLASSES = bittuple(BlockType.MESSAGE, *RUNNABLE, BlockType.VARIABLE, BlockType.DATABASE)


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


@enum_(EnumType.QUERY_TYPE)
class QueryType(IdEnum):
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
    DELETE = 26
    RESTORE = 27
    ERASE = 28

    @property
    def kind(self) -> "AccessKind":
        return AccessKind.EDIT


@enum_(EnumType.CHANGE_CATEGORY)
class ChangeCategory(IdEnum):
    """Optional classification for edits."""

    SPACE = 10
    RUNTIME = 20


@enum_(EnumType.EDIT_OPERATION_TYPE)
class EditOperationType(IdEnum):
    """The type of edit operation."""

    # basic (idempotent)
    SET = 1
    CLEAR = 2

    # list
    # APPEND, REMOVE, ...?

    # math
    # ADD, SUBTRACT, ...?

    # text
    # ...?


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
    AccessType = QueryType | EditType | UseType
else:
    AccessType = IdEnum.combine("AccessType", QueryType, EditType, UseType)
    AccessType.kind = property(lambda self: ACCESS_KIND_BY_ACCESS[self])
    enum_(EnumType.ACCESS_TYPE)(AccessType)

READ_TYPES: bittuple[QueryType] = bittuple(*QueryType)
EDIT_TYPES: bittuple[EditType] = bittuple(*EditType)
USE_TYPES: bittuple[UseType] = bittuple(*UseType)
ACCESS_TYPES: bittuple[AccessType] = bittuple(*AccessType)  # type: ignore
ACCESS_CLASSES: tuple[type[AccessType], ...] = (QueryType, EditType, UseType, AccessType)  # type: ignore
ACCESS_KINDS = bittuple(*AccessKind)
ACCESS_TYPES_BY_KIND: dict[AccessKind, bittuple[AccessType]] = {
    AccessKind.READ: bittuple(*READ_TYPES),
    AccessKind.EDIT: bittuple(*EDIT_TYPES),
    AccessKind.USE: bittuple(*USE_TYPES),
}
ACCESS_CLASS_BY_KIND: dict[AccessKind, type[AccessType]] = {
    AccessKind.READ: QueryType,
    AccessKind.EDIT: EditType,
    AccessKind.USE: UseType,
}
ACCESS_KIND_BY_ACCESS: dict[AccessType, AccessKind] = {
    access: kind for kind, access_types in ACCESS_TYPES_BY_KIND.items() for access in access_types
}
CASCADING_EDIT_TYPES: bittuple[EditType] = bittuple(
    EditType.DELETE, EditType.RESTORE, EditType.ERASE
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
    # time
    DATETIME = 30
    DATE = 31
    TIME = 32
    DURATION = 33

    @property
    def is_numeric(self) -> bool:
        return self.id >= 2 and self.id < 20

    @property
    def is_int(self) -> bool:
        return self.id >= 2 and self.id <= 10

    @property
    def is_float(self) -> bool:
        return self.id >= 15 and self.id < 20


PrimitiveValue = bool | int | float | str | bytes | UUID | datetime | date | time | timedelta

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
    PrimitiveType.DATE: date,
    PrimitiveType.TIME: time,
    PrimitiveType.DURATION: timedelta,
}
PRIMITIVE_TYPE_BY_PY_TYPE: dict[type, PrimitiveType] = {
    bool: PrimitiveType.BOOLEAN,
    int: PrimitiveType.INT32,
    float: PrimitiveType.FLOAT32,
    str: PrimitiveType.STRING,
    bytes: PrimitiveType.BYTES,
    datetime: PrimitiveType.DATETIME,
    date: PrimitiveType.DATE,
    time: PrimitiveType.TIME,
    timedelta: PrimitiveType.DURATION,
    UUID: PrimitiveType.UUID,
}


@enum_(EnumType.TYPE_FORMAT)
class TypeFormat(IdEnum):  # :TypeFormat
    """The fine-grained format of some Type."""

    # strings
    URL = 2000
    EMAIL = 2001
    EMOJI = 2002
    PHONE_NUMBER = 2003
    SLUG = 2004

    @property
    def primitive_type(self) -> PrimitiveType:
        return PrimitiveType(self // 100)


@enum_(EnumType.TYPE_KIND)
class TypeKind(IdEnum):
    """The 'kind' of a Type."""

    PRIMITIVE = 1
    STRUCT = 2
    NODE = 3
    ENUM = 4
    BASED_NODE = 5
    CUSTOM_OBJECT = 6
    PARTIAL_OBJECT = 7
    LITERAL = 8
    UNION = 9


@enum_(EnumType.PARTIAL_OBJECT_SCOPE)
class PartialObjectScope(IdEnum):
    """The scope of properties in a partial Node object."""

    FULL = 1
    BASE = 2
    SUBTYPE = 3


@enum_(EnumType.FIELD_ZONE)
class FieldType(IdEnum):
    """The type of a Field within its Block. Overlaps with ObjectKind."""

    VARIABLE = 1
    MEMBER = 2
    INPUT = 3
    OUTPUT = 4
    OPTION = 5


@enum_(EnumType.OBJECT_KIND)
class ObjectKind(IdEnum):
    """The type of an Object. Overlaps with FieldType."""

    VARIABLE = 1
    MEMBER = 2
    INPUT = 3
    OUTPUT = 4
    BUILTIN = 10


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


@enum_(EnumType.NODE_MODE)
class NodeMode(IdEnum):
    BUILTIN = 1
    PRODUCTION = 2
    DEVELOPMENT = 4
    TEST = 6
    PREVIEW = 8
    ARCHIVE = 10


@enum_(EnumType.RUN_TYPE)
class RunType(IdEnum):
    CODE = 1
    ACTION = 10
    FLOW = 11
    PIPE = 12


@enum_(EnumType.RUN_ERROR_KIND)
class RunErrorKind(IdEnum):
    INTERNAL = 1
    RUNTIME = 5


@enum_(EnumType.RUN_STATUS)
class RunStatus(IdEnum):
    SCHEDULED = 1
    QUEUED = 2
    # active
    PREPARING = 3
    RUNNING = 4
    # interrupted
    PAUSED = 5
    YIELDED = 6
    WAITING = 7
    # terminal
    CANCELLED = 8
    ABORTED = 9
    FAILED = 10
    COMPLETED = 11

    @property
    def is_active(self) -> bool:
        return self in ACTIVE_RUN_STATUSES

    @property
    def is_interrupted(self) -> bool:
        return self in INTERRUPTED_RUN_STATUSES

    @property
    def is_terminal(self) -> bool:
        return self in TERMINAL_RUN_STATUSES


INTERRUPTED_RUN_STATUSES = bittuple(RunStatus.PAUSED, RunStatus.YIELDED)
ACTIVE_RUN_STATUSES = bittuple(RunStatus.RUNNING, *INTERRUPTED_RUN_STATUSES)
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


@enum_(EnumType.LITERAL_TYPE)
class LiteralType(IdEnum):
    VALUE = 100  # any freeform value
    NONE = 101
    TRUE = 102
    FALSE = 103

    @property
    def kind(self) -> "ExpressionKind":
        return ExpressionKind.LITERAL


@enum_(EnumType.FUNCTIONAL_TYPE)
class FunctionalType(IdEnum):
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


@enum_(EnumType.CONDITIONAL_TYPE)
class ConditionalType(IdEnum):
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
    MATCHES = 320
    STARTS_WITH = 321
    ENDS_WITH = 322
    MATCHES_REGEX = 323
    # collections
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


@enum_(EnumType.AGGREGATION_TYPE)
class AggregationType(IdEnum):
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


@enum_(EnumType.SORT_TYPE)
class SortType(IdEnum):
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


EXPRESSION_OPS_BY_KIND: Mapping[ExpressionKind, bittuple["ExpressionType"]] = {  # type: ignore
    ExpressionKind.LITERAL: bittuple(*LiteralType),
    ExpressionKind.FUNCTIONAL: bittuple(*FunctionalType),
    ExpressionKind.CONDITIONAL: bittuple(*ConditionalType),
    ExpressionKind.AGGREGATION: bittuple(*AggregationType),
    ExpressionKind.SORT: bittuple(*SortType),
}
EXPRESSION_KIND_BY_OP: Mapping["ExpressionType", ExpressionKind] = {  # type: ignore
    op: kind
    for kind, ops in EXPRESSION_OPS_BY_KIND.items()  # type: ignore
    for op in ops
}

if typing.TYPE_CHECKING:
    ExpressionType = LiteralType | FunctionalType | ConditionalType | AggregationType | SortType
else:
    ExpressionType = IdEnum.combine(
        "ExpressionType", LiteralType, FunctionalType, ConditionalType, AggregationType, SortType
    )
    ExpressionType.kind = property(lambda self: EXPRESSION_KIND_BY_OP[self])
    enum_(EnumType.EXPRESSION_OP)(ExpressionType)


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
    ACTIVATED = 10  # has bench, all ready to go


@enum_(EnumType.ORGANIZATION_STATUS)
class OrganizationStatus(IdEnum):
    # NOTE UserStatus/OrganizationStatus ids for same statuses should match
    REGISTERED = 4  # created org
    ACTIVATED = 10  # has main bench


#
# Other global stuff
#

CLOUD = get_from_env("CLOUD", typ=Cloud, description="Cloud we're running in")
REGION = get_from_env("REGION", typ=Region, description="Region we're running in")
REGION_ZONE = REGION.zone
REGION_AREA = REGION.area
TRACING = get_from_env("TRACING", typ=bool, description="Enable tracing")


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

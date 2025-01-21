import contextvars
import enum
import secrets
import typing
from datetime import date, datetime, time, timedelta
from decimal import Decimal
from typing import TYPE_CHECKING, Any, Generator, Mapping, Optional, TypeGuard, cast
from uuid import UUID, uuid4, uuid5

from opentelemetry.trace import Tracer
from opentelemetry.util._decorator import _agnosticcontextmanager

from bench.utils.func import IdEnum, bittuple
from bench.utils.utils import frozendict, get_from_env

if TYPE_CHECKING:
    from bench.language import LogLevel, Node, Session, Text, Transaction
    from bench.language.runtime.run import RunSpan, RunSpanType
    from bench.runtime.core import Runner


class _Unset:
    def __repr__(self):
        return "<UNSET!>"

    def __str__(self):
        return "<UNSET!>"


# forever constants
BENCH_SLUG = "bench"
SYSTEM_SLUG = "system"
UUID_NAMESPACE = uuid5(UUID(int=0), b"bench")
VERSION = "2025.01.21.2"
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
    #
    # Global (20000-21000)
    #

    # core (20000-20099)
    ENUM_TYPE = 20001  # so meta
    NODE_TYPE = 20002
    STRUCT_TYPE = 20003
    OBJECT_TYPE = 20004  # NodeType | StructType
    BENCH_TYPE = 20005
    PROPERTY_REFERENCE_TYPE = 20010
    NODE_MODE = 20020

    # time (20100-20199)
    DAY = 20100
    MONTH = 20101
    TIME_INTERVAL = 20102

    # bench (20200-20299)
    PACKAGE_TYPE = 20200
    CLOUD = 20210
    REGION = 20220
    REGION_ZONE = 20221
    REGION_AREA = 20222
    REGION_CONTINENT = 20223

    # auth (20300-20399)
    ACCESS_MODE = 20300
    ACCESS_KIND = 20301
    POLICY_EFFECT = 20302

    # access types (20400-20499)
    QUERY_TYPE = 20400
    EDIT_TYPE = 20401
    USE_TYPE = 20402
    ACCESS_TYPE = 20403  # ReadType | EditType | UseType
    CHANGE_CATEGORY = 20404
    EDIT_OPERATION_TYPE = 20405

    #
    # Regional (21000-22000)
    #

    # resources (21000-21199)
    RESOURCE_STATUS = 21000
    RESOURCE_OCCUPANCY = 21001
    SCALER_TYPE = 21010
    SCALER_STRATEGY = 21011
    MACHINE_TYPE = 21020
    BROWSER_TYPE = 21030
    STORE_TYPE = 21040
    CLIENT_TYPE = 21050

    # files (21200-21249)
    FILE_RETENTION_MODE = 21200
    FILE_KIND = 21201
    FILE_TYPE = 21202
    FILE_FORMAT = 21203
    ICON_KIND = 21210

    # streams (21250-21299)
    STREAM_TYPE = 21250

    # types (21300-21399)
    PRIMITIVE_TYPE = 21300
    FIELD_ZONE = 21301
    TYPE_KIND = 21302
    TYPE_FORMAT = 21303
    BLOCK_TYPE = 21304

    # text (21400-21499)
    TEXT_LINE_TYPE = 21400

    # expressions (21500-21599)
    EXPRESSION_KIND = 21500
    EXPRESSION_OP = 21501
    LITERAL_TYPE = 21502
    FUNCTIONAL_TYPE = 21503
    CONDITIONAL_TYPE = 21504
    AGGREGATION_TYPE = 21505
    SORT_MODE = 21506
    SORT_TYPE = 21507
    COMPUTED_VALUE_KIND = 21510
    COMPUTED_VALUE_MODE = 21511
    PATH_ELEMENT_TYPE = 21520
    PATH_RUN_SELECTOR = 21521

    #
    # Local (22000-23000)
    #

    # runtime core (22000-22099)
    RUN_STATUS = 22000
    RUN_TYPE = 22001
    RUN_SPAN_TYPE = 22002
    RUN_ERROR_KIND = 22003
    RUN_ERROR_TYPE = 22004
    SESSION_STATUS = 22010
    TRIGGER_TYPE = 22020
    CACHE_MODE = 22030
    SCHEDULE_FREQUENCY = 22040
    CALL_MODE = 22050

    # logging (22100-22199)
    LOG_TYPE = 22100
    LOG_LEVEL = 22101

    # debugging (22200-22299)
    BREAKPOINT_SITE = 22200
    BREAKPOINT_ACTION = 22201
    BREAKPOINT_TARGET = 22202
    INTERRUPTION_TYPE = 22210
    INTERRUPTION_STATUS = 22211

    # models (22300-22399)
    MODEL_DEVELOPER = 22300
    MODEL_TYPE = 22301
    MODEL_FAMILY = 22302
    MODEL_PROVIDER = 22303

    # code (22400-22499)
    CODE_TYPE = 22400
    CODE_LANGUAGE = 22401

    # flow (22500-22599)
    ACTION_TYPE = 22500
    PORT_SIDE = 22501
    PIPE_TYPE = 22502

    # views (22600-22699)
    SPACE_TYPE = 22600
    VIEW_TYPE = 22601
    COLOR_TYPE = 22602
    COLOR_SHADE = 22603
    FONT_TYPE = 22604
    FONT_WEIGHT = 22605
    FONT_SIZE = 22606
    SPACING = 22607
    ANCHOR = 22608
    ORIENTATION = 22609
    ALIGNMENT = 22610
    USER_WIZARD_STAGE = 22611
    TREE_VIEW_PRESET = 22612
    HUB_ASPECT = 22613
    HELP_ASPECT = 22614
    BUTTON_VARIANT = 22615
    PICKER_VARIANT = 22616

    # user (22700-22799)
    USER_STATUS = 22700
    ORGANIZATION_STATUS = 22701

    # messages (22800-22899)
    MESSAGE_TYPE = 22800
    MESSAGE_STATUS = 22801


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
    # Global (1-2000)
    #

    # cosmos
    BENCH = 1
    HANDLE = 2

    # auth
    USER = 100
    ORGANIZATION = 101
    # TEAM?
    MEMBERSHIP = 110
    INVITE = 111
    CLIENT = 120
    # CHALLENGE?

    # finance
    # BALANCE, BUDGET, TRANSFER, INVOICE, ...

    #
    # Regional (2000-4000)
    #

    # resource (static)
    SCALER = 2000
    STORE = 2001  # real database

    # resource (dynamic)
    MACHINE = 2100
    BROWSER = 2110
    # MODEL, APPLICATION, ...
    FILE = 2200
    STREAM = 2210
    SECRET = 2220
    # ACCOUNT, DOMAIN, EMAIL, PHONE, ...

    # synchronization
    # CURSOR, POOL, LOCK, BARRIER, CONDITION, ...?

    #
    # Local (5000-)
    #

    # source (named, versioned, templatable)
    PACKAGE = 5000
    DEPENDENCY = 5002
    BLOCK = 5020
    FIELD = 5022  # (based)
    VIEW = 5030
    ACTION = 5040
    PIPE = 5041
    # BADGE? POLICY? (POLICY_)RULE?
    # TAG?
    SPACE = 5200

    # state
    MESSAGE = 5500  # (based, timed)
    RECORD = 5510  # (based)

    # runtime
    SESSION = 6000  # (timed)
    RUN = 6010  # (based, timed)
    RUN_SPAN = 6011  # (timed)
    INTERRUPTION = 6020  # (timed)
    LOG = 6030  # (timed)

    #
    # Misc
    #

    SKIP = 9000
    EMPTY = 9999

    @property
    def is_global(self) -> bool:
        return self.id < 2000

    @property
    def is_regional(self) -> bool:
        return self.id >= 2000 and self.id < 5000

    @property
    def is_local(self) -> bool:
        return self.id >= 5000

    @property
    def area(self) -> NodeArea:
        return AREA_BY_NODE_TYPE[self]

    @property
    def is_resource(self) -> bool:
        return self.id >= 2000 and self.id < 3000

    @property
    def is_source(self) -> bool:
        return self.id >= 5000 and self.id < 5500

    @property
    def is_state(self) -> bool:
        return self.id >= 5500 and self.id < 6000

    @property
    def is_runtime(self) -> bool:
        return self.id >= 6000 and self.id < 6500


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
    node_types = bittuple(
        *tuple(nt for nt in NODE_TYPES if nt.id >= start and nt.id < end), enum_cls=NodeType
    )
    if extra_node_types:
        node_types = node_types | bittuple(*extra_node_types)
    return node_types


COSMOS_NODE_TYPES = _get_node_types(None, 100)
AUTH_NODE_TYPES = _get_node_types(100, 200)
FINANCE_NODE_TYPES = _get_node_types(200, 300)

GLOBAL_NODE_TYPES = _get_node_types(None, 2000)
REGIONAL_NODE_TYPES = _get_node_types(2000, 5000)
LOCAL_NODE_TYPES = _get_node_types(5000, None)
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
RESOURCE_NODE_TYPES = _get_node_types(2000, 3000)
STATIC_RESOURCE_NODE_TYPES = _get_node_types(2000, 2100)
DYNAMIC_RESOURCE_NODE_TYPES = _get_node_types(2100, 3000)
SOURCE_NODE_TYPES = _get_node_types(5000, 5500)
STATE_NODE_TYPES = _get_node_types(5500, 6000)
RUNTIME_NODE_TYPES = _get_node_types(6000, 6500)
BASED_NODE_TYPES = bittuple(  # :HasBase
    NodeType.FIELD, NodeType.RUN, NodeType.INTERRUPTION, NodeType.MESSAGE, NodeType.RECORD
)
PACKAGE_NODE_TYPES = _get_node_types(5000, 5500, NodeType.SKIP, NodeType.EMPTY)
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
    CONTEXT = 10001
    EDIT_CONTEXT = 10002
    EDIT = 10003
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
    TYPE = 11500
    TYPE_CONSTRAINT = 11501
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
    PATH_ELEMENT = 12201
    EXPRESSION = 12210
    AGGREGATION_RESULT = 12211
    SELECTION = 12220
    SELECT_OPTIONS = 12230
    VALUE = 12240
    COMPUTED_VALUE = 12241

    # run (12700-13199)
    RUN_ERROR = 12700
    RUN_OPTIONS = 12701
    RUN_TRACE = 12703
    RUN_FRAME = 12704
    CALL = 12730
    CALL_PLAN = 12731
    BREAKPOINT = 12740
    # model
    TEXT_OPTIONS = 12800
    AUDIO_OPTIONS = 12801
    IMAGE_OPTIONS = 12802
    VIDEO_OPTIONS = 12803

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


@enum_(EnumType.REGION_CONTINENT)
class RegionContinent(IdEnum):
    """
    'Continents' of Regions.
    """

    EUROPE = 1000
    NORTH_AMERICA = 2000
    SOUTH_AMERICA = 3000
    MIDDLE_EAST = 4000
    AFRICA = 5000
    ASIA = 6000
    AUSTRALIA = 7000
    PRIVATE = 9000

    @property
    def slug(self) -> str:
        return REGION_CONTINENT_SLUGS[self]

    @staticmethod
    def get_by_slug(slug: str) -> "RegionContinent":
        return REGION_CONTINENT_BY_SLUG[slug]


REGION_CONTINENT_SLUGS: dict[RegionContinent, str] = {
    RegionContinent.EUROPE: "eu",
    RegionContinent.NORTH_AMERICA: "na",
    RegionContinent.SOUTH_AMERICA: "sa",
    RegionContinent.MIDDLE_EAST: "me",
    RegionContinent.AFRICA: "af",
    RegionContinent.ASIA: "as",
    RegionContinent.AUSTRALIA: "au",
}
REGION_CONTINENT_BY_SLUG = {v: k for k, v in REGION_CONTINENT_SLUGS.items()}


@enum_(EnumType.REGION_AREA)
class RegionArea(IdEnum):
    """
    A larger RegionArea of Regions within a RegionContinent.
    """

    EUROPE_CENTRAL = 1000
    NORTH_AMERICA_EAST = 2000
    NORTH_AMERICA_WEST = 2200
    SOUTH_AMERICA_EAST = 3000
    MIDDLE_EAST_CENTRAL = 4000
    MIDDLE_EAST_WEST = 4200
    AFRICA_SOUTH = 5000
    ASIA_WEST = 6000
    ASIA_SOUTH = 6200
    ASIA_EAST = 6400
    AUSTRALIA_SOUTH = 7000

    @property
    def continent(self) -> RegionContinent:
        return RegionContinent((self.id // 1000) * 1000)

    @property
    def slug(self) -> str:
        return REGION_AREA_SLUGS[self]

    @staticmethod
    def get_by_slug(slug: str) -> "RegionArea":
        return REGION_AREA_BY_SLUG[slug]


REGION_AREA_SLUGS: dict[RegionArea, str] = {
    RegionArea.EUROPE_CENTRAL: "eu-central",
    RegionArea.NORTH_AMERICA_EAST: "na-east",
    RegionArea.NORTH_AMERICA_WEST: "na-west",
    RegionArea.SOUTH_AMERICA_EAST: "sa-east",
    RegionArea.MIDDLE_EAST_CENTRAL: "me-central",
    RegionArea.MIDDLE_EAST_WEST: "me-west",
    RegionArea.AFRICA_SOUTH: "af-south",
    RegionArea.ASIA_WEST: "as-west",
    RegionArea.ASIA_SOUTH: "as-south",
    RegionArea.ASIA_EAST: "as-east",
    RegionArea.AUSTRALIA_SOUTH: "au-south",
}
REGION_AREA_BY_SLUG = {v: k for k, v in REGION_AREA_SLUGS.items()}


@enum_(EnumType.REGION)
class Region(IdEnum):
    """Regions in a RegionArea, comprising RegionZones."""

    # eu-central
    ZURICH = 1000
    FRANKFURT = 1010

    # na-east
    VIRGINIA = 2000
    OHIO = 2010

    # na-west
    OREGON = 2200

    # sa-east
    SAO_PAULO = 3000

    ...

    # af-south
    CAPE_TOWN = 5000

    # as-east
    MUMBAI = 6000

    # as-south
    SINGAPORE = 6200

    # as-east
    TOKYO = 6400

    # au-south
    SYDNEY = 7000

    @property
    def continent(self) -> RegionContinent:
        return RegionContinent((self.id // 1000) * 1000)

    @property
    def area(self) -> RegionArea:
        return RegionArea((self.id // 200) * 200)

    @property
    def slug(self) -> str:
        continent = self.continent
        return continent.slug + "-" + self.name.replace("_", "-").lower()

    @staticmethod
    def get_by_slug(slug: str) -> "Region":
        return REGION_BY_SLUG[slug]


@enum_(EnumType.REGION_ZONE)
class RegionZone(IdEnum):
    """An available region within a specific Region."""

    ...


REGION_SLUGS: dict[Region, str] = {r: r.slug for r in Region}
REGION_BY_SLUG = {v: k for k, v in REGION_SLUGS.items()}


@enum_(EnumType.BLOCK_TYPE)
class BlockType(IdEnum):
    PAGE = 1, "Page of Blocks"
    TEXT = 2, "Line of rich Text"
    # ALIAS   # refer to / 'redefine' an existing block or builtin (like a 'newtype')

    # types
    # CLASS?
    CHOICE = 11, "Choice of Field options"
    MESSAGE = 12, "Message type to communicate"
    # SECRET, RESOURCE, PROTOCOL, TAG, ISSUE, METRIC, BLOCK, ...?

    # runnable
    # LIBRARY?
    FLOW = 22, "Flow of connected Actions"

    # data
    VARIABLE = 30, "Single-value Variable"
    DATABASE = 31, "Database of Records"

    # view
    VIEW = 40, "User interface"

    # auth
    ROLE = 50, "Role to assign"
    IDENTITY = 51, "Unique Identity"

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


@enum_(EnumType.LOG_TYPE)
class LogType(IdEnum):
    # code
    PRINT = 100
    # access
    CHANGE = 200
    EDIT = 201


@enum_(EnumType.LOG_LEVEL)
class LogLevel(IdEnum):  # :LogLevel
    TRACE = 1
    DEBUG = 2
    INFO = 3
    WARNING = 4
    ERROR = 5
    PANIC = 6


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
    PrimitiveType.DECIMAL: Decimal,
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
    Decimal: PrimitiveType.DECIMAL,
    float: PrimitiveType.FLOAT32,
    str: PrimitiveType.STRING,
    UUID: PrimitiveType.UUID,
    dict: PrimitiveType.JSON,
    bytes: PrimitiveType.BYTES,
    datetime: PrimitiveType.DATETIME,
    date: PrimitiveType.DATE,
    time: PrimitiveType.TIME,
    timedelta: PrimitiveType.DURATION,
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


@enum_(EnumType.FIELD_ZONE)
class FieldType(IdEnum):
    """The type of a Field within its Block. Overlaps with ObjectKind."""

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
    """The day of the week."""

    MONDAY = 1
    TUESDAY = 2
    WEDNESDAY = 3
    THURSDAY = 4
    FRIDAY = 5
    SATURDAY = 6
    SUNDAY = 7


@enum_(EnumType.MONTH)
class Month(IdEnum):
    """The month of the year."""

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


@enum_(EnumType.RUN_SPAN_TYPE)
class RunSpanType(IdEnum):
    # general
    ATTEMPT = 1
    DELEGATE = 10
    WAIT = 11
    # flow
    FLOW_GENERATE_CALLS = 100
    # action
    # ...
    # application (action)
    # ...
    # model
    MODEL_PREPARE = 300
    MODEL_GENERATE = 310
    MODEL_PARSE = 320
    # file
    FILE_UPLOAD = 500
    FILE_PREPARE_UPLOAD = 501
    FILE_DOWNLOAD = 502
    FILE_PREPARE_DOWNLOAD = 503


@enum_(EnumType.RUN_ERROR_KIND)
class RunErrorKind(IdEnum):
    INTERNAL = 1
    RUNTIME = 5


@enum_(EnumType.RUN_STATUS)
class RunStatus(IdEnum):
    SCHEDULED = 1
    QUEUED = 2
    # active
    RUNNING = 10
    PREPARING = 11
    # interrupted
    PAUSED = 20
    YIELDED = 21
    WAITING = 22
    # terminal
    CANCELLED = 30
    ABORTED = 31
    FAILED = 32
    COMPLETED = 33

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
    EXISTENCE = 400
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
    WEB = 1
    BROWSER_PLUGIN = 2
    DESKTOP = 3
    MOBILE = 4

    # server
    MACHINE = 10


#
# Other global stuff
#

CLOUD = get_from_env("CLOUD", typ=Cloud, description="Cloud we're running in")
REGION = get_from_env("REGION", typ=Region, description="Region we're running in")
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


@_agnosticcontextmanager
def run_span(
    tracer: Tracer,
    key: str,
    type: "RunSpanType",
    *,
    level: "LogLevel | None" = None,
    nodes: list["Node"] | None = None,
    title: str | None = None,
    text: "Text | None" = None,
    runner: "Runner[Any] | None" = None,
) -> Generator["RunSpan | None", None, None]:
    """Decorate or annotate a RunSpan in the current Run (noop if not inside a Run)."""
    from bench.language import LogLevel, RunSpan

    if runner is None:
        session = _active_session.get()
        runtime = session._runtime if session is not None else None
        runner = runtime.active_runner if runtime is not None else None
        run = runner.closest_tracked_run if runner is not None else None
    else:
        runtime = runner.runtime
        session = runner.session
        run = runner.closest_tracked_run

    if runner is None or runtime is None or run is None:
        # not inside a Run
        with tracer.start_as_current_span(key):
            yield
    else:
        span = RunSpan(
            type=type,
            level=level or LogLevel.INFO,
            nodes=nodes or [],
            title=title,
            text=text,
            started_at=runtime.oracle.utc(),
            _skip_validate_self=True,
        )
        run.spans.append(span)
        try:
            with tracer.start_as_current_span(key):
                yield span
        finally:
            span._do_set("terminated_at", runtime.oracle.utc(), validate=False)
            span._do_set("duration", span.terminated_at - span.started_at, validate=False)  # type: ignore

import contextvars
import enum
import functools
import secrets
import typing
from datetime import date, datetime, time, timedelta
from decimal import Decimal
from typing import (
    TYPE_CHECKING,
    Any,
    Collection,
    Generator,
    Iterable,
    Mapping,
    Optional,
    TypeGuard,
    TypeVar,
    Union,
    cast,
)
from uuid import UUID, uuid4, uuid5

from bitarray import bitarray
from more_itertools import first
from opentelemetry.trace import Tracer
from opentelemetry.util._decorator import _agnosticcontextmanager

from bench.utils.env import IS_TEST
from bench.utils.utils import frozendict, get_from_env

if TYPE_CHECKING:
    from bench.language import (
        Node,
        Session,
        Severity,
        Span,
        SpanType,
        Text,
        Transaction,
    )
    from bench.runtime import Runner


class _Unset:
    def __repr__(self):
        return "<UNSET!>"

    def __str__(self):
        return "<UNSET!>"


# forever constants
VERSION = "2025.04.14.1"
UUID_NAMESPACE = uuid5(UUID(int=0), b"bench")
CK_LENGTH_B64 = 24  # 1.5 * CK_LENGTH_BYTES (must be integer)
FLOAT_EPSILON = 1e-6

# builtin benches :Builtins
BENCH_SLUG = "bench"
BENCH_ID = UUID("11111111-1111-1111-1111-000000000000")
BENCH_BENCH_PACKAGE_SLUG = "bench"
BENCH_BENCH_PACKAGE_ID = UUID("11111111-1111-1111-1111-000000000001")
SYSTEM_SLUG = "system"
SYSTEM_ID = UUID("22222222-2222-2222-2222-000000000000")
SYSTEM_SYSTEM_PACKAGE_SLUG = "system"
SYSTEM_SYSTEM_PACKAGE_ID = UUID("22222222-2222-2222-2222-000000000001")

# runtime constants
NONCE = uuid4()
UNSET = cast(Any, _Unset())
EMPTY_LIST: list = []
EMPTY_SET: frozenset = frozenset()
EMPTY_DICT: dict[Any, Any] = frozendict()

if not IS_TEST:
    DEFAULT_WAIT_TIMEOUT = timedelta(seconds=30)
    DEFAULT_RESOURCE_TIMEOUT = timedelta(seconds=30)
else:
    DEFAULT_WAIT_TIMEOUT = timedelta(seconds=5)
    DEFAULT_RESOURCE_TIMEOUT = timedelta(seconds=5)


def new_struct_id() -> int:
    id = secrets.randbits(31)
    if id < 0:
        id = -id
    return id


#
# Builtin Enums
#

_MIN_ID_BY_ENUM: dict[type, int] = {}
_MAX_ID_BY_ENUM: dict[type, int] = {}

BuiltinEnumT = TypeVar("BuiltinEnumT", bound="BuiltinEnum")


class BuiltinEnum(enum.IntEnum):
    ord: int
    id: int
    title: str | None
    text: str | None
    icon: str | None
    color: "ColorType | None"

    def __new__(
        cls,
        id: int,
        title: str | None = None,
        text: str | None = None,
        icon: str | None = None,
        color: "ColorType | None" = None,
    ):
        obj = int.__new__(cls, id)
        obj._value_ = id
        obj.ord = len(cls)
        obj.id = id
        obj.text = text
        obj.title = title
        obj.icon = icon
        obj.color = color
        obj.__doc__ = text

        # check id
        assert id > 0, f"invalid id {id}"
        existing = first((v for v in cls if v.id == id), None)
        assert existing is None, f"{cls} has duplicate id {id} for {id} and {existing}"

        return obj

    @functools.cached_property
    def bench_name(self):
        from bench.utils.string import Casing, to_casing

        return to_casing(self.name, Casing.CAMEL)

    @classmethod
    def get_min_id(cls) -> int:
        """Get the minimum id."""
        if cls not in _MIN_ID_BY_ENUM:
            _MIN_ID_BY_ENUM[cls] = min(v.id for v in cls)
        return _MIN_ID_BY_ENUM[cls]

    @classmethod
    def get_max_id(cls) -> int:
        """Get the maximum id."""
        if cls not in _MAX_ID_BY_ENUM:
            _MAX_ID_BY_ENUM[cls] = max(v.id for v in cls)
        return _MAX_ID_BY_ENUM[cls]

    @classmethod
    def get_min_ord(cls) -> int:
        """Get the minimum ord."""
        return 0

    @classmethod
    def get_max_ord(cls) -> int:
        """Get the maximum ord."""
        return len(cls)

    def to(self, combined_type: Union["BuiltinEnumT", "BuiltinEnumOrUnion"]) -> "BuiltinEnumT":
        return combined_type(self.id)  # type: ignore

    @staticmethod
    def combine(name: str, *enums: type["BuiltinEnum"]) -> type["BuiltinEnum"]:
        combined_ids = {}
        for e in enums:
            for t in e:
                if t.name in combined_ids:
                    raise ValueError(f"duplicate enum name: {t.name} from {enums}")
                combined_ids[t.name] = t.id
        combined = BuiltinEnum(name, combined_ids)
        return typing.cast(type["BuiltinEnum"], combined)


BuiltinEnumOrUnion = Union[BuiltinEnum, Union[BuiltinEnum, Any]]
# NOTE: BuiltinEnumOrOnion is intended for stuff like AccessType = BuiltinEnum.combine("AccessType", ReadType, ...)
#  But for type checking we have it as AccessType = ReadType | ...
#  So we make these methods accept 'Any' for compliance. Not great but it's a small footprint.
EnumT = TypeVar("EnumT", bound=BuiltinEnum)
_ENUM_MEMBERS_BY_ORD: dict[type[BuiltinEnum], list[BuiltinEnum]] = {}


def _get_enum_members_by_ord(enum_cls: type[BuiltinEnum]) -> list[BuiltinEnum]:
    if enum_cls not in _ENUM_MEMBERS_BY_ORD:
        _ENUM_MEMBERS_BY_ORD[enum_cls] = list(enum_cls.__members__.values())
    return _ENUM_MEMBERS_BY_ORD[enum_cls]


# noinspection PyPep8Naming

# NOTE: we have the enum registry here to avoid circular imports
_ENUM_CLASS_BY_TYPE: dict["EnumType", type[BuiltinEnum]] = {}

BuiltinEnumT = typing.TypeVar("BuiltinEnumT", bound=BuiltinEnum)


def enum_(enum_type: "EnumType"):
    """Register a Bench enum."""

    def register_enum(cls: type[BuiltinEnumT]) -> type[BuiltinEnumT]:
        if enum_type in _ENUM_CLASS_BY_TYPE:
            raise ValueError(f"enum {enum_type} duplicate: {_ENUM_CLASS_BY_TYPE[enum_type]}")
        _ENUM_CLASS_BY_TYPE[enum_type] = cls
        return cls

    return register_enum


class bittuple(typing.Generic[EnumT], Collection[EnumT]):  # noqa: N801
    """
    Tuple with a bitarray for fast membership check.
    We accept only BuiltinEnum instances because we use its ordinals for a compact bitarray.
    """

    def __init__(self, *items: EnumT, enum_cls: type[EnumT] | Union[EnumT, Any] | None = None):
        if len(items) == 1 and isinstance(items[0], Collection):
            items = items[0] if type(items[0]) is tuple else tuple(items[0])
        self.tuple = items
        if enum_cls is None:
            assert len(items) > 0, "enum_cls or args is required"
            enum_cls = items[0].__class__
        assert isinstance(enum_cls, type) and issubclass(
            enum_cls, BuiltinEnum
        ), f"invalid bittuple {enum_cls}: {items}"
        self.enum_cls = enum_cls
        self.bits = bitarray(enum_cls.get_max_ord() + 1)
        for arg in items:
            self.bits[arg.ord] = True

    def __bool__(self):
        return bool(self.tuple)

    def has(self, item: EnumT) -> bool:
        """Checks whether the item is of the correct type and is in the tuple."""
        assert isinstance(item, self.enum_cls), f"want {self.enum_cls}, got {item!r} ({type(item)})"
        return bool(self.bits[item.ord])

    def __contains__(self, item: Any) -> bool:
        if type(item) is int:
            item = self.enum_cls(item)
        assert type(item) is self.enum_cls, f"want {self.enum_cls}, got {item!r} ({type(item)})"
        return bool(self.bits[item.ord])

    def __and__(self, other: "bittuple[EnumT]") -> "bittuple[EnumT]":
        assert type(other) is bittuple, f"invalid type: {type(other)}"
        assert (
            self.enum_cls == other.enum_cls
        ), f"invalid enum_cls: {self.enum_cls} != {other.enum_cls}"
        combined = self.bits & other.bits
        ordered_members = _get_enum_members_by_ord(self.enum_cls)
        items = tuple(ordered_members[o] for o in combined.search(True))
        return bittuple(items, enum_cls=self.enum_cls)  # type: ignore

    def __or__(self, other: "bittuple[EnumT]") -> "bittuple[EnumT]":
        assert type(other) is bittuple, f"invalid type: {type(other)}"
        assert (
            self.enum_cls == other.enum_cls
        ), f"invalid enum_cls: {self.enum_cls} != {other.enum_cls}"
        combined = self.bits | other.bits
        ordered_members = _get_enum_members_by_ord(self.enum_cls)
        items = tuple(ordered_members[o] for o in combined.search(True))
        return bittuple(items, enum_cls=self.enum_cls)  # type: ignore

    def __sub__(self, other: "bittuple[EnumT]") -> "bittuple[EnumT]":
        assert type(other) is bittuple, f"invalid type: {type(other)}"
        assert (
            self.enum_cls == other.enum_cls
        ), f"invalid enum_cls: {self.enum_cls} != {other.enum_cls}"
        combined = self.bits & ~other.bits
        ordered_members = _get_enum_members_by_ord(self.enum_cls)
        items = tuple(ordered_members[o] for o in combined.search(True))
        return bittuple(items, enum_cls=self.enum_cls)  # type: ignore

    def __iter__(self):
        return iter(self.tuple)

    def __len__(self):
        return len(self.tuple)

    def __getitem__(self, index):
        return self.tuple[index]

    def __repr__(self):
        return f"{self.__class__.__name__}({self.tuple})"

    def __str__(self):
        return f"{self.__class__.__name__}({self.tuple})"

    @staticmethod
    def from_ord(enum_cls: type[EnumT], ords: bitarray) -> "bittuple[EnumT]":
        ordered_members = _get_enum_members_by_ord(enum_cls)
        items = tuple(ordered_members[o] for o in ords.search(True))
        return bittuple(items, enum_cls=enum_cls)  # type: ignore


#
# Enums
# NOTE: enum/struct id 'regions' should be roughly in sync with each other
#


class EnumType(BuiltinEnum):
    #
    # Global (20000-21000)
    #

    # core (20000-20049)
    ENUM_TYPE = 20000
    NODE_TYPE = 20001
    STRUCT_TYPE = 20002
    OBJECT_TYPE = 20003
    BENCH_TYPE = 20004
    NODE_MODE = 20005
    NODE_AREA = 20006
    PROPERTY_REFERENCE_TYPE = 20007
    EDIT_OPERATION_TYPE = 20008
    CHANGE_CATEGORY = 20009

    # bench (20050-20099)
    PACKAGE_TYPE = 20050
    CLOUD = 20051
    REGION = 20052
    REGION_ZONE = 20053
    REGION_AREA = 20054
    REGION_CONTINENT = 20055
    BENCH_STATUS = 20056

    # auth (20100-20149)
    ACCESS_MODE = 20100
    ACCESS_KIND = 20101
    POLICY_EFFECT = 20102

    # access types (20150-20199)
    QUERY_TYPE = 20150
    EDIT_TYPE = 20151
    USE_TYPE = 20152
    ACCESS_TYPE = 20153

    #
    # Regional (21000-22000)
    #

    # resources (21000-21049)
    RESOURCE_STATUS = 21000
    SCALER_TYPE = 21010
    SCALER_STRATEGY = 21011
    COMPUTER_TYPE = 21020
    APPLICATION_TYPE = 21030
    STORE_TYPE = 21040
    CLIENT_TYPE = 21041

    # files (21050-21099)
    FILE_RETENTION_MODE = 21050
    FILE_KIND = 21051
    FILE_TYPE = 21052
    FILE_FORMAT = 21053
    ICON_TYPE = 21054

    # streams (21100-21149)
    STREAM_TYPE = 21100

    # type (21150-21199)
    PRIMITIVE_TYPE = 21150
    FIELD_ZONE = 21151
    TYPE_KIND = 21152
    TYPE_FORMAT = 21153
    BLOCK_TYPE = 21154
    DAY = 21155
    MONTH = 21156
    TIME_INTERVAL = 21157

    # text (21200-21249)
    TEXT_LINE_TYPE = 21200
    TEXT_SPAN_TYPE = 21201

    # expressions (21250-21299)
    EXPRESSION_KIND = 21250
    EXPRESSION_OP = 21251
    LITERAL_TYPE = 21252
    FUNCTIONAL_TYPE = 21253
    CONDITIONAL_TYPE = 21254
    AGGREGATION_TYPE = 21255
    SORT_MODE = 21256
    SORT_TYPE = 21257
    SELECTION_TYPE = 21265

    #
    # Local (22000-23000)
    #

    # runtime core (22000-22100)
    PROCESS_STATUS = 22000
    RUN_TYPE = 22001
    SPAN_TYPE = 22002
    SESSION_STATUS = 22020
    LOG_TYPE = 22060
    SEVERITY = 22061
    TRIGGER_TYPE = 22030
    TRIGGER_EFFECT = 22031
    TRIGGER_STATUS = 22032
    SCHEDULE_FREQUENCY = 22041
    CLAIM_TYPE = 22050
    CLAIM_STATUS = 22051
    CURSOR_TYPE = 22070
    CURSOR_STATUS = 22071

    # error (22100-22149)
    ERROR_KIND = 22100
    ERROR_TYPE = 22101

    # debugging (22200-22249)
    INTERRUPTION_TYPE = 22210
    INTERRUPTION_STATUS = 22211
    INTERRUPTION_RESPONSE = 22212

    # models (22250-22299)
    MODEL_DEVELOPER = 22250
    MODEL_PROVIDER = 22251

    # code (22300-22349)
    CODE_TYPE = 22300

    # flow (22350-22399)
    ACTION_TYPE = 22350
    PORT_SIDE = 22352
    TRANSITION_TYPE = 22353
    FLOW_TYPE = 22360

    # views (22400-22449)
    SPACE_TYPE = 22400
    VIEW_TYPE = 22401
    COLOR_TYPE = 22402
    COLOR_SHADE = 22403
    FONT_TYPE = 22404
    FONT_WEIGHT = 22405
    FONT_SIZE = 22406
    SPACING = 22407
    ANCHOR = 22408
    ORIENTATION = 22409
    ALIGNMENT = 22410
    USER_WIZARD_STAGE = 22411
    BUTTON_VARIANT = 22415
    PICKER_VARIANT = 22416

    # user (22450-22499)
    USER_STATUS = 22450
    ORGANIZATION_STATUS = 22451

    # messaging (22500-22549)
    CHANNEL_STATUS = 22500
    THREAD_STATUS = 22502
    MESSAGE_TYPE = 22503
    MESSAGE_STATUS = 22504
    NOTIFICATION_TYPE = 22506
    NOTIFICATION_STATUS = 22507


enum_(EnumType.ENUM_TYPE)(EnumType)


ENUM_TYPES: bittuple[EnumType] = bittuple(*EnumType)
ENUM_TYPES_SET: frozenset[EnumType] = frozenset(ENUM_TYPES)


@enum_(EnumType.COLOR_TYPE)
class ColorType(BuiltinEnum):
    """Built-in color types a la SwiftUI or Tailwind."""

    # surface
    PRIMARY = 1
    SECONDARY = 2
    ACCENT = 3
    CANVAS = 4
    # semantic
    SUCCESS = 10
    HINT = 11
    WARNING = 12
    DANGER = 13
    # actual
    GRAY = 30
    RED = 31
    ORANGE = 32
    AMBER = 33
    YELLOW = 34
    LIME = 35
    GREEN = 36
    EMERALD = 37
    TEAL = 38
    CYAN = 39
    SKY = 40
    BLUE = 41
    INDIGO = 42
    VIOLET = 43
    PURPLE = 44
    FUCHSIA = 45
    PINK = 46
    ROSE = 47


@enum_(EnumType.NODE_MODE)
class NodeMode(BuiltinEnum):
    BUILTIN = 10, "Builtin", "Provided by Bench", "fas fa-cog", ColorType.YELLOW
    MAIN = 20, "Main", "Active and available", "fas fa-globe", ColorType.GREEN
    TEST = 30, "Test", "Active in test", "fas fa-flask", ColorType.BLUE
    TEMPLATE = 40, "Template", "Template to use", "fas fa-puzzle-piece", ColorType.VIOLET
    ARCHIVE = 50, "Archive", "Inactive and hidden", "fas fa-box-archive", ColorType.GRAY


@enum_(EnumType.NODE_TYPE)
class NodeType(BuiltinEnum):
    #
    # Global (1-2000)
    #

    # cosmos
    BENCH = 1, "Bench", "Universal workbench", "fas fa-layer-group"
    HANDLE = 2, "Handle", "Unique identifier", "fas fa-at"
    USER = 10, "User", "Human user", "fas fa-user"
    ORGANIZATION = 20, "Organization", "Organization", "fas fa-building"
    CLIENT = 50, "Client", "Client device", "fas fa-desktop"
    # CHALLENGE?

    #
    # Regional (2000-8000)
    #

    # compute
    SCALER = 2000, "Scaler", "Autoscale Resources", "fas fa-scale-unbalanced"
    STORE = 2010, "Store", "Store custom data", "fas fa-database"
    # VAULT?
    # CACHE?
    COMPUTER = 2100, "Computer", "Machine for computing", "fas fa-computer-classic"
    APPLICATION = 2110, "Application", "End-user application", "fas fa-globe"
    # SNAPSHOT, NETWORK, ...

    # data
    FILE = 2200, "File", "File", "fas fa-file"
    STREAM = 2210, "Stream", "Stream", "fas fa-stream"
    # SECRET?
    # LINK = 2220
    # REPOSITORY, SCHEMA, CONNECTION/API, ...?

    # finance
    # BALANCE, BUDGET, TRANSFER, GRANT, INVOICE, ...

    # web
    # ACCOUNT, APPLICATION, DOMAIN, EMAIL, ...

    # world?
    # PHONE, ADDRESS, ...

    # model?
    # ...

    # source
    PACKAGE = 5000, "Package", "Isolated sub-Bench", "fas fa-box-open"
    DEPENDENCY = 5010, "Dependency", "Dependency to something", "fas fa-turn-down-right"
    PAGE = 5020, "Page", "Page of Blocks", "far fa-file"
    BLOCK = 5021, "Block", "Rich Block on a Page", "fas fa-cube"
    CHOICE = 5030, "Choice", "Choice between Options", "fas fa-circle-chevron-down"
    CLASS = 5031, "Class", "Class", "fas fa-shapes"
    # UNION?
    FIELD = 5035, "Field", "Field", "fas fa-triangle"
    OPTION = 5036, "Option", "Option", "far fa-square-check"
    TAG = 5040, "Tag", "Tag", "fas fa-tag"
    FLOW = 5050, "Flow", "Link Actions together", "fas fa-diagram-project"
    ACTION = 5051, "Action", "Action", "fas fa-step-forward"
    TRANSITION = 5052, "Transition", "Transition between Nodes", "fas fa-link"
    TRIGGER = 5053, "Trigger", "Trigger to do something", "fas fa-bolt"
    KIT = 5060, "Kit", "Kit of stuff", "fas fa-screwdriver-wrench"
    DATABASE = 5090, "Database", "Database of Records", "fas fa-database"

    # communication
    CHANNEL = 5500, "Channel", "Channel", "fas fa-hashtag"
    THREAD = 5510, "Thread", "Thread", "fas fa-reel"
    MESSAGE = 5520, "Message", "Message", "fas fa-message"
    # POLL?
    # REACTION?
    NOTIFICATION = 5540, "Notification", "Notification", "fas fa-bell"

    # identity
    TEAM = 5600, "Team", "Group of Users or Agents", "fas fa-users"
    MEMBERSHIP = 5610, "Membership", "Membership to something", "fas fa-users"
    INVITE = 5620, "Invite", "Invite to something", "fas fa-user-plus"
    ROLE = 5630, "Role", "Role", "fas fa-user-tag"
    AGENT = 5640, "Agent", "Identity for an AI", "fas fa-robot"
    # CHALLENGE?
    # BADGE? POLICY? RULE?

    # runtime
    SESSION = 6000, "Session", "Session", "fas fa-circle-play"
    RUN = 6010, "Run", "Run", "fas fa-play"
    SPAN = 6011, "Span", "Span", "fas fa-ruler-horizontal"
    INTERRUPTION = 6020, "Interruption", "Interruption", "fas fa-hand"
    LOG = 6030, "Log", "Log", "fas fa-file-alt"
    # BREAKPOINT?

    # orchestration
    PLAN = 6100, "Plan", "Plan with Tasks", "fas fa-list-check"
    TASK = 6110, "Task", "Task", "far fa-square-check"
    CLAIM = 6150, "Claim", "Claim", "fas fa-stamp"
    CURSOR = 6170, "Cursor", "Cursor", "fas fa-mouse"
    # ENTITLEMENT, POOL, LOCK, BARRIER, ...?

    # view
    VIEW = 7000, "View", "View", "fas fa-window-frame"  # :PolyViews
    # ... all the Views once we have :PolcyViews
    SPACE = 7900, "Space", "Space", "fas fa-space-between"

    #
    # Local (8000-10000)
    #

    RECORD = 8000, "Record", "Record in a Database", "fas fa-database"

    #
    # Misc
    #

    SKIP = 9998
    EMPTY = 9999

    @property
    def is_global(self) -> bool:
        return self.id < 2000

    @property
    def is_regional(self) -> bool:
        return self.id >= 2000 and self.id < 8000

    @property
    def is_local(self) -> bool:
        return self.id >= 8000

    @property
    def area(self) -> "NodeArea":
        return AREA_BY_NODE_TYPE[self]

    @property
    def is_cosmos(self) -> bool:
        return self.id < 100

    @property
    def is_auth(self) -> bool:
        return self.id >= 100 and self.id < 200

    @property
    def is_finance(self) -> bool:
        return self.id >= 200 and self.id < 300

    @property
    def is_resource(self) -> bool:
        return self.id >= 2000 and self.id < 3000

    @property
    def is_source(self) -> bool:
        return self.id >= 5000 and self.id < 5500

    @property
    def is_state(self) -> bool:
        return self.id >= 5500 and self.id < 6000


@enum_(EnumType.NODE_AREA)
class NodeArea(BuiltinEnum):
    GLOBAL = 1
    REGIONAL = 2
    LOCAL = 3


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
GLOBAL_NODE_TYPES = _get_node_types(None, 2000)
REGIONAL_NODE_TYPES = _get_node_types(2000, 8000)
LOCAL_NODE_TYPES = _get_node_types(8000, None)
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
SOURCE_NODE_TYPES = _get_node_types(5000, 5500)
COMMUNICATION_NODE_TYPES = _get_node_types(5500, 5600)
RUNTIME_NODE_TYPES = _get_node_types(6000, 6100)
PACKAGE_NODE_TYPES = _get_node_types(5000, 900, NodeType.SKIP, NodeType.EMPTY)
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
# NOTE: these traits should also be in trait.py but we need the constants in property.py
#  (which also depends on trait.py, and we can't have a circular dependency)
BASED_NODE_TYPES = bittuple(NodeType.RECORD, NodeType.MESSAGE, NodeType.RUN)
INLINE_NODE_TYPES = bittuple(
    *RESOURCE_NODE_TYPES,
    NodeType.CHOICE,
    NodeType.CLASS,
    NodeType.DATABASE,
    NodeType.FLOW,
    NodeType.KIT,
    NodeType.PAGE,
    NodeType.ROLE,
    NodeType.VIEW,
    NodeType.TAG,
    NodeType.TASK,
    NodeType.PLAN,
    NodeType.THREAD,
    NodeType.CHANNEL,
    NodeType.TEAM,
    NodeType.AGENT,
)
INSTANTIABLE_NODE_TYPES = bittuple(
    *RESOURCE_NODE_TYPES,
    NodeType.ACTION,
    NodeType.FIELD,
    NodeType.OPTION,
    NodeType.DATABASE,
    NodeType.PLAN,
    NodeType.TASK,
    NodeType.THREAD,
    NodeType.CLAIM,
    NodeType.CHANNEL,
    NodeType.TEAM,
    NodeType.AGENT,
    NodeType.MEMBERSHIP,
)
TEMPLATABLE_NODE_TYPES = bittuple(
    *SOURCE_NODE_TYPES,
    *RESOURCE_NODE_TYPES,
    *INSTANTIABLE_NODE_TYPES,
    NodeType.CHANNEL,
    NodeType.ROLE,
    NodeType.SPACE,
    NodeType.VIEW,
)


# NOTE :Performance: we load too much and too coarsely :NodeOverload :RichGraph
UNLOADED_RESOURCE_NODE_TYPES = bittuple(NodeType.FILE, NodeType.STREAM)
LOADED_PACKAGE_NODE_TYPES = bittuple(
    *SOURCE_NODE_TYPES,
    *(RESOURCE_NODE_TYPES - UNLOADED_RESOURCE_NODE_TYPES),
    NodeType.CHANNEL,
    NodeType.TEAM,
    NodeType.MEMBERSHIP,
    NodeType.ROLE,
    NodeType.AGENT,
    NodeType.PLAN,
    NodeType.TASK,
    NodeType.CLAIM,
    NodeType.SPACE,
    NodeType.VIEW,
)


# automatically included descendants :AutoLoading
AUTOLOAD_DESCENDANT_TYPES: dict[NodeType, tuple[NodeType, ...]] = {
    NodeType.THREAD: (
        NodeType.FILE,
        NodeType.MEMBERSHIP,
        NodeType.CLAIM,
        NodeType.AGENT,
        NodeType.CURSOR,
    ),
}

#
# Struct metatypes
# NOTE: enum/struct id 'regions' should be roughly in sync with each other
#


@enum_(EnumType.STRUCT_TYPE)
class StructType(BuiltinEnum):
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
    POLICY_SUBJECT = 10502
    ACCESS_ZONE = 10503
    ACCESS_MATRIX = 10504
    ACCESS = 10505

    # bench (11000-11499)

    # type (11500-11999)
    TYPE = 11500
    TYPE_CONSTRAINT = 11501
    FILE_INFO = 11520
    ICON = 11530
    SCHEDULE = 11540

    # text (12000-12099)
    TEXT = 12000, None, None, "fas fa-text"
    TEXT_LINE = 12001
    TEXT_SPAN = 12002

    # code (12100-12199)
    CODE = 12100, None, None, "fas fa-code"

    # expression (12200-12699)
    EXPRESSION = 12210
    AGGREGATION_RESULT = 12211
    SELECTION = 12220
    SELECT_OPTIONS = 12230

    # run (12700-13199)
    ERROR = 12700
    RUN_TRACE = 12703
    RUN_FRAME = 12704
    # action

    # space/views (13200-13699)
    COLOR = 13200, None, None, "fas fa-palette"
    FONT = 13201, None, None, "fas fa-font"
    RECTANGLE = 13202, None, None, "fas fa-box"
    OFFSET = 13203, None, None, "fas fa-arrows-alt"
    TRANSFORM = 13204, None, None, "fas fa-transform"
    VECTOR2 = 13205, None, None, "fas fa-vector-square"
    VECTOR3 = 13206, None, None, "fas fa-vector-square"
    VECTOR4 = 13207, None, None, "fas fa-vector-square"
    LINE = 13208, None, None, "fas fa-bezier-curve"
    RECTANGLE_CONSTRAINT = 13209, None, None, "fas fa-box-constraint"


STRUCT_TYPES: bittuple[StructType] = bittuple(*StructType)
STRUCT_TYPES_SET: frozenset[StructType] = frozenset(STRUCT_TYPES)

if typing.TYPE_CHECKING:
    ObjectType = NodeType | StructType
    BenchType = NodeType | StructType | EnumType
else:
    ObjectType = BuiltinEnum.combine("ObjectType", NodeType, StructType)
    enum_(EnumType.OBJECT_TYPE)(ObjectType)
    BenchType = BuiltinEnum.combine("BenchType", NodeType, StructType, EnumType)
    enum_(EnumType.BENCH_TYPE)(BenchType)

OBJECT_TYPES: bittuple[ObjectType] = bittuple(*ObjectType)  # type: ignore
OBJECT_TYPES_SET: frozenset[ObjectType] = frozenset(OBJECT_TYPES)
BENCH_TYPES: bittuple[BenchType] = bittuple(*BenchType)  # type: ignore


def is_node_type(obj: BuiltinEnum | int | Any) -> TypeGuard[NodeType]:
    return isinstance(obj, int) and obj in NODE_TYPES_SET


def is_struct_type(obj: BuiltinEnum | int | Any) -> TypeGuard[StructType]:
    return isinstance(obj, int) and obj in STRUCT_TYPES_SET


def is_object_type(obj: BuiltinEnum | int | Any) -> TypeGuard[ObjectType]:
    return isinstance(obj, int) and obj in OBJECT_TYPES_SET


def is_enum_type(obj: BuiltinEnum | int | Any) -> TypeGuard[EnumType]:
    return isinstance(obj, int) and obj in ENUM_TYPES_SET


@enum_(EnumType.CLOUD)
class Cloud(BuiltinEnum):
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
class RegionContinent(BuiltinEnum):
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
class RegionArea(BuiltinEnum):
    """
    A larger RegionArea of Regions within a RegionContinent.
    """

    EUROPE_CENTRAL = 1000, None, None, "🇪🇺"
    NORTH_AMERICA_EAST = 2000, None, None, "🇺🇸"
    NORTH_AMERICA_WEST = 2200, None, None, "🇺🇸"
    SOUTH_AMERICA_EAST = 3000, None, None, "🇧🇷"
    MIDDLE_EAST_CENTRAL = 4000, None, None, "🇸🇦"
    MIDDLE_EAST_WEST = 4200, None, None, "🇸🇦"
    AFRICA_SOUTH = 5000, None, None, "🇿🇦"
    ASIA_WEST = 6000
    ASIA_SOUTH = 6200
    ASIA_EAST = 6400
    AUSTRALIA_SOUTH = 7000, None, None, "🇦🇺"

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
class Region(BuiltinEnum):
    """Regions in a RegionArea, comprising RegionZones."""

    # eu-central
    ZURICH = 1000, None, None, "🇨🇭"
    FRANKFURT = 1010, None, None, "🇩🇪"

    # na-east
    VIRGINIA = 2000, None, None, "🇺🇸"
    OHIO = 2010, None, None, "🇺🇸"

    # na-west
    OREGON = 2200, None, None, "🇺🇸"

    # sa-east
    SAO_PAULO = 3000, None, None, "🇧🇷"

    ...

    # af-south
    CAPE_TOWN = 5000, None, None, "🇿🇦"

    # as-east
    MUMBAI = 6000, None, None, "🇮🇳"

    # as-south
    SINGAPORE = 6200, None, None, "🇸🇬"

    # as-east
    TOKYO = 6400, None, None, "🇯🇵"

    # au-south
    SYDNEY = 7000, None, None, "🇦🇺"

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
class RegionZone(BuiltinEnum):
    """An available region within a specific Region."""

    ...


REGION_SLUGS: dict[Region, str] = {r: r.slug for r in Region}
REGION_BY_SLUG = {v: k for k, v in REGION_SLUGS.items()}


class ReferenceKind(BuiltinEnum):
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
class QueryType(BuiltinEnum):
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
class EditType(BuiltinEnum):
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


@enum_(EnumType.CHANGE_CATEGORY)
class ChangeCategory(BuiltinEnum):
    """Optional classification for edits."""

    SPACE = 10
    RUNTIME = 20


@enum_(EnumType.EDIT_OPERATION_TYPE)
class EditOperationType(BuiltinEnum):
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
class UseType(BuiltinEnum):
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
class AccessKind(BuiltinEnum):
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
    AccessType = BuiltinEnum.combine("AccessType", QueryType, EditType, UseType)
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


@enum_(EnumType.ACCESS_MODE)
class AccessMode(BuiltinEnum):
    ADAPTIVE = 1
    ATOMIC = 2


@enum_(EnumType.COLOR_SHADE)
class ColorShade(BuiltinEnum):
    """Built-in color shades a la Tailwind."""

    # surface
    ...
    # actual
    S50 = 50
    S100 = 100
    S200 = 200
    S300 = 300
    S400 = 400
    S500 = 500
    S600 = 600
    S700 = 700
    S800 = 800
    S900 = 900
    S950 = 950


@enum_(EnumType.SEVERITY)
class Severity(BuiltinEnum):
    TRACE = 1, None, None, "fas fa-bug", ColorType.GRAY
    DEBUG = 2, None, None, "fas fa-bug", ColorType.GRAY
    INFO = 3, None, None, "fas fa-circle-check", ColorType.GRAY
    WARNING = 4, None, None, "fas fa-circle-exclamation", ColorType.YELLOW
    ERROR = 5, None, None, "fas fa-circle-exclamation", ColorType.RED
    PANIC = 6, None, None, "fas fa-skull", ColorType.RED


@enum_(EnumType.LOG_TYPE)
class LogType(BuiltinEnum):
    # code
    PRINT = 100
    # access
    CHANGE = 200
    EDIT = 201


@enum_(EnumType.POLICY_EFFECT)
class PolicyEffect(BuiltinEnum):
    ALLOW = 1
    DENY = 2
    # YIELD?, METER, LIMIT, ...


@enum_(EnumType.PRIMITIVE_TYPE)
class PrimitiveType(BuiltinEnum):
    """
    Fundamental column / storage types we support (subset of SQL types, used directly in sql/core).
    NOTE: the ids here are used in type identity keys, so any change is breaking.
    """

    BOOLEAN = 1, "Boolean", "Yes or no", "fas fa-toggle-large-on"
    # ...
    # INT8? UINTs?
    # range: -32768 to 32767
    INT16 = 4, "Integer", "Very small integer", "fas fa-tally"
    # range: -2147483648 to 2147483647
    INT32 = 6, "Integer", "Small integer", "fas fa-tally"
    # range: -9223372036854775808 to 9223372036854775807
    INT64 = 8, "Integer", "Integer number", "fas fa-tally"
    # numeric(precision, scale)
    DECIMAL = 10, "Decimal", "Decimal number", "fas fa-tally"
    # ...
    # FLOAT16?
    # range: 1.175494351e-38 to 3.402823466e+38
    FLOAT32 = 16, "Float", "Small float", "fas fa-hashtag"
    # range: 2.2250738585072014e-308 to 1.7976931348623157e+308
    FLOAT64 = 17, "Float", "Floating point number", "fas fa-hashtag"
    # ...
    STRING = 20, "String", "Plain text", "fas fa-font-case"
    UUID = 21, "UUID", "UUID", "fas fa-fingerprint"
    JSON = 22, "JSON", "JSON", "fas fa-brackets-curly"
    BYTES = 25, "Bytes", "Binary data", "fas fa-file-lines"
    VECTOR = 26, "Vector", "Vector", "fas fa-vector-square"
    # time
    DATETIME = 30, "Date & Time", "Date & time", "fas fa-calendar-days"
    DATE = 31, "Date", "Date", "fas fa-calendar-days"
    TIME = 32, "Time", "Time", "fas fa-clock"
    DURATION = 33, "Duration", "Duration", "fas fa-stopwatch"

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
class TypeFormat(BuiltinEnum):  # :TypeFormat
    """The fine-grained format of some Type."""

    # strings
    URL = 2000, "Url", "Web address", "fas fa-link"
    EMAIL = 2001, "Email", "Email address", "fas fa-at"
    EMOJI = 2002, "Emoji", "Emoji", "fas fa-smile"
    PHONE_NUMBER = 2003, "Phone number", "Phone number", "fas fa-phone"
    SLUG = 2004, "Slug", "Slug", "fas fa-at"

    @property
    def primitive_type(self) -> PrimitiveType:
        return PrimitiveType(self // 100)


@enum_(EnumType.TYPE_KIND)
class TypeKind(BuiltinEnum):
    """The 'kind' of a Type."""

    PRIMITIVE = 1
    STRUCT = 2
    NODE = 3
    ENUM = 4
    BASED_NODE = 5
    CUSTOM_OBJECT = 6
    PARTIAL_OBJECT = 7


@enum_(EnumType.FIELD_ZONE)
class FieldType(BuiltinEnum):
    """The type of a Field within its Block. Overlaps with ObjectKind."""

    MEMBER = 10, "Member", "Member of an object", "fas objects-columns"
    INPUT = 20, "Input to a runnable", "fas fa-arrow-down"
    OUTPUT = 30, "Output from a runnable", "fas fa-arrow-up"
    # VARIABLE?


@enum_(EnumType.TIME_INTERVAL)
class TimeInterval(BuiltinEnum):
    SECOND = 2
    MINUTE = 3
    HOUR = 4
    DAY = 5
    WEEK = 6
    MONTH = 7
    YEAR = 8


@enum_(EnumType.DAY)
class Day(BuiltinEnum):
    """The day of the week."""

    MONDAY = 1
    TUESDAY = 2
    WEDNESDAY = 3
    THURSDAY = 4
    FRIDAY = 5
    SATURDAY = 6
    SUNDAY = 7


@enum_(EnumType.MONTH)
class Month(BuiltinEnum):
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


@enum_(EnumType.RUN_TYPE)
class RunType(BuiltinEnum):
    CODE = 1
    ACTION = 10
    FLOW = 11
    TRANSITION = 12
    AGENT = 15


@enum_(EnumType.SPAN_TYPE)
class SpanType(BuiltinEnum):
    # general
    ATTEMPT = 1, None, None, "fas fa-play"
    WAIT = 2, None, None, "fas fa-hourglass-end"
    ACQUIRE = 3, None, None, "fas fa-toolbox"
    CODE = 10, None, None, "fas fa-code"
    # flow
    AGENT_THINK = 100, None, None, "fas fa-hexagon-nodes"
    # action
    # ...
    # application (action)
    # ...
    # model
    MODEL_PREPARE = 300, None, None, "fas fa-hexagon-nodes"
    MODEL_GENERATE = 310, None, None, "fas fa-hexagon-nodes"
    MODEL_PARSE = 320, None, None, "fas fa-hexagon-nodes"
    # file
    FILE_UPLOAD = 500, None, None, "fas fa-upload"
    FILE_PREPARE_UPLOAD = 501, None, None, "fas fa-upload"
    FILE_DOWNLOAD = 502, None, None, "fas fa-download"
    FILE_PREPARE_DOWNLOAD = 503, None, None, "fas fa-download"
    # ...


@enum_(EnumType.ERROR_KIND)
class ErrorKind(BuiltinEnum):
    INTERNAL = 1
    RUNTIME = 5


@enum_(EnumType.PROCESS_STATUS)
class ProcessStatus(BuiltinEnum):
    # pre
    CREATED = 1, "Created", "Created but not yet assigned", "fas fa-circle", ColorType.GRAY
    ASSIGNED = 2, "Assigned", "Assigned to someone", "fas fa-clock", ColorType.GRAY
    SCHEDULED = 4, "Scheduled", "Scheduled for sometime", "fas fa-clock", ColorType.GRAY
    QUEUED = 5, "Queued", "Queued to happen soon", "fas fa-clock", ColorType.GRAY
    # active
    RUNNING = 10, "Running", "Actively running", "fas fa-circle-notch", ColorType.BLUE
    FAILING = 11, "Failing", "Experiencing issues", "fas fa-circle-exclamation", ColorType.ORANGE
    # interrupted
    PAUSED = 20, "Paused", "Paused manually", "fas fa-circle-pause", ColorType.PINK
    YIELDED = 21, "Yielded", "Yielded to someone", "fas fa-circle-pause", ColorType.PINK
    WAITING = 22, "Waiting", "Waiting for a condition", "fas fa-circle-pause", ColorType.PINK
    # inactive
    IDLE = 30, "Idle", "Waiting for work", "fas fa-zzz", ColorType.GRAY
    # terminal
    CANCELLED = 50, "Cancelled", "Cancelled before running", "fas fa-circle-xmark", ColorType.GRAY
    ABORTED = 51, "Aborted", "Aborted while running", "fas fa-circle-xmark", ColorType.GRAY
    DIED = 52, "Died", "Unresponsive while running", "fas fa-skull", ColorType.RED
    FAILED = 53, "Failed", "Failed due to an error", "fas fa-circle-xmark", ColorType.RED
    COMPLETED = 54, "Completed", "Completed successfully", "fas fa-circle-check", ColorType.GREEN
    SKIPPED = (
        55,
        "Skipped",
        "Skipped due to a condition",
        "fas fa-circle-exclamation",
        ColorType.GRAY,
    )

    @property
    def is_pre(self) -> bool:
        return self < 10

    @property
    def is_active(self) -> bool:
        return self >= 10 and self < 20

    @property
    def is_interrupted(self) -> bool:
        return self >= 20 and self < 30

    @property
    def is_inactive(self) -> bool:
        return self >= 30 and self < 40

    @property
    def is_terminal(self) -> bool:
        return self >= 50

    @property
    def is_bad(self) -> bool:
        return self in (ProcessStatus.FAILED, ProcessStatus.ABORTED, ProcessStatus.CANCELLED)


INTERRUPTED_PROCESS_STATUSES = bittuple(*(s for s in ProcessStatus if s.is_interrupted))
ACTIVE_PROCESS_STATUSES = bittuple(*(s for s in ProcessStatus if s.is_active))
INACTIVE_PROCESS_STATUSES = bittuple(*(s for s in ProcessStatus if s.is_inactive))
TERMINAL_PROCESS_STATUSES = bittuple(*(s for s in ProcessStatus if s.is_terminal))


@enum_(EnumType.SESSION_STATUS)
class SessionStatus(BuiltinEnum):
    PENDING = 1
    OPEN = 10
    CLOSED = 30


@enum_(EnumType.EXPRESSION_KIND)
class ExpressionKind(BuiltinEnum):
    LITERAL = 1
    FUNCTIONAL = 2
    CONDITIONAL = 3
    SORT = 4
    AGGREGATION = 5


@enum_(EnumType.LITERAL_TYPE)
class LiteralType(BuiltinEnum):
    VALUE = 100  # any freeform value
    NONE = 101
    TRUE = 102
    FALSE = 103

    @property
    def kind(self) -> "ExpressionKind":
        return ExpressionKind.LITERAL


@enum_(EnumType.FUNCTIONAL_TYPE)
class FunctionalType(BuiltinEnum):
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
class ConditionalType(BuiltinEnum):
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
class AggregationType(BuiltinEnum):
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
class SortType(BuiltinEnum):
    ASCENDING = 500
    DESCENDING = 501

    @property
    def kind(self) -> "ExpressionKind":
        return ExpressionKind.SORT


@enum_(EnumType.SORT_MODE)
class SortMode(BuiltinEnum):
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
    ExpressionType = BuiltinEnum.combine(
        "ExpressionType", LiteralType, FunctionalType, ConditionalType, AggregationType, SortType
    )
    ExpressionType.kind = property(lambda self: EXPRESSION_KIND_BY_OP[self])
    enum_(EnumType.EXPRESSION_OP)(ExpressionType)


@enum_(EnumType.CLIENT_TYPE)
class ClientType(BuiltinEnum):
    # user
    WEB = 1
    BROWSER_PLUGIN = 2
    DESKTOP = 3
    MOBILE = 4

    # system
    COMPUTER = 10


#
# Other global stuff
#

CLOUD = get_from_env("CLOUD", typ=Cloud, description="Cloud we're running in")
REGION = get_from_env("REGION", typ=Region, description="Region we're running in")
TRACING = get_from_env("TRACING", typ=bool, description="Enable tracing")


class BenchError(Exception):
    """Common base class for any regular errors."""

    pass


IS_IN_USER_CODE = contextvars.ContextVar("is_in_user_code", default=False)
ACTIVE_SESSION: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)


def active_session() -> "Session":
    """Gets the currently active Session (error if none)."""
    session = ACTIVE_SESSION.get()
    assert session is not None, "no active session"
    return session


def get_active_session() -> Optional["Session"]:
    """Gets the currently active Session (if any)."""
    return ACTIVE_SESSION.get()


def active_tx() -> "Transaction":
    """Gets the currently active Transaction (error if none)."""
    session = ACTIVE_SESSION.get()
    assert session is not None, "no active session"
    assert session._tx is not None, f"no active transaction in {session!r}"
    return session._tx


def get_active_tx() -> Optional["Transaction"]:
    """Gets the currently active Transaction (if any)."""
    session = ACTIVE_SESSION.get()
    if session is None:
        return None
    return session._tx


@_agnosticcontextmanager
def capture_span(
    tracer: Tracer,
    key: str,
    type: "SpanType",
    *,
    level: "Severity | None" = None,
    nodes: list["Node"] | None = None,
    title: str | None = None,
    text: "Text | None" = None,
    runner: "Runner[Any] | None" = None,
) -> Generator["Span | None", None, None]:
    """Decorate or annotate a Span in the current Run (noop if not inside a Run)."""
    from bench.language import Severity, Span

    if runner is None:
        session = ACTIVE_SESSION.get()
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
        span = Span(
            type=type,
            severity=level or Severity.INFO,
            nodes=nodes or [],
            title=title,
            text=text,
            started_at=runtime.oracle.utc(),
            _skip_validate_self=True,
        )
        run._copy_context_to(span)
        run.spans.append(span)
        try:
            with tracer.start_as_current_span(key):
                yield span
        finally:
            assert span.started_at is not None, f"no started_at for {span!r}"
            span.terminated_at = runtime.oracle.utc()
            span.duration = span.terminated_at - span.started_at


def repr_enums(enums: Iterable[BuiltinEnum]) -> str:
    return "|".join(e.bench_name for e in enums)

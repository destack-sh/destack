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
    Optional,
    TypeGuard,
    TypeVar,
    Union,
    cast,
)

from bitarray import bitarray
from fastuuid import UUID, uuid4, uuid5
from more_itertools import first
from opentelemetry.trace import Tracer
from opentelemetry.util._decorator import _agnosticcontextmanager

from bench.utils.env import IS_TEST
from bench.utils.string import Casing, to_casing
from bench.utils.utils import frozendict, get_from_env

if TYPE_CHECKING:
    from bench.language import (
        Session,
        Span,
        SpanType,
    )
    from bench.runtime import Runner


class _Unset:
    def __repr__(self):
        return "<UNSET!>"

    def __str__(self):
        return "<UNSET!>"


# forever constants
VERSION = "2025.05.19.0"
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

    def __new__(
        cls,
        id: int,
        title: str | None = None,
        text: str | None = None,
        icon: str | None = None,
    ):
        obj = int.__new__(cls, id)
        obj._value_ = id
        obj.ord = len(cls)
        obj.id = id
        obj.text = text
        obj.title = title
        obj.icon = icon
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


BuiltinEnumOrUnion = Union[BuiltinEnum, Union[BuiltinEnum, Any]]
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
        enum_name = to_casing(cls.__name__, Casing.ALL_CAPS)
        assert enum_type.name == enum_name, f"enum name mismatch: {enum_type.name} != {enum_name}"
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
#


class EnumType(BuiltinEnum):
    # bench [40000-40200]
    ENUM_TYPE = 40000
    NODE_TYPE = 40001
    STRUCT_TYPE = 40002
    TRAIT = 40004
    NODE_MODE = 40005
    NODE_AREA = 40006
    USER_STATUS = 40010
    ORGANIZATION_STATUS = 40011
    BENCH_STATUS = 40056
    ERROR_TYPE = 40061
    SEVERITY = 40062
    VARIABLE_TYPE = 40063
    EDIT_TYPE = 40070
    EDIT_OPERATION = 40071
    # PROFILE, CREDENTIAL, FRIENDSHIP, ...
    # query
    CONDITIONAL_TYPE = 40102
    AGGREGATION_TYPE = 40103
    SORT_MODE = 40104
    SORT_TYPE = 40105
    JOIN_TYPE = 40106
    FUNCTION_TYPE = 40107
    EXPRESSION_TYPE = 40108
    RELATION_TYPE = 40109
    ATTRIBUTE_TYPE = 40110

    # package [41000-41200]
    PACKAGE_TYPE = 40050
    RESOURCE_STATUS = 41000
    BLOCK_TYPE = 41010
    # APP, PLUGIN, ...

    # infra [41200-41400]
    CLOUD = 40051
    REGION = 40052
    AREA = 40054
    CONTINENT = 40055
    COMPUTER_TYPE = 41210
    DATABASE_TYPE = 41220
    CLIENT_TYPE = 41221
    # VAULT, CACHE, ENDPOINT, DEPLOYMENT, NETWORK, ...

    # data [41400-41600]
    TEXT_LINE_TYPE = 41400
    TEXT_SPAN_TYPE = 41401
    CODE_TYPE = 41410
    FILE_RETENTION_MODE = 41420
    FILE_SOURCE = 41421
    FILE_TYPE = 41422
    FILE_FORMAT = 41423
    ICON_TYPE = 41430
    LINK_TYPE = 41440
    PRIMITIVE_TYPE = 41500
    TYPE_CARDINALITY = 41501
    SCALAR_TYPE = 41502
    STRING_FORMAT = 41510
    NUMBER_FORMAT = 41511
    FIELD_TYPE = 41520
    CASCADE_ACTION = 41521
    DAY = 41530
    MONTH = 41531
    TIME_INTERVAL = 41532
    # ...

    # chat [41800-42000]
    CHANNEL_STATUS = 41800
    THREAD_STATUS = 41801
    MESSAGE_TYPE = 41802
    NOTIFICATION_TYPE = 41810
    NOTIFICATION_STATUS = 41811
    # ...

    # plan [42000-42200]
    CLAIM_TYPE = 42000
    CLAIM_STATUS = 42001
    CURSOR_TYPE = 42010
    CURSOR_STATUS = 42011
    # ...

    # logic [42200-42400]
    ACTION_TYPE = 42220
    FLOW_TYPE = 42222
    FLOW_EDGE_TYPE = 42223
    #  ...

    # quality [42400-42600]
    # ...

    # runtime [42600-42800]
    PROCESS_STATUS = 42600
    RUN_TYPE = 42601
    SPAN_TYPE = 42602
    SCHEDULE_FREQUENCY = 42610
    INTERRUPTION_TYPE = 42620
    INTERRUPTION_STATUS = 42621
    INTERRUPTION_RESPONSE = 42622
    # EVENT, SIGNAL, ...

    # auth [42800-43000]
    QUERY_TYPE = 42850
    # ...

    # version [43200-43400]
    # ...

    # publish [43400-43600]
    # ...

    # analytics [43600-43800]
    # ...

    # locale [43800-44000]
    # ...

    # model [44000-44200]
    MODEL_DEVELOPER = 44000
    MODEL_PROVIDER = 44001

    # finance [44200-44400]
    # ...

    # web [44400-44600]
    # ...

    # world [44600-44800]
    # ...

    # ui [48000-50000]
    SPACE_TYPE = 48000

    # space [48000-48100]
    # ...

    # container views [48100-48200]
    # ...

    # content views [48200-48400]
    # ...

    # node views [48400-48500]
    # ...

    # style [49000-49200]
    POSITION_TYPE = 49000
    COLOR_TYPE = 49010
    COLOR_SHADE = 49011
    COLOR_HUE = 49012
    FONT_WEIGHT = 49021
    FONT_SIZE = 49022
    FONT_TYPE = 49023
    TEXT_ALIGN = 49024
    TEXT_DECORATION = 49025
    TEXT_TRANSFORM = 49026
    SHADOW_TYPE = 49030
    SHADOW_POSITION = 49031
    BORDER_TYPE = 49040
    GRADIENT_TYPE = 49050
    FILL_TYPE = 49060
    FILL_POSITION = 49061
    FILL_SIZE = 49062
    LENGTH_UNIT = 49070
    LAYOUT = 49071
    DISTRIBUTE = 49072
    ALIGN = 49073
    DIRECTION = 49074
    OVERFLOW = 49075
    TRANSITION_TYPE = 49076
    SPRING_TYPE = 49077
    DIMENSION_TYPE = 49078
    THEME_COLOR = 49079
    EFFECT_TYPE = 49080
    REPEAT_TYPE = 49081
    TEXT_SPLIT_TYPE = 49083
    OFFSCREEN_BEHAVIOR = 49084

    # drawing
    # ...

    # audio/media?
    # ...


enum_(EnumType.ENUM_TYPE)(EnumType)


@enum_(EnumType.STRUCT_TYPE)
class StructType(BuiltinEnum):
    # bench [20000-20200]
    SCOPE = 20000
    EDIT = 20001
    ORIGIN = 20003
    NODE_REFERENCE = 20004
    PROPERTY_REFERENCE = 20005
    VARIABLE = 20006
    EXPRESSION = 20100
    FUNCTION = 20101
    JOIN = 20102
    AGGREGATION = 20103
    CONDITION = 20104
    SORT = 20105
    QUERY = 20106
    RELATION_REFERENCE = 20110
    ATTRIBUTE_REFERENCE = 20111

    # package [21000-21200]
    # APP, PLUGIN, ...

    # infra [21200-21400]
    # VAULT, CACHE, ENDPOINT, DEPLOYMENT, NETWORK, ...

    # data [21400-21800]
    TYPE = 21400
    NUMBER_CONSTRAINT = 21401
    STRING_CONSTRAINT = 21402
    COLLECTION_CONSTRAINT = 21403
    NODE_CONSTRAINT = 21404

    VALUE = 21410
    # SCHEMA, UNION, TAG, ...
    TEXT = 21500, None, None, "fas fa-text"
    TEXT_LINE = 21501, None, None, "fas fa-text"
    TEXT_SPAN = 21502, None, None, "fas fa-text"
    CODE = 21510, None, None, "fas fa-code"
    ICON = 21530
    SELECTION = 21470
    # ...

    # social [21800-22000]
    # ...

    # logic [22000-22400]
    SCHEDULE = 22000
    # ...

    # quality [22400-22600]
    # ...

    # runtime [22600-22800]
    ERROR = 22600
    # EVENT, SIGNAL, ...

    # auth [22800-23000]
    # PROFILE? (for User, or maybe global?)

    # version [23200-23400]
    # ...

    # publish [23400-23600]
    # ...

    # analytics [23600-23800]
    # ...

    # locale [23800-24000]
    # ...

    # model [24000-24200]
    # ...

    # finance [24200-24400]
    # ...

    # web [24400-24600]
    # ...

    # world [24600-24800]
    # ...

    # ui [28000-30000]

    # space [28000-28100]
    # ...

    # container views [28100-28200]
    # ...

    # content views [28200-28400]
    # ...

    # node views [28400-28500]
    # ...

    # style [29000-29100]
    VECTOR2 = 29000, None, None, "fas fa-vector-square"
    VECTOR3 = 29002, None, None, "fas fa-vector-square"
    VECTOR4 = 29004, None, None, "fas fa-vector-square"
    AXIS2 = 29006, None, None, "fas fa-vector-square"
    AXIS3 = 29008, None, None, "fas fa-vector-square"
    COLOR = 29010, None, None, "fas fa-palette"
    SHADOW = 29011, None, None, "fas fa-eclipse"
    BORDER = 29012, None, None, "fas fa-border-outer"
    FONT = 29013, None, None, "fas fa-text"
    GRADIENT_STOP = 29014, None, None, "fas fa-gradient"
    GRADIENT = 29015, None, None, "fas fa-gradient"
    FILL = 29016, None, None, "fas fa-fill"
    LENGTH = 29017, None, None, "fas fa-ruler"
    POSITION = 29019, None, None, "fas fa-location-crosshair"
    DIMENSION = 29021, None, None, "fas fa-ruler"
    TRANSITION = 29023, None, None, "fas fa-bezier-curve"
    EFFECT = 29024, None, None, "fas fa-sparkle"
    GRID = 29025, None, None, "fas fa-grid-2"
    GRID_SPAN = 29027, None, None, "fas fa-grid-2"
    INSETS = 29029, None, None, "fas fa-corner"
    CORNERS = 29031, None, None, "fas fa-corner"

    # drawing
    # ...

    # audio/media
    # ...


@enum_(EnumType.NODE_TYPE)
class NodeType(BuiltinEnum):
    # bench [1-200]
    BENCH = 10, "Bench", "Universal workbench", "https://heybench.com/favicon.ico"
    BENCH_MEMBERSHIP = 11, "Bench Membership", "Membership in a Bench", "fas fa-user-group"
    BENCH_INVITE = 12, "Bench Invite", "Invite to a Bench", "fas fa-user-plus"
    HANDLE = 20, "Handle", "Unique identifier", "fas fa-at"
    USER = 30, "User", "User", "fas fa-user"
    ORGANIZATION = 40, "Organization", "Organization", "fas fa-building"
    ORGANIZATION_MEMBERSHIP = (
        41,
        "Organization Membership",
        "Membership in an Organization",
        "fas fa-user-group",
    )
    ORGANIZATION_INVITE = 42, "Organization Invite", "Invite to an Organization", "fas fa-user-plus"
    CLIENT = 50, "Client", "Client to a Bench", "fas fa-desktop"

    # package [1000-1200]
    PACKAGE = 1000, "Package", "Isolated sub-Bench", "fas fa-box-open"
    DEPENDENCY = 1010, "Dependency", "Dependency to something", "fas fa-turn-down-right"
    PAGE = 1020, "Page", "Page of Blocks", "far fa-file"
    BLOCK = 1030, "Block", "Rich Block on a Page", "fas fa-cube"
    APPLICATION = 1040, "Application", "Interactive Application", "fas fa-app"
    # PLUGIN, ...

    # infra [1200-1400]
    DATABASE = 1200, "Store", "Store custom data", "fas fa-database"
    COMPUTER = 1210, "Computer", "Machine for computing", "fas fa-computer-classic"
    # VAULT, CACHE, ENDPOINT, DEPLOYMENT, NETWORK, AUTOSCALER, ...

    # data [1400-1800]
    SCHEMA = 1400, "Schema", "Schema", "fas fa-shapes"
    FIELD = 1410, "Field", "Field", "fas fa-triangle"
    TABLE = 1500, "Table", "Table of Records", "fas fa-table"
    RECORD = 1510, "Record", "Record in a Database", "fas fa-database"
    FILE = 1520, "File", "File", "fas fa-file"
    LINK = 1550, "Link", "Link to something", "fas fa-link"
    # STREAM, SECRET, INDEX, CONSTRAINT, MIGRATION, ...

    # social [1800-2000]
    CHANNEL = 1800, "Channel", "Channel", "fas fa-hashtag"
    THREAD = 1810, "Thread", "Thread", "fas fa-reel"
    MESSAGE = 1820, "Message", "Message", "fas fa-message"
    NOTIFICATION = 1850, "Notification", "Notification", "fas fa-bell"
    # FEED, FEED_ITEM, ...
    # POLL, VOTE, RATING, REACTION, ...

    # logic [2000-2400]
    SERVICE = 2000, "Service", "Service", "fas fa-screwdriver-wrench"
    ACTION = 2020, "Action", "Action", "fas fa-step-forward"
    FLOW = 2040, "Flow", "Sequence Actions", "fas fa-diagram-project"
    FLOW_EDGE = 2041, "Flow Edge", "Edge between Actions", "fas fa-link"
    AGENT = 2100, "Agent", "Identity for an AI", "fas fa-robot"
    TASK = 2200, "Task", "To-do item", "far fa-square-check"
    CURSOR = 2220, "Cursor", "Position in something", "fas fa-mouse"
    # ROOM, JOB, PLAN, LOCK, ...
    # TRAIT, INTERFACE, ...
    # TRIGGER, TIMER, BREAKPOINT, ...

    # quality [2400-2600]
    # TEST, TEST_SUITE, TEST_CASE, TEST_RESULT, ...
    # ROLLOUT, ...

    # runtime [2600-2800]
    RUN = 2610, "Run", "Run", "fas fa-play"
    SPAN = 2620, "Span", "Span", "fas fa-ruler-horizontal"
    INTERRUPTION = 2630, "Interruption", "Interruption", "fas fa-hand"
    # EVENT, SIGNAL, ...

    # auth [2800-3000]
    TEAM = 2820, "Team", "Group of Users or Agents", "fas fa-users"
    ROLE = 2830, "Role", "Role", "fas fa-user-tag"
    # PROFILE? (for User, or maybe global?)
    CLAIM = 2840, "Claim", "Control over something", "fas fa-stamp"
    # PERMISSION, PERMISSION_GROUP, ...
    # CHALLENGE, FRIENDSHIP, BADGE, ENTITLEMENT, POLICY, RULE, ...
    # KICK/BAN, ...

    # version [3200-3400]
    # CHANGE, HISTORY, BRANCH, ...

    # publish [3400-3600]
    # PUBLICATION, PREVIEW, RELEASE, WISHLIST/WATCHLIST, ...

    # analytics [3600-3800]
    # METER, METRIC, SURVEY, REPLAY, ...

    # locale [3800-4000]
    # LOCALE, TRANSLATION, ...

    # model [4000-4200]
    # MODEL, FINETUNE, ...

    # finance [4200-4400] (also see Stripe API?)
    # WALLET, BALANCE, BUDGET, TRANSFER, CREDIT, ...
    # TIER, SUBSCRIPTION, PRODUCT, PRICE, ORDER, INVOICE, DISCOUNT, DISPUTE, REFUND, ...

    # web [4400-4600]
    # ACCOUNT, DOMAIN, EMAIL, ...

    # world [4600-4800]
    # PHONE, ADDRESS, ...

    # ui [8000-10000]

    # space [8000-8100]
    SPACE = 8000, "Space", "Space", "fas fa-galaxy"
    SCENE = 8010, "Scene", "Scene of an Application", "fas fa-masks-theater"
    ROUTE = 8020, "Route", "Route to a Scene", "fas fa-route"
    # COMMAND, OVERLAY, WIDGET, ...

    # container views [8100-8200]
    FRAME_VIEW = 8100, "Frame View", "Fixed Container", "fas fa-frame"
    LABEL_VIEW = 8101, "Label View", "Label Container", "fas fa-font-case"
    # FORM_VIEW, MENU_VIEW, EMAIL_VIEW, ...
    # COMPONENT_VIEW = 8110, "Component View", "Component Container", "fas fa-cube"
    SPLIT_VIEW = 8120, "Split View", "Split Container", "fas fa-columns"
    # TAB_VIEW = 8120, "Tab Container View", "Tab Container", "fas fa-tabs"
    # DRAWER_VIEW, SPLIT_DRAWER_VIEW, GRID/GRID_ELEMENT_VIEW, ...

    # content views [8200-8300]
    TEXT_VIEW = 8200, "Text View", "Text", "fas fa-text"
    # CODE_VIEW = 8201, "Code View", "Code", "fas fa-code"
    # ICON_VIEW = 8202, "Icon View", "Icon", "fas fa-icons"
    # BUTTON_VIEW = 8210, "Button View", "Button", "fas fa-hand-pointer"
    # LINK_VIEW = 8211, "Link View", "Link", "fas fa-link"
    # IMAGE_VIEW = 8220, "Image View", "Image", "fas fa-image"
    # AUDIO_VIEW = 8221, "Audio View", "Audio", "fas fa-volume"
    # VIDEO_VIEW = 8222, "Video View", "Video", "fas fa-video"
    # DOCUMENT_VIEW = 8223, "Document View", "Document", "fas fa-file-alt"

    # input views [8300-8400]
    NUMBER_INPUT_VIEW = 8300, "Number Input View", "Number Input", "fas fa-hashtag"
    SLIDER_INPUT_VIEW = 8301, "Slider Input View", "Slider Input", "fas fa-slider"
    # STRING_INPUT_VIEW = 8302, "String Input View", "String", "fas fa-font-case"
    # TOGGLE_INPUT_VIEW = 8303, "Toggle Input View", "Toggle", "fas fa-square-check"
    # PICKER_INPUT_VIEW = 8310, "Picker Input View", "Picker", "fas fa-caret-circle-down"
    # COLOR_INPUT_VIEW = 8311, "Color Input View", "Color", "fas fa-palette"
    # ICON_INPUT_VIEW = 8312, "Icon Input View", "Icon", "fas fa-icons"
    # FILE_INPUT_VIEW = 8313, "File Input View", "File", "fas fa-file"
    # DATETIME_INPUT_VIEW = 8320, "Datetime Input View", "Datetime", "fas fa-calendar-days"
    # DURATION_INPUT_VIEW = 8321, "Duration Input View", "Duration", "fas fa-stopwatch"

    # node views [8400-8500]
    # NODE_VIEW = 8400, "Node View", "Node", "fas fa-hexagon"
    # NODE_CHIP_VIEW = 8401, "Node Chip View", "Node Chip", "fas fa-hexagon"
    # NODE_PATH = 8405, "Node Path", "Node Path", "fas fa-sitemap"
    # PAGE_VIEW = 8410, "Page View", "Page", "far fa-file"
    # PAGE_PREVIEW_VIEW, ...
    # TABLE_VIEW = 8420, "Table View", "Table", "fas fa-table"
    # TABLE_PREVIEW_VIEW, ...
    THREAD_VIEW = 8430, "Thread View", "Thread", "fas fa-reel"
    # THREAD_PREVIEW_VIEW, ...
    # FILE_VIEW, FILE_CHIP_VIEW, FILE_PREVIEW_VIEW, ...

    # internal views [8500-8600]
    WIZARD_VIEW = 8500, "Wizard View", "Wizard", "fas fa-wand-sparkles"
    # SIDEBAR_VIEW = 8501, "Sidebar View", "Sidebar", "fas fa-bars"
    # CONTEXT_VIEW = 8502, "Context View", "Context", "fas fa-sitemap"

    # style [9000-9100]
    THEME = 9000, "Theme", "Theme", "fas fa-palette"
    COLOR_STYLE = 9010, "Color Style", "Color Style", "fas fa-palette"
    FONT_STYLE = 9011, "Font Style", "Font Style", "fas fa-text"
    BORDER_STYLE = 9012, "Border Style", "Border Style", "fas fa-border-outer"
    SHADOW_STYLE = 9013, "Shadow Style", "Shadow Style", "fas fa-eclipse"
    GRADIENT_STYLE = 9014, "Gradient Style", "Gradient Style", "fas fa-gradient"
    TRANSITION_STYLE = 9015, "Transition Style", "Transition Style", "fas fa-bezier-curve"
    EFFECT_STYLE = 9016, "Effect Style", "Effect Style", "fas fa-sparkle"
    # VARIANT :RichGraph

    # canvas/drawing?
    # CANVAS, BRUSH, SHAPE, ...

    # audio/media?
    # SOUND, ...?


@enum_(EnumType.TRAIT)
class Trait(BuiltinEnum):
    # bench [1-200]
    GLOBAL = 1, "Global", "Is global", "fas fa-globe"
    LOCAL = 3, "Local", "Is local", "fas fa-globe"
    MODAL = 10, "Modal", "Has a mode", "fas fa-window-maximize"
    ARCHIVABLE = 11, "Archivable", "Can be archived", "fas fa-box-archive"
    DELETABLE = 12, "Deletable", "Can be deleted", "fas fa-trash"
    NAMED = 20, "Named", "Has a name", "fas fa-font-case"
    TITLED = 21, "Titled", "Has a title", "fas fa-font-case"
    SLUG = 22, "Slug", "Has a slug", "fas fa-hashtag"
    ICON = 23, "Icon", "Has an Icon", "fas fa-icons"
    ORDERED = 24, "Ordered", "Has an order", "fas fa-sort"
    TEMPLATABLE = 30, "Templatable", "Can be templated", "fas fa-puzzle-piece"
    INSTANTIABLE = 31, "Instantiable", "Can be instantiated", "fas fa-clone"
    EXTENSIBLE = 32, "Extensible", "Can be extended", "fas fa-expand"
    BASED = 33, "Based", "Can be based on", "fas fa-baseball-bat-ball"
    IN_BENCH = 40, "Bench", "In a Bench", "fas fa-bench"
    IN_PACKAGE = 41, "Package", "In a Package", "fas fa-box"
    REGIONAL = 50, "Regional", "Is regional", "fas fa-globe"
    RESOURCE = 51, "Resource", "Is a Resource"
    PROVISIONABLE = 52, "Provisionable", "Can be provisioned", "fas fa-server"
    # package [1000-1200]
    PAGEABLE = 1020, "Page", "In a Page", "far fa-file"
    BLOCKABLE = 1030, "Block", "Can be a Block on a Page", "fas fa-cube"
    # logic [2000-2400]
    RUNNABLE = 2000, "Runnable", "Can be run", "fas fa-play"
    PROCESSABLE = 2001, "Processable", "Can be processed", "fas fa-cogs"
    COMPUTABLE = 2002, "Computable", "Can be computed", "fas fa-calculator"
    # auth [2800-3000]
    OWNABLE = 2800, "Ownable", "Can be owned", "fas fa-user"
    CLAIMABLE = 2801, "Claimable", "Can be claimed", "fas fa-stamp"
    JOINABLE = 2802, "Joinable", "Can be joined", "fas fa-users"
    SUBJECT = 2805, "Subject", "Is a Subject", "fas fa-user"
    MEMBERSHIP = 2810, "Membership", "Is a Membership", "fas fa-users"
    INVITE = 2811, "Invite", "Is an Invite", "fas fa-envelope"
    # ui [8000-10000]
    VIEW = 8001, "View", "Is a View", "fas fa-eye"
    CONTAINER_VIEW = 8100, "Container View", "Is a Container View", "fas fa-container"
    CONTENT_VIEW = 8200, "Content View", "Is a Content View", "fas fa-content"
    INPUT_VIEW = 8300, "Input View", "Is an Input View", "fas fa-input"
    NODE_VIEW = 8400, "Node View", "Is a Node View", "fas fa-node"
    INTERNAL_VIEW = 8500, "Internal View", "Is an Internal View", "fas fa-internal"
    STYLE = 9000, "Style", "Is a Style", "fas fa-palette"


@enum_(EnumType.NODE_AREA)
class NodeArea(BuiltinEnum):
    GLOBAL_POSTGRES = 100
    REGIONAL_POSTGRES = 200
    # REGIONAL_REDIS, REGIONAL_ELASTICSEARCH, ...
    LOCAL_POSTGRES = 300
    # LOCAL_REDIS, LOCAL_ELASTICSEARCH, ...


@enum_(EnumType.NODE_MODE)
class NodeMode(BuiltinEnum):
    KERNEL = 3, "Kernel", "Managed by Bench (hidden)", "fas fa-cog"
    SYSTEM = 6, "System", "Managed by Bench", "fas fa-cog"
    BUILTIN = 10, "Builtin", "Provided by Bench", "fas fa-cog"
    MAIN = 20, "Main", "Active and available", "fas fa-globe"
    TEST = 30, "Test", "Active in test", "fas fa-flask"
    TEMPLATE = 40, "Template", "Template to use", "fas fa-puzzle-piece"
    ARCHIVE = 50, "Archive", "Inactive and hidden", "fas fa-box-archive"


ENUM_TYPES: bittuple[EnumType] = bittuple(*EnumType)
NODE_TYPES = bittuple(*NodeType)
STRUCT_TYPES: bittuple[StructType] = bittuple(*StructType)


def is_node_type(obj: BuiltinEnum | int | Any) -> TypeGuard[NodeType]:
    return isinstance(obj, int) and obj in NODE_TYPES


def is_struct_type(obj: BuiltinEnum | int | Any) -> TypeGuard[StructType]:
    return isinstance(obj, int) and obj in STRUCT_TYPES


def is_enum_type(obj: BuiltinEnum | int | Any) -> TypeGuard[EnumType]:
    return isinstance(obj, int) and obj in ENUM_TYPES


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


@enum_(EnumType.CONTINENT)
class Continent(BuiltinEnum):
    """
    'Continents' of Regions.
    """

    EUROPE = 1000, "Europe", None, "🇪🇺"
    NORTH_AMERICA = 2000, "North America", None, "🇺🇸"
    SOUTH_AMERICA = 3000, "South America", None, "🇧🇷"
    MIDDLE_EAST = 4000, "Middle East", None, "🇸🇦"
    AFRICA = 5000, "Africa", None, "🇿🇦"
    ASIA = 6000, "Asia", None, "🇮🇳"
    AUSTRALIA = 7000, "Australia", None, "🇦🇺"
    PRIVATE = 9000

    @property
    def slug(self) -> str:
        return REGION_CONTINENT_SLUGS[self]

    @staticmethod
    def get_by_slug(slug: str) -> "Continent":
        return REGION_CONTINENT_BY_SLUG[slug]


REGION_CONTINENT_SLUGS: dict[Continent, str] = {
    Continent.EUROPE: "eu",
    Continent.NORTH_AMERICA: "na",
    Continent.SOUTH_AMERICA: "sa",
    Continent.MIDDLE_EAST: "me",
    Continent.AFRICA: "af",
    Continent.ASIA: "as",
    Continent.AUSTRALIA: "au",
}
REGION_CONTINENT_BY_SLUG = {v: k for k, v in REGION_CONTINENT_SLUGS.items()}


@enum_(EnumType.AREA)
class Area(BuiltinEnum):
    """
    A larger Area of Regions within a Continent.
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
    def continent(self) -> Continent:
        return Continent((self.id // 1000) * 1000)

    @property
    def slug(self) -> str:
        return REGION_AREA_SLUGS[self]

    @staticmethod
    def get_by_slug(slug: str) -> "Area":
        return REGION_AREA_BY_SLUG[slug]


REGION_AREA_SLUGS: dict[Area, str] = {
    Area.EUROPE_CENTRAL: "eu-central",
    Area.NORTH_AMERICA_EAST: "na-east",
    Area.NORTH_AMERICA_WEST: "na-west",
    Area.SOUTH_AMERICA_EAST: "sa-east",
    Area.MIDDLE_EAST_CENTRAL: "me-central",
    Area.MIDDLE_EAST_WEST: "me-west",
    Area.AFRICA_SOUTH: "af-south",
    Area.ASIA_WEST: "as-west",
    Area.ASIA_SOUTH: "as-south",
    Area.ASIA_EAST: "as-east",
    Area.AUSTRALIA_SOUTH: "au-south",
}
REGION_AREA_BY_SLUG = {v: k for k, v in REGION_AREA_SLUGS.items()}


@enum_(EnumType.REGION)
class Region(BuiltinEnum):
    """Regions in an Area on a Continent."""

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
    def continent(self) -> Continent:
        return Continent((self.id // 1000) * 1000)

    @property
    def area(self) -> Area:
        return Area((self.id // 200) * 200)

    @property
    def slug(self) -> str:
        continent = self.continent
        return continent.slug + "-" + self.name.replace("_", "-").lower()

    @staticmethod
    def get_by_slug(slug: str) -> "Region":
        return REGION_BY_SLUG[slug]


REGION_BY_SLUG = {r.slug: r for r in Region}


class NodeReferenceKind(BuiltinEnum):
    NODE_PARENT = 1
    NODE_ANCESTOR = 2
    NODE_REGULAR = 5
    NODE_TEMPLATE = 6

    @property
    def is_node_tree(self):
        return self.id <= 4

    @property
    def is_node(self):
        return self.id < 10

    @property
    def is_struct_tree(self):
        return self.id >= 10 and self.id <= 20


class NodeReferenceProperty(BuiltinEnum):
    NODE_TYPE = 30
    NODE_ID = 31
    NODE_CK = 32
    NODE_BENCH_ID = 33
    NODE_BASE_ID = 34


@enum_(EnumType.SEVERITY)
class Severity(BuiltinEnum):
    TRACE = 1, None, None, "fas fa-bug"
    DEBUG = 2, None, None, "fas fa-bug"
    INFO = 3, None, None, "fas fa-circle-check"
    WARNING = 4, None, None, "fas fa-circle-exclamation"
    ERROR = 5, None, None, "fas fa-circle-exclamation"
    PANIC = 6, None, None, "fas fa-skull"


@enum_(EnumType.CASCADE_ACTION)
class CascadeAction(BuiltinEnum):
    RESTRICT = 1
    CASCADE = 2
    SET_NULL = 3
    # SET_DEFAULT, NONE, ...


@enum_(EnumType.PRIMITIVE_TYPE)
class PrimitiveType(BuiltinEnum):
    """
    Fundamental column / storage types we support (subset of SQL types, used directly in sql/core).
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
    AGENT = 15


@enum_(EnumType.SPAN_TYPE)
class SpanType(BuiltinEnum):
    # general
    ATTEMPT = 1, None, None, "fas fa-play"
    WAIT = 2, None, None, "fas fa-hourglass-end"
    ACQUIRE = 3, None, None, "fas fa-toolbox"
    CODE = 10, None, None, "fas fa-code"
    # agent
    AGENT_TURN = 100, None, None, "fas fa-hexagon-nodes"
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


@enum_(EnumType.RESOURCE_STATUS)
class ResourceStatus(BuiltinEnum):
    """Generalized status of a Resource in its lifecycle."""

    # pre
    PENDING = (1, "Pending", "Waiting for provisioning", "fas fa-hourglass-start")
    CREATING = (2, "Creating", "Actively provisioning", "fas fa-hourglass-start")
    RETRYING = (3, "Retrying", "Retrying provisioning", "fas fa-exclamation-triangle")

    # active states
    AVAILABLE = (10, "Available", "Operational and available", "fas fa-check-circle")
    SLEEPING = (11, "Sleeping", "Available but not running", "fas fa-moon")
    UNAVAILABLE = (15, "Unavailable", "Unavailable or not responding", "fas fa-plug-circle-xmark")
    IMPAIRED = (
        16,
        "Impaired",
        "Operational but experiencing issues",
        "fas fa-exclamation-triangle",
    )
    # terminal
    OFFLINE = (30, "Offline", "Decommissioned and unavailable", "fas fa-power-off")
    FAILED = (31, "Failed", "Failed to provision", "fas fa-exclamation-triangle")

    @property
    def is_pre(self) -> bool:
        """Whether this Resource is in the pre-provisioning state."""
        return 1 <= self.value < 10

    @property
    def is_extant(self) -> bool:
        """Whether this Resource does/should exist."""
        return 10 <= self.value <= 20

    @property
    def is_terminal(self) -> bool:
        """Whether this Resource is terminal."""
        return 30 <= self.value <= 40


@enum_(EnumType.PROCESS_STATUS)
class ProcessStatus(BuiltinEnum):
    # pre
    CREATED = 1, "Created", "Created but not yet assigned", "fas fa-circle"
    ASSIGNED = 2, "Assigned", "Assigned to someone", "fas fa-clock"
    SCHEDULED = 4, "Scheduled", "Scheduled for sometime", "fas fa-clock"
    QUEUED = 5, "Queued", "Queued to happen soon", "fas fa-clock"
    # active
    RUNNING = 10, "Running", "Actively running", "fas fa-circle-notch"
    FAILING = 11, "Failing", "Experiencing issues", "fas fa-circle-exclamation"
    # interrupted
    PAUSED = 20, "Paused", "Paused manually", "fas fa-circle-pause"
    YIELDED = 21, "Yielded", "Yielded to someone", "fas fa-circle-pause"
    WAITING = 22, "Waiting", "Waiting for a condition", "fas fa-circle-pause"
    # inactive
    IDLE = 30, "Idle", "Waiting for work", "fas fa-zzz"
    # terminal
    CANCELLED = 50, "Cancelled", "Cancelled before running", "fas fa-circle-xmark"
    ABORTED = 51, "Aborted", "Aborted while running", "fas fa-circle-xmark"
    DIED = 52, "Died", "Unresponsive while running", "fas fa-skull"
    FAILED = 53, "Failed", "Failed due to an error", "fas fa-circle-xmark"
    COMPLETED = 54, "Completed", "Completed successfully", "fas fa-circle-check"
    SKIPPED = 55, "Skipped", "Skipped due to a condition", "fas fa-circle-exclamation"

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


@_agnosticcontextmanager
def capture_span(
    tracer: Tracer,
    key: str,
    type: "SpanType",
    *,
    level: "Severity | None" = None,
    title: str | None = None,
    runner: "Runner[Any] | None" = None,
) -> Generator["Span | None", None, None]:
    """Decorate or annotate a Span in the current Run (noop if not inside a Run)."""
    from bench.language import Span

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
        span = Span(type=type, title=title, started_at=runtime.oracle.utc())
        # run._copy_context_to(span)
        run.add_child(span)
        try:
            with tracer.start_as_current_span(key):
                yield span
        finally:
            assert span.started_at is not None, f"no started_at for {span!r}"
            span.terminated_at = runtime.oracle.utc()
            span.duration = span.terminated_at - span.started_at


def repr_enums(enums: Iterable[BuiltinEnum]) -> str:
    return "|".join(e.bench_name for e in enums)

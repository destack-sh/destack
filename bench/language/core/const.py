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
        Span,
        SpanType,
        Transaction,
    )
    from bench.runtime import Runner


class _Unset:
    def __repr__(self):
        return "<UNSET!>"

    def __str__(self):
        return "<UNSET!>"


# forever constants
VERSION = "2025.05.15.1"
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
#


class EnumType(BuiltinEnum):
    # meta [20000-20200]
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
    USER_STATUS = 20010
    ORGANIZATION_STATUS = 20011
    BENCH_STATUS = 20056
    ERROR_KIND = 20100
    ERROR_TYPE = 20101
    SEVERITY = 20102
    # PROFILE, CREDENTIAL, FRIENDSHIP, ...

    # package [21000-21200]
    PACKAGE_TYPE = 20050
    RESOURCE_STATUS = 21000
    BLOCK_TYPE = 21010
    # APP, PLUGIN, ...

    # infra [21200-21400]
    CLOUD = 20051
    REGION = 20052
    AREA = 20054
    CONTINENT = 20055
    SCALER_TYPE = 21200
    SCALER_STRATEGY = 21201
    COMPUTER_TYPE = 21210
    DATABASE_TYPE = 21220
    CLIENT_TYPE = 21221
    # VAULT, CACHE, ENDPOINT, DEPLOYMENT, NETWORK, ...

    # type [21400-21600]
    TEXT_LINE_TYPE = 21400
    TEXT_SPAN_TYPE = 21401
    CODE_TYPE = 21410
    FILE_RETENTION_MODE = 21420
    FILE_SOURCE = 21421
    FILE_TYPE = 21422
    FILE_FORMAT = 21423
    ICON_TYPE = 21430
    LINK_TYPE = 21440
    # SCHEMA, UNION, TAG, ...

    # data [21600-21800]
    PRIMITIVE_TYPE = 21600
    FIELD_ZONE = 21601
    TYPE_KIND = 21602
    TYPE_FORMAT = 21603
    DAY = 21610
    MONTH = 21611
    TIME_INTERVAL = 21612
    EXPRESSION_KIND = 21640
    EXPRESSION_OP = 21641
    LITERAL_TYPE = 21642
    FUNCTIONAL_TYPE = 21643
    CONDITIONAL_TYPE = 21644
    AGGREGATION_TYPE = 21645
    SORT_MODE = 21650
    SORT_TYPE = 21651
    SELECTION_TYPE = 21652
    # STREAM, SECRET, INDEX, CONSTRAINT, ...

    # chat [21800-22000]
    CHANNEL_STATUS = 21800
    THREAD_STATUS = 21801
    MESSAGE_TYPE = 21802
    NOTIFICATION_TYPE = 21810
    NOTIFICATION_STATUS = 21811
    # POLL, VOTE, REACTION, ...

    # plan [22000-22200]
    CLAIM_TYPE = 22000
    CLAIM_STATUS = 22001
    CURSOR_TYPE = 22010
    CURSOR_STATUS = 22011
    # JOB, PLAN, ENTITLEMENT, POOL, LOCK, BARRIER, ...

    # logic [22200-22400]
    ACTION_TYPE = 22220
    FLOW_EDGE_TYPE = 22222
    FLOW_TYPE = 22223
    # TRIGGER, TIMER, BREAKPOINT, ...

    # qa [22400-22600]
    # ...

    # runtime [22600-22800]
    PROCESS_STATUS = 22600
    RUN_TYPE = 22601
    SPAN_TYPE = 22602
    SESSION_STATUS = 22603
    SCHEDULE_FREQUENCY = 22610
    INTERRUPTION_TYPE = 22620
    INTERRUPTION_STATUS = 22621
    INTERRUPTION_RESPONSE = 22622
    # EVENT, SIGNAL, ...

    # identity [22800-23000]
    ACCESS_MODE = 22800
    ACCESS_KIND = 22801
    POLICY_EFFECT = 22802
    ACCESS_TYPE = 22803
    QUERY_TYPE = 22850
    EDIT_TYPE = 22851
    USE_TYPE = 22852
    # PROFILE? (for User, or maybe global?)

    # access [23000-23200]
    # ...

    # version [23200-23400]
    # ...

    # publish [23400-23600]
    # ...

    # analytics [23600-23800]
    # ...

    # locale [23800-24000]
    # ...

    # model [24000-24200]
    MODEL_DEVELOPER = 24000
    MODEL_PROVIDER = 24001

    # finance [24200-24400]
    # ...

    # web [24400-24600]
    # ...

    # world [24600-24800]
    # ...

    # view [28000-29000]
    SPACE_TYPE = 28000
    POSITION_TYPE = 28080

    # space [28000-28100]
    # ...

    # container views [28100-28200]
    # ...

    # content views [28200-28400]
    # ...

    # node views [28400-28500]
    # ...

    # style [28500-28600]
    COLOR_TYPE = 28010
    COLOR_SHADE = 28011
    COLOR_HUE = 28012
    FONT_TYPE = 28020
    FONT_WEIGHT = 28021
    FONT_SIZE = 28022
    TEXT_TYPE = 28023
    TEXT_ALIGN = 28024
    TEXT_DECORATION = 28025
    TEXT_TRANSFORM = 28026
    SHADOW_TYPE = 28030
    SHADOW_POSITION = 28031
    BORDER_TYPE = 28040
    GRADIENT_TYPE = 28050
    FILL_TYPE = 28060
    FILL_POSITION = 28061
    FILL_SIZE = 28062
    LENGTH_UNIT = 28070
    LAYOUT = 28071
    DISTRIBUTE = 28072
    ALIGN = 28073
    DIRECTION = 28074
    OVERFLOW = 28075
    TRANSITION_TYPE = 28076
    SPRING_TYPE = 28077
    EFFECT_TYPE = 28078
    DIMENSION_TYPE = 28079


enum_(EnumType.ENUM_TYPE)(EnumType)


@enum_(EnumType.NODE_TYPE)
class NodeType(BuiltinEnum):
    # meta [1-200]
    BENCH = 1, "Bench", "Universal workbench", "https://heybench.com/favicon.ico"
    HANDLE = 2, "Handle", "Unique identifier", "fas fa-at"
    USER = 10, "User", "User", "fas fa-user"
    ORGANIZATION = 20, "Organization", "Organization", "fas fa-building"
    CLIENT = 50, "Client", "Client to a Bench", "fas fa-desktop"
    # PROFILE, CREDENTIAL, FRIENDSHIP, ...

    # ...materialized global stuff?

    # package [1000-1200]
    PACKAGE = 1000, "Package", "Isolated sub-Bench", "fas fa-box-open"
    DEPENDENCY = 1010, "Dependency", "Dependency to something", "fas fa-turn-down-right"
    PAGE = 1020, "Page", "Page of Blocks", "far fa-file"
    BLOCK = 1030, "Block", "Rich Block on a Page", "fas fa-cube"
    # APP, PLUGIN, ...

    # infra [1200-1400]
    DATABASE = 1200, "Store", "Store custom data", "fas fa-database"
    COMPUTER = 1210, "Computer", "Machine for computing", "fas fa-computer-classic"
    SCALER = 1250, "Scaler", "Autoscale Resources", "fas fa-scale-unbalanced"
    # VAULT, CACHE, ENDPOINT, DEPLOYMENT, NETWORK, ...

    # type [1400-1600]
    CHOICE = 1400, "Choice", "Choice between Options", "fas fa-circle-chevron-down"
    CLASS = 1410, "Class", "Class", "fas fa-shapes"
    FIELD = 1420, "Field", "Field", "fas fa-triangle"
    OPTION = 1430, "Option", "Option", "far fa-square-check"
    # SCHEMA, UNION, TAG, ...

    # data [1600-1800]
    TABLE = 1600, "Table", "Table of Records", "fas fa-table"
    RECORD = 1610, "Record", "Record in a Database", "fas fa-database"
    FILE = 1620, "File", "File", "fas fa-file"
    LINK = 1650, "Link", "Link to something", "fas fa-link"
    # STREAM, SECRET, INDEX, CONSTRAINT, ...

    # chat [1800-2000]
    CHANNEL = 1800, "Channel", "Channel", "fas fa-hashtag"
    THREAD = 1810, "Thread", "Thread", "fas fa-reel"
    MESSAGE = 1820, "Message", "Message", "fas fa-message"
    NOTIFICATION = 1850, "Notification", "Notification", "fas fa-bell"
    # POLL, VOTE, RATING, REACTION, ...

    # plan [2000-2200]
    TASK = 2000, "Task", "To-do item", "far fa-square-check"
    CLAIM = 2010, "Claim", "Control over something", "fas fa-stamp"
    CURSOR = 2020, "Cursor", "Position in something", "fas fa-mouse"
    # JOB, PLAN, ENTITLEMENT, POOL, LOCK, BARRIER, ...

    # logic [2200-2400]
    SERVICE = 2200, "Service", "Service", "fas fa-screwdriver-wrench"
    ACTION = 2210, "Action", "Action", "fas fa-step-forward"
    FLOW = 2220, "Flow", "Sequence Actions", "fas fa-diagram-project"
    FLOW_EDGE = 2221, "Flow Edge", "Edge between Actions", "fas fa-link"
    AGENT = 2250, "Agent", "Identity for an AI", "fas fa-robot"
    # TRIGGER, TIMER, BREAKPOINT, ...

    # qa [2400-2600]
    # ...

    # runtime [2600-2800]
    SESSION = 2600, "Session", "Session", "fas fa-circle-play"
    RUN = 2610, "Run", "Run", "fas fa-play"
    SPAN = 2620, "Span", "Span", "fas fa-ruler-horizontal"
    INTERRUPTION = 2630, "Interruption", "Interruption", "fas fa-hand"
    # EVENT, SIGNAL, ...

    # identity [2800-3000]
    MEMBERSHIP = 2800, "Membership", "Membership to something", "fas fa-users"
    INVITE = 2810, "Invite", "Invite to something", "fas fa-user-plus"
    TEAM = 2820, "Team", "Group of Users or Agents", "fas fa-users"
    ROLE = 2830, "Role", "Role", "fas fa-user-tag"
    # PROFILE? (for User, or maybe global?)

    # access [3000-3200]
    # CHALLENGE, BADGE, POLICY, RULE, ...

    # version [3200-3400]
    # CHANGE, HISTORY, BRANCH, ...

    # publish [3400-3600]
    # PUBLICATION, RELEASE, WISHLIST/WATCHLIST, ...

    # analytics [3600-3800]
    # METER, METRIC, SURVEY, REPLAY, ...

    # locale [3800-4000]
    # LOCALE, TRANSLATION, ...

    # model [4000-4200]
    # MODEL, FINETUNE, ...

    # finance [4200-4400] (also see Stripe API?)
    # WALLET, BALANCE, BUDGET, TRANSFER, CREDIT, TIER, PRICE, ORDER, INVOICE, DISCOUNT, DISPUTE, REFUND, ...

    # web [4400-4600]
    # ACCOUNT, APPLICATION, DOMAIN, EMAIL, ...

    # world [4600-4800]
    # PHONE, ADDRESS, ...

    # view [8000-9000]

    # space [8000-8100]
    SPACE = 8000, "Space", "Space", "fas fa-galaxy"
    WIZARD_VIEW = 8010, "Wizard View", "Wizard", "fas fa-wand-sparkles"
    # SIDEBAR_VIEW = 8011, "Sidebar View", "Sidebar", "fas fa-bars"
    # CONTEXT_VIEW = 8012, "Context View", "Context", "fas fa-sitemap"
    # SCENE, OVERLAY, WIDGET, ROUTE, ...

    # container views [8100-8200]
    FRAME_VIEW = 8100, "Frame View", "Fixed Container", "fas fa-frame"
    LABEL_VIEW = 8101, "Label View", "Label Container", "fas fa-font-case"
    # STACK_VIEW?, SCROLL_VIEW?, CARD_VIEW?, FORM_VIEW, ...
    COMPONENT_VIEW = 8110, "Component View", "Component Container", "fas fa-cube"
    SPLIT_VIEW = 8120, "Split View", "Split Container", "fas fa-columns"
    # TAB_VIEW = 8120, "Tab Container View", "Tab Container", "fas fa-tabs"
    # DRAWER_VIEW, SPLIT_DRAWER_VIEW, GRID/GRID_ELEMENT_VIEW, ...

    # content views [8200-8300]
    TEXT_VIEW = 8222, "Text View", "Text", "fas fa-text"
    # CODE_VIEW = 8224, "Code View", "Code", "fas fa-code"
    # BUTTON_VIEW = 8225, "Button View", "Button", "fas fa-hand-pointer"
    # LINK_VIEW = 8226, "Link View", "Link", "fas fa-link"
    # ICON_VIEW = 8231, "Icon View", "Icon", "fas fa-icons"
    # IMAGE_VIEW = 8232, "Image View", "Image", "fas fa-image"
    # AUDIO_VIEW = 8233, "Audio View", "Audio", "fas fa-volume"
    # VIDEO_VIEW = 8229, "Video View", "Video", "fas fa-video"
    # DOCUMENT_VIEW = 8230, "Document View", "Document", "fas fa-file-alt"

    # input views [8300-8400]
    NUMBER_INPUT_VIEW = 8300, "Number Input View", "Number or String Input", "fas fa-hashtag"
    SLIDER_INPUT_VIEW = 8301, "Slider Input View", "Slider", "fas fa-slider"
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
    # PAGE_VIEW = 8410, "Page View", "Page", "far fa-file"
    # PAGE_PREVIEW_VIEW, ...
    # TABLE_VIEW = 8420, "Table View", "Table", "fas fa-table"
    # TABLE_PREVIEW_VIEW, ...
    THREAD_VIEW = 8430, "Thread View", "Thread", "fas fa-reel"
    # THREAD_PREVIEW_VIEW, ...
    # FILE_VIEW, FILE_CHIP_VIEW, FILE_PREVIEW_VIEW, ...

    # style [8500-8600]
    THEME = 8500, "Theme", "Theme", "fas fa-palette"
    COLOR_STYLE = 8510, "Color Style", "Color Style", "fas fa-palette"
    FONT_STYLE = 8511, "Font Style", "Font Style", "fas fa-text"
    BORDER_STYLE = 8512, "Border Style", "Border Style", "fas fa-border-all"
    SHADOW_STYLE = 8513, "Shadow Style", "Shadow Style", "fas fa-shadow"
    GRADIENT_STYLE = 8514, "Gradient Style", "Gradient Style", "fas fa-gradient"
    TRANSITION_STYLE = 8515, "Transition Style", "Transition Style", "fas fa-transition"
    EFFECT_STYLE = 8516, "Effect Style", "Effect Style", "fas fa-effect"
    # ANIMATION, ...

    # canvas?
    # CANVAS/DRAWING, SHAPE, BRUSH, ...

    EMPTY = 9999

    @property
    def area(self) -> "NodeArea":
        return AREA_BY_NODE_TYPE[self]


@enum_(EnumType.STRUCT_TYPE)
class StructType(BuiltinEnum):
    # meta [10000-10200]
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

    # package [11000-11200]
    # APP, PLUGIN, ...

    # infra [11200-11400]
    # VAULT, CACHE, ENDPOINT, DEPLOYMENT, NETWORK, ...

    # type [11400-11600]
    TYPE = 11400
    TYPE_CONSTRAINT = 11401
    FILE_INFO = 11420
    SCHEDULE = 11440
    # SCHEMA, UNION, TAG, ...

    # data [11600-11800]
    TEXT = 11600, None, None, "fas fa-text"
    TEXT_LINE = 11601, None, None, "fas fa-text"
    TEXT_SPAN = 11602, None, None, "fas fa-text"
    CODE = 11610, None, None, "fas fa-code"
    ICON = 11430
    # ...

    # chat [11800-12000]
    # ...

    # plan [12000-12200]
    # ...

    # logic [12200-12400]
    EXPRESSION = 12200
    AGGREGATION_RESULT = 12201
    SELECTION = 12210
    SELECT_OPTIONS = 12220
    # TRIGGER, TIMER, BREAKPOINT, ...

    # qa [12400-12600]
    # ...

    # runtime [12600-12800]
    ERROR = 12600
    RUN_TRACE = 12603
    RUN_FRAME = 12604
    # EVENT, SIGNAL, ...

    # identity [12800-13000]
    # PROFILE? (for User, or maybe global?)

    # access [13000-13200]
    POLICY = 13000
    POLICY_RULE = 13001
    POLICY_SUBJECT = 13002
    ACCESS_ZONE = 13003
    ACCESS_MATRIX = 13004
    ACCESS = 13005
    # ...

    # version [13200-13400]
    # ...

    # publish [13400-13600]
    # ...

    # analytics [13600-13800]
    # ...

    # locale [13800-14000]
    # ...

    # model [14000-14200]
    # ...

    # finance [14200-14400]
    # ...

    # web [14400-14600]
    # ...

    # world [14600-14800]
    # ...

    # view [18000-19000]

    # space [18000-18100]
    # ...

    # container views [18100-18200]
    # ...

    # content views [18200-18400]
    # ...

    # node views [18400-18500]
    # ...

    # style [18500-18600]
    COLOR = 18500, None, None, "fas fa-palette"
    SHADOW = 18503, None, None, "fas fa-shadow"
    BORDER = 18504, None, None, "fas fa-border-all"
    FONT = 18505, None, None, "fas fa-text"
    VECTOR2 = 18506, None, None, "fas fa-vector-square"
    VECTOR3 = 18507, None, None, "fas fa-vector-square"
    VECTOR4 = 18508, None, None, "fas fa-vector-square"
    GRADIENT_STOP = 18509, None, None, "fas fa-gradient"
    GRADIENT = 18510, None, None, "fas fa-gradient"
    FILL = 18511, None, None, "fas fa-fill"
    LENGTH = 18512, None, None, "fas fa-length"
    POSITION = 18513, None, None, "fas fa-position"
    DIMENSION = 18514, None, None, "fas fa-dimension"
    TRANSITION = 18515, None, None, "fas fa-transition"
    EFFECT = 18516, None, None, "fas fa-effect"
    GRID = 18518, None, None, "fas fa-grid"
    GRID_SPAN = 18519, None, None, "fas fa-grid"
    INSETS = 18520, None, None, "fas fa-padding"
    CORNERS = 18521, None, None, "fas fa-corners"
    AXIS_2 = 18522, None, None, "fas fa-gap"
    AXIS_3 = 18523, None, None, "fas fa-rotation"
    # ANIMATION, ...

    # canvas [18600-18800]
    # CANVAS/DRAWING, SHAPE, BRUSH, ...


@enum_(EnumType.NODE_AREA)
class NodeArea(BuiltinEnum):
    GLOBAL = 1
    REGIONAL = 2
    LOCAL = 3


@enum_(EnumType.NODE_MODE)
class NodeMode(BuiltinEnum):
    KERNEL = 3, "Kernel", "Managed by Bench (hidden)", "fas fa-cog"
    SYSTEM = 6, "System", "Managed by Bench", "fas fa-cog"
    BUILTIN = 10, "Builtin", "Provided by Bench", "fas fa-cog"
    MAIN = 20, "Main", "Active and available", "fas fa-globe"
    TEST = 30, "Test", "Active in test", "fas fa-flask"
    TEMPLATE = 40, "Template", "Template to use", "fas fa-puzzle-piece"
    ARCHIVE = 50, "Archive", "Inactive and hidden", "fas fa-box-archive"


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


ENUM_TYPES: bittuple[EnumType] = bittuple(*EnumType)
ENUM_TYPES_SET: frozenset[EnumType] = frozenset(ENUM_TYPES)

NODE_TYPES = bittuple(*NodeType)
NODE_TYPES_SET: frozenset[NodeType] = frozenset(NODE_TYPES)

GLOBAL_NODE_TYPES = _get_node_types(None, 1000)
LOCAL_NODE_TYPES = bittuple(NodeType.RECORD)
REGIONAL_NODE_TYPES = _get_node_types(1000, 10000) - LOCAL_NODE_TYPES
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
RESOURCE_NODE_TYPES = bittuple(
    NodeType.DATABASE, NodeType.COMPUTER, NodeType.SCALER, NodeType.FILE, NodeType.LINK
)
PROVISIONABLE_RESOURCE_NODE_TYPES = bittuple(NodeType.SCALER, NodeType.DATABASE, NodeType.COMPUTER)
COMMUNICATION_NODE_TYPES = _get_node_types(5500, 5600)
RUNTIME_NODE_TYPES = _get_node_types(2400, 2500)
PACKAGE_NODE_TYPES = _get_node_types(1000, 9000)
BENCH_NODE_TYPES = _get_node_types(
    1000,
    10000,
    NodeType.BENCH,
    NodeType.PACKAGE,
    NodeType.HANDLE,
    NodeType.MEMBERSHIP,
    NodeType.INVITE,
    NodeType.CLIENT,
)
PUBLIC_NODE_TYPES = bittuple(NodeType.USER, NodeType.ORGANIZATION, NodeType.BENCH)
# NOTE: these traits should also be in trait.py but we need the constants in property.py
#  (which also depends on trait.py, and we can't have a circular dependency)
BASED_NODE_TYPES = bittuple(NodeType.RECORD, NodeType.MESSAGE, NodeType.RUN)
VIEW_NODE_TYPES = bittuple(*(n for n in NODE_TYPES if n.name.endswith("VIEW")))
STYLE_NODE_TYPES = bittuple(*(n for n in NODE_TYPES if n.name.endswith("STYLE")))
PAGE_NODE_TYPES = bittuple(
    *RESOURCE_NODE_TYPES,
    *VIEW_NODE_TYPES,
    *STYLE_NODE_TYPES,
    NodeType.CHOICE,
    NodeType.CLASS,
    NodeType.TABLE,
    NodeType.FLOW,
    NodeType.SERVICE,
    NodeType.PAGE,
    NodeType.ROLE,
    NodeType.TASK,
    NodeType.THREAD,
    NodeType.CHANNEL,
    NodeType.TEAM,
    NodeType.AGENT,
    NodeType.THEME,
)
INSTANTIABLE_NODE_TYPES = bittuple(
    *RESOURCE_NODE_TYPES,
    NodeType.ACTION,
    NodeType.FIELD,
    NodeType.OPTION,
    NodeType.TABLE,
    NodeType.TASK,
    NodeType.THREAD,
    NodeType.CLAIM,
    NodeType.CHANNEL,
    NodeType.TEAM,
    NodeType.AGENT,
    NodeType.MEMBERSHIP,
)
TEMPLATABLE_NODE_TYPES = bittuple(
    *RESOURCE_NODE_TYPES,
    *INSTANTIABLE_NODE_TYPES,
    *VIEW_NODE_TYPES,
    *STYLE_NODE_TYPES,
    NodeType.PACKAGE,
    NodeType.DEPENDENCY,
    NodeType.PAGE,
    NodeType.BLOCK,
    NodeType.CHOICE,
    NodeType.CLASS,
    NodeType.FIELD,
    NodeType.OPTION,
    NodeType.SERVICE,
    NodeType.ACTION,
    NodeType.FLOW,
    NodeType.FLOW_EDGE,
    NodeType.TABLE,
    NodeType.CHANNEL,
    NodeType.ROLE,
    NodeType.SPACE,
)


# TODO :Broken: :Performance: we load too much and too coarsely :NodeOverload :RichGraph
UNLOADED_RESOURCE_NODE_TYPES = bittuple(NodeType.FILE)
LOADED_PACKAGE_NODE_TYPES = bittuple(
    *(RESOURCE_NODE_TYPES - UNLOADED_RESOURCE_NODE_TYPES),
    *VIEW_NODE_TYPES,
    NodeType.PACKAGE,
    NodeType.DEPENDENCY,
    NodeType.PAGE,
    NodeType.BLOCK,
    NodeType.CHOICE,
    NodeType.CLASS,
    NodeType.FIELD,
    NodeType.OPTION,
    NodeType.SERVICE,
    NodeType.ACTION,
    NodeType.FLOW,
    NodeType.FLOW_EDGE,
    NodeType.TABLE,
    NodeType.CHANNEL,
    NodeType.TEAM,
    NodeType.MEMBERSHIP,
    NodeType.ROLE,
    NodeType.AGENT,
    NodeType.TASK,
    NodeType.CLAIM,
    NodeType.SPACE,
)


# automatically included descendants :AutoLoading :RichGraph
AUTOLOAD_DESCENDANT_TYPES: dict[NodeType, tuple[NodeType, ...]] = {
    NodeType.THREAD: (
        NodeType.FILE,
        NodeType.MEMBERSHIP,
        NodeType.CLAIM,
        NodeType.AGENT,
        NodeType.CURSOR,
    ),
}


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
    # APPEND, REMOVE, ...

    # math
    # ADD, SUBTRACT, ...

    # text
    # ...


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


@enum_(EnumType.SEVERITY)
class Severity(BuiltinEnum):
    TRACE = 1, None, None, "fas fa-bug"
    DEBUG = 2, None, None, "fas fa-bug"
    INFO = 3, None, None, "fas fa-circle-check"
    WARNING = 4, None, None, "fas fa-circle-exclamation"
    ERROR = 5, None, None, "fas fa-circle-exclamation"
    PANIC = 6, None, None, "fas fa-skull"


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
    VARIABLE = 20, "Variable", "Variable", "fas fa-arrow-down"
    INPUT = 30, "Input to a runnable", "fas fa-arrow-down"
    OUTPUT = 40, "Output from a runnable", "fas fa-arrow-up"


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


@enum_(EnumType.ERROR_KIND)
class ErrorKind(BuiltinEnum):
    INTERNAL = 1
    RUNTIME = 5


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
        span = Span(
            type=type,
            nodes=nodes or [],
            title=title,
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

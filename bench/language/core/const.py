import contextvars
import enum
import functools
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
VERSION = "2025.05.28.0"
UUID_NAMESPACE = uuid5(UUID(int=0), b"bench")
CK_LENGTH_B64 = 24  # 1.5 * CK_LENGTH_BYTES (must be integer)
FLOAT_EPSILON = 1e-6
BEGINNING_OF_TIME = datetime.fromisoformat("1970-01-01T00:00:00+00:00")

# builtin benches :Builtins
BENCH_SLUG = "bench"
BENCH_ID = UUID("11111111-1111-1111-1111-000000000000")
BENCH_BENCH_PACKAGE_ID = UUID("11111111-1111-1111-1111-000000000001")
SYSTEM_SLUG = "system"
SYSTEM_ID = UUID("22222222-2222-2222-2222-000000000000")
SYSTEM_SYSTEM_PACKAGE_ID = UUID("22222222-2222-2222-2222-000000000001")

# runtime constants
NONCE = uuid4()
UNSET = cast(Any, _Unset())
EMPTY_LIST: list = []
EMPTY_SET: frozenset = frozenset()
EMPTY_DICT: dict[Any, Any] = frozendict()
IS_IN_USER_CODE = contextvars.ContextVar("is_in_user_code", default=False)
ACTIVE_SESSION: contextvars.ContextVar[Optional["Session"]] = contextvars.ContextVar(
    "active_session", default=None
)

if not IS_TEST:
    DEFAULT_WAIT_TIMEOUT = timedelta(seconds=30)
    DEFAULT_RESOURCE_TIMEOUT = timedelta(seconds=30)
else:
    DEFAULT_WAIT_TIMEOUT = timedelta(seconds=5)
    DEFAULT_RESOURCE_TIMEOUT = timedelta(seconds=5)


def active_session() -> "Session":
    """Gets the currently active Session (error if none)."""
    session = ACTIVE_SESSION.get()
    assert session is not None, "no active session"
    return session


def get_active_session() -> Optional["Session"]:
    """Gets the currently active Session (if any)."""
    return ACTIVE_SESSION.get()


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
    TRAIT_TYPE = 40004
    NODE_MODE = 40005
    NODE_AREA = 40006
    USER_STATUS = 40010
    ORGANIZATION_STATUS = 40011
    BENCH_STATUS = 40056
    PACKAGE_TYPE = 40050
    ERROR_TYPE = 40061
    VARIABLE_TYPE = 40063
    EDIT_TYPE = 40070
    UPDATE_TYPE = 40071
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
    QUERY_TYPE = 40811

    # auth [40200-40600]
    BENCH_ROLE_TYPE = 40200
    ORGANIZATION_ROLE_TYPE = 40210
    PACKAGE_ROLE_TYPE = 40220
    # ...

    # space [40600-40800]
    SPACE_TYPE = 40600
    BLOCK_TYPE = 40610
    # ...

    # history [40800-41000]
    # ...

    # infra [41000-41200]
    CLOUD = 40051
    REGION = 40052
    AREA = 40054
    CONTINENT = 40055
    MACHINE_TYPE = 41010
    DATABASE_TYPE = 41020
    CLIENT_TYPE = 41021
    # ...

    # logic [41200-41600]
    ACTION_CARDINALITY = 41220
    FLOW_TYPE = 41240
    FLOW_EDGE_TYPE = 41241
    CURSOR_TYPE = 41320
    CURSOR_STATUS = 41321
    # ...

    # runtime [41600-42000]
    PROCESS_STATUS = 41610
    RUN_TYPE = 41611
    SPAN_TYPE = 41620
    SCHEDULE_FREQUENCY = 41630
    INTERRUPTION_TYPE = 41631
    INTERRUPTION_STATUS = 41632
    INTERRUPTION_RESPONSE = 41633
    # ...

    # data [42000-42400]
    TEXT_LINE_TYPE = 42000
    TEXT_SPAN_TYPE = 42001
    CODE_TYPE = 42010
    FILE_RETENTION_MODE = 42020
    FILE_SOURCE = 42021
    FILE_TYPE = 42022
    FILE_FORMAT = 42023
    ICON_TYPE = 42030
    LINK_TYPE = 42050
    PRIMITIVE_TYPE = 42100
    TYPE_CARDINALITY = 42101
    SCALAR_TYPE = 42102
    DEFAULT_FACTORY = 42103
    STRING_FORMAT = 42110
    NUMBER_FORMAT = 42111
    FIELD_TYPE = 42120
    EDGE_TYPE = 42121
    CASCADE_ACTION = 42122
    DAY = 42130
    MONTH = 42131
    TIME_INTERVAL = 42132
    RESOURCE_STATUS = 42200
    # ...

    # social [42400-42800]
    THREAD_STATUS = 42401
    MESSAGE_TYPE = 42420
    # ...

    # product [42800-43200]
    # ...

    # finance [43200-43600]
    # ...

    # locale [43600-44000]
    # ...

    # web [44000-44200]
    # ...

    # world [44200-44400]
    # ...

    # model [44400-44600]
    MODEL_DEVELOPER = 44400
    MODEL_PROVIDER = 44401

    # ui [48000-50000]

    # space [48000-48100]
    # ...

    # container views [48100-48200]
    # ...

    # content views [48200-48300]
    # ...

    # input views [48300-48400]
    # ...

    # node views [48400-48500]
    # ...

    # internal views [48500-48600]
    # ...

    # style [49000-49100]
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
    RELATION_REFERENCE = 20106
    ATTRIBUTE_REFERENCE = 20107
    QUERY = 20110
    QUERY_RESULT = 20111
    QUERY_UPDATE = 20112

    # auth [20200-20600]
    # PROFILE? (for User, or maybe global?)

    # space [20600-20800]
    # ...

    # history [20800-21000]
    # ...

    # infra [21000-21200]
    # VAULT, CACHE, ENDPOINT, DEPLOYMENT, NETWORK, ...

    # logic [21200-21600]
    SCHEDULE = 21200
    # ...

    # runtime [21600-22000]
    ERROR = 21600
    # EVENT, SIGNAL, ...

    # data [22000-22400]
    TYPE = 22000
    NUMBER_CONSTRAINT = 22001
    STRING_CONSTRAINT = 22002
    COLLECTION_CONSTRAINT = 22003
    NODE_CONSTRAINT = 22004

    VALUE = 22010
    # SCHEMA, UNION, TAG, ...
    TEXT = 22100, None, None, "fas fa-text"
    TEXT_LINE = 22101, None, None, "fas fa-text"
    TEXT_SPAN = 22102, None, None, "fas fa-text"
    CODE = 22110, None, None, "fas fa-code"
    ICON = 22130
    SELECTION = 22070
    # ...

    # social [22400-22800]
    # ...

    # product [22800-23200]
    # ...

    # finance [23200-23600]
    # ...

    # locale [23600-24000]
    # ...

    # web [24000-24200]
    # ...

    # world [24200-24400]
    # ...

    # ui [28000-30000]

    # space [28000-28100]
    # ...

    # container views [28100-28200]
    # ...

    # content views [28200-28300]
    # ...

    # input views [28300-28400]
    # ...

    # node views [28400-28500]
    # ...

    # internal views [28500-28600]
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

    # canvas/drawing?
    # CANVAS, BRUSH, SHAPE, ...

    # audio/media?
    # SOUND, ...?


@enum_(EnumType.NODE_TYPE)
class NodeType(BuiltinEnum):
    # bench [1-200]
    BENCH = 1, "Bench", "Universal workbench", "https://heybench.com/favicon.ico"
    BENCH_MEMBERSHIP = 2, "Bench Membership", "Membership in a Bench", "fas fa-user-group"
    BENCH_INVITE = 3, "Bench Invite", "Invite to a Bench", "fas fa-user-plus"
    PACKAGE = 20, "Package", "Isolated sub-Bench", "fas fa-box-open"
    PACKAGE_MEMBERSHIP = 21, "Package Membership", "Membership in a Package", "fas fa-user-group"
    PACKAGE_INVITE = 22, "Package Invite", "Invite to a Package", "fas fa-user-plus"
    HANDLE = 50, "Handle", "Unique @handle", "fas fa-at"
    # DEPENDENCY, PLUGIN, ...

    # auth [200-600]
    USER = 200, "User", "User", "fas fa-user"
    CLIENT = 210, "Client", "Client to a Bench", "fas fa-desktop"
    ORGANIZATION = 300, "Organization", "Organization", "fas fa-building"
    ORGANIZATION_MEMBERSHIP = (
        301,
        "Organization Membership",
        "Membership in an Organization",
        "fas fa-user-group",
    )
    ORGANIZATION_INVITE = (
        302,
        "Organization Invite",
        "Invite to an Organization",
        "fas fa-user-plus",
    )
    # TEAM, ...
    # CREDENTIAL, ACCOUNT, PROFILE, ...
    # PERMISSION, PERMISSION_GROUP, ...
    # CHALLENGE, FRIENDSHIP, ENTITLEMENT, POLICY, RULE, KICK/BAN, ...

    # space [600-800]
    SPACE = 600, "Space", "Space", "fas fa-galaxy"
    SCENE = 610, "Scene", "Scene of an Application", "fas fa-masks-theater"
    ROUTE = 620, "Route", "Route to a Scene", "fas fa-route"
    # COMMAND, OVERLAY, WIDGET, ...
    PAGE = 700, "Page", "Page of Blocks", "far fa-file"
    BLOCK = 701, "Block", "Rich Block on a Page", "fas fa-cube"

    # history [800-1000]
    # CHANGE, HISTORY, OVERLAY, BRANCH, ...

    # infra [1000-1200]
    DATABASE = 1000, "Store", "Store custom data", "fas fa-database"
    MACHINE = 1010, "Machine", "Machine for computing", "fas fa-machine-classic"
    # VAULT, CACHE, ENDPOINT, DEPLOYMENT, NETWORK, AUTOSCALER, ...

    # logic [1200-1600]
    SERVICE = 1200, "Service", "Service", "fas fa-screwdriver-wrench"
    ACTION = 1220, "Action", "Action", "fas fa-step-forward"
    FLOW = 1240, "Flow", "Sequence Actions", "fas fa-diagram-project"
    FLOW_EDGE = 1241, "Flow Edge", "Edge between Actions", "fas fa-link"
    AGENT = 1300, "Agent", "Identity for an AI", "fas fa-robot"
    TASK = 1310, "Task", "To-do item", "far fa-square-check"
    CURSOR = 1320, "Cursor", "Position in something", "fas fa-mouse"
    # ROOM, JOB, PLAN, LOCK, ...
    # TRAIT/INTERFACE, ...
    # TRIGGER, TIMER, BREAKPOINT, ...
    # TEST, TEST_SUITE, TEST_CASE, TEST_RESULT, ...

    # runtime [1600-2000]
    RUN = 1610, "Run", "Run", "fas fa-play"
    SPAN = 1620, "Span", "Span", "fas fa-ruler-horizontal"
    INTERRUPTION = 1630, "Interruption", "Interruption", "fas fa-hand"
    # EVENT, SIGNAL, TRACE, LOG, ...

    # data [2000-2400]
    SCHEMA = 2000, "Schema", "Schema", "fas fa-shapes"
    FIELD = 2010, "Field", "Field", "fas fa-triangle"
    FILE = 2020, "File", "File", "fas fa-file"
    LINK = 2050, "Link", "Link to something", "fas fa-link"
    # STREAM, SECRET, ...
    TABLE = 2200, "Table", "Table of Records", "fas fa-table"
    RECORD = 2210, "Record", "Record in a Database", "fas fa-database"
    # INDEX, CONSTRAINT, MIGRATION, ...

    # social [2400-2800]
    THREAD = 2400, "Thread", "Thread", "fas fa-reel"
    MESSAGE = 2420, "Message", "Message", "fas fa-message"
    # POLL, STAR, VOTE, REVIEW, RATING, REACTION, ...
    # FOLLOW, FEED, FEED_ITEM, ...
    # CHANNEL, NOTIFICATION, ...
    # ACHIEVEMENT, WISHLIST/WATCHLIST, ...

    # product [2800-3200]
    # PREVIEW, RELEASE, ROLLOUT, ...
    # METER, METRIC, RECORDING/REPLAY, SURVEY, ...
    # TOUR, FUNNEL, COHORT, JOURNEY, ..
    # SEGMENT, EXPERIMENT, FEATURE, FEATURE_FLAG, FEATURE_GATE, ...

    # finance [3200-3600]
    # WALLET, BALANCE, BUDGET, TRANSFER, CREDIT, ...
    # TIER, SUBSCRIPTION, PRODUCT, PRICE, ORDER, INVOICE, DISCOUNT, DISPUTE, REFUND, ...

    # locale [3600-4000]
    # LOCALE, STRING, TRANSLATION, ...

    # web [4000-4200]
    # DOMAIN, ...
    # EMAIL, EMAIL_ATTEMPT, ...

    # world [4200-4400]
    # PHONE, ADDRESS, ...

    # ui [8000-10000]

    # space [8000-8100]

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


@enum_(EnumType.TRAIT_TYPE)
class TraitType(BuiltinEnum):
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
    NODE_TYPE = 33, "NodeType", "Is a NodeType", "fas fa-node"
    NODE_INSTANCE = 34, "NodeInstance", "Is a NodeInstance", "fas fa-node"
    IN_BENCH = 40, "Bench", "In a Bench", "fas fa-bench"
    IN_PACKAGE = 41, "Package", "In a Package", "fas fa-box"
    REGIONAL = 50, "Regional", "Is regional", "fas fa-globe"
    RESOURCE = 51, "Resource", "Is a Resource"
    PROVISIONABLE = 52, "Provisionable", "Can be provisioned", "fas fa-server"
    # TAG, TAGGABLE, ...

    # auth [200-600]
    OWNABLE = 200, "Ownable", "Can be owned", "fas fa-user"
    JOINABLE = 202, "Joinable", "Can be joined", "fas fa-users"
    SUBJECT = 205, "Subject", "Is a Subject", "fas fa-user"
    MEMBERSHIP = 210, "Membership", "Is a Membership", "fas fa-users"
    INVITE = 211, "Invite", "Is an Invite", "fas fa-envelope"
    ROLE = 212, "Role", "Is a Role", "fas fa-user-tag"

    # space [600-800]
    BLOCKABLE = 630, "Block", "Can be a Block on a Page", "fas fa-cube"

    # history [800-1000]
    # ...

    # infra [1000-1200]
    # ...

    # logic [1200-1600]
    RUNNABLE = 1200, "Runnable", "Can be run", "fas fa-play"
    PROCESSABLE = 1201, "Processable", "Can be processed", "fas fa-cogs"
    COMPUTABLE = 1202, "Computable", "Can be computed", "fas fa-calculator"

    # runtime [1600-2000]
    # ...

    # data [2000-2400]
    # ...

    # social [2400-2800]
    # MESSAGE, THREAD, ...
    # STARRABLE, RATEABLE, REACTABLE, VOTABLE, ..
    # ASSIGNABLE, MESSAGEABLE, CLOSABLE, LOCKABLE, ...

    # product [2800-3200]
    # ...

    # finance [3200-3600]
    # ...

    # locale [3600-4000]
    # ...

    # web [4000-4200]
    # ...

    # world [4200-4400]
    # ...

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


@enum_(EnumType.EDGE_TYPE)
class EdgeType(BuiltinEnum):
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
    int: PrimitiveType.INT64,
    Decimal: PrimitiveType.DECIMAL,
    float: PrimitiveType.FLOAT64,
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
    MACHINE = 10


CLOUD = get_from_env("CLOUD", typ=Cloud, description="Cloud we're running in")
REGION = get_from_env("REGION", typ=Region, description="Region we're running in")
TRACING = get_from_env("TRACING", typ=bool, description="Enable tracing")


class BenchError(Exception):
    """Common base class for any regular errors."""

    pass


@_agnosticcontextmanager
def capture_span(
    tracer: Tracer,
    key: str,
    type: "SpanType",
    *,
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

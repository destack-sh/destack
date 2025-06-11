import contextvars
import enum
import functools
import typing
from collections.abc import Collection, Iterable
from datetime import date, datetime, time, timedelta
from decimal import Decimal
from typing import (
    TYPE_CHECKING,
    Any,
    Optional,
    TypeVar,
    Union,
    cast,
)

from bitarray import bitarray
from fastuuid import UUID, uuid4, uuid5
from more_itertools import first

from destack.utils.env import IS_TEST, get_from_env
from destack.utils.frozen import frozendict
from destack.utils.string import Casing, to_casing

if TYPE_CHECKING:
    from destack.language import (
        Session,
    )


class _Unset:
    def __repr__(self):
        return "<UNSET!>"

    def __str__(self):
        return "<UNSET!>"


# forever constants
VERSION = "2025.06.10.0"
UUID_NAMESPACE = uuid5(UUID(int=0), b"destack")
CK_LENGTH_B64 = 24  # 1.5 * CK_LENGTH_BYTES (must be integer)
FLOAT_EPSILON = 1e-6
BEGINNING_OF_TIME = datetime.fromisoformat("1970-01-01T00:00:00+00:00")

# builtin destackes :Builtins
DESTACK_SLUG = "destack"
DESTACK_ID = UUID("11111111-1111-1111-1111-000000000000")

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
    def camel_name(self):
        from destack.utils.string import Casing, to_casing

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
_ENUM_TYPE_BY_CLASS: dict[type[BuiltinEnum], "EnumType"] = {}

BuiltinEnumT = typing.TypeVar("BuiltinEnumT", bound=BuiltinEnum)


def enum_(enum_type: "EnumType"):
    """Register a Destack enum."""

    def register_enum(cls: type[BuiltinEnumT]) -> type[BuiltinEnumT]:
        if enum_type in _ENUM_CLASS_BY_TYPE:
            raise ValueError(f"enum {enum_type} duplicate: {_ENUM_CLASS_BY_TYPE[enum_type]}")
        _ENUM_CLASS_BY_TYPE[enum_type] = cls
        _ENUM_TYPE_BY_CLASS[cls] = enum_type
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
        assert isinstance(enum_cls, type) and issubclass(enum_cls, BuiltinEnum), (
            f"invalid bittuple {enum_cls}: {items}"
        )
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
        assert self.enum_cls == other.enum_cls, (
            f"invalid enum_cls: {self.enum_cls} != {other.enum_cls}"
        )
        combined = self.bits & other.bits
        ordered_members = _get_enum_members_by_ord(self.enum_cls)
        items = tuple(ordered_members[o] for o in combined.search(True))
        return bittuple(items, enum_cls=self.enum_cls)  # type: ignore

    def __or__(self, other: "bittuple[EnumT]") -> "bittuple[EnumT]":
        assert type(other) is bittuple, f"invalid type: {type(other)}"
        assert self.enum_cls == other.enum_cls, (
            f"invalid enum_cls: {self.enum_cls} != {other.enum_cls}"
        )
        combined = self.bits | other.bits
        ordered_members = _get_enum_members_by_ord(self.enum_cls)
        items = tuple(ordered_members[o] for o in combined.search(True))
        return bittuple(items, enum_cls=self.enum_cls)  # type: ignore

    def __sub__(self, other: "bittuple[EnumT]") -> "bittuple[EnumT]":
        assert type(other) is bittuple, f"invalid type: {type(other)}"
        assert self.enum_cls == other.enum_cls, (
            f"invalid enum_cls: {self.enum_cls} != {other.enum_cls}"
        )
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
    # destack [1-200]
    ENUM_TYPE = 1
    NODE_TYPE = 2
    STRUCT_TYPE = 3
    TRAIT_TYPE = 5
    ENVIRONMENT_TYPE = 6
    AREA_TYPE = 7
    RUNTIME_TYPE = 8
    RUNTIME_LANGUAGE = 9
    PROPERTY_REFERENCE_TYPE = 10
    USER_STATUS = 11
    ORGANIZATION_STATUS = 12
    SPACE_STATUS = 57
    FOLDER_TYPE = 51
    ERROR_TYPE = 62
    VARIABLE_TYPE = 64
    EDIT_TYPE = 71
    EDIT_OPERATION = 72
    CHANGE_STATUS = 76
    # query
    CONDITIONAL_TYPE = 103
    AGGREGATION_TYPE = 104
    SORT_MODE = 105
    SORT_TYPE = 106
    JOIN_TYPE = 107
    FUNCTION_TYPE = 108
    EXPRESSION_TYPE = 109
    RELATION_TYPE = 110
    ATTRIBUTE_TYPE = 111
    QUERY_TYPE = 812
    QUERY_UPDATE_TYPE = 813

    # access [400-600]
    SPACE_ROLE_TYPE = 401
    ORGANIZATION_ROLE_TYPE = 411
    FOLDER_ROLE_TYPE = 421
    CLIENT_TYPE = 431
    # ...

    # folder [600-800]
    WINDOW_TYPE = 601
    # ...

    # history [800-1000]
    # ...

    # entity [1000-1400]
    # ...

    # data [1400-1800]
    TEXT_LINE_TYPE = 1401
    TEXT_SPAN_TYPE = 1402
    FILE_RETENTION_MODE = 1421
    FILE_SOURCE = 1422
    FILE_TYPE = 1423
    FILE_FORMAT = 1424
    ICON_TYPE = 1431
    LINK_TYPE = 1451
    PRIMITIVE_TYPE = 1501
    TYPE_CARDINALITY = 1502
    SCALAR_TYPE = 1503
    DEFAULT_FACTORY = 1504
    STRING_FORMAT = 1511
    NUMBER_FORMAT = 1512
    FIELD_TYPE = 1521
    EDGE_TYPE = 1522
    EDGE_DIRECTION = 1523
    CASCADE_ACTION = 1524
    DAY = 1531
    MONTH = 1532
    TIME_INTERVAL = 1533
    RESOURCE_STATUS = 1701
    # ...

    # logic [1800-2000]
    ACTION_CARDINALITY = 1821
    CURSOR_STATUS = 1922
    SCHEDULE_FREQUENCY = 1930
    TIMER_TYPE = 1931
    TRIGGER_TYPE = 1932
    # ...

    # test [2000-2400]
    # ...

    # runtime [2400-2800]
    RUN_STATUS = 2411
    RUN_TYPE = 2412
    INTERRUPTION_TYPE = 2432
    INTERRUPTION_STATUS = 2433
    INTERRUPTION_RESPONSE = 2434
    # ...

    # deployment [2800-3000]
    # ...

    # product [3000-3400]
    # ...

    # finance [3400-3800]
    # ...

    # social [3800-4200]
    THREAD_STATUS = 3802
    MESSAGE_TYPE = 3821
    # ...

    # locale [4200-4600]
    # ...

    # internet [4600-5000]
    # ...

    # infra [5000-5400]
    CLOUD = 5000
    REGION = 5001
    REGION_AREA = 5002
    REGION_CONTINENT = 5003
    TENANCY = 5004
    DATABASE_TYPE = 5010
    MACHINE_TYPE = 5020
    # SEARCH_TYPE, WAREHOUSE_TYPE, ...
    # ...

    # world [5400-5800]
    # ...

    # model [5800-6000]
    MODEL_DEVELOPER = 5801
    MODEL_PROVIDER = 5802

    # scene [8000-8100]
    # ...

    # media [8100-8200]
    # ...

    # container views [8200-8300]
    # ...

    # content views [8300-8400]
    # ...

    # input views [8400-8500]
    # ...

    # node views [8500-8600]
    # ...

    # internal views [8600-8700]
    # ...

    # style [9000-9200]
    POSITION_TYPE = 9001
    COLOR_TYPE = 9011
    COLOR_SHADE = 9012
    COLOR_HUE = 9013
    FONT_WEIGHT = 9022
    FONT_SIZE = 9023
    FONT_TYPE = 9024
    TEXT_ALIGN = 9025
    TEXT_DECORATION = 9026
    TEXT_TRANSFORM = 9027
    SHADOW_TYPE = 9031
    SHADOW_POSITION = 9032
    BORDER_TYPE = 9041
    GRADIENT_TYPE = 9051
    FILL_TYPE = 9061
    FILL_POSITION = 9062
    FILL_SIZE = 9063
    LENGTH_UNIT = 9071
    LAYOUT = 9072
    DISTRIBUTE = 9073
    ALIGN = 9074
    DIRECTION = 9075
    OVERFLOW = 9076
    TRANSITION_TYPE = 9077
    SPRING_TYPE = 9078
    DIMENSION_TYPE = 9079
    THEME_COLOR = 9080
    EFFECT_TYPE = 9081
    REPEAT_TYPE = 9082
    TEXT_SPLIT_TYPE = 9084
    OFFSCREEN_BEHAVIOR = 9085

    # canvas [9200-9400]
    # ...

    # animation [9400-9600]
    # ...


enum_(EnumType.ENUM_TYPE)(EnumType)


@enum_(EnumType.STRUCT_TYPE)
class StructType(BuiltinEnum):
    # destack [1-200]
    SCOPE = 1
    ORIGIN = 2
    NODE_REFERENCE = 3
    PROPERTY_REFERENCE = 5
    PROPERTY_INFO = 11
    TRAIT_INFO = 12
    NODE_INFO = 13
    STRUCT_INFO = 14
    ENUM_INFO = 15
    ENUM_OPTION_INFO = 16
    # METHOD_INFO, ...?
    EDIT = 21
    CHANGE = 22
    CHANGE_RESULT = 23
    EXPRESSION = 101
    FUNCTION = 102
    JOIN = 103
    AGGREGATION = 104
    CONDITION = 105
    SORT = 106
    SELECT = 107
    RELATION_REFERENCE = 108
    ATTRIBUTE_REFERENCE = 109
    QUERY = 111
    QUERY_RESULT = 112
    QUERY_RESULT_GROUP = 113
    QUERY_UPDATE = 116
    HISTOGRAM = 114
    VARIABLE = 121

    # access [400-600]
    # PROFILE? (for User, or maybe global?)

    # folder [600-800]
    # ...

    # history [800-1000]
    # ...

    # entity [1000-1400]
    # ...

    # data [1400-1800]
    VALUE = 1400
    TYPE = 1401
    NUMBER_CONSTRAINT = 1402
    STRING_CONSTRAINT = 1403
    COLLECTION_CONSTRAINT = 1404
    NODE_CONSTRAINT = 1405
    TEXT = 1421, None, None, "fas fa-text"
    TEXT_LINE = 1422, None, None, "fas fa-text"
    TEXT_SPAN = 1423, None, None, "fas fa-text"
    ICON = 1431
    SELECTION = 1471
    # SCHEMA, UNION, TAG, ...

    # logic [1800-2000]
    SCHEDULE = 1801
    # ...

    # test [2000-2400]
    # ...

    # runtime [2400-2800]
    ERROR = 2401
    # ...

    # deployment [2800-3000]
    # ...

    # product [3000-3400]
    # ...

    # finance [3400-3800]
    # ...

    # social [3800-4200]
    # ...

    # locale [4200-4600]
    # ...

    # internet [4600-5000]
    # ...

    # infra [5000-5400]
    DATABASE_INFO = 5001
    CELL_INFO = 5101

    # world [5400-5800]
    # ...

    # visual [8000-10000]
    VECTOR2 = 8000, None, None, "fas fa-vector-square"
    VECTOR3 = 8001, None, None, "fas fa-vector-square"
    VECTOR4 = 8002, None, None, "fas fa-vector-square"
    VECTOR2I = 8003, None, None, "fas fa-vector-square"
    VECTOR3I = 8004, None, None, "fas fa-vector-square"
    VECTOR4I = 8005, None, None, "fas fa-vector-square"
    AXIS2 = 8007, None, None, "fas fa-vector-square"
    AXIS3 = 8009, None, None, "fas fa-vector-square"

    # scene [8000-8100]
    # ...

    # media [8100-8200]
    # ...

    # container views [8200-8300]
    # ...

    # content views [8300-8400]
    # ...

    # input views [8400-8500]
    # ...

    # node views [8500-8600]
    # ...

    # internal views [8600-8700]
    # ...

    # style [9000-9200]
    COLOR = 9011, None, None, "fas fa-palette"
    SHADOW = 9012, None, None, "fas fa-eclipse"
    BORDER = 9013, None, None, "fas fa-border-outer"
    FONT = 9014, None, None, "fas fa-text"
    GRADIENT_STOP = 9015, None, None, "fas fa-gradient"
    GRADIENT = 9016, None, None, "fas fa-gradient"
    FILL = 9017, None, None, "fas fa-fill"
    LENGTH = 9018, None, None, "fas fa-ruler"
    POSITION = 9020, None, None, "fas fa-location-crosshair"
    DIMENSION = 9022, None, None, "fas fa-ruler"
    TRANSITION = 9024, None, None, "fas fa-bezier-curve"
    EFFECT = 9025, None, None, "fas fa-sparkle"
    GRID = 9026, None, None, "fas fa-grid-2"
    GRID_SPAN = 9028, None, None, "fas fa-grid-2"
    INSETS = 9030, None, None, "fas fa-corner"
    CORNERS = 9032, None, None, "fas fa-corner"

    # canvas [9200-9400]
    # CANVAS, BRUSH, SHAPE, ...

    # animation [9400-9600]
    # SOUND, ...?


@enum_(EnumType.NODE_TYPE)
class NodeType(BuiltinEnum):
    # space [1-400]
    SPACE = 1, "Space", "Universal Space", "https://heydestack.com/favicon.ico"
    SPACE_MEMBERSHIP = 2, "Space Membership", "Membership in a Space", "fas fa-user-group"
    SPACE_INVITE = 3, "Space Invite", "Invite to a Space", "fas fa-user-plus"
    HANDLE = 10, "Handle", "Unique @handle", "fas fa-at"
    USER = 20, "User", "User", "fas fa-user"
    FRIENDSHIP = 30, "Friendship", "Friendship between two Users", "fas fa-user-friends"
    FRIENDSHIP_INVITE = (
        31,
        "Friendship Invite",
        "Invite to be friends with another User",
        "fas fa-user-plus",
    )
    ORGANIZATION = 40, "Organization", "Organization", "fas fa-building"
    ORGANIZATION_MEMBERSHIP = (
        41,
        "Organization Membership",
        "Membership in an Organization",
        "fas fa-user-group",
    )
    ORGANIZATION_INVITE = (
        42,
        "Organization Invite",
        "Invite to an Organization",
        "fas fa-user-plus",
    )
    CLIENT = 50, "Client", "Client to a Destack", "fas fa-desktop"
    # TEAM, TEAM_MEMBERSHIP, TEAM_INVITE, ...
    # CREDENTIAL, ACCOUNT, PROFILE, ...

    # access [400-600]
    # PERMISSION, PERMISSION_GROUP, ...
    # POLICY, RULE, ...
    # CHALLENGE, ENTITLEMENT,
    # KICK/BAN, ...

    # folder [600-800]
    FOLDER = 600, "Folder", "Sub-space of a Space", "fas fa-folder-open"
    FOLDER_MEMBERSHIP = 601, "Folder Membership", "Membership in a Folder", "fas fa-user-group"
    FOLDER_INVITE = 602, "Folder Invite", "Invite to a Folder", "fas fa-user-plus"
    # DEPENDENCY, ...
    # TAG/TAGGING, ...

    # history [800-1000]
    # HISTORY, SNAPSHOT/SAVEPOINT, OVERLAY, BRANCH, ...

    # entity [1000-1400]
    CUSTOM_ENTITY_DEFINITION = (
        1000,
        "Custom Node Definition",
        "Custom Node Definition",
        "fas fa-table",
    )
    CUSTOM_ENTITY = 1001, "Custom Node Instance", "Custom Node Instance", "fas fa-database"
    # INDEX, CONSTRAINT, MIGRATION, ...
    # SYNC, ...
    # TRAIT/INTERFACE/CUSTOM_TRAIT, ...

    # data [1400-1800]
    SCHEMA = 1400, "Schema", "Schema", "fas fa-shapes"
    FIELD = 1401, "Field", "Field", "fas fa-triangle"
    FILE = 1410, "File", "File", "fas fa-file"
    LINK = 1411, "Link", "Link to something", "fas fa-link"
    # STREAM, SECRET, ...

    # logic [1800-2000]
    SCRIPT = 1800, "Script", "Script", "fas fa-code"
    SERVICE = 1810, "Service", "Service", "fas fa-screwdriver-wrench"
    ACTION = 1820, "Action", "Action", "fas fa-step-forward"
    ROUTE = 1830, "Route", "Route", "fas fa-route"
    TRIGGER = 1840, "Trigger", "Trigger", "fas fa-bolt"
    TIMER = 1841, "Timer", "Timer", "fas fa-clock"
    # BREAKPOINT, ...
    SCREEN_CURSOR = 1900, "Mouse Cursor", "Mouse Cursor", "fas fa-mouse"
    QUERY_CURSOR = 1901, "Query Cursor", "Query Cursor", "fas fa-magnifying-glass"
    # WEB_CURSOR, ...
    # ROOM, CHANNEL, LOCK, ...
    # TASK, ...

    # test [2000-2400]
    # TEST, TEST_SUITE, TEST_CASE, TEST_RESULT, ...

    # runtime [2400-2800]
    RUN = 2400, "Run", "Run", "fas fa-play"
    # RUN_QUEUE = 2401, "Run Queue", "Run Queue", "fas fa-list-check"
    SPAN = 2410, "Span", "Span", "fas fa-ruler-horizontal"
    INTERRUPTION = 2420, "Interruption", "Interruption", "fas fa-hand"
    LOG = 2430, "Log", "Log", "fas fa-file-lines"
    # JOB, ...
    CUSTOM_EVENT_DEFINITION = (
        2500,
        "Custom Event Definition",
        "Custom Event Definition",
        "fas fa-signal",
    )
    CUSTOM_EVENT = 2501, "Custom Event", "Custom Event", "fas fa-signal"
    EDIT_EVENT = 2502, "Edit Event", "Edit Event", "fas fa-file-lines"
    CHANGE_EVENT = 2503, "Change Event", "Change Event", "fas fa-file-lines"
    QUERY_EVENT = 2504, "Query Event", "Query Event", "fas fa-file-lines"
    # ERROR_EVENT, TRIGGER_EVENT, RUN_EVENT, ...
    GAUGE_METRIC = 2600, "Gauge Metric", "Gauge Metric", "fas fa-gauge"
    GAUGE_MEASUREMENT = 2601, "Gauge Measurement", "Gauge Measurement", "fas fa-gauge"
    COUNTER_METRIC = 2602, "Counter Metric", "Counter Metric", "fas fa-gauge"
    COUNTER_MEASUREMENT = 2603, "Counter Measurement", "Counter Measurement", "fas fa-gauge"
    HISTOGRAM_METRIC = 2604, "Histogram Metric", "Histogram Metric", "fas fa-gauge"
    HISTOGRAM_MEASUREMENT = 2605, "Histogram Measurement", "Histogram Measurement", "fas fa-gauge"

    # deployment [2600-3000]
    # DEPLOYMENT, ...
    # PREVIEW, RELEASE, ROLLOUT, ...
    # INCIDENT, ESCALATION, ...

    # product [3000-3400]
    # RECORDING/REPLAY, SURVEY, ...
    # ONBOARDING, TOUR, FUNNEL, COHORT, JOURNEY, ..
    # FEATURE, FEATURE_FLAG, FEATURE_GATE, ...
    # SEGMENT, EXPERIMENT, ...

    # finance [3400-3800]
    # WALLET, BALANCE, BUDGET, TRANSFER, CREDIT, ...
    # TIER, SUBSCRIPTION, PRODUCT, PRICE, ...
    # ORDER, INVOICE, DISCOUNT, DISPUTE, REFUND, ...

    # social [3800-4200]
    # nocheckin: figure out how to have global *and* in-space Stars/Follows/... (Traits?)
    #  (same goes for Files... and maybe Threads)
    THREAD = 3800, "Thread", "Thread", "fas fa-reel"
    # THREAD_MEMBERSHIP, ...
    MESSAGE = 3810, "Message", "Message", "fas fa-message"
    REACTION = 3820, "Reaction", "Reaction", "fas fa-heart"
    STAR = 3830, "Star", "Star", "fas fa-star"
    # FOLLOW, FEED, FEED_ITEM, ...
    # POLL, VOTE, REVIEW, RATING, RANK, ...
    # ACHIEVEMENT, BADGE, WISHLIST/WATCHLIST, ...
    NOTIFICATION = 3900, "Notification", "Notification", "fas fa-bell"

    # locale [4200-4600]
    # LOCALE, STRING, TRANSLATION, ...

    # internet [4600-5000]
    # DOMAIN, ...
    # EMAIL, EMAIL_ATTEMPT, ...

    # infra [5000-5400]
    DATABASE = 5000, "Database", "Database for Postgres data", "fas fa-database"
    # SEARCH/INDEX, VAULT, CACHE, S3, ...
    # CELL = 5010, "Cell", "Cell", "fas fa-cell"
    MACHINE = 5020, "Machine", "Machine for ephemeral computing", "fas fa-machine-classic"
    # HOST, ENDPOINT, DEPLOYMENT, NETWORK, AUTOSCALER, ...

    # world [5000-5400]
    # PHONE_NUMBER, ADDRESS, ...

    # scene [8000-8100]
    WINDOW = 8000, "Window", "Window", "fas fa-galaxy"
    SCENE = 8010, "Scene", "Scene of an Application", "fas fa-masks-theater"
    # COMMAND, MENU, OVERLAY, WIDGET, ...

    # media [8100-8200]
    # CAMERA, GESTURE, MICROPHONE, ...

    # container views [8200-8300]
    CUSTOM_VIEW_DEFINITION = (
        8200,
        "Custom View Definition",
        "Custom View Definition",
        "fas fa-table",
    )
    CUSTOM_VIEW = 8201, "Custom View", "Custom View", "fas fa-table"
    FRAME_VIEW = 8202, "Frame View", "Fixed Container", "fas fa-frame"
    LABEL_VIEW = 8203, "Label View", "Label Container", "fas fa-font-case"
    # FORM_VIEW, MENU_VIEW, ...
    SPLIT_VIEW = 8210, "Split View", "Split Container", "fas fa-columns"
    # SLOT_DEFINITION_VIEW = 8230, "Slot Definition View", "Slot Definition View", "fas fa-columns"
    # SLOT_VIEW = 8231, "Slot View", "Slot View", "fas fa-columns"
    # TAB_VIEW = 8221, "Tab Container View", "Tab Container", "fas fa-tabs"
    # DRAWER_VIEW, SPLIT_DRAWER_VIEW, GRID/GRID_ELEMENT_VIEW, ...

    # content views [8300-8400]
    TEXT_VIEW = 8300, "Text View", "Text", "fas fa-text"
    # CODE_VIEW, ICON_VIEW, IMAGE_VIEW, AUDIO_VIEW, VIDEO_VIEW, DOCUMENT_VIEW, ...

    # input views [8400-8500]
    NUMBER_INPUT_VIEW = 8400, "Number Input View", "Number Input", "fas fa-hashtag"
    SLIDER_INPUT_VIEW = 8401, "Slider Input View", "Slider Input", "fas fa-slider"
    # STRING_INPUT_VIEW, TOGGLE_INPUT_VIEW, PICKER_INPUT_VIEW, COLOR_INPUT_VIEW, ...
    # ICON_INPUT_VIEW, FILE_INPUT_VIEW, DATETIME_INPUT_VIEW, DURATION_INPUT_VIEW, ...

    # NOTE :Architecture: node and internal views should probably be defined in user space?
    # node views [8500-8600]
    THREAD_VIEW = 8530, "Thread View", "Thread", "fas fa-reel"
    # internal views [8600-8700]
    WIZARD_VIEW = 8600, "Wizard View", "Wizard", "fas fa-wand-sparkles"

    # style [9000-9200]
    THEME = 9000, "Theme", "Theme", "fas fa-palette"
    COLOR_STYLE = 9010, "Color Style", "Color Style", "fas fa-palette"
    FILL_STYLE = 9011, "Fill Style", "Fill Style", "fas fa-fill"
    FONT_STYLE = 9012, "Font Style", "Font Style", "fas fa-text"
    BORDER_STYLE = 9013, "Border Style", "Border Style", "fas fa-border-outer"
    SHADOW_STYLE = 9014, "Shadow Style", "Shadow Style", "fas fa-eclipse"
    GRADIENT_STYLE = 9015, "Gradient Style", "Gradient Style", "fas fa-gradient"
    TRANSITION_STYLE = 9016, "Transition Style", "Transition Style", "fas fa-bezier-curve"
    EFFECT_STYLE = 9017, "Effect Style", "Effect Style", "fas fa-sparkle"
    # BRUSH_STYLE, ...
    # SHADER, MATERIAL, ...

    # canvas [9200-9400]
    # CANVAS, SKETCH, LAYER, ...
    # BITMAP, ...
    # SHAPE, ...
    # ANNOTATION, ...

    # animation [9400-9600]
    # STAGE, ...
    # ANIMATION, TRACK, KEYFRAME, FRAME, ...
    # SOUND, ...


@enum_(EnumType.TRAIT_TYPE)
class TraitType(BuiltinEnum):
    # destack [1-200]
    # nocheckin: categorize Nodes (Entities/Particles/Assets/Views/Visuals/...?)
    # where
    GLOBAL = 1, "Global", "Is global", "fas fa-globe"
    IN_SPACE = 2, "Space", "Is in a Space", "fas fa-destack"
    IN_FOLDER = 3, "Folder", "Is in a Folder", "fas fa-folder-open"
    # what
    ENTITY = 10, "Entity", "Is an Entity", "fas fa-hexagon"
    PARTICLE = 11, "Particle", "Is a Particle", "fas fa-atom"
    ASSET = 12, "Asset", "Is an Asset", "fas fa-file"
    RESOURCE = 13, "Resource", "Is a Resource", "fas fa-server"
    EVENT = 14, "Event", "Is an Event", "fas fa-bolt"
    CUSTOM_NODE_DEFINITION = (
        18,
        "Custom Node Definition",
        "Is a Custom Node Definition",
        "fas fa-table",
    )
    CUSTOM_NODE = 19, "Custom Node", "Is a Custom Node", "fas fa-database"

    # behavior
    FROZEN = 20, "Frozen", "Is frozen", "fas fa-snowflake"
    TRACKED = 21, "Tracked", "Is tracked", "fas fa-clock"
    ARCHIVABLE = 22, "Archivable", "Can be archived", "fas fa-box-archive"
    DELETABLE = 23, "Deletable", "Can be deleted", "fas fa-trash"
    TEMPLATABLE = 24, "Templatable", "Is templatable", "fas fa-puzzle-piece"
    EXTENSIBLE = 25, "Extensible", "Is extensible", "fas fa-expand"
    ORDERED = 26, "Ordered", "Is ordered", "fas fa-sort"
    ENVIRONMENTAL = 27, "Environment", "Has an environment", "fas fa-window-maximize"

    # attribute
    HAS_NAME = 30, "Name", "Has a name", "fas fa-font-case"
    HAS_SLUG = 31, "Slug", "Has a slug", "fas fa-hashtag"
    HAS_ICON = 32, "Icon", "Has an icon", "fas fa-icons"

    # access [400-600]
    OWNABLE = 400, "Ownable", "Is ownable", "fas fa-user"
    JOINABLE = 402, "Joinable", "Is joinable", "fas fa-users"
    SUBJECT = 405, "Subject", "Is a Subject", "fas fa-user"
    MEMBERSHIP = 410, "Membership", "Is a Membership", "fas fa-users"
    INVITE = 411, "Invite", "Is an Invite", "fas fa-envelope"
    ROLE = 412, "Role", "Is a Role", "fas fa-user-tag"

    # folder [600-800]
    # ...

    # history [800-1000]
    # ...

    # infra [1000-1200]
    # ...

    # logic [1800-2200]
    RUNNABLE = 1800, "Runnable", "Can be run", "fas fa-play"
    SCRIPTABLE = 1801, "Scriptable", "Can be scripted", "fas fa-code"
    SOURCEABLE = 1802, "Sourcable", "Can be defined in a Script", "fas fa-code"
    METRIC = 1810, "Instrument", "Is an Instrument", "fas fa-microscope"
    MEASUREMENT = 1811, "Measurement", "Is a Measurement", "fas fa-microscope"
    CURSOR = 1812, "Cursor", "Is a Cursor", "fas fa-mouse-pointer"

    # test [2000-2400]
    # ...

    # runtime [2400-2800]
    # ...

    # deployment [2800-3000]
    # ...

    # product [3000-3400]
    # ...

    # finance [3400-3800]
    # ...

    # social [3800-4200]
    # MESSAGE, THREAD, ...
    STARABLE = 3830, "Starable", "Can be starred", "fas fa-star"
    REACTABLE = 3831, "Reactable", "Can be reacted to", "fas fa-heart"
    # RATEABLE, VOTABLE, ...
    # ASSIGNABLE, MESSAGEABLE, CLOSABLE, LOCKABLE, ...

    # locale [4200-4600]
    # ...

    # internet [4600-5000]
    # ...

    # infra [5000-5400]
    # ...

    # world [5400-5800]
    # ...

    # visual [8000-10000]

    # scene [8000-8100]
    VISUAL = 8000, "Visual", "Is a Visual", "fas fa-eye"
    VIEW = 8001, "View", "Is a View", "fas fa-eye"

    # media [8100-8200]
    # ...

    # container views [8200-8300]
    CONTAINER_VIEW = 8200, "Container View", "Is a Container View", "fas fa-container"

    # content views [8300-8400]
    CONTENT_VIEW = 8300, "Content View", "Is a Content View", "fas fa-content"

    # input views [8400-8500]
    INPUT_VIEW = 8400, "Input View", "Is an Input View", "fas fa-input"

    # node views [8500-8600]
    NODE_VIEW = 8500, "Node View", "Is a Node View", "fas fa-node"

    # internal views [8600-8700]
    INTERNAL_VIEW = 8600, "Internal View", "Is an Internal View", "fas fa-internal"

    # style [9000-9200]
    STYLE = 9000, "Style", "Is a Style", "fas fa-palette"

    # canvas [9200-9400]
    # ...

    # animation [9400-9600]
    # ...


@enum_(EnumType.AREA_TYPE)
class AreaType(BuiltinEnum):
    GLOBAL_DATABASE = 1
    # GLOBAL_SEARCH?
    SPACE_DATABASE = 100
    # SPACE_SEARCH, SPACE_WAREHOUSE, ...


@enum_(EnumType.RUNTIME_LANGUAGE)
class RuntimeLanguage(BuiltinEnum):
    PYTHON = 1
    JAVASCRIPT = 2
    # RUST, ...


@enum_(EnumType.RUNTIME_TYPE)
class RuntimeType(BuiltinEnum):
    SERVER = 1
    WEB = 2
    # MOBILE = 3
    # DESKTOP = 4


@enum_(EnumType.ENVIRONMENT_TYPE)
class EnvironmentType(BuiltinEnum):
    SYSTEM = 1, "System", "Managed by Destack", "fas fa-cog"
    DEVELOPMENT = 3, "Development", "Active in development", "fas fa-flask"
    TEST = 5, "Test", "Active in test", "fas fa-flask"
    STAGING = 7, "Staging", "Active in staging", "fas fa-globe"
    PRODUCTION = 10, "Production", "Active in production", "fas fa-globe"


ENUM_TYPES: bittuple[EnumType] = bittuple(*EnumType)
NODE_TYPES = bittuple(*NodeType)
STRUCT_TYPES: bittuple[StructType] = bittuple(*StructType)


@enum_(EnumType.CLOUD)
class Cloud(BuiltinEnum):
    """The cloud provider."""

    # own
    ...
    PRIVATE = 1
    # big general
    AWS = 10
    AZURE = 11
    GCP = 12
    HETZNER = 20

    @property
    def slug(self) -> str:
        return self.name.lower().replace("_", "-")


@enum_(EnumType.REGION_CONTINENT)
class RegionContinent(BuiltinEnum):
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


REGION_BY_SLUG = {r.slug: r for r in Region}


@enum_(EnumType.EDGE_TYPE)
class EdgeType(BuiltinEnum):
    PARENT = 1
    ANCESTOR = 2
    REGULAR = 5
    TEMPLATE = 6

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


@enum_(EnumType.EDGE_DIRECTION)
class EdgeDirection(BuiltinEnum):
    PARENT = 1
    CHILD = 2
    SIDE = 3


@enum_(EnumType.PRIMITIVE_TYPE)
class PrimitiveType(BuiltinEnum):
    """
    A fundamental scalar data type.
    """

    BOOLEAN = 1, "Boolean", "Yes or no", "fas fa-toggle-large-on"
    # INT8? UINTs?
    # range: -32768 to 32767
    INT16 = 4, "Integer", "Very small integer", "fas fa-tally"
    # range: -2147483648 to 2147483647
    INT32 = 5, "Integer", "Small integer", "fas fa-tally"
    # range: -9223372036854775808 to 9223372036854775807
    INT64 = 6, "Integer", "Integer number", "fas fa-tally"
    # numeric(precision, scale)
    DECIMAL = 10, "Decimal", "Decimal number", "fas fa-tally"
    # FLOAT16?
    # range: 1.175494351e-38 to 3.402823466e+38
    FLOAT32 = 16, "Float", "Small float", "fas fa-hashtag"
    # range: 2.2250738585072014e-308 to 1.7976931348623157e+308
    FLOAT64 = 17, "Float", "Floating point number", "fas fa-hashtag"
    STRING = 20, "String", "Plain text", "fas fa-font-case"
    UUID = 21, "UUID", "UUID", "fas fa-fingerprint"
    JSON = 22, "JSON", "JSON", "fas fa-brackets-curly"
    BYTES = 25, "Bytes", "Binary data", "fas fa-file-lines"
    # VECTOR?
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
    bytes: PrimitiveType.BYTES,
    datetime: PrimitiveType.DATETIME,
    date: PrimitiveType.DATE,
    time: PrimitiveType.TIME,
    timedelta: PrimitiveType.DURATION,
}
PRIMITIVE_PY_TYPES = tuple(PRIMITIVE_TYPE_BY_PY_TYPE.keys())


@enum_(EnumType.TYPE_CARDINALITY)
class TypeCardinality(BuiltinEnum):
    """The 'kind' of a Type."""

    SCALAR = 1
    LIST = 2
    # SET?
    MAP = 4
    # OPTION = 5
    # LITERAL = 6
    # UNION = 7


@enum_(EnumType.SCALAR_TYPE)
class ScalarType(BuiltinEnum):
    """The type of a scalar."""

    PRIMITIVE = 1
    ENUM = 2
    NODE_REFERENCE = 3
    NODE_VALUE = 4
    STRUCT = 5


@enum_(EnumType.DEFAULT_FACTORY)
class DefaultFactory(BuiltinEnum):
    """The factory to use for default values."""

    UUID = 1
    NOW = 2
    REGION = 3


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


@enum_(EnumType.RUN_STATUS)
class RunStatus(BuiltinEnum):
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
        return self in (RunStatus.FAILED, RunStatus.ABORTED, RunStatus.CANCELLED)


@enum_(EnumType.CLIENT_TYPE)
class ClientType(BuiltinEnum):
    # user
    WEB = 1
    BROWSER_PLUGIN = 2
    DESKTOP = 3
    MOBILE = 4
    # system
    MACHINE = 10


@enum_(EnumType.TENANCY)
class Tenancy(BuiltinEnum):
    DEDICATED = 1
    SHARED = 2


CLOUD = get_from_env("CLOUD", typ=Cloud, description="Cloud we're running in")
REGION = get_from_env("REGION", typ=Region, description="Region we're running in")
TRACING = get_from_env("TRACING", typ=bool, description="Enable tracing")


class DestackError(Exception):
    """Common base class for any regular errors."""

    pass


def repr_enums(enums: Iterable[BuiltinEnum]) -> str:
    return "|".join(e.camel_name for e in enums)

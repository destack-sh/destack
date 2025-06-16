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
from more_itertools import first

from destack.utils.env import IS_TEST, get_from_env
from destack.utils.frozen import frozendict
from destack.utils.string import Casing, to_casing
from destack.utils.uuid import UUID, uuid4

if TYPE_CHECKING:
    from destack.language import Session


class _Unset:
    def __repr__(self):
        return "<UNSET!>"

    def __str__(self):
        return "<UNSET!>"


# forever constants
VERSION = "2025.06.16.0"
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


class Enum(enum.IntEnum):
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


BuiltinEnumOrUnion = Union[Enum, Union[Enum, Any]]
EnumT = TypeVar("EnumT", bound=Enum)
_ENUM_MEMBERS_BY_ORD: dict[type[Enum], list[Enum]] = {}


def _get_enum_members_by_ord(enum_cls: type[Enum]) -> list[Enum]:
    if enum_cls not in _ENUM_MEMBERS_BY_ORD:
        _ENUM_MEMBERS_BY_ORD[enum_cls] = list(enum_cls.__members__.values())
    return _ENUM_MEMBERS_BY_ORD[enum_cls]


# noinspection PyPep8Naming

# NOTE: we have the enum registry here to avoid circular imports
_ENUM_CLASS_BY_TYPE: dict["EnumType", type[Enum]] = {}
_ENUM_TYPE_BY_CLASS: dict[type[Enum], "EnumType"] = {}

BuiltinEnumT = typing.TypeVar("BuiltinEnumT", bound=Enum)


def builtin_enum(enum_type: "EnumType"):
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
        assert isinstance(enum_cls, type) and issubclass(enum_cls, Enum), (
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


class EnumType(Enum):
    # space [1-500]
    SPACE_STATUS = 1
    USER_STATUS = 20
    FRIENDSHIP_INVITE_EVENT_TYPE = 31
    ORGANIZATION_STATUS = 40
    CLIENT_TYPE = 100
    # query
    CONDITIONAL_TYPE = 103
    AGGREGATION_TYPE = 104
    SORT_MODE = 105
    SORT_TYPE = 106
    JOIN_TYPE = 107
    FUNCTION_TYPE = 108
    EXPRESSION_TYPE = 109
    QUERY_TYPE = 120
    QUERY_UPDATE_TYPE = 121

    # access [500-1000]
    MEMBERSHIP_EVENT_TYPE = 500
    MEMBERSHIP_PERMISSION = 501
    INVITE_EVENT_TYPE = 510
    ROLE_TYPE = 520
    ROLE_EVENT_TYPE = 521
    PERMISSION_TYPE = 530
    SANCTION_TYPE = 540
    SANCTION_EVENT_TYPE = 541
    ENTITLEMENT_TYPE = 550
    ENTITLEMENT_EVENT_TYPE = 551
    # ...

    # folder [1000-1500]
    FOLDER_TYPE = 1000

    # history [1500-2000]
    # ...

    # entity [2000-2500]
    # ...

    # data [2500-3000]
    TEXT_LINE_TYPE = 2521
    TEXT_SPAN_TYPE = 2522
    FILE_RETENTION_MODE = 2540
    FILE_SOURCE = 2541
    FILE_TYPE = 2542
    FILE_FORMAT = 2543
    ICON_TYPE = 2531
    LINK_TYPE = 2550
    PRIMITIVE_TYPE = 2560
    TYPE_CARDINALITY = 2561
    SCALAR_TYPE = 2562
    DEFAULT_FACTORY = 2563
    STRING_FORMAT = 2570
    NUMBER_FORMAT = 2571
    FIELD_TYPE = 2580
    EDGE_TYPE = 2581
    EDGE_DIRECTION = 2582
    CASCADE_ACTION = 2583
    RESOURCE_STATUS = 2590
    # ...

    # logic [3000-3500]
    ACTION_CARDINALITY = 3020
    CURSOR_STATUS = 3100
    SCHEDULE_FREQUENCY = 3050
    DAY_OF_WEEK = 3051
    MONTH = 3052
    TIMER_TYPE = 3053
    TIMER_EVENT_TYPE = 3054
    TRIGGER_TYPE = 3040
    TRIGGER_EVENT_TYPE = 3041
    # ...

    # test [3500-4000]
    # ...

    # runtime [4000-4500]
    RUN_STATUS = 4000
    RUN_EVENT_TYPE = 4001
    INTERRUPTION_TYPE = 4020
    INTERRUPTION_STATUS = 4021
    INTERRUPTION_RESPONSE = 4022
    # ...

    # deployment [4500-5000]
    ENVIRONMENT_TYPE = 4500

    # product [5000-5500]
    # ...

    # social [5500-6000]
    THREAD_STATUS = 5500
    NOTIFICATION_STATUS = 5600
    NOTIFICATION_EVENT_TYPE = 5601

    # finance [6000-6500]
    # ...

    # locale [6500-7000]
    # ...

    # internet [7000-7500]
    # ...

    # infra [7500-8000]
    CLOUD = 7500
    REGION = 7501
    REGION_AREA = 7502
    REGION_CONTINENT = 7503
    TENANCY = 7504
    DATABASE_TYPE = 7505
    MACHINE_TYPE = 7600
    # SEARCH_TYPE, WAREHOUSE_TYPE, ...
    # ...

    # intelligence [8000-8500]
    MODEL_DEVELOPER = 8000
    MODEL_PROVIDER = 8001
    # ...

    # world [8500-9000]
    # ...

    # scene [9000-9500]
    WINDOW_TYPE = 9000
    SCENE_EVENT_TYPE = 9011
    LAYER_TYPE = 9020
    VARIANT_TYPE = 9030
    VARIANT_STATE_TYPE = 9031

    # interaction [9500-10000]
    MODE_TYPE = 9500
    TOOL_TYPE = 9501

    # container views [10000-10200]
    # ...

    # content views [10200-10400]
    # ...

    # input views [10400-10600]
    # ...

    # node/internal views [10600-10800]
    # ...

    # canvas [11000-11500]
    CANVAS_TYPE = 11000
    PLANE_SHAPE_TYPE = 11011
    ARROW_HEAD_TYPE = 11012
    LINE_TYPE = 11010

    # animation [11500-12000]
    # ...

    # style [12000-12500]
    POSITION_TYPE = 12000
    COLOR_TYPE = 12020
    COLOR_SHADE = 12021
    COLOR_HUE = 12022
    COLOR_INTENT = 12023
    FONT_WEIGHT = 12024
    FONT_SIZE = 12025
    FONT_TYPE = 12026
    TEXT_ALIGN = 12027
    TEXT_DECORATION = 12028
    TEXT_TRANSFORM = 12029
    SHADOW_TYPE = 12030
    SHADOW_POSITION = 12031
    BORDER_TYPE = 12032
    GRADIENT_TYPE = 12033
    FILL_TYPE = 12034
    FILL_POSITION = 12035
    FILL_SIZE = 12036
    LENGTH_UNIT = 12037
    LAYOUT = 12038
    DISTRIBUTE = 12039
    ALIGN = 12040
    DIRECTION = 12041
    OVERFLOW = 12042
    TRANSITION_TYPE = 12043
    SPRING_TYPE = 12044
    DIMENSION_TYPE = 12045
    EFFECT_TYPE = 12046
    REPEAT_TYPE = 12047
    TEXT_SPLIT_TYPE = 12048
    OFFSCREEN_BEHAVIOR = 12049

    # meta [50000-51000]
    ENUM_TYPE = 50000
    NODE_TYPE = 50001
    STRUCT_TYPE = 50002
    TRAIT_TYPE = 50003
    RELATION_TYPE = 50010
    ATTRIBUTE_TYPE = 50011
    PROPERTY_REFERENCE_TYPE = 50012
    INSTANCE_MODE = 50013
    STORE_ZONE = 50020
    STORE_TYPE = 50021
    STORE_IMPLEMENTATION = 50022
    PLATFORM_TYPE = 50030
    RUNTIME_TYPE = 50031
    OPERATING_SYSTEM = 50040
    ERROR_TYPE = 50041
    EDIT_TYPE = 50050
    EDIT_OPERATION = 50051
    CHANGE_STATUS = 50052
    NODE_PERMISSION = 50100
    JOINABLE_PERMISSION = 50101


builtin_enum(EnumType.ENUM_TYPE)(EnumType)


@builtin_enum(EnumType.STRUCT_TYPE)
class StructType(Enum):
    # space [1-500]
    # ...

    # access [500-1000]
    # ...

    # folder [1000-1500]
    # ...

    # spacetime [1500-2000]
    # ...

    # entity [2000-2500]
    # ...

    # data [2500-3000]
    VALUE = 2500
    TYPE = 2501
    NUMBER_CONSTRAINT = 2502
    STRING_CONSTRAINT = 2503
    COLLECTION_CONSTRAINT = 2504
    NODE_CONSTRAINT = 2505
    TEXT = 2521, None, None, "fas fa-text"
    TEXT_LINE = 2522, None, None, "fas fa-text"
    TEXT_SPAN = 2523, None, None, "fas fa-text"
    ICON = 2531
    SELECTION = 2571
    # SCHEMA, UNION, TAG, ...

    # logic [3000-3500]
    SCHEDULE = 3001
    # ...

    # test [3500-4000]
    # ...

    # runtime [4000-4500]
    ERROR = 4001
    # ...

    # deployment [4500-5000]
    # ...

    # product [5000-5500]
    # ...

    # social [5500-6000]
    # ...

    # finance [6000-6500]
    # ...

    # locale [6500-7000]
    # ...

    # internet [7000-7500]
    # ...

    # infra [7500-8000]
    DATABASE_INFO = 7501
    GALAXY_INFO = 7601

    # intelligence [8000-8500]
    # ...

    # world [8500-9000]
    # ...

    # scene [9000-9500]
    # ...

    # interaction [9500-10000]
    # ...

    # container views [10000-10200]
    # ...

    # content views [10200-10400]
    # ...

    # input views [10400-10600]
    # ...

    # node/internal views [10600-10800]
    # ...

    # canvas [11000-11500]
    # ...

    # animation [11500-12000]
    # ...

    # style [12000-12500]
    COLOR = 12011, None, None, "fas fa-palette"
    SHADOW = 12012, None, None, "fas fa-eclipse"
    BORDER = 12013, None, None, "fas fa-border-outer"
    FONT = 12014, None, None, "fas fa-text"
    GRADIENT_STOP = 12015, None, None, "fas fa-gradient"
    GRADIENT = 12016, None, None, "fas fa-gradient"
    FILL = 12017, None, None, "fas fa-fill"
    LENGTH = 12018, None, None, "fas fa-ruler"
    POSITION = 12020, None, None, "fas fa-location-crosshair"
    DIMENSION = 12022, None, None, "fas fa-ruler"
    TRANSITION = 12024, None, None, "fas fa-bezier-curve"
    EFFECT = 12025, None, None, "fas fa-sparkle"
    GRID = 12026, None, None, "fas fa-grid-2"
    GRID_SPAN = 12028, None, None, "fas fa-grid-2"
    INSETS = 12030, None, None, "fas fa-corner"
    CORNERS = 12032, None, None, "fas fa-corner"

    # meta [50000-51000]
    SCOPE = 50000
    ORIGIN = 50001
    NODE_REFERENCE = 50002
    PROPERTY_REFERENCE = 50003
    PROPERTY_DEFINITION = 50004
    TRAIT_DEFINITION = 50005
    NODE_DEFINITION = 50006
    STRUCT_DEFINITION = 50007
    ENUM_DEFINITION = 50008
    ENUM_OPTION_DEFINITION = 50009
    PERMISSION_DEFINITION = 50010
    # ACTION_DEFINITION, ...?
    EDIT = 50020
    CHANGE = 50021
    CHANGE_RESULT = 50022
    EXPRESSION = 50100
    FUNCTION = 50101
    JOIN = 50102
    AGGREGATION = 50103
    CONDITION = 50104
    SORT = 50105
    SELECT = 50106
    RELATION_REFERENCE = 50107
    ATTRIBUTE_REFERENCE = 50108
    QUERY = 50109
    QUERY_RESULT = 50110
    QUERY_RESULT_GROUP = 50111
    QUERY_UPDATE = 50112
    HISTOGRAM = 50113
    VECTOR2 = 50200, None, None, "fas fa-vector-square"
    VECTOR3 = 50201, None, None, "fas fa-vector-square"
    VECTOR4 = 50202, None, None, "fas fa-vector-square"
    VECTOR2I = 50203, None, None, "fas fa-vector-square"
    VECTOR3I = 50204, None, None, "fas fa-vector-square"
    VECTOR4I = 50205, None, None, "fas fa-vector-square"
    AXIS2 = 50207, None, None, "fas fa-vector-square"
    AXIS3 = 50209, None, None, "fas fa-vector-square"


@builtin_enum(EnumType.NODE_TYPE)
class NodeType(Enum):
    # space [1-500]
    SPACE = 1, "Space", "Universal Space", "https://heydestack.com/favicon.ico"
    HANDLE = 10, "Handle", "Unique @handle", "fas fa-at"
    USER = 20, "User", "User", "fas fa-user"
    FRIENDSHIP = 30, "Friendship", "Friendship between two Users", "fas fa-user-friends"
    FRIENDSHIP_INVITE = (
        31,
        "Friendship Invite",
        "Invite to be friends with another User",
        "fas fa-user-plus",
    )
    FRIENDSHIP_INVITE_EVENT = (
        32,
        "Friendship Invite Event",
        "Friendship Invite Event",
        "fas fa-user-plus",
    )
    ORGANIZATION = 40, "Organization", "Organization", "fas fa-building"
    TEAM = 50, "Team", "Team in an Organization", "fas fa-users"
    # CREDENTIAL, ACCOUNT, PROFILE, ...
    CLIENT = 100, "Client", "Client", "fas fa-desktop"

    # access [500-1000]
    MEMBERSHIP = 500, "Membership", "Membership in a Space/Folder", "fas fa-user-group"
    MEMBERSHIP_EVENT = 501, "Membership Event", "Membership Event", "fas fa-user-group"
    INVITE = 510, "Invite", "Invite to a Space/Folder", "fas fa-user-plus"
    INVITE_EVENT = 511, "Invite Event", "Invite Event", "fas fa-user-plus"
    ROLE = 520, "Role", "Role in something", "fas fa-user-tag"
    ROLE_EVENT = 521, "Role Event", "Role Event", "fas fa-user-tag"
    PERMISSION = 530, "Permission", "Permission for something", "fas fa-user-shield"
    SANCTION = 540, "Sanction", "Temporary or permanent restriction", "fas fa-user-minus"
    SANCTION_EVENT = 541, "Sanction Event", "Sanction Event", "fas fa-user-minus"
    ENTITLEMENT = 550, "Entitlement", "Temporary or permanent grant", "fas fa-user-check"
    ENTITLEMENT_EVENT = 551, "Entitlement Event", "Entitlement Event", "fas fa-user-check"
    AGENT = 600, "Agent", "Agent", "fas fa-robot"
    # CHALLENGE, ...

    # folder [1000-1500]
    FOLDER = 1000, "Folder", "Sub-space of a Space", "fas fa-folder-open"
    # DEPENDENCY, ...
    TAG = 1010, "Tag", "Tag", "fas fa-tag"
    TAGGING = 1011, "Tagging", "Tagging", "fas fa-tag"

    # spacetime [1500-2000]
    SNAPSHOT = 1500, "Snapshot", "Snapshot", "fas fa-save"
    BRANCH = 1510, "Branch", "Branch", "fas fa-code-branch"
    # HISTORY, REPLAY, ...
    # FORK, ...

    # entity [2000-2500]
    CUSTOM_ENTITY_DEFINITION = (
        2000,
        "Custom Node Definition",
        "Custom Node Definition",
        "fas fa-table",
    )
    CUSTOM_ENTITY = 2001, "Custom Node Instance", "Custom Node Instance", "fas fa-database"
    # INDEX, CONSTRAINT, MIGRATION, ...
    # MIRROR/SYNC, ...
    # TRAIT_DEFINITION/TRAIT_IMPLEMENTATION, INTERFACE, ...

    # data [2500-3000]
    CUSTOM_STRUCT_DEFINITION = 2500, "Struct", "Struct", "fas fa-shapes"
    CUSTOM_ENUM_DEFINITION = 2510, "Enum", "Enum", "fas fa-shapes"
    FIELD = 2520, "Field", "Field", "fas fa-triangle"
    OPTION = 2530, "Option", "Option", "fas fa-circle"
    FILE = 2540, "File", "File", "fas fa-file"
    LINK = 2550, "Link", "Link to something", "fas fa-link"
    # STREAM, SECRET, ...

    # logic [3000-3500]
    SCRIPT = 3000, "Script", "Script", "fas fa-code"
    SERVICE = 3010, "Service", "Service", "fas fa-screwdriver-wrench"
    ACTION = 3020, "Action", "Action", "fas fa-step-forward"
    ROUTE = 3030, "Route", "Route", "fas fa-route"
    TRIGGER = 3040, "Trigger", "Trigger", "fas fa-bolt"
    TRIGGER_EVENT = 3041, "Trigger Event", "Trigger Event", "fas fa-bolt"
    TIMER = 3050, "Timer", "Timer", "fas fa-clock"
    TIMER_EVENT = 3051, "Timer Event", "Timer Event", "fas fa-clock"
    # BREAKPOINT, ...
    EVENT_CURSOR = 3100, "Event Cursor", "Event Cursor", "fas fa-signal"
    SCREEN_CURSOR = 3101, "Mouse Cursor", "Mouse Cursor", "fas fa-mouse"
    THREAD_CURSOR = 3102, "Query Cursor", "Query Cursor", "fas fa-magnifying-glass"
    # QUERY_CURSOR, WEB_CURSOR, ...
    # ROOM, CHANNEL, LOCK, ...
    # TASK, ...
    # RATE_LIMIT, ...

    # test [3500-4000]
    # TEST, TEST_SUITE, TEST_CASE, TEST_RESULT, ...
    # FIXTURE, MOCK, ...
    # LINT, WARNING, ERROR, ...

    # runtime [4000-4500]
    RUN = 4000, "Run", "Run", "fas fa-play"
    RUN_EVENT = 4001, "Run Event", "Run Event", "fas fa-play"
    # RUN_QUEUE = 4001, "Run Queue", "Run Queue", "fas fa-list-check"
    SPAN = 4010, "Span", "Span", "fas fa-ruler-horizontal"
    INTERRUPTION = 4020, "Interruption", "Interruption", "fas fa-hand"
    # JOB, ...
    LOG = 4100, "Log", "Log", "fas fa-file-lines"
    GAUGE_METRIC = 4110, "Gauge Metric", "Gauge Metric", "fas fa-gauge"
    GAUGE_MEASUREMENT = 4111, "Gauge Measurement", "Gauge Measurement", "fas fa-gauge"
    COUNTER_METRIC = 4112, "Counter Metric", "Counter Metric", "fas fa-gauge"
    COUNTER_MEASUREMENT = 4113, "Counter Measurement", "Counter Measurement", "fas fa-gauge"
    HISTOGRAM_METRIC = 4114, "Histogram Metric", "Histogram Metric", "fas fa-gauge"
    HISTOGRAM_MEASUREMENT = 4115, "Histogram Measurement", "Histogram Measurement", "fas fa-gauge"
    CUSTOM_EVENT_DEFINITION = (
        4200,
        "Custom Event Definition",
        "Custom Event Definition",
        "fas fa-signal",
    )
    CUSTOM_EVENT = 4201, "Custom Event", "Custom Event", "fas fa-signal"
    EDIT_EVENT = 4202, "Edit Event", "Edit Event", "fas fa-file-lines"
    # CHANGE_EVENT, QUERY_EVENT, ...

    # deployment [4500-5000]
    ENVIRONMENT = 4500, "Environment", "Environment", "fas fa-environment"
    # DEPLOYMENT, ...
    # PREVIEW, DRAFT, RELEASE, ROLLOUT, ...
    # INCIDENT, ESCALATION, ...

    # product [5000-5500]
    # SETTINGS, ...
    # VISIT, RECORDING/REPLAY, SURVEY, ...
    # ONBOARDING, TOUR, FUNNEL, COHORT, JOURNEY, ..
    # FEATURE, FEATURE_FLAG, FEATURE_GATE, ...
    # SEGMENT, EXPERIMENT, ...

    # social [5500-6000]
    THREAD = 5500, "Thread", "Thread", "fas fa-reel"
    MESSAGE = 5510, "Message", "Message", "fas fa-message"
    REACTION = 5520, "Reaction", "Reaction", "fas fa-heart"
    STAR = 5521, "Star", "Star", "fas fa-star"
    FOLLOW = 5530, "Follow", "Follow", "fas fa-plus"
    # FEED, FEED_ITEM, ...
    # POLL, VOTE, REVIEW, RATING, RANK, ...
    # ACHIEVEMENT, BADGE, WISHLIST/WATCHLIST, ...
    NOTIFICATION = 5600, "Notification", "Notification", "fas fa-bell"
    NOTIFICATION_EVENT = 5601, "Notification Event", "Notification Event", "fas fa-bell"

    # finance [6000-6500]
    # WALLET, BALANCE, BUDGET, TRANSFER, CREDIT, ...
    # TIER, SUBSCRIPTION, PRODUCT, PRICE, ...
    # ORDER, INVOICE, DISCOUNT, DISPUTE, REFUND, ...

    # locale [6500-7000]
    # LOCALE, STRING, TRANSLATION, ...

    # internet [7000-7500]
    # DOMAIN, ...
    # EMAIL, EMAIL_ATTEMPT, ...

    # infra [7500-8000]
    DATABASE = 7500, "Database", "Database for Postgres data", "fas fa-database"
    # SEARCH/INDEX, VAULT, CACHE, S3, ...
    # GALAXY = 5010, "Galaxy", "Galaxy", "fas fa-galaxy"
    MACHINE = 7600, "Machine", "Machine for ephemeral computing", "fas fa-machine-classic"
    # HOST, ENDPOINT, DEPLOYMENT, NETWORK, AUTOSCALER, ...

    # intelligence [8000-8500]
    # MODEL, FINETUNE, ...
    # INFERENCE, PROMPT, ...
    # RECOMMENDATION, ...

    # world [8500-9000]
    # PHONE_NUMBER, ADDRESS, ...

    # scene [9000-9500]
    WINDOW = 9000, "Window", "Window", "fas fa-galaxy"
    SCENE = 9010, "Scene", "Scene of an Application", "fas fa-masks-theater"
    SCENE_EVENT = 9011, "Scene Event", "Scene Event", "fas fa-masks-theater"
    LAYER = 9020, "Layer", "Layer of a Scene", "fas fa-layer-group"
    VARIANT = 9030, "Variant", "Variant of a Scene", "fas fa-shapes"
    # VIEWPORT, OVERLAY, WIDGET, MENU, ...

    # interaction [9500-10000]
    # COMMAND, MODE, TOOL, SHORTCUT/KEYBINDING, ...
    # GESTURE, ...
    # CAMERA, SPEAKER, MICROPHONE, ...

    # container views [10000-10200]
    CUSTOM_VIEW_DEFINITION = (
        10000,
        "Custom View Definition",
        "Custom View Definition",
        "fas fa-table",
    )
    CUSTOM_VIEW = 10001, "Custom View", "Custom View", "fas fa-table"
    # SLOT_DEFINITION_VIEW, SLOT_VIEW, ...
    FRAME_VIEW = 10020, "Frame View", "Fixed Container", "fas fa-frame"
    LABEL_VIEW = 10030, "Label View", "Label Container", "fas fa-font-case"
    # FORM_VIEW, MENU_VIEW, ...
    SPLIT_VIEW = 10040, "Split View", "Split Container", "fas fa-columns"
    # TAB_VIEW, ...
    # DRAWER_VIEW, SPLIT_DRAWER_VIEW, GRID/GRID_ELEMENT_VIEW, ...

    # content views [10200-10400]
    TEXT_VIEW = 10200, "Text View", "Text", "fas fa-text"
    # CODE_VIEW, ICON_VIEW, IMAGE_VIEW, AUDIO_VIEW, VIDEO_VIEW, DOCUMENT_VIEW, ...

    # input views [10400-10600]
    NUMBER_INPUT_VIEW = 10400, "Number Input View", "Number Input", "fas fa-hashtag"
    SLIDER_INPUT_VIEW = 10401, "Slider Input View", "Slider Input", "fas fa-slider"
    # STRING_INPUT_VIEW, TOGGLE_INPUT_VIEW, PICKER_INPUT_VIEW, COLOR_INPUT_VIEW, ...
    # ICON_INPUT_VIEW, FILE_INPUT_VIEW, DATETIME_INPUT_VIEW, DURATION_INPUT_VIEW, ...

    # NOTE :Architecture: node and internal views should probably be defined in user space?
    # node/internal views [10600-10800]
    THREAD_VIEW = 10600, "Thread View", "Thread", "fas fa-reel"
    WIZARD_VIEW = 10650, "Wizard View", "Wizard", "fas fa-wand-sparkles"

    # canvas [11000-11500]
    CANVAS = 11000, "Canvas", "Canvas", "fas fa-canvas"
    LINE_SHAPE = 11010, "Line Shape", "Line Shape", "fas fa-line"
    PLANE_SHAPE = 11011, "Plane Shape", "Plane Shape", "fas fa-shapes"
    ARROW_SHAPE = 11012, "Arrow Shape", "Arrow Shape", "fas fa-arrow-right"
    ANNOTATION_SHAPE = 11013, "Annotation Shape", "Annotation Shape", "fas fa-comment"
    # BITMAP, ...

    # animation [11500-12000]
    # ANIMATION, TRACK, KEYFRAME, ...
    # SOUND, ...

    # style [12000-12500]
    THEME = 12000, "Theme", "Theme", "fas fa-palette"
    PALETTE = 12010, "Palette", "Palette", "fas fa-palette"
    COLOR_STYLE = 12020, "Color Style", "Color Style", "fas fa-palette"
    FILL_STYLE = 12021, "Fill Style", "Fill Style", "fas fa-fill"
    FONT_STYLE = 12022, "Font Style", "Font Style", "fas fa-text"
    BORDER_STYLE = 12023, "Border Style", "Border Style", "fas fa-border-outer"
    SHADOW_STYLE = 12024, "Shadow Style", "Shadow Style", "fas fa-eclipse"
    GRADIENT_STYLE = 12025, "Gradient Style", "Gradient Style", "fas fa-gradient"
    TRANSITION_STYLE = 12026, "Transition Style", "Transition Style", "fas fa-bezier-curve"
    EFFECT_STYLE = 12027, "Effect Style", "Effect Style", "fas fa-sparkle"
    # BRUSH_STYLE, ...
    # SHADER, MATERIAL, ...

    # meta [50000-51000]
    # ...


@builtin_enum(EnumType.TRAIT_TYPE)
class TraitType(Enum):
    # destack [1-400]
    # where
    GLOBAL = 1, "Global", "Is global", "fas fa-globe"
    SPATIAL = 2, "Spatial", "Is in a Space", "fas fa-solar-system"
    # LOCAL?
    # kind
    ENTITY = 10, "Entity", "Is an Entity", "fas fa-hexagon"
    PARTICLE = 11, "Particle", "Is a Particle", "fas fa-atom"
    ANALYTIC = 12, "Analytic", "Is an Analytic", "fas fa-chart-line"
    INDEXED = 13, "Indexed", "Is indexed", "fas fa-search"
    # type
    RESOURCE = 21, "Resource", "Is a Resource", "fas fa-server"
    EVENT = 22, "Event", "Is an Event", "fas fa-bolt"
    CUSTOM_NODE_DEFINITION = (
        23,
        "Custom Node Definition",
        "Is a Custom Node Definition",
        "fas fa-table",
    )
    CUSTOM_NODE = 24, "Custom Node", "Is a Custom Node", "fas fa-database"
    # behavior
    FROZEN = 50, "Frozen", "Is frozen", "fas fa-snowflake"
    TRACKED = 51, "Tracked", "Is tracked", "fas fa-clock"
    ARCHIVABLE = 52, "Archivable", "Can be archived", "fas fa-box-archive"
    DELETABLE = 53, "Deletable", "Can be deleted", "fas fa-trash"
    EXTENSIBLE = 55, "Extensible", "Is extensible", "fas fa-expand"
    ORDERED = 56, "Ordered", "Is ordered", "fas fa-sort"
    # attribute
    HAS_NAME = 100, "Name", "Has a name", "fas fa-font-case"
    HAS_SLUG = 101, "Slug", "Has a slug", "fas fa-hashtag"
    HAS_ICON = 102, "Icon", "Has an icon", "fas fa-icons"

    # access [500-1000]
    OWNABLE = 500, "Ownable", "Is ownable", "fas fa-user"
    JOINABLE = 502, "Joinable", "Is joinable", "fas fa-users"
    SUBJECT = 505, "Subject", "Is a Subject", "fas fa-user"
    OWNER = 506, "Owner", "Is an Owner", "fas fa-user"
    MEMBERSHIP = 510, "Membership", "Is a Membership", "fas fa-users"
    INVITE = 511, "Invite", "Is an Invite", "fas fa-envelope"

    # folder [1000-1500]
    TAGGABLE = 1000, "Taggable", "Can be tagged", "fas fa-tag"
    TAG = 1001, "Tag", "Is a Tag", "fas fa-tag"

    # spacetime [1500-2000]
    # ...

    # entity [2000-2500]
    # ...

    # data [2500-3000]
    # ...

    # logic [3000-3500]
    ACTIONABLE = 3000, "Actionable", "Can define an Action", "fas fa-play"
    RUNNABLE = 3001, "Runnable", "Can be run", "fas fa-play"
    SCRIPTABLE = 3002, "Scriptable", "Can be scripted", "fas fa-code"
    SOURCEABLE = 3003, "Sourcable", "Can be defined in a Script", "fas fa-code"
    # PAUSEABLE?
    CURSOR = 3012, "Cursor", "Is a Cursor", "fas fa-mouse-pointer"

    # test [3500-4000]
    # ...

    # runtime [4000-4500]
    METRIC = 4010, "Instrument", "Is an Instrument", "fas fa-microscope"
    MEASUREMENT = 4011, "Measurement", "Is a Measurement", "fas fa-microscope"
    # ...

    # deployment [4500-5000]
    # ...

    # product [5000-5500]
    SETTINGS = 5000, "Settings", "Defines Settings", "fas fa-cog"

    # social [5500-6000]
    # MESSAGE, THREAD, ...
    STARABLE = 5530, "Starable", "Can be starred", "fas fa-star"
    REACTABLE = 5532, "Reactable", "Can be reacted to", "fas fa-heart"
    FOLLOWABLE = 5534, "Followable", "Can be followed", "fas fa-plus"
    FOLLOW = 5535, "Follow", "Follow", "fas fa-plus"
    # RATEABLE, VOTABLE, ...
    # ASSIGNABLE, MESSAGEABLE, CLOSABLE, LOCKABLE, ...

    # finance [6000-6500]
    # ...

    # locale [6500-7000]
    # ...

    # internet [7000-7500]
    # ...

    # infra [7500-8000]
    # ...

    # intelligence [8000-8500]
    # ...

    # world [8500-9000]
    # ...

    # scene [9000-9500]
    VISUAL = 9000, "Visual", "Is a Visual", "fas fa-eye"
    VIEW = 9001, "View", "Is a View", "fas fa-eye"

    # interaction [9500-10000]
    # ...

    # container views [10000-10200]
    CONTAINER_VIEW = 10000, "Container View", "Is a Container View", "fas fa-container"

    # content views [10200-10400]
    CONTENT_VIEW = 10200, "Content View", "Is a Content View", "fas fa-content"

    # input views [10400-10600]
    INPUT_VIEW = 10400, "Input View", "Is an Input View", "fas fa-input"

    # node/internal views [10600-10800]
    NODE_VIEW = 10600, "Node View", "Is a Node View", "fas fa-node"
    INTERNAL_VIEW = 10650, "Internal View", "Is an Internal View", "fas fa-internal"

    # canvas [11000-11500]
    SHAPE = 11000, "Shape", "Is a Shape", "fas fa-shapes"

    # animation [11500-12000]
    # ...

    # style [12000-12500]
    STYLE = 12000, "Style", "Is a Style", "fas fa-palette"

    # meta [50000-51000]
    # ...


@builtin_enum(EnumType.STORE_ZONE)
class StoreZone(Enum):
    GLOBAL = 1
    SPATIAL = 2
    LOCAL = 3


@builtin_enum(EnumType.STORE_TYPE)
class StoreType(Enum):
    GLOBAL_ENTITY = 100
    # GLOBAL_SEARCH?
    SPATIAL_ENTITY = 200
    # SPATIAL_PARTICLE, SPATIAL_ANALYTIC, ...
    # SPATIAL_SEARCH, SPATIAL_CACHE, ...
    LOCAL_MEMORY = 300

    @property
    def zone(self) -> StoreZone:
        return StoreZone(self.value // 100)


@builtin_enum(EnumType.STORE_IMPLEMENTATION)
class StoreImplementation(Enum):
    POSTGRES = 1
    # CASSANDRA, ELASTICSEARCH, REDIS, ...
    MEMORY = 10


@builtin_enum(EnumType.RUNTIME_TYPE)
class RuntimeType(Enum):
    PYTHON = 1
    JAVASCRIPT = 2
    # RUST, JAVA, SWIFT, ...


@builtin_enum(EnumType.PLATFORM_TYPE)
class PlatformType(Enum):
    SERVER = 1
    WEB = 10
    # MOBILE, DESKTOP, ...
    # EMAIL?


@builtin_enum(EnumType.OPERATING_SYSTEM)
class OperatingSystem(Enum):
    # desktop
    LINUX = 1, "Linux", "Linux operating system", "fab fa-linux"
    WINDOWS = 2, "Windows", "Microsoft Windows", "fab fa-windows"
    MACOS = 3, "macOS", "Apple macOS", "fab fa-apple"
    # mobile
    ANDROID = 50, "Android", "Google Android", "fab fa-android"
    IOS = 51, "iOS", "Apple iOS", "fab fa-apple"
    # WATCHOS, TVOS, IPADOS, ...


@builtin_enum(EnumType.ENVIRONMENT_TYPE)
class EnvironmentType(Enum):
    SYSTEM = 1, "System", "Managed by the system", "fas fa-cog"
    DEVELOPMENT = 3, "Development", "Active in development", "fas fa-flask"
    TEST = 5, "Test", "Active in test", "fas fa-flask"
    STAGING = 7, "Staging", "Active in staging", "fas fa-globe"
    PRODUCTION = 10, "Production", "Active in production", "fas fa-globe"


@builtin_enum(EnumType.NODE_PERMISSION)
class NodePermission(Enum):
    # read
    READ = 1, "Read"
    # write
    ADD = 10, "Create, Upsert, Unarchive, Restore"
    UPDATE = 11, "Update"
    REMOVE = 12, "Archive, Delete, Erase"


@builtin_enum(EnumType.INSTANCE_MODE)
class InstanceMode(Enum):
    PARTIAL_NODE = 1, "Partial Node"
    PARTIAL_GRAPH = 2, "Full Node, Partial Graph"
    FULL_GRAPH = 3, "Full"


@builtin_enum(EnumType.MODE_TYPE)
class ModeType(Enum):
    EDIT = 1
    DEBUG = 2
    INSPECT = 3
    PREVIEW = 4
    USE = 5


@builtin_enum(EnumType.TOOL_TYPE)
class ToolType(Enum):
    SELECT = 1
    DRAG = 2
    INSPECT = 10
    ANNOTATE = 11
    # ...


ENUM_TYPES: bittuple[EnumType] = bittuple(*EnumType)
NODE_TYPES = bittuple(*NodeType)
STRUCT_TYPES: bittuple[StructType] = bittuple(*StructType)


@builtin_enum(EnumType.CLOUD)
class Cloud(Enum):
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


@builtin_enum(EnumType.REGION_CONTINENT)
class RegionContinent(Enum):
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


@builtin_enum(EnumType.REGION_AREA)
class RegionArea(Enum):
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


@builtin_enum(EnumType.REGION)
class Region(Enum):
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


@builtin_enum(EnumType.EDGE_TYPE)
class EdgeType(Enum):
    PARENT = 1
    REGULAR = 5

    @property
    def is_node_tree(self):
        return self.id <= 4

    @property
    def is_node(self):
        return self.id < 10


@builtin_enum(EnumType.CASCADE_ACTION)
class CascadeAction(Enum):
    RESTRICT = 1
    CASCADE = 2
    SET_NULL = 3
    # SET_DEFAULT, NONE, ...


@builtin_enum(EnumType.EDGE_DIRECTION)
class EdgeDirection(Enum):
    PARENT = 1
    CHILD = 2
    SIDE = 3


@builtin_enum(EnumType.PRIMITIVE_TYPE)
class PrimitiveType(Enum):
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


@builtin_enum(EnumType.TYPE_CARDINALITY)
class TypeCardinality(Enum):
    """The 'kind' of a Type."""

    SCALAR = 1
    LIST = 2
    # SET?
    MAP = 4
    # OPTION = 5
    # LITERAL = 6
    # UNION = 7


@builtin_enum(EnumType.SCALAR_TYPE)
class ScalarType(Enum):
    """The type of a scalar."""

    PRIMITIVE = 1
    ENUM = 2
    NODE_REFERENCE = 3
    NODE_VALUE = 4
    STRUCT = 5
    # CUSTOM_ENUM, CUSTOM_STRUCT, ...


@builtin_enum(EnumType.DEFAULT_FACTORY)
class DefaultFactory(Enum):
    """The factory to use for default values."""

    UUID = 1
    NOW = 2
    REGION = 3


@builtin_enum(EnumType.ROLE_TYPE)
class RoleType(Enum):
    SYSTEM = 1
    OWNER = 2
    ADMIN = 3
    DEVELOPER = 5
    USER = 7
    SPECTATOR = 10


@builtin_enum(EnumType.RESOURCE_STATUS)
class ResourceStatus(Enum):
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


@builtin_enum(EnumType.CLIENT_TYPE)
class ClientType(Enum):
    # user
    WEB = 1
    BROWSER_PLUGIN = 2
    DESKTOP = 3
    MOBILE = 4
    # system
    MACHINE = 10


@builtin_enum(EnumType.TENANCY)
class Tenancy(Enum):
    DEDICATED = 1
    SHARED = 2


CLOUD = get_from_env("CLOUD", typ=Cloud, description="Cloud we're running in")
REGION = get_from_env("REGION", typ=Region, description="Region we're running in")
TRACING = get_from_env("TRACING", typ=bool, description="Enable tracing")


class DestackError(Exception):
    """Common base class for any regular errors."""

    pass


def repr_enums(enums: Iterable[Enum]) -> str:
    return "|".join(e.camel_name for e in enums)

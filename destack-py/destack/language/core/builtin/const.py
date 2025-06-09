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
VERSION = "2025.06.09.1"
UUID_NAMESPACE = uuid5(UUID(int=0), b"destack")
CK_LENGTH_B64 = 24  # 1.5 * CK_LENGTH_BYTES (must be integer)
FLOAT_EPSILON = 1e-6
BEGINNING_OF_TIME = datetime.fromisoformat("1970-01-01T00:00:00+00:00")

# builtin destackes :Builtins
DESTACK_SLUG = "destack"
DESTACK_ID = UUID("11111111-1111-1111-1111-000000000000")
DESTACK_DESTACK_PACKAGE_ID = UUID("11111111-1111-1111-1111-000000000001")
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
    # auth [200-600]
    SPACE_ROLE_TYPE = 201
    ORGANIZATION_ROLE_TYPE = 211
    FOLDER_ROLE_TYPE = 221
    CLIENT_TYPE = 231
    # ...

    # space [600-800]
    WINDOW_TYPE = 601
    # ...

    # history [800-1000]
    # ...

    # infra [1000-1200]
    CLOUD = 1000
    REGION = 1001
    REGION_AREA = 1002
    CONTINENT = 1003
    TENANCY = 1004
    DATABASE_TYPE = 1010
    MACHINE_TYPE = 1011
    # SEARCH_TYPE, WAREHOUSE_TYPE, ...
    # ...

    # logic [1200-1600]
    ACTION_CARDINALITY = 1221
    CURSOR_TYPE = 1321
    CURSOR_STATUS = 1322
    # ...

    # runtime [1600-2000]
    RUN_STATUS = 1611
    RUN_TYPE = 1612
    SCHEDULE_FREQUENCY = 1631
    INTERRUPTION_TYPE = 1632
    INTERRUPTION_STATUS = 1633
    INTERRUPTION_RESPONSE = 1634
    # ...

    # data [2000-2200]
    TEXT_LINE_TYPE = 2001
    TEXT_SPAN_TYPE = 2002
    FILE_RETENTION_MODE = 2021
    FILE_SOURCE = 2022
    FILE_TYPE = 2023
    FILE_FORMAT = 2024
    ICON_TYPE = 2031
    LINK_TYPE = 2051
    PRIMITIVE_TYPE = 2101
    TYPE_CARDINALITY = 2102
    SCALAR_TYPE = 2103
    DEFAULT_FACTORY = 2104
    STRING_FORMAT = 2111
    NUMBER_FORMAT = 2112
    FIELD_TYPE = 2121
    EDGE_TYPE = 2122
    EDGE_DIRECTION = 2123
    CASCADE_ACTION = 2124
    DAY = 2131
    MONTH = 2132
    TIME_INTERVAL = 2133
    RESOURCE_STATUS = 2201
    # ...

    # custom [2200-2400]
    # ...

    # social [2400-2800]
    THREAD_STATUS = 2402
    MESSAGE_TYPE = 2421
    # ...

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

    # model [4400-4600]
    MODEL_DEVELOPER = 4401
    MODEL_PROVIDER = 4402

    # ui [8000-10000]

    # space [8000-8100]
    # ...

    # container views [8100-8200]
    # ...

    # content views [8200-8300]
    # ...

    # input views [8300-8400]
    # ...

    # node views [8400-8500]
    # ...

    # internal views [8500-8600]
    # ...

    # style [9000-9100]
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

    # drawing
    # ...

    # audio/media?
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

    # auth [200-600]
    # PROFILE? (for User, or maybe global?)

    # space [600-800]
    # ...

    # history [800-1000]
    # ...

    # infra [1000-1200]
    DATABASE_INFO = 1001
    CELL_INFO = 1101

    # logic [1200-1600]
    SCHEDULE = 1201
    # ...

    # runtime [1600-2000]
    ERROR = 1601
    # ...

    # data [2000-2200]
    TYPE = 2001
    NUMBER_CONSTRAINT = 2002
    STRING_CONSTRAINT = 2003
    COLLECTION_CONSTRAINT = 2004
    NODE_CONSTRAINT = 2005
    TEXT = 2101, None, None, "fas fa-text"
    TEXT_LINE = 2102, None, None, "fas fa-text"
    TEXT_SPAN = 2103, None, None, "fas fa-text"
    ICON = 2131
    SELECTION = 2071
    # SCHEMA, UNION, TAG, ...

    # custom [2200-2400]
    VALUE = 2201
    # ...

    # social [2400-2800]
    # ...

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

    # space [8000-8100]
    # ...

    # container views [8100-8200]
    # ...

    # content views [8200-8300]
    # ...

    # input views [8300-8400]
    # ...

    # node views [8400-8500]
    # ...

    # internal views [8500-8600]
    # ...

    # style [9000-9100]
    VECTOR2 = 9000, None, None, "fas fa-vector-square"
    VECTOR3 = 9001, None, None, "fas fa-vector-square"
    VECTOR4 = 9002, None, None, "fas fa-vector-square"
    VECTOR2I = 9003, None, None, "fas fa-vector-square"
    VECTOR3I = 9004, None, None, "fas fa-vector-square"
    VECTOR4I = 9005, None, None, "fas fa-vector-square"
    AXIS2 = 9007, None, None, "fas fa-vector-square"
    AXIS3 = 9009, None, None, "fas fa-vector-square"
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

    # canvas/drawing?
    # CANVAS, BRUSH, SHAPE, ...

    # audio/media?
    # SOUND, ...?


@enum_(EnumType.NODE_TYPE)
class NodeType(BuiltinEnum):
    # space [1-200]
    SPACE = 1, "Space", "Universal Space", "https://heydestack.com/favicon.ico"
    SPACE_MEMBERSHIP = 2, "Space Membership", "Membership in a Space", "fas fa-user-group"
    SPACE_INVITE = 3, "Space Invite", "Invite to a Space", "fas fa-user-plus"
    # DESTACK_MIGRATION?
    FOLDER = 50, "Folder", "Sub-space of a Space", "fas fa-folder-open"
    FOLDER_MEMBERSHIP = 51, "Folder Membership", "Membership in a Folder", "fas fa-user-group"
    FOLDER_INVITE = 52, "Folder Invite", "Invite to a Folder", "fas fa-user-plus"
    HANDLE = 100, "Handle", "Unique @handle", "fas fa-at"
    # DEPENDENCY, PLUGIN, ...

    # auth [200-600]
    USER = 200, "User", "User", "fas fa-user"
    FRIENDSHIP = 210, "Friendship", "Friendship between two Users", "fas fa-user-friends"
    FRIENDSHIP_INVITE = (
        211,
        "Friendship Invite",
        "Invite to be friends with another User",
        "fas fa-user-plus",
    )
    ORGANIZATION = 250, "Organization", "Organization", "fas fa-building"
    ORGANIZATION_MEMBERSHIP = (
        251,
        "Organization Membership",
        "Membership in an Organization",
        "fas fa-user-group",
    )
    ORGANIZATION_INVITE = (
        252,
        "Organization Invite",
        "Invite to an Organization",
        "fas fa-user-plus",
    )
    CLIENT = 300, "Client", "Client to a Destack", "fas fa-desktop"
    # TEAM, ...
    # CREDENTIAL, ACCOUNT, PROFILE, ...
    # PERMISSION, PERMISSION_GROUP, ...
    # CHALLENGE, FRIENDSHIP, ENTITLEMENT, POLICY, RULE, KICK/BAN, ...

    # scene [600-800]
    WINDOW = 600, "Window", "Window", "fas fa-galaxy"
    SCENE = 610, "Scene", "Scene of an Application", "fas fa-masks-theater"
    ROUTE = 620, "Route", "Route to a Scene", "fas fa-route"
    # COMMAND, OVERLAY, WIDGET, ...

    # history [800-1000]
    # CHANGE, HISTORY, SNAPSHOT, OVERLAY, BRANCH, ...

    # infra [1000-1200]
    DATABASE = 1000, "Database", "Database for Postgres data", "fas fa-database"
    # SEARCH/INDEX, VAULT, CACHE, S3, ...
    # CELL = 1100, "Cell", "Cell of Destack services", "fas fa-cell"
    MACHINE = 1150, "Machine", "Machine for ephemeral computing", "fas fa-machine-classic"
    # HOST, ENDPOINT, DEPLOYMENT, NETWORK, AUTOSCALER, ...

    # logic [1200-1600]
    SCRIPT = 1200, "Script", "Script", "fas fa-code"
    # SCRIPT_FILE, ...
    SERVICE = 1210, "Service", "Service", "fas fa-screwdriver-wrench"
    ACTION = 1220, "Action", "Action", "fas fa-step-forward"
    AGENT = 1300, "Agent", "Identity for an AI", "fas fa-robot"
    TASK = 1310, "Task", "To-do item", "far fa-square-check"
    CURSOR = 1320, "Cursor", "Position in something", "fas fa-mouse"
    # TRIGGER, ...
    # TRAIT/INTERFACE, ...
    # TEST, TEST_SUITE, TEST_CASE, TEST_RESULT, ...

    # runtime [1600-2000]
    RUN = 1610, "Run", "Run", "fas fa-play"
    SPAN = 1620, "Span", "Span", "fas fa-ruler-horizontal"
    INTERRUPTION = 1630, "Interruption", "Interruption", "fas fa-hand"
    LOG = 1640, "Log", "Log", "fas fa-file-lines"
    # ROOM, JOB, PLAN, LOCK, ...
    # EVENT, SIGNAL, ...
    # TIMER, BREAKPOINT, ...
    # INSTRUMENT, MEASUREMENT,
    CUSTOM_EVENT_DEFINITION = 1700, "Signal Definition", "Signal Definition", "fas fa-signal"
    CUSTOM_EVENT = 1701, "Signal Instance", "Signal Instance", "fas fa-signal"
    EDIT_EVENT = 1710, "Edit Event", "Edit Event", "fas fa-file-lines"
    CHANGE_EVENT = 1711, "Change Event", "Change Event", "fas fa-file-lines"
    QUERY_EVENT = 1712, "Query Event", "Query Event", "fas fa-file-lines"

    # data [2000-2400]
    SCHEMA = 2000, "Schema", "Schema", "fas fa-shapes"
    FIELD = 2010, "Field", "Field", "fas fa-triangle"
    FILE = 2020, "File", "File", "fas fa-file"
    LINK = 2050, "Link", "Link to something", "fas fa-link"
    # STREAM, SECRET, ...
    CUSTOM_ENTITY_DEFINITION = (
        2200,
        "Custom Node Definition",
        "Custom Node Definition",
        "fas fa-table",
    )
    CUSTOM_ENTITY = 2201, "Custom Node Instance", "Custom Node Instance", "fas fa-database"
    # INDEX, CONSTRAINT, MIGRATION, ...

    # social [2400-2800]
    THREAD = 2400, "Thread", "Thread", "fas fa-reel"
    MESSAGE = 2420, "Message", "Message", "fas fa-message"
    # POLL, STAR, VOTE, REVIEW, RATING, RANK, REACTION, ...
    # FOLLOW, FEED, FEED_ITEM, ...
    # CHANNEL, NOTIFICATION, ...
    # ACHIEVEMENT, BADGE, WISHLIST/WATCHLIST, ...

    # product [2800-3200]
    # PREVIEW, RELEASE, ROLLOUT, ...
    # METER/METRIC, RECORDING/REPLAY, SURVEY, ...
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
    CUSTOM_VIEW_DEFINITION = (
        8110,
        "Custom View Definition",
        "Custom View Definition",
        "fas fa-table",
    )
    CUSTOM_VIEW = 8111, "Custom View", "Custom View", "fas fa-table"
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

    # NOTE :Architecture: node and internal views should probably be defined in user space?

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
    FILL_STYLE = 9011, "Fill Style", "Fill Style", "fas fa-fill"
    FONT_STYLE = 9012, "Font Style", "Font Style", "fas fa-text"
    BORDER_STYLE = 9013, "Border Style", "Border Style", "fas fa-border-outer"
    SHADOW_STYLE = 9014, "Shadow Style", "Shadow Style", "fas fa-eclipse"
    GRADIENT_STYLE = 9015, "Gradient Style", "Gradient Style", "fas fa-gradient"
    TRANSITION_STYLE = 9016, "Transition Style", "Transition Style", "fas fa-bezier-curve"
    EFFECT_STYLE = 9017, "Effect Style", "Effect Style", "fas fa-sparkle"
    # SHADER, MATERIAL, ANIMATION, ...

    # canvas/drawing?
    # CANVAS, LAYER, BITMAP, BRUSH, SHAPE, ...

    # audio/media?
    # SOUND, ...?


@enum_(EnumType.TRAIT_TYPE)
class TraitType(BuiltinEnum):
    # destack [1-200]
    # nocheckin: traits for kind/area: GLOBAL/... & ENTITY/RESOURCE/ASSET/VIEW/EVENT?
    GLOBAL = 1, "Global", "Is global", "fas fa-globe"
    # behavior
    FROZEN = 4, "Frozen", "Is frozen", "fas fa-snowflake"
    TRACKED = 10, "Tracked", "Is tracked", "fas fa-clock"
    ARCHIVABLE = 11, "Archivable", "Can be archived", "fas fa-box-archive"
    DELETABLE = 12, "Deletable", "Can be deleted", "fas fa-trash"
    TEMPLATABLE = 13, "Templatable", "Is templatable", "fas fa-puzzle-piece"
    EXTENSIBLE = 14, "Extensible", "Is extensible", "fas fa-expand"
    ORDERED = 25, "Ordered", "Is ordered", "fas fa-sort"
    CUSTOM_NODE_DEFINITION = (
        15,
        "Custom Node Definition",
        "Is a Custom Node Definition",
        "fas fa-table",
    )
    CUSTOM_NODE = 16, "Custom Node", "Is a Custom Node", "fas fa-database"
    ASSET = 51, "Resource", "Is a Resource", "fas fa-server"
    RESOURCE = 52, "Provisionable", "Is provisionable", "fas fa-server"
    ENVIRONMENTAL = 20, "Environment", "Has an environment", "fas fa-window-maximize"
    HAS_NAME = 21, "Name", "Has a name", "fas fa-font-case"
    HAS_TITLE = 22, "Title", "Has a title", "fas fa-font-case"
    HAS_SLUG = 23, "Slug", "Has a slug", "fas fa-hashtag"
    HAS_ICON = 24, "Icon", "Has an icon", "fas fa-icons"
    IN_SPACE = 40, "Space", "Is in a Space", "fas fa-destack"
    IN_PACKAGE = 41, "Package", "Is in a Package", "fas fa-box"
    # TAG, TAGGABLE, ...

    # auth [200-600]
    OWNABLE = 200, "Ownable", "Is ownable", "fas fa-user"
    JOINABLE = 202, "Joinable", "Is joinable", "fas fa-users"
    SUBJECT = 205, "Subject", "Is a Subject", "fas fa-user"
    MEMBERSHIP = 210, "Membership", "Is a Membership", "fas fa-users"
    INVITE = 211, "Invite", "Is an Invite", "fas fa-envelope"
    ROLE = 212, "Role", "Is a Role", "fas fa-user-tag"

    # space [600-800]
    # ...

    # history [800-1000]
    # ...

    # infra [1000-1200]
    # ...

    # logic [1200-1600]
    RUNNABLE = 1200, "Runnable", "Can be run", "fas fa-play"
    SCRIPTABLE = 1201, "Scriptable", "Can be scripted", "fas fa-code"
    SOURCEABLE = 1202, "Script Sourceable", "Can be sourced from a Script", "fas fa-code"
    INSTRUMENT = 1210, "Instrument", "Is an instrument", "fas fa-microscope"
    MEASUREMENT = 1211, "Measurement", "Is a measurement", "fas fa-microscope"
    EVENT = 1212, "Log", "Is a log", "fas fa-file-lines"

    # runtime [1600-2000]
    # LOG, ...

    # data [2000-2200]
    # ...

    # custom [2200-2400]
    # METRIC, ...

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


@enum_(EnumType.AREA_TYPE)
class AreaType(BuiltinEnum):
    GLOBAL_DATABASE = 1
    # GLOBAL_SEARCH?
    MAIN_DATABASE = 100
    # MAIN_SEARCH, MAIN_WAREHOUSE, ...


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
    # big general
    AWS = 10
    AZURE = 20
    GCP = 30
    HETZNER = 40
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
    def continent(self) -> Continent:
        return Continent((self.id // 1000) * 1000)

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
    def continent(self) -> Continent:
        return Continent((self.id // 1000) * 1000)

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
    Fundamental column / storage types we support (subset of SQL types, used directly in sql/core).
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

from datetime import date, datetime, time, timedelta
from typing import TYPE_CHECKING, Any, NamedTuple, TypeAliasType

from destack.utils.uuid import UUID

from .builtin import EnumType
from .enum import Enum, builtin_enum
from .types import (
    Boolean,
    Bytes,
    Cson,
    Date,
    Datetime,
    Duration,
    Float16,
    Float32,
    Float64,
    Json,
    Kompakt,
    SInt8,
    SInt16,
    SInt32,
    SInt64,
    SInt128,
    String,
    Time,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    UInt128,
)

if TYPE_CHECKING:
    pass


@builtin_enum(EnumType.PROPERTY_TYPE)
class PropertyType(Enum):
    MEMBER = 1, "Member", None, None
    CONSTANT = 2, "Constant", None, None
    # COMPUTED?
    INPUT = 10, "Input", None, None
    OUTPUT = 11, "Output", None, None


@builtin_enum(EnumType.GRAPH_KEY)
class GraphKey(Enum):
    """The role of a Graph (scope + domain + tier)."""

    ENTITY_PRIMARY = 1110
    # ENTITY_SEARCH, ENTITY_BACKUP, ...
    EVENT_PRIMARY = 2110
    # EVENT_SEARCH, EVENT_AGGREGATE, ...


@builtin_enum(EnumType.GRAPH_DOMAIN)
class GraphDomain(Enum):
    ENTITY = 1
    EVENT = 2


@builtin_enum(EnumType.RUNTIME_LANGUAGE)
class RuntimeLanguage(Enum):
    PYTHON = 1
    JAVASCRIPT = 2
    # RUST, JAVA, SWIFT, ...


@builtin_enum(EnumType.PLATFORM_TYPE)
class PlatformType(Enum):
    SYSTEM = 1
    RUNTIME = 2
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

    EUROPE = 1_000, "Europe", None, "🇪🇺"
    NORTH_AMERICA = 2_000, "North America", None, "🇺🇸"
    SOUTH_AMERICA = 3_000, "South America", None, "🇧🇷"
    MIDDLE_EAST = 4_000, "Middle East", None, "🇸🇦"
    AFRICA = 5_000, "Africa", None, "🇿🇦"
    ASIA = 6_000, "Asia", None, "🇮🇳"
    AUSTRALIA = 7_000, "Australia", None, "🇦🇺"
    PRIVATE = 9_000

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

    EUROPE_CENTRAL = 1_000, None, None, "🇪🇺"
    NORTH_AMERICA_EAST = 2_000, None, None, "🇺🇸"
    NORTH_AMERICA_WEST = 2_200, None, None, "🇺🇸"
    SOUTH_AMERICA_EAST = 3_000, None, None, "🇧🇷"
    MIDDLE_EAST_CENTRAL = 4_000, None, None, "🇸🇦"
    MIDDLE_EAST_WEST = 4_200, None, None, "🇸🇦"
    AFRICA_SOUTH = 5_000, None, None, "🇿🇦"
    ASIA_WEST = 6_000
    ASIA_SOUTH = 6_200
    ASIA_EAST = 6_400
    AUSTRALIA_SOUTH = 7_000, None, None, "🇦🇺"

    @property
    def continent(self) -> RegionContinent:
        return RegionContinent((self.id // 1_000) * 1_000)

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
# register_constant("REGION_AREA_BY_SLUG", REGION_AREA_BY_SLUG)


@builtin_enum(EnumType.REGION)
class Region(Enum):
    """Regions in an Area on a Continent."""

    # eu-central
    ZURICH = 1_000, None, None, "🇨🇭"
    FRANKFURT = 1_010, None, None, "🇩🇪"

    # na-east
    VIRGINIA = 2_000, None, None, "🇺🇸"
    OHIO = 2_010, None, None, "🇺🇸"

    # na-west
    OREGON = 2_200, None, None, "🇺🇸"

    # sa-east
    SAO_PAULO = 3_000, None, None, "🇧🇷"

    ...

    # af-south
    CAPE_TOWN = 5_000, None, None, "🇿🇦"

    # as-east
    MUMBAI = 6_000, None, None, "🇮🇳"

    # as-south
    SINGAPORE = 6_200, None, None, "🇸🇬"

    # as-east
    TOKYO = 6_400, None, None, "🇯🇵"

    # au-south
    SYDNEY = 7_000, None, None, "🇦🇺"

    @property
    def continent(self) -> RegionContinent:
        return RegionContinent((self.id // 1_000) * 1_000)

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
    DEFINITION = 10
    INSTANCE = 11
    SIDE = 20


@builtin_enum(EnumType.ENCODING)
class Encoding(Enum):
    """The encoding scheme."""

    JSON = 1, "JSON", "JSON encoding"
    CSON = 2, "CSON", "Constant folded JSON encoding"
    KOMPAKT = 11, "KOMPAKT", "KOMPAKT encoding"
    # CUSTOM, ...


@builtin_enum(EnumType.TYPE_CARDINALITY)
class TypeCardinality(Enum):
    """The 'kind' of a Type."""

    SCALAR = 1
    LIST = 2
    # TUPLE
    # SET?
    MAP = 5
    # UNION = 6


assert max(TypeCardinality) < 8, "TypeCardinality must be less than 8"  # for :Encoding


@builtin_enum(EnumType.SCALAR_TYPE)
class ScalarType(Enum):
    """The type of a scalar."""

    PRIMITIVE = 1
    ENUM = 2
    NODE_REFERENCE = 3
    NODE_VALUE = 4
    STRUCT = 5


assert max(ScalarType) < 8, "ScalarType must be less than 8"  # for :Encoding


@builtin_enum(EnumType.PRIMITIVE_TYPE)
class PrimitiveType(Enum):
    """
    A fundamental scalar data type.
    """

    # NULL/NONE?L
    BOOLEAN = 2, "Boolean", "Boolean flag", "fas fa-toggle-large-on"
    # integer
    SINT8 = 10, "SInt8", "8-bit signed integer", "fas fa-tally"
    SINT16 = 11, "SInt16", "16-bit signed integer", "fas fa-tally"
    SINT32 = 12, "SInt32", "32-bit signed integer", "fas fa-tally"
    SINT64 = 13, "SInt64", "64-bit signed integer", "fas fa-tally"
    SINT128 = 14, "SInt128", "128-bit signed integer", "fas fa-tally"
    UINT8 = 15, "UInt8", "8-bit unsigned integer", "fas fa-tally"
    UINT16 = 16, "UInt16", "16-bit unsigned integer", "fas fa-tally"
    UINT32 = 17, "UInt32", "32-bit unsigned integer", "fas fa-tally"
    UINT64 = 18, "UInt64", "64-bit unsigned integer", "fas fa-tally"
    UINT128 = 19, "UInt128", "128-bit unsigned integer", "fas fa-tally"
    # float
    FLOAT16 = 21, "Float16", "16-bit half-precision float", "fas fa-hashtag"
    FLOAT32 = 22, "Float32", "32-bit single-precision float", "fas fa-hashtag"
    FLOAT64 = 23, "Float64", "64-bit double-precision float", "fas fa-hashtag"
    # complex, other numeric, ...?
    # time
    DATETIME = 30, "Datetime", "Date & time (with timezone)", "fas fa-calendar-days"
    DATE = 31, "Date", "Date", "fas fa-calendar-days"
    TIME = 32, "Time", "Time", "fas fa-clock"
    DURATION = 33, "Duration", "Duration", "fas fa-stopwatch"
    # string
    STRING = 40, "String", "Plain text", "fas fa-font-case"
    UUID = 41, "UUID", "UUID", "fas fa-fingerprint"
    BYTES = 42, "Bytes", "Binary data", "fas fa-file-lines"
    # VECTOR?
    JSON = 45, "JSON", "JSON", "fas fa-brackets-curly"


assert max(PrimitiveType) < 256, "PrimitiveType must be less than 256"  # for :Encoding


PRIMITIVE_TYPE_BY_ANNOTATION: dict[type | TypeAliasType, PrimitiveType] = {
    # boolean
    bool: PrimitiveType.BOOLEAN,
    Boolean: PrimitiveType.BOOLEAN,
    # integer
    int: PrimitiveType.SINT64,
    SInt8: PrimitiveType.SINT8,
    SInt16: PrimitiveType.SINT16,
    SInt32: PrimitiveType.SINT32,
    SInt64: PrimitiveType.SINT64,
    SInt128: PrimitiveType.SINT128,
    UInt8: PrimitiveType.UINT8,
    UInt16: PrimitiveType.UINT16,
    UInt32: PrimitiveType.UINT32,
    UInt64: PrimitiveType.UINT64,
    UInt128: PrimitiveType.UINT128,
    # float
    float: PrimitiveType.FLOAT64,
    Float16: PrimitiveType.FLOAT16,
    Float32: PrimitiveType.FLOAT32,
    Float64: PrimitiveType.FLOAT64,
    # time
    Datetime: PrimitiveType.DATETIME,
    datetime: PrimitiveType.DATETIME,
    Date: PrimitiveType.DATE,
    date: PrimitiveType.DATE,
    Time: PrimitiveType.TIME,
    time: PrimitiveType.TIME,
    Duration: PrimitiveType.DURATION,
    timedelta: PrimitiveType.DURATION,
    # string
    str: PrimitiveType.STRING,
    String: PrimitiveType.STRING,
    UUID: PrimitiveType.UUID,
    bytes: PrimitiveType.BYTES,
    Bytes: PrimitiveType.BYTES,
    Cson: PrimitiveType.JSON,
    Json: PrimitiveType.JSON,
    Kompakt: PrimitiveType.BYTES,
}
PRIMITIVE_PY_TYPES = tuple(t for t in PRIMITIVE_TYPE_BY_ANNOTATION if isinstance(t, type))


@builtin_enum(EnumType.VALUE_FACTORY)
class ValueFactory(Enum):
    """The factory to use for generating values."""

    UUID4 = 1, "UUID4", "Generate a random UUIDv4"
    UUID7 = 2, "UUID7", "Generate a random (time-sorted) UUIDv7"
    NOW = 10, "Now", "Get the current timestamp (system)"
    EPOCH = 11, "Epoch", "Get the current logical time (system)"
    ACTOR = 12, "Actor", "Get the current Actor"
    CLIENT = 13, "Client", "Get the current Client"
    CLIENT_NONCE = 14, "ClientNonce", "Get the current Client nonce"
    REGION = 20, "Region", "Get the current Region"
    SELF = 30, "Self", "Get the current Node"
    SPACE = 31, "Space", "Get the current Space"
    BRANCH = 32, "Branch", "Get the current Branch"
    SNAPSHOT = 33, "Snapshot", "Get the current Snapshot"
    NAME = 40, "Name", "Generate a relevant name"


@builtin_enum(EnumType.ROLE_TYPE)
class RoleType(Enum):
    SYSTEM = 1
    OWNER = 2
    ADMIN = 3
    DEVELOPER = 5
    USER = 7
    SPECTATOR = 10


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


class PackedCache(NamedTuple):
    encoding: Encoding
    is_bytes: bool
    packed: Any

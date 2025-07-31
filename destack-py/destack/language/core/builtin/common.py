from datetime import date, datetime, time, timedelta
from typing import TYPE_CHECKING, TypeAliasType

from destack.utils.uuid import UUID

from .builtin import EnumType
from .enum import Enum, builtin_enum
from .types import (
    Boolean,
    Bytes,
    Date,
    Datetime,
    Duration,
    Float16,
    Float32,
    Float64,
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    Json,
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


@builtin_enum(EnumType.PROPERTY_ZONE)
class PropertyZone(Enum):
    MEMBER = 1, "Member", None, None
    INPUT = 10, "Input", None, None
    OUTPUT = 11, "Output", None, None


@builtin_enum(EnumType.GRAPH_KEY)
class GraphKey(Enum):
    """The role of a Graph (scope + domain + tier)."""

    ENTITY_PRIMARY = 1110
    # ENTITY_SEARCH, ENTITY_BACKUP, ...
    EVENT_PRIMARY = 2110
    # EVENT_SEARCH, EVENT_AGGREGATE, ...


@builtin_enum(EnumType.RUNTIME_LANGUAGE)
class RuntimeLanguage(Enum):
    PYTHON = 1
    JAVASCRIPT = 2
    # RUST, JAVA, SWIFT, ...


@builtin_enum(EnumType.PLATFORM_TYPE)
class PlatformType(Enum):
    SYSTEM = 1, "System", "The Destack system (internal server)"
    SERVER = 10, "Server", "Server environment"
    WEB = 20, "Web", "Web browser"
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
    """Encoding scheme."""

    JSON = 1, "JSON", "JSON encoding"
    JSONC = 2, "JSONC", "Constant-keyed JSON encoding"
    KOMPAKT = 3, "KOMPAKT", "Kompakt encoding"


assert len(Encoding) < 8, "Encoding must be less than 8"  # for :Encoding


@builtin_enum(EnumType.TYPE_CARDINALITY)
class TypeCardinality(Enum):
    """The order of a Type (scalar, list, map, etc.)."""

    SCALAR = 1, "Scalar", "Single value"
    LIST = 2, "List", "Dynamic sequence of homogeneous values"
    TUPLE = 3, "Tuple", "Fixed sequence of heterogeneous values"
    # ARRAY/MATRIX/NDARRAY?
    # SET? = 5, "Set", "Set of unique values (dynamic length)"
    MAP = 7, "Map", "Mapping of homogenous keys to homogeneous values"


assert max(TypeCardinality) < 8, "TypeCardinality must be less than 8"  # for :Encoding


@builtin_enum(EnumType.SCALAR_TYPE)
class ScalarType(Enum):
    """The type of a scalar (single value like primitive, enum, struct, etc.)."""

    PRIMITIVE = (
        1,
        "Primitive",
        "Primitive value (boolean, number, time, string, etc.)",
        "fas fa-hashtag",
    )
    ENUM = (
        2,
        "Enum",
        "Enum value (enumeration of options)",
        "fas fa-shapes",
    )
    NODE_REFERENCE = (
        3,
        "Node Reference",
        "Reference to a Node (NodeReference)",
        "fas fa-link",
    )
    NODE_VALUE = (
        4,
        "Node Value",
        "Value of a Node",
        "fas fa-link",
    )
    STRUCT = (
        5,
        "Struct",
        "Struct value (structured data)",
        "fas fa-shapes",
    )
    HANDLE = (
        6,
        "Handle",
        "Handle (runtime-only)",
        "fas fa-link",
    )
    # LITERAL = 7, "Literal", "Literal value (constant value)"
    # UNION = 8, "Union", "Tagged union of heterogeneous values"
    # NOTE :Incomplete: unions are annoying to handle in encoders/decoders


assert max(ScalarType) <= 8, "ScalarType must be less than 8"  # for :Encoding


@builtin_enum(EnumType.PRIMITIVE_TYPE)
class PrimitiveType(Enum):
    """
    A fundamental scalar data type.
    """

    # :PrimitiveType
    NONE = 1, "Null", "Null value", "fas fa-null"
    BOOLEAN = (
        2,
        "Boolean",
        "Boolean flag (True or False)",
        "fas fa-toggle-large-on",
    )
    # integer
    INT8 = (
        3,
        "Int8",
        "8-bit signed integer (-2^7 to 2^7-1)",
        "fas fa-tally",
    )
    INT16 = (
        4,
        "Int16",
        "16-bit signed integer (-2^15 to 2^15-1)",
        "fas fa-tally",
    )
    INT32 = (
        5,
        "Int32",
        "32-bit signed integer (-2^31 to 2^31-1)",
        "fas fa-tally",
    )
    INT64 = (
        6,
        "Int64",
        "64-bit signed integer (-2^63 to 2^63-1)",
        "fas fa-tally",
    )
    INT128 = (
        7,
        "Int128",
        "128-bit signed integer (-2^127 to 2^127-1)",
        "fas fa-tally",
    )
    # INT256, ...
    UINT8 = (
        10,
        "UInt8",
        "8-bit unsigned integer (0 to 2^8-1)",
        "fas fa-tally",
    )
    UINT16 = (
        11,
        "UInt16",
        "16-bit unsigned integer (0 to 2^16-1)",
        "fas fa-tally",
    )
    UINT32 = (
        12,
        "UInt32",
        "32-bit unsigned integer (0 to 2^32-1)",
        "fas fa-tally",
    )
    UINT64 = (
        13,
        "UInt64",
        "64-bit unsigned integer (0 to 2^64-1)",
        "fas fa-tally",
    )
    UINT128 = (
        14,
        "UInt128",
        "128-bit unsigned integer (0 to 2^128-1)",
        "fas fa-tally",
    )
    # UINT256, ...
    # float
    FLOAT16 = (
        21,
        "Float16",
        "16-bit half-precision float (±2^14)",
        "fas fa-hashtag",
    )
    FLOAT32 = (
        22,
        "Float32",
        "32-bit single-precision float (±2^127)",
        "fas fa-hashtag",
    )
    FLOAT64 = (
        23,
        "Float64",
        "64-bit double-precision float (±2^1023)",
        "fas fa-hashtag",
    )
    # COMPLEX16, COMPLEX32, COMPLEX64, ...
    # DECIMAL, ...
    # time
    DATETIME = (
        40,
        "Datetime",
        "Datetime (microsecond precision, with timezone)",
        "fas fa-calendar-days",
    )
    DATE = (
        41,
        "Date",
        "Date (day precision, no timezone)",
        "fas fa-calendar-days",
    )
    TIME = (
        42,
        "Time",
        "Time (microsecond precision, no timezone)",
        "fas fa-clock",
    )
    DURATION = (
        43,
        "Duration",
        "Duration (microsecond precision)",
        "fas fa-stopwatch",
    )
    # string
    STRING = (
        50,
        "String",
        "Plain text",
        "fas fa-font-case",
    )
    UUID = (
        51,
        "UUID",
        "Universally unique identifier (UUID4 or UUID7, 16 bytes)",
        "fas fa-fingerprint",
    )
    BYTES = (
        52,
        "Bytes",
        "Binary data (arbitrary bytes)",
        "fas fa-file-lines",
    )
    # VECTOR?
    JSON = (
        55,
        "JSON",
        "JSON (arbitrary JSON data)",
        "fas fa-brackets-curly",
    )


assert max(PrimitiveType) < 64, "PrimitiveType must be less than 64"  # for :Encoding


PRIMITIVE_TYPE_BY_ANNOTATION: dict[type | TypeAliasType, PrimitiveType] = {
    # boolean
    bool: PrimitiveType.BOOLEAN,
    Boolean: PrimitiveType.BOOLEAN,
    # integer
    int: PrimitiveType.INT64,
    Int8: PrimitiveType.INT8,
    Int16: PrimitiveType.INT16,
    Int32: PrimitiveType.INT32,
    Int64: PrimitiveType.INT64,
    Int128: PrimitiveType.INT128,
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
    Json: PrimitiveType.JSON,
}
PRIMITIVE_PY_TYPES = tuple(t for t in PRIMITIVE_TYPE_BY_ANNOTATION if isinstance(t, type))


@builtin_enum(EnumType.VALUE_FACTORY)
class ValueFactory(Enum):
    """The factory to use for generating values."""

    UUID4 = 1, "UUID4", "Generate a random UUIDv4"
    UUID7 = 2, "UUID7", "Generate a random (time-sorted) UUIDv7"
    NOW = 10, "Now", "Get the current timestamp (system)"
    REMOTE_EPOCH = 11, "Remote Epoch", "Get the current logical time (system)"
    LOCAL_EPOCH = 12, "Local Epoch", "Get the current logical time (system)"
    ACTOR = 20, "Actor", "Get the current Actor"
    CLIENT = 21, "Client", "Get the current Client"
    CLIENT_NONCE = 22, "ClientNonce", "Get the current Client nonce"
    REGION = 30, "Region", "Get the current Region"
    SELF = 40, "Self", "Get the current Node"
    SPACE = 41, "Space", "Get the current Space"
    BRANCH = 42, "Branch", "Get the current Branch"
    SNAPSHOT = 43, "Snapshot", "Get the current Snapshot"
    NAME = 50, "Name", "Generate a relevant name"


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


@builtin_enum(EnumType.CONSTRAINT_TYPE)
class ConstraintType(Enum):
    """Type of a Constraint."""

    UNIQUE = 1
    # CHECK, ...


@builtin_enum(EnumType.INDEX_TYPE)
class IndexType(Enum):
    """Type of an Index."""

    BTREE = 1
    # HASH, ...


@builtin_enum(EnumType.METHOD_TYPE)
class MethodType(Enum):
    PROPERTY = 1, "Property", "Computed property"
    INSTANCE = 2, "Instance", "Instance method"
    STATIC = 3, "Static", "Static method"


@builtin_enum(EnumType.ACTION_TYPE)
class ActionType(Enum):
    UNARY_IN_UNARY_OUT = 1, "Unary In, Unary Out", "Single in, single out"
    UNARY_IN_STREAM_OUT = 2, "Unary In, Stream Out", "Single in, stream out"
    STREAM_IN_UNARY_OUT = 3, "Stream In, Unary Out", "Stream in, single out"
    STREAM_IN_STREAM_OUT = 4, "Stream In, Stream Out", "Stream in, stream out"


@builtin_enum(EnumType.FUNCTION_OPERATOR)
class FunctionOperator(Enum):
    """The overridable builtin operators for Functions (depends on runtime language)."""

    # comparison operators
    EQ = 1, "Equals", "="
    NEQ = 2, "Not Equals", "!="
    GT = 3, "Greater Than", ">"
    LT = 4, "Less Than", "<"
    GTE = 5, "Greater Than or Equal To", ">="
    LTE = 6, "Less Than or Equal To", "<="
    IN = 7, "In", "in"
    NOT_IN = 8, "Not In", "not in"
    IS = 9, "Is", "is"
    IS_NOT = 10, "Is Not", "is not"

    # arithmetic operators
    ADD = 20, "Add", "+"
    SUB = 21, "Subtract", "-"
    MUL = 22, "Multiply", "*"
    TRUEDIV = 23, "True Divide", "/"
    FLOORDIV = 24, "Floor Divide", "//"
    MOD = 25, "Modulo", "%"
    POW = 26, "Power", "**"
    DIVMOD = 27, "Divmod", "divmod"

    # unary operators
    POS = 30, "Positive", "+"
    NEG = 31, "Negative", "-"
    ABS = 32, "Absolute", "abs"
    INVERT = 33, "Invert", "~"

    # bitwise operators
    AND = 40, "Bitwise And", "&"
    OR = 41, "Bitwise Or", "|"
    XOR = 42, "Bitwise Xor", "^"
    LSHIFT = 43, "Left Shift", "<<"
    RSHIFT = 44, "Right Shift", ">>"

    # logical operators
    BOOL = 50, "Boolean", "bool"
    NOT = 51, "Not", "not"

    # container operators
    LEN = 60, "Length", "len"
    GETITEM = 61, "Get Item", "[]"
    SETITEM = 62, "Set Item", "[]="
    DELITEM = 63, "Delete Item", "del []"
    CONTAINS = 64, "Contains", "in"
    ITER = 65, "Iterator", "iter"
    NEXT = 66, "Next", "next"

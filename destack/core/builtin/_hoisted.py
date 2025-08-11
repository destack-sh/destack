"""
Hoisted declarations which belong elsewhere but are needed early in the import chain.
"""

from datetime import date, datetime, time, timedelta
from typing import TYPE_CHECKING, TypeAliasType

from .enum import FlagEnum, OptionEnum, declare_enum, declare_option
from .types import (
    UUID,
    Boolean,
    Bytes,
    Character,
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
from .universe import EnumType

if TYPE_CHECKING:
    pass


@declare_enum(EnumType.PROPERTY_ZONE)
class PropertyZone(OptionEnum):
    MEMBER = declare_option(1)
    INPUT = declare_option(10)
    OUTPUT = declare_option(11)


@declare_enum(EnumType.RUNTIME_LANGUAGE)
class RuntimeLanguage(OptionEnum):
    """The language of the Runtime."""

    PYTHON = declare_option(1)
    TYPESCRIPT = declare_option(2)
    RUST = declare_option(3)
    # GPU/CUDA/GLSL?


@declare_enum(EnumType.RUNTIME_PLATFORM)
class RuntimePlatform(OptionEnum):
    """The platform of the Runtime."""

    CORE = declare_option(
        100,
        "Core",
        description="Core platform",
    )
    SYSTEM = declare_option(
        200,
        "System",
        description="Destack system (internal)",
    )
    SERVER = declare_option(
        300,
        "Server",
        description="Server environment",
    )
    WEB = declare_option(
        400,
        "Web",
        description="Web browser",
    )
    MOBILE = declare_option(
        500,
        "Mobile",
        description="Mobile device",
    )
    DESKTOP = declare_option(
        600,
        "Desktop",
        description="Desktop computer",
    )
    # EMAIL, AR/VR/XR, ...


@declare_enum(EnumType.RUNTIME_TYPE)
class RuntimeType(OptionEnum):
    """The specific Runtime (RuntimeLanguage x RuntimePlatform)."""

    CORE_PYTHON = declare_option(101, "destack-py", description="Destack Python library")
    CORE_TYPESCRIPT = declare_option(102, "destack-ts", description="Destack TypeScript library")
    CORE_RUST = declare_option(103, "destack-rs", description="Destack Rust library")
    SYSTEM_TYPESCRIPT = declare_option(
        202, "destack-ts-system", description="Destack TypeScript system runtime (internal)"
    )
    SYSTEM_RUST = declare_option(
        203, "destack-rs-system", description="Destack Rust system runtime (internal)"
    )
    SERVER_PYTHON = declare_option(
        301, "destack-py-server", description="Destack Python server runtime"
    )
    SERVER_TYPESCRIPT = declare_option(
        302, "destack-ts-server", description="Destack TypeScript server runtime"
    )
    WEB_TYPESCRIPT = declare_option(
        402, "destack-ts-web", description="Destack TypeScript web runtime"
    )


@declare_enum(EnumType.REGION_CONTINENT)
class RegionContinent(OptionEnum):
    """
    'Continents' of Regions.
    """

    EUROPE = declare_option(1_000, "Europe")
    NORTH_AMERICA = declare_option(2_000, "North America")
    SOUTH_AMERICA = declare_option(3_000, "South America")
    MIDDLE_EAST = declare_option(4_000, "Middle East")
    AFRICA = declare_option(5_000, "Africa")
    ASIA = declare_option(6_000, "Asia")
    AUSTRALIA = declare_option(7_000, "Australia")


@declare_enum(EnumType.REGION_AREA)
class RegionArea(OptionEnum):
    """
    A larger Area of Regions within a Continent.
    """

    EUROPE_CENTRAL = declare_option(1_000)
    NORTH_AMERICA_EAST = declare_option(2_000)
    NORTH_AMERICA_WEST = declare_option(2_200)
    SOUTH_AMERICA_EAST = declare_option(3_000)
    MIDDLE_EAST_CENTRAL = declare_option(4_000)
    MIDDLE_EAST_WEST = declare_option(4_200)
    AFRICA_SOUTH = declare_option(5_000)
    ASIA_WEST = declare_option(6_000)
    ASIA_SOUTH = declare_option(6_200)
    ASIA_EAST = declare_option(6_400)
    AUSTRALIA_SOUTH = declare_option(7_000)


@declare_enum(EnumType.REGION)
class Region(OptionEnum):
    """Regions in an Area on a Continent."""

    # eu-central
    ZURICH = declare_option(1_000)
    FRANKFURT = declare_option(1_010)

    # na-east
    VIRGINIA = declare_option(2_000)
    OHIO = declare_option(2_010)

    # na-west
    OREGON = declare_option(2_200)

    # sa-east
    SAO_PAULO = declare_option(3_000)

    # af-south
    CAPE_TOWN = declare_option(5_000)

    # as-east
    MUMBAI = declare_option(6_000)

    # as-south
    SINGAPORE = declare_option(6_200)

    # as-east
    TOKYO = declare_option(6_400)

    # au-south
    SYDNEY = declare_option(7_000)


@declare_enum(EnumType.REFERENCE_TYPE)
class ReferenceType(OptionEnum):
    """The type of a Node reference."""

    UNTYPED_IDENTITY = declare_option(
        1,
        description="Identity reference (id, for internal use)",
        is_internal=True,
    )
    IDENTITY = declare_option(
        2,
        description="Identity reference (type + id, for internal use)",
        is_internal=True,
    )
    LOCATION = declare_option(
        3,
        description="Identity + Space reference (type + id + space, assumed time)",
    )
    MOMENT = declare_option(
        4,
        description="Identity + Space + time reference (type + id + space + time)",
    )


@declare_enum(EnumType.ENCODING)
class Encoding(OptionEnum):
    """Encoding scheme."""

    JSON = declare_option(1, "JSON", description="JSON encoding")
    KOMPAKT = declare_option(3, "KOMPAKT", description="Kompakt encoding (optimized for size)")
    # BREIT = declare_option(4, "BREIT", description="Breit encoding (optimized for speed)")
    # KONSTANT, ...
    # C?


@declare_enum(EnumType.ENCODER_FLAG)
class EncoderFlag(FlagEnum):
    """Flags for Encoders."""

    DEFAULT = declare_option(0)
    # whether to omit the metatype of the object (if possible)
    OMIT_METATYPE = declare_option(1)
    # whether to omit the key of properties (if possible)
    # OMIT_KEY = declare_optiouiltn(1 << 1)
    # whether to omit the type of the value (if possible)
    # OMIT_TYPE = declare_option(1 << 2)
    # whether to omit None values (if possible)
    OMIT_NONE = declare_option(1 << 3)
    # whether to unwrap Values (if possible)
    UNWRAP_VALUE = declare_option(1 << 4)


@declare_enum(EnumType.ENCODER_STABILITY)
class EncoderStability(OptionEnum):
    """Stability of an Encoder's encoded format."""

    DYNAMIC = declare_option(1, "Dynamic", description="Can handle version drift")
    STATIC = declare_option(7, "Static", description="Assumes identical versions")


assert len(Encoding) < 8, "Encoding must be less than 8"  # for :Encoding


@declare_enum(EnumType.TYPE_CARDINALITY)
class TypeCardinality(OptionEnum):
    """The order of a Type (scalar, list, map, etc.)."""

    SCALAR = declare_option(1, "Scalar", description="Single value")
    LIST = declare_option(2, "List", description="Dynamic sequence of homogeneous values")
    TUPLE = declare_option(3, "Tuple", description="Fixed sequence of heterogeneous values")
    # ARRAY = 4, "Array", "Dense multi-dimensional array of homogeneous values"
    # SPARSE_ARRAY = 5?, "Sparse Array", "Sparse multi-dimensional array of homogeneous values"
    # SET = 9?, "Set", "Set of unique values"
    MAP = declare_option(10, "Map", description="Mapping of homogenous keys to homogeneous values")


assert max(TypeCardinality) < 16, "TypeCardinality must be less than 8"  # for :Encoding


@declare_enum(EnumType.SCALAR_TYPE)
class ScalarType(OptionEnum):
    """The type of a scalar (single value like primitive, enum, struct, etc.)."""

    PRIMITIVE = declare_option(
        1,
        "Primitive",
        description="Primitive value (boolean, number, time, string, etc.)",
    )
    ENUM = declare_option(
        2,
        "Enum",
        description="Enum value (enumeration of options)",
    )
    NODE = declare_option(
        3,
        "Node",
        description="Node as a value (Node)",
    )
    NODE_UNTYPED_IDENTITY = declare_option(
        4,
        "Node Naked Identity",
        description="Reference to a Node (id only, for internal use)",
    )
    NODE_IDENTITY = declare_option(
        5,
        "Node Typed Identity",
        description="Reference to a Node (type + id, assumed Space)",
    )
    NODE_LOCATION = declare_option(
        6,
        "Node Location",
        description="Reference to a Node (type + id + space, assumed time)",
    )
    NODE_MOMENT = declare_option(
        7,
        "Node Moment",
        description="Reference to a Node (type + id + space + time)",
    )
    STRUCT = declare_option(
        8,
        "Struct",
        description="Struct value (structured data)",
    )
    HANDLE = declare_option(
        9,
        "Handle",
        description="Handle (runtime-only)",
    )
    UNION = declare_option(
        10,
        "Union",
        description="Tagged union of heterogeneous values",
    )


assert max(ScalarType) < 16, "ScalarType must be less than 16"  # for :Encoding


@declare_enum(EnumType.PRIMITIVE_TYPE)
class PrimitiveType(OptionEnum):
    """
    A fundamental scalar data type.
    """

    # :PrimitiveType
    NONE = declare_option(1, "Null", description="Null value")
    BOOLEAN = declare_option(
        2,
        "Boolean",
        description="""\
Boolean flag (True or False, 1 byte)
Range: False, True
""",
    )
    # integer
    INT8 = declare_option(
        3,
        "Int8",
        description="""\
8-bit signed integer
Range: -2^7 to 2^7-1
""",
    )
    INT16 = declare_option(
        4,
        "Int16",
        description="""\
16-bit signed integer
Range: -2^15 to 2^15-1
""",
    )
    INT32 = declare_option(
        5,
        "Int32",
        description="""\
32-bit signed integer
Range: -2^31 to 2^31-1
""",
    )
    INT64 = declare_option(
        6,
        "Int64",
        description="""\
64-bit signed integer
Range: -2^63 to 2^63-1
""",
    )
    INT128 = declare_option(
        7,
        "Int128",
        description="""\
128-bit signed integer
Range: -2^127 to 2^127-1
""",
    )
    # INT256, ...
    UINT8 = declare_option(
        10,
        "UInt8",
        description="""\
8-bit unsigned integer
Range: 0 to 2^8-1
""",
    )
    UINT16 = declare_option(
        11,
        "UInt16",
        description="""\
16-bit unsigned integer
Range: 0 to 2^16-1
""",
    )
    UINT32 = declare_option(
        12,
        "UInt32",
        description="""\
32-bit unsigned integer
Range: 0 to 2^32-1
""",
    )
    UINT64 = declare_option(
        13,
        "UInt64",
        description="""\
64-bit unsigned integer
Range: 0 to 2^64-1
""",
    )
    UINT128 = declare_option(
        14,
        "UInt128",
        description="""\
128-bit unsigned integer
Range: 0 to 2^128-1
""",
    )
    # UINT256, ...
    # float
    # FLOAT4, FLOAT8, ...
    FLOAT16 = declare_option(
        22,
        "Float16",
        description="""\
16-bit half-precision float
Range: ±2^-14 to ±2^15-1
""",
    )
    FLOAT32 = declare_option(
        23,
        "Float32",
        description="""\
32-bit single-precision float
Range: ±2^-126 to ±2^127-1
""",
    )
    FLOAT64 = declare_option(
        24,
        "Float64",
        description="""\
64-bit double-precision float
Range: ±2^-1022 to ±2^1023-1
""",
    )
    # COMPLEX16, COMPLEX32, COMPLEX64, ...
    # DECIMAL, ...
    # time
    DATETIME = declare_option(
        40,
        "Datetime",
        description="""\
Datetime in signed 64-bit microsecond precision since epoch (UTC)
Range: ±292,277 years
""",
    )
    DATE = declare_option(
        41,
        "Date",
        description="""\
Date in signed 64-bit day precision since epoch
Range: ±2.525x10^16 days
""",
    )
    TIME = declare_option(
        42,
        "Time",
        description="""\
Time in unsigned 64-bit nanosecond precision
Range: 00:00:00.000000000 to 23:59:59.999999999
""",
    )
    DURATION = declare_option(
        43,
        "Duration",
        description="""\
Duration in signed 64-bit nanosecond precision
Range: ±292.277 years
""",
    )
    # DATETIME_WITH_ZONE, TIME_WITH_ZONE, ...
    # string
    STRING = declare_option(
        50,
        "String",
        description="""\
Plain text (UTF-8, 32-bit variable length)
Range: 0 to 2^32-1
""",
    )
    CHARACTER = declare_option(
        51,
        "Character",
        description="""\
Single character (UTF-8, 32-bit)
Range: 0 to 2^32-1
""",
    )
    UUID = declare_option(
        55,
        "UUID",
        description="""\
Universally unique identifier (UUID4, UUID5 or UUID7, 128-bit)
Range: 0 to 2^128-1
""",
    )
    BYTES = declare_option(
        56,
        "Bytes",
        description="""\
Binary data (32-bit variable length)
Range: 0 to 2^32-1
""",
    )
    # VECTOR?
    JSON = declare_option(
        557,
        "JSON",
        description="""\
JSON (32-bit variable length)
Range: 0 to 2^32-1
""",
    )


# number of primitive kinds must fit into 6 bits for compact encoding
assert len(PrimitiveType) < 64, "PrimitiveType count must be less than 64"  # for :Encoding


PRIMITIVE_TYPE_BY_ANNOTATION: dict[type | TypeAliasType, PrimitiveType] = {
    # boolean
    type(None): PrimitiveType.NONE,
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
    Character: PrimitiveType.CHARACTER,
    UUID: PrimitiveType.UUID,
    bytes: PrimitiveType.BYTES,
    Bytes: PrimitiveType.BYTES,
    bytearray: PrimitiveType.BYTES,
    Json: PrimitiveType.JSON,
}
PRIMITIVE_PY_ANNOTATION_BY_TYPE: dict[PrimitiveType, type | TypeAliasType] = {
    v: k for k, v in PRIMITIVE_TYPE_BY_ANNOTATION.items()
}
if len(PRIMITIVE_PY_ANNOTATION_BY_TYPE) != len(PrimitiveType):
    missing_types = [t for t in PrimitiveType if t not in PRIMITIVE_PY_ANNOTATION_BY_TYPE]
    raise ValueError(f"missing primitive type annotations: {missing_types!r}")


@declare_enum(EnumType.VALUE_FACTORY)
class ValueFactory(OptionEnum):
    """The factory to use for generating values."""

    UUID4 = declare_option(1, "UUID4", description="Generate a random UUIDv4")
    UUID7 = declare_option(2, "UUID7", description="Generate a random (time-sorted) UUIDv7")
    NOW = declare_option(10, "Now", description="Get the current timestamp (system)")
    REMOTE_EPOCH = declare_option(
        11, "Remote Epoch", description="Get the current logical time (system)"
    )
    LOCAL_EPOCH = declare_option(
        12, "Local Epoch", description="Get the current logical time (system)"
    )
    ACTOR = declare_option(20, "Actor", description="Get the current Actor")
    CLIENT = declare_option(21, "Client", description="Get the current Client")
    CLIENT_NONCE = declare_option(22, "ClientNonce", description="Get the current Client nonce")
    REGION = declare_option(30, "Region", description="Get the current Region")
    SELF = declare_option(40, "Self", description="Get the current Node")
    SPACE = declare_option(41, "Space", description="Get the current Space")
    BRANCH = declare_option(42, "Branch", description="Get the current Branch")
    SNAPSHOT = declare_option(43, "Snapshot", description="Get the current Snapshot")
    NAME = declare_option(50, "Name", description="Generate a relevant name")


@declare_enum(EnumType.ROLE_TYPE)
class RoleType(OptionEnum):
    SYSTEM = declare_option(1)
    OWNER = declare_option(2)
    ADMIN = declare_option(3)
    DEVELOPER = declare_option(5)
    USER = declare_option(7)
    SPECTATOR = declare_option(10)


@declare_enum(EnumType.CLIENT_TYPE)
class ClientType(OptionEnum):
    # user
    WEB = declare_option(1)
    BROWSER_PLUGIN = declare_option(2)
    DESKTOP = declare_option(3)
    MOBILE = declare_option(4)
    # system
    MACHINE = declare_option(10)


@declare_enum(EnumType.CONSTRAINT_TYPE)
class ConstraintType(OptionEnum):
    """Type of a Constraint."""

    UNIQUE = declare_option(1)
    # CHECK, ...


@declare_enum(EnumType.INDEX_TYPE)
class IndexType(OptionEnum):
    """Type of an Index."""

    BTREE = declare_option(1)
    # HASH, ...


@declare_enum(EnumType.METHOD_TYPE)
class MethodType(OptionEnum):
    # nocheckin: property Method -> computed property?
    #  (like for Context computed properties or Entity.is_partial?)
    PROPERTY = declare_option(1, "Property", description="Computed property")
    INSTANCE = declare_option(2, "Instance", description="Instance method")
    CLASS = declare_option(3, "Class", description="Class method")
    STATIC = declare_option(4, "Static", description="Static method")


@declare_enum(EnumType.ACTION_TYPE)
class ActionType(OptionEnum):
    UNARY_IN_UNARY_OUT = declare_option(
        1, "Unary In, Unary Out", description="Single in, single out"
    )
    UNARY_IN_STREAM_OUT = declare_option(
        2, "Unary In, Stream Out", description="Single in, stream out"
    )
    STREAM_IN_UNARY_OUT = declare_option(
        3, "Stream In, Unary Out", description="Stream in, single out"
    )
    STREAM_IN_STREAM_OUT = declare_option(
        4, "Stream In, Stream Out", description="Stream in, stream out"
    )


@declare_enum(EnumType.FUNCTION_OPERATOR)
class FunctionOperator(OptionEnum):
    """The overridable builtin operators for Functions (depends on runtime language)."""

    # comparison operators
    EQ = declare_option(1, "Equals", description="=")
    NEQ = declare_option(2, "Not Equals", description="!=")
    GT = declare_option(3, "Greater Than", description=">")
    LT = declare_option(4, "Less Than", description="<")
    GTE = declare_option(5, "Greater Than or Equal To", description=">=")
    LTE = declare_option(6, "Less Than or Equal To", description="<=")
    IN = declare_option(7, "In", description="in")
    NOT_IN = declare_option(8, "Not In", description="not in")

    # arithmetic operators
    ADD = declare_option(20, "Add", description="+")
    SUB = declare_option(21, "Subtract", description="-")
    MUL = declare_option(22, "Multiply", description="*")
    TRUEDIV = declare_option(23, "True Divide", description="/")
    FLOORDIV = declare_option(24, "Floor Divide", description="//")
    MOD = declare_option(25, "Modulo", description="%")
    POW = declare_option(26, "Power", description="**")
    DIVMOD = declare_option(27, "Divmod", description="divmod")

    # unary operators
    POS = declare_option(30, "Positive", description="+")
    NEG = declare_option(31, "Negative", description="-")
    ABS = declare_option(32, "Absolute", description="abs")
    INVERT = declare_option(33, "Invert", description="~")

    # bitwise operators
    AND = declare_option(40, "Bitwise And", description="&")
    OR = declare_option(41, "Bitwise Or", description="|")
    XOR = declare_option(42, "Bitwise Xor", description="^")
    LSHIFT = declare_option(43, "Left Shift", description="<<")
    RSHIFT = declare_option(44, "Right Shift", description=">>")

    # logical operators
    BOOL = declare_option(50, "Boolean", description="bool")
    NOT = declare_option(51, "Not", description="not")

    # container operators
    LEN = declare_option(60, "Length", description="len")
    GETITEM = declare_option(61, "Get Item", description="[]")
    SETITEM = declare_option(62, "Set Item", description="[]=")
    DELITEM = declare_option(63, "Delete Item", description="del []")
    CONTAINS = declare_option(64, "Contains", description="in")
    ITER = declare_option(65, "Iterator", description="iter")
    NEXT = declare_option(66, "Next", description="next")

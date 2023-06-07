import enum
from dataclasses import dataclass

from bench.language import wire


class FieldType(enum.StrEnum):
    """
    An OpenSearch field type.
    In order of https://opensearch.org/docs/2.5/field-types/index/
    """

    ALIAS = "alias"
    BINARY = "binary"
    # numeric
    BYTE = "byte"
    DOUBLE = "double"
    FLOAT = "float"
    HALF_FLOAT = "half_float"
    INTEGER = "integer"
    LONG = "long"
    UNSIGNED_LONG = "unsigned_long"
    SHORT = "short"
    # boolean
    BOOLEAN = "boolean"
    # other
    DATE = "date"
    IP = "ip"
    # range
    INTEGER_RANGE = "integer_range"
    LONG_RANGE = "long_range"
    DOUBLE_RANGE = "double_range"
    FLOAT_RANGE = "float_range"
    IP_RANGE = "ip_ange"
    DATE_RANGE = "date_range"
    # object
    OBJECT = "object"
    NESTED = "nested"
    JOIN = "join"
    # string
    KEYWORD = "keyword"
    TEXT = "text"
    TOKEN_COUNT = "token_count"
    COMPLETION = "completion"
    SEARCH_AS_YOU_TYPE = "search_as_you_type"
    # geographic
    GEO_POINT = "geo_point"
    GEO_SHAPE = "geo_shape"
    # cartesian
    XY_POINT = "xy_point"
    XY_SHAPE = "xy_shape"
    # rank
    RANK_FEATURE = "rank_feature"
    RANK_FEATURES = "rank_features"
    # knn
    KNN_VECTOR = "knn_vector"


@dataclass(repr=False, slots=True)
class Field:
    """
    An OpenSearch field.
    See https://opensearch.org/docs/2.5/field-types/mappings/
    and
    """

    type: FieldType
    ignore_malformed: bool = True
    fields: dict[str, "Field"] = None
    properties: dict[str, "Field"] = None
    meta: dict[str, str] = None
    index: bool = True
    store: bool = True
    coerce: bool = True
    copy_to: list[str] = None
    ignore_above: int = None


@dataclass(repr=False, slots=True)
class Vector:
    """
    The value of an KNN vector field.
    """

    dims: int
    values: list[float]


@dataclass(repr=False, slots=True)
class XYPoint:
    """
    The value of a geo_point field.
    """

    x: float
    y: float


@dataclass(repr=False, slots=True)
class XYShape:
    """
    The value of a geo_shape field.
    """

    type: str
    coordinates: list[list[float]]


Record = wire.RecordData

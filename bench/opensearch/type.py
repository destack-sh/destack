import enum
from dataclasses import dataclass
from typing import Any
from uuid import UUID

import opensearchpy as os


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

    @property
    def coercible(self) -> bool:
        return self in COERCIBLE_TYPES

    @property
    def can_ignore_malformed(self) -> bool:
        return self in IGNORE_MALFORMED_TYPES


COERCIBLE_TYPES = {
    FieldType.BYTE,
    FieldType.DOUBLE,
    FieldType.FLOAT,
    FieldType.HALF_FLOAT,
    FieldType.INTEGER,
    FieldType.LONG,
    FieldType.UNSIGNED_LONG,
    FieldType.SHORT,
}

IGNORE_MALFORMED_TYPES = {
    FieldType.BYTE,
    FieldType.DOUBLE,
    FieldType.FLOAT,
    FieldType.HALF_FLOAT,
    FieldType.INTEGER,
    FieldType.LONG,
    FieldType.UNSIGNED_LONG,
    FieldType.SHORT,
    FieldType.DATE,
}


@dataclass(repr=False, slots=True)
class Field:
    """
    An OpenSearch field.
    See https://opensearch.org/docs/2.5/field-types/mappings/
    and https://www.elastic.co/guide/en/elasticsearch/reference/current/mapping-types.html
    """

    type: FieldType
    fields: dict[str, "Field"] = None
    properties: dict[str, "Field"] = None
    meta: dict[str, str] = None
    index: bool = None  # default: true
    store: bool = None  # default: true
    coerce: bool = None
    copy_to: list[str] = None
    ignore_malformed: bool = None
    ignore_above: int = None

    def __post_init__(self):
        if self.coerce is None and self.type.coercible:
            self.coerce = True
        if self.ignore_malformed is None and self.type.can_ignore_malformed:
            self.ignore_malformed = True

    def __str__(self):
        fields_str = ",˜".join(f"{k}={v}" for k, v in self.fields.items())
        if fields_str:
            return f"{self.type} (fields={fields_str})"
        properties_str = ",˜".join(f"{k}={v}" for k, v in self.properties.items())
        if properties_str:
            return f"{self.type} (properties={properties_str})"
        return f"{self.type}"

    def __repr__(self):
        return f"<Field {self.type}>"

    def __eq__(self, other):
        return self.to_dict() == other.to_dict()  # not efficient, I know

    def to_dict(self) -> dict[str, Any]:
        """
        Convert this field to a dict wireable to OpenSearch.
        Empty fields are omitted.
        """
        d = {"type": self.type.value}
        if self.ignore_malformed is not None:
            d["ignore_malformed"] = self.ignore_malformed
        if self.fields is not None:
            d["fields"] = {k: v.to_dict() for k, v in self.fields.items()}
        if self.properties is not None:
            d["properties"] = {k: v.to_dict() for k, v in self.properties.items()}
        if self.meta is not None:
            d["meta"] = self.meta
        if self.index is not None:
            d["index"] = self.index
        if self.store is not None:
            d["store"] = self.store
        if self.coerce is not None:
            d["coerce"] = self.coerce
        if self.copy_to is not None:
            d["copy_to"] = self.copy_to
        if self.ignore_above is not None:
            d["ignore_above"] = self.ignore_above
        return d

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "Field":
        """
        Convert a dict wireable from OpenSearch to a Field.
        """
        type = FieldType(d.pop("type"))
        fields = d.pop("fields", None)
        properties = d.pop("properties", None)
        return cls(
            type=type,
            fields={k: cls.from_dict(v) for k, v in fields.items()} if fields else None,
            properties={k: cls.from_dict(v) for k, v in properties.items()} if properties else None,
            **d,
        )


# The value of a KNN vector field.
Vector = list[float]


@dataclass(repr=False, slots=True)
class XYPoint:
    """
    The value of a geo_point field.
    """

    x: float
    y: float


class XYShapeType(enum.StrEnum):
    """
    The type of a geo_shape field.
    """

    POINT = "point"
    LINE_STRING = "line_string"
    POLYGON = "polygon"
    MULTI_POINT = "multi_point"
    MULTI_LINE_STRING = "multi_line_string"
    MULTI_POLYGON = "multi_polygon"
    GEOMETRY_COLLECTION = "geometry_collection"
    ENVELOPE = "envelope"


@dataclass(repr=False, slots=True)
class XYShape:
    """
    The value of a geo_shape field.
    """

    type: XYShapeType
    coordinates: list[list[float]]


class Document(os.Document):
    """
    Custom OpenSearch document for using ouw own Fields etc.
    Also prevent ORM / index mutations.
    """

    @classmethod
    def fields(cls) -> dict[str, Field]:
        """
        Return the fields of this document.
        """
        properties = cls._doc_type.mapping.to_dict().get("properties", {})
        return {k: Field.from_dict(v) for k, v in properties.items()}


class IndexType(enum.StrEnum):
    """The index type within Bench."""

    GLOBAL = "global"
    PROJECT = "project"
    DATASETS = "datasets"
    SESSIONS = "sessions"

    @property
    def is_project_scoped(self) -> bool:
        return self in (IndexType.PROJECT, IndexType.DATASETS, IndexType.SESSIONS)

    def get_index_name(self, project_id: UUID = None):
        if self.is_project_scoped != (project_id is not None):
            raise ValueError(f"project scoped index -> project id: {self} -> {project_id}")
        if self.is_project_scoped:
            return f"bench-user-{project_id}-{self.value}"
        else:
            return f"bench-{self.value}"

import dataclasses
import enum
import inspect
import typing
from dataclasses import dataclass
from datetime import datetime
from typing import Any, ClassVar
from uuid import UUID


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
        return self in MALFORMABLE_TYPES


FT = FieldType

COERCIBLE_TYPES = {
    FT.BYTE,
    FT.DOUBLE,
    FT.FLOAT,
    FT.HALF_FLOAT,
    FT.INTEGER,
    FT.LONG,
    FT.UNSIGNED_LONG,
    FT.SHORT,
}

MALFORMABLE_TYPES = {
    FT.BYTE,
    FT.DOUBLE,
    FT.FLOAT,
    FT.HALF_FLOAT,
    FT.INTEGER,
    FT.LONG,
    FT.UNSIGNED_LONG,
    FT.SHORT,
    FT.DATE,
}


@dataclass(repr=False, slots=True)
class Field:
    """
    An OpenSearch field.
    See https://opensearch.org/docs/2.5/field-types/mappings/
    and https://www.elastic.co/guide/en/elasticsearch/reference/current/mapping-types.html
    """

    type: FT
    fields: dict[FieldType, "Field"] = None
    properties: dict[str, "Field"] = None
    meta: dict[str, str] = None
    index: bool = None  # default: true
    store: bool = None  # default: true
    coerce: bool = None
    dynamic: bool = None
    copy_to: list[str] = None
    ignore_malformed: bool = None
    ignore_above: int = None
    analyzer: "Analyzer" = None

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
            d["fields"] = {k.value: v.to_dict() for k, v in self.fields.items()}
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
        if self.dynamic is not None:
            d["dynamic"] = self.dynamic
        if self.copy_to is not None:
            d["copy_to"] = self.copy_to
        if self.ignore_above is not None:
            d["ignore_above"] = self.ignore_above
        if self.analyzer is not None:
            d["analyzer"] = self.analyzer.value
        return d

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "Field":
        """
        Convert a dict wireable from OpenSearch to a Field.
        """
        type = FT(d.pop("type"))
        fields = d.pop("fields", None)
        properties = d.pop("properties", None)
        return cls(
            type=type,
            fields={k: cls.from_dict(v) for k, v in fields.items()} if fields else None,
            properties={k: cls.from_dict(v) for k, v in properties.items()} if properties else None,
            **d,
        )


field = Field


class Document:
    """
    Base for OpenSearch-style dataclass document.
    """

    Partial: ClassVar[typing.Type["Document"]] = None
    fields: ClassVar[dict[str, Field]] = {}

    id: UUID = field(type=FT.KEYWORD)

    def to_dict(self) -> dict[str, Any]:
        """
        Convert this document to a dict wireable to OpenSearch.
        We convert top level fields to JSON-able types - inner fields are left as is,
         as they are all user-defined and thus already JSON-able.
        """
        d = {}
        for name, field in self.fields.items():
            value = getattr(self, name)
            if value is None:
                continue
            if isinstance(value, datetime):
                value = value.isoformat()
            elif isinstance(value, UUID):
                value = str(value)
            elif isinstance(value, enum.Enum):
                value = value.value
            d[name] = value
        return d

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> "Document":
        """
        Convert a dict wireable from OpenSearch to a document, converting to pythonic types.
        """
        d = {**d}
        for name, field in cls.fields.items():
            value = d.pop(name, None)
            if value is None:
                continue
            if field.type == FT.DATE:
                value = datetime.fromisoformat(value)
            elif field.type == FT.KEYWORD:
                value = UUID(value)
            elif field.type == FT.TEXT:
                value = value
            d[name] = value
        return cls(**d)


PYTHON_RESERVED_NAMES = {
    "__annotations__",
    "__dict__",
    "__weakref__",
    "__slots__",
    "__doc__",
    "__module__",
    "__qualname__",
    "__parameters__",
}


def document(cls: typing.Optional[typing.Type[Document]] = None):
    """Decorator for mapping a class as an OpenSearch-style dataclass."""

    def decorator(cls: typing.Type[Document]):
        # first convert the fields to dataclass fields (and store the original fields)
        fields = {}
        for name, field in cls.__dict__.items():
            # ignore reserved names
            if name in PYTHON_RESERVED_NAMES:
                continue
            # ignore methods
            if inspect.isfunction(field):
                continue
            if not isinstance(field, Field):
                raise TypeError(f"{name} is not a Field in {cls.__name__}")
            fields[name] = field
        # remove fields values
        for name in fields.keys():
            delattr(cls, name)
        # then convert the class to a dataclass
        cls = dataclasses.dataclass(cls, repr=False, slots=True)
        # then add the fields back
        cls.fields = fields
        # add a partial class with all fields optional (copy and set fields with default None)
        partial_fields = {
            **{name: dataclasses.field(default=None) for name, field in fields.items()},
        }
        partial_cls = type(cls.__name__ + "Partial", (object,), partial_fields)
        cls.Partial = partial_cls
        return cls

    if cls is None:
        return decorator
    else:
        return decorator(cls)


if typing.TYPE_CHECKING:
    document = dataclasses.dataclass


class Analyzer(enum.StrEnum):
    HTML = "html"


ANALYZERS = {Analyzer.HTML: {"tokenizer": "standard", "char_filter": ["html_strip"]}}


class IndexType(enum.StrEnum):
    """The index type within Bench."""

    GLOBAL = "global"
    PROJECT = "project"
    DATASETS = "datasets"
    SESSIONS = "sessions"

    @property
    def is_project_scoped(self) -> bool:
        return self in (IndexType.DATASETS, IndexType.SESSIONS)

    def get_index_name(self, project_id: UUID = None):
        if self.is_project_scoped != (project_id is not None):
            raise ValueError(f"project scoped index -> project id: {self} -> {project_id}")
        if self.is_project_scoped:
            return f"bench-user-{project_id}-{self.value}"
        else:
            return f"bench-{self.value}"

import dataclasses
import enum
import inspect
import typing
from dataclasses import dataclass
from datetime import datetime
from typing import Any, ClassVar
from uuid import UUID

from bench.bench.query import SubfieldType


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


@dataclass
class Field:
    """
    An OpenSearch field.
    See https://opensearch.org/docs/2.5/field-types/mappings/
    and https://www.elastic.co/guide/en/elasticsearch/reference/current/mapping-types.html
    """

    type: FT
    fields: dict[SubfieldType, "Field"] = None
    properties: dict[str, "Field"] = None
    meta: dict[str, str] = None
    index: bool = None  # default: true
    store: bool = None  # default: true
    coerce: bool = None
    dynamic: bool | typing.Literal["strict"] = None
    copy_to: str | list[str] = None
    ignore_malformed: bool = None
    ignore_above: int = None
    analyzer: "Analyzer" = None
    can_set_directly: bool = True
    dimension: int = None  # for knn_vector
    method: "KnnMethod" = None  # for knn_vector
    _annotation: Any = None  # type annotation on the LHS of a field in a document

    def __post_init__(self):
        if self.coerce is None and self.type.coercible:
            self.coerce = True
        if self.ignore_malformed is None and self.type.can_ignore_malformed:
            self.ignore_malformed = True

    def __str__(self):
        fields_str = ",˜".join(f"{k}={v}" for k, v in (self.fields or {}).items())
        if fields_str:
            return f"{self.type} (fields={fields_str})"
        properties_str = ",˜".join(f"{k}={v}" for k, v in (self.properties or {}).items())
        if properties_str:
            return f"{self.type} (properties={properties_str})"
        return f"{self.type}"

    def __repr__(self):
        return f"<Field {self.type}>"

    def __eq__(self, other):
        return id(self) == id(other) or self.to_dict() == other.to_dict()  # not efficient, I know

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
        if self.dimension is not None:
            d["dimension"] = self.dimension
        if self.method is not None:
            d["method"] = self.method.to_dict()
        return d


# see https://opensearch.org/docs/latest/search-plugins/knn/knn-index/ for KNN stuff


class KnnMethodName(enum.StrEnum):
    HNSW = "hnsw"
    IVF = "ivf"


class KnnEngine(enum.StrEnum):
    NMSLIB = "nmslib"
    FAISS = "faiss"


class KnnSpaceType(enum.StrEnum):
    L2 = "l2"
    L1 = "l1"
    DOT_PRODUCT = "innerproduct"
    COSINE = "cosinesimil"
    LINF = "linf"


@dataclass
class KnnMethod:
    name: KnnMethodName
    space_type: KnnSpaceType
    engine: KnnEngine
    parameters: "HnswParameters"

    def to_dict(self) -> dict[str, Any]:
        return {
            "name": self.name.value,
            "space_type": self.space_type.value,
            "engine": self.engine.value,
            "parameters": self.parameters.to_dict(),
        }


@dataclass
class HnswParameters:
    ef_construction: int
    m: int

    def to_dict(self) -> dict[str, Any]:
        return {"ef_construction": self.ef_construction, "m": self.m}


field = Field

TYPE_DISCRIMINATOR_FIELD = Field(FT.KEYWORD)
TYPE_DISCRIMINATOR_KEY = "_type"


@dataclass
class Document:
    """
    Base for OpenSearch-style dataclass document.
    """

    Partial: ClassVar[typing.Type["Document"]] = None
    __fields__: ClassVar[dict[str, Field]] = {}
    __type__: ClassVar[typing.Optional[str]] = None
    __store_type__: ClassVar[bool] = True

    id: UUID  # not technically a field on the document, so no Field annotation (goes into _id)

    def to_dict(self) -> dict[str, Any]:
        """
        Convert this document to a dict wireable to OpenSearch.
        We convert top level fields to JSON-able types - inner fields are left as is,
         as they are all user-defined and thus already JSON-able.
        """
        d = {}
        if self.__type__ is not None and self.__store_type__:
            d[TYPE_DISCRIMINATOR_KEY] = self.__type__
        for name, field in self.__fields__.items():
            if not field.can_set_directly:
                continue
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
    def from_dict(cls, d: dict[str, Any], id: str, version: str) -> "Document":
        """
        Convert a dict wireable from OpenSearch to a document, converting to pythonic types.
        """
        d = {**d, "id": UUID(id), "revision": version}
        for name, field in cls.__fields__.items():
            if not field.can_set_directly:
                continue
            value = d.pop(name, None)
            if value is None:
                pass  # just leave it as None
            elif field.type == FT.DATE:
                if value == "-":
                    value = None  # used to reset a date field
                else:
                    value = datetime.fromisoformat(value)
            elif field._annotation == UUID:
                value = UUID(value)
            elif field.type == FT.TEXT:
                value = value
            d[name] = value
        # add id and revision ("version") from meta
        return cls(**d)


def document(
    cls: typing.Optional[typing.Type[Document]], _type: str = None, store_type: bool = None
):
    """Decorator for mapping a class as an OpenSearch-style dataclass."""

    def decorator(cls: typing.Type[Document]):
        # first convert the fields to dataclass fields (and store the original fields)
        fields = {}

        for name, field in cls.__dict__.items():
            # ignore reserved names and non-fields
            if (
                name.startswith("_")
                or name == "Partial"
                or field is None
                or inspect.ismethod(field)
                or inspect.isfunction(field)
                or isinstance(field, property)
            ):
                continue
            if not isinstance(field, Field):
                raise TypeError(f"{name} is not a Field in {cls.__name__}")
            field._annotation = cls.__annotations__[name]
            fields[name] = field
        # remove field values from annotation (since it's not a dataclass field)
        for name, field in fields.items():
            if not field.can_set_directly:
                setattr(cls, name, dataclasses.field(init=False))
            else:
                delattr(cls, name)
        # add fields from parent classes
        for base in reversed(cls.__bases__):
            if base is Document:
                continue
            fields.update(base.__fields__)
        # then convert the class to a dataclass
        cls = dataclasses.dataclass(cls, repr=False)
        # then add the fields back
        cls.__fields__ = fields
        # add a partial class with all fields optional (copy and set fields with default None)
        partial_fields = {
            **{name: dataclasses.field(default=None) for name, field in fields.items()},
            "id": dataclasses.field(default=None),
        }
        partial_cls = type(cls.__name__ + "Partial", (cls,), partial_fields)
        # copy all type annotations from the original class and its bases
        for c in cls.__mro__[:-2]:  # skip object and Document
            for name in fields:
                if name in c.__annotations__:
                    partial_cls.__annotations__[name] = c.__annotations__[name]
        partial_cls.__annotations__["id"] = UUID
        partial_cls = dataclasses.dataclass(partial_cls, repr=False)
        cls.Partial = partial_cls
        if _type is not None:
            cls.__type__ = _type
        cls.__store_type__ = store_type
        return cls

    if cls is None:
        return decorator
    else:
        return decorator(cls)


if typing.TYPE_CHECKING:
    document = dataclasses.dataclass  # noqa


class Tokenizer(enum.StrEnum):
    # built in
    STANDARD = "standard"
    LETTER = "letter"
    LOWERCASE = "lowercase"
    WHITESPACE = "whitespace"
    UAX_URL_EMAIL = "uax_url_email"
    CLASSIC = "classic"
    THAI = "thai"
    # custom
    CHAR = "char"


CUSTOM_TOKENIZERS = {Tokenizer.CHAR: {"type": "char_group", "tokenize_on_chars": []}}


class Analyzer(enum.StrEnum):
    # built in
    STANDARD = "standard"
    SIMPLE = "simple"
    WHITESPACE = "whitespace"
    STOP = "stop"
    KEYWORD = "keyword"
    PATTERN = "pattern"
    LANGUAGE = "language"
    FINGERPRINT = "fingerprint"
    # custom
    HTML = "html"
    CHAR_COUNT = "char_count"


CUSTOM_ANALYZERS = {
    Analyzer.HTML: {"tokenizer": "standard", "char_filter": ["html_strip"]},
    Analyzer.CHAR_COUNT: {"tokenizer": Tokenizer.CHAR},
}


class IndexType(enum.StrEnum):
    """The index type within Bench."""

    GLOBAL = "global"
    BENCH = "project"

    @property
    def is_project_scoped(self) -> bool:
        return self in (IndexType.BENCH,)

    def get_index_name(self, project_id: UUID = None):
        if self.is_project_scoped != (project_id is not None):
            raise ValueError(f"project scoped index -> project id: {self} -> {project_id}")
        if self.is_project_scoped:
            return f"bench-user-{project_id}-{self.value}"
        else:
            return f"bench-{self.value}"

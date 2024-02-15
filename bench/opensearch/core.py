import enum
import typing
from dataclasses import dataclass
from typing import Any


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
    FLAT_OBJECT = "flat_object"
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


class SubfieldType(enum.StrEnum):
    # :QuerySubfields
    key = "key"
    starts_with = "starts_with"
    token_count = "token_count"
    char_count = "char_count"


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
    fields: dict[SubfieldType, "Field"] | None = None
    properties: dict[str, "Field"] | None = None
    meta: dict[str, str] | None = None
    index: bool | None = None  # default: true
    store: bool | None = None  # default: true
    coerce: bool | None = None
    dynamic: bool | typing.Literal["strict"] = None
    enabled: bool | None = None  # default: true
    copy_to: str | list[str] = None
    ignore_malformed: bool | None = None
    ignore_above: int | None = None
    analyzer: typing.Optional["Analyzer"] = None
    can_set_directly: bool | None = True
    dimension: int | None = None  # for knn_vector
    data_type: typing.Optional[str] = None  # for knn_vector
    method: typing.Optional["KnnMethod"] = None  # for knn_vector
    _annotation: typing.Optional[Any] = None  # type annotation on the LHS of a field in a document

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

    def walk(self) -> typing.Iterator["Field"]:
        """
        Iterate over this field and all its subfields.
        """
        yield self
        if self.fields is not None:
            for f in self.fields.values():
                yield from f.walk()
        if self.properties is not None:
            for f in self.properties.values():
                yield from f.walk()

    def to_dict(self) -> dict[str, Any]:
        """
        Convert this field to a dict wireable to OpenSearch.
        Empty fields are omitted.
        """
        d: dict[str, Any] = {"type": self.type.value}
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
        if self.enabled is not None:
            d["enabled"] = self.enabled
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
        if self.data_type is not None:
            d["data_type"] = self.data_type
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
    LUCENE = "lucene"


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

METATYPE_FIELD = Field(FT.KEYWORD)


@dataclass
class Document:
    """
    Base for OpenSearch-style dataclass document.
    """

    fields: dict[str, Field]


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


CUSTOM_TOKENIZERS = {
    # TODO @Performance: character length by tokenizing each character seems pretty inefficient
    #  (even though this isn't actually stored or indexed directly, just the count)
    # Maybe this should just be a user-side scripted field, but we don't have those yet.
    # Would also be nice later for quantized vector fields.
    Tokenizer.CHAR: {"type": "char_group", "tokenize_on_chars": [], "max_token_length": 1}
}


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
    Analyzer.CHAR_COUNT: {
        "tokenizer": Tokenizer.CHAR,
    },
}

GLOBAL_INDEX_NAME = "bench-global"

from __future__ import annotations

import enum


#
# Collection of enums for use without circular import hell.
#


class StatementType(enum.StrEnum):
    """The type of Bench statement."""

    TAG = "tag"
    TEXT = "text"
    BLANK = "blank"
    TYPE = "type"
    TASK = "task"
    EXPECTATION = "expectation"
    CODE = "code"
    MODEL = "model"
    VALUE = "value"
    DATASET = "dataset"
    REFERENCE = "reference"
    BLOCK = "block"


RUNNABLE_STATEMENT_TYPES = {
    StatementType.CODE,
    StatementType.MODEL,
    StatementType.TASK,
}


class DatasetBackend(enum.StrEnum):
    OPENSEARCH = "os"


class DatasetViewLayout(enum.StrEnum):
    """The layout of a dataset view."""

    TABLE = "table"


class TypeTag(enum.StrEnum):
    """The Bench primitive type of a field/type."""

    STRING = "string"
    NUMBER = "number"
    BOOLEAN = "boolean"
    VECTOR = "vector"
    FILE = "file"
    STRUCT = "struct"
    JSON = "json"
    FUNCTION = "function"
    UNION = "union"
    ENUM = "enum"
    LITERAL = "literal"
    NULL = "null"
    ANY = "any"
    TYPE_REFERENCE = "ref"


class TypeHint(enum.StrEnum):
    """Extra representation/semantics of a field/type."""

    # string
    NAME = "name"
    UUID = "uuid"
    DATE = "date"
    DATETIME = "datetime"
    TIME = "time"
    DURATION = "duration"
    EMAIL = "email"
    URL = "url"
    MARKDOWN = "markdown"
    RICH_TEXT = "rich_text"
    HTML = "html"
    CODE = "code"
    KEY = "key"
    SECRET = "secret"
    # number
    INTEGER = "integer"
    FLOAT = "float"
    SLIDER = "slider"
    PHONE = "phone"
    RATING = "rating"
    # boolean
    TOGGLE = "toggle"
    CHECKBOX = "checkbox"
    THUMBS = "thumbs"
    # vector
    EMBEDDING = "embedding"
    # file
    IMAGE = "image"
    VIDEO = "video"
    AUDIO = "audio"


class TypeFlag(enum.IntFlag):
    """Extra information for fields"""

    # :TypeFlags
    Zero = 0
    IsOutput = 2**0
    IsArray = 2**1
    IsOptional = 2**2
    IsUnionWith = 2**3
    IsSecret = 2**4  # TODO @Cleanup: IsSecret shouldn't be a flag
    IsStoreOnly = 2**5
    IsArrayable = 2**6

    @property
    def short_name(self) -> str:
        return self.name.replace("Is", "")


class TypeStorageFormat(enum.StrEnum):
    """
    The fundamental form of a field/type.
    Since we're using OpenSearch for our user data backend, this
    needs to be compatible with OpenSearch's field types.
    However, be mindful of other future storage backends.
    :TypeStorageFormat
    """

    STRING = "str"
    DOUBLE = "f64"
    LONG = "s64"
    VECTOR = "vec"
    BINARY = "bin"
    BOOLEAN = "bool"
    DATE = "date"
    KEYWORD = "key"
    OBJECT = "obj"
    RELATION = "rel"


class RemoteObjectStatus(enum.StrEnum):
    PREPARED = "prepared"
    UPLOADING = "uploading"
    AVAILABLE = "available"


class WorkerTenancy(enum.StrEnum):
    COMMUNITY = "COMMUNITY"
    DEDICATED = "DEDICATED"


class RunTriggerType(enum.StrEnum):
    API = "rest"
    UI = "ui"

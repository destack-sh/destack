from __future__ import annotations

import enum

#
# Collection of enums for use without circular import hell.
#


class StatementType(enum.StrEnum):
    """The type of Bench statement."""

    TEXT = "text"
    BLANK = "blank"
    TYPE = "type"
    TASK = "task"
    EXPECTATION = "expectation"
    CODE = "code"
    MODEL = "model"
    VALUE = "value"
    DATASET = "dataset"
    BLOCK = "block"


class DatasetBackend(enum.StrEnum):
    OPENSEARCH = "os"


class DatasetViewLayout(enum.StrEnum):
    """The layout of a dataset view."""

    TABLE = "table"


class ExpectationModifier(enum.StrEnum):
    """A modifier to a Bench statement."""

    LIKE = "like"
    UNLIKE = "unlike"
    CHECK = "check"


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
    IsNullable = 2**2
    IsUnionWith = 2**3
    IsSecret = 2**4
    IsStoreOnly = 2**5
    IsArrayable = 2**6


class RemoteObjectStatus(enum.StrEnum):
    PREPARED = "prepared"
    UPLOADING = "uploading"
    AVAILABLE = "available"


class WorkerTenancy(enum.StrEnum):
    COMMUNITY = "COMMUNITY"
    DEDICATED = "DEDICATED"


class ExecutionTriggerType(enum.StrEnum):
    API = "rest"
    UI = "ui"
    REACTIVE = "reactive"
    SCHEDULED = "scheduled"

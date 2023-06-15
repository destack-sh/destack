from __future__ import annotations

import asyncio
import enum
import itertools
import re
import typing
from dataclasses import dataclass
from typing import Any, NamedTuple, Union
from uuid import UUID

import numpy
import pandas
from more_itertools import first, last


class ModuleObjectType(enum.StrEnum):
    # source
    MODULE = "MODULE"
    FILE = "FILE"
    STATEMENT = "STATEMENT"
    FIELD = "FIELD"
    RECORD = "RECORD"
    DATASET_VIEW = "DATASET_VIEW"
    # interp
    ISSUE = "ISSUE"
    RESOLVED_FIELD = "RESOLVED_FIELD"
    # user
    COMMENT = "COMMENT"


MOT = ModuleObjectType


class InterpScope(enum.StrEnum):  # not sure if we still need this?
    MODULE = "module"
    FILE = "file"
    STATEMENT = "statement"


class StatementType(enum.StrEnum):
    """The type of Bench statement."""

    TEXT = "text"
    BLANK = "blank"
    TYPE = "type"
    TASK = "task"
    EXPECTATION = "expect"
    CODE = "code"
    MODEL = "model"
    VALUE = "value"
    DATASET = "dataset"
    REQUIREMENT = "require"
    BLOCK = "block"


class TypeTag(enum.StrEnum):
    """The actual value type of a type node."""

    STRING = "string"
    NUMBER = "number"
    BOOLEAN = "boolean"
    VECTOR = "vector"
    FILE = "file"
    SHAPE = "shape"
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
    """The representation of a type node"""

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
    # :TypeFlags
    Zero = 0
    IsOutput = 2**0
    IsArray = 2**1
    IsNullable = 2**2
    IsUnionWith = 2**3
    IsSecret = 2**4


Vector = list[float]


@dataclass
class XYPoint:
    """The value of an OpenSearch/GeoJSON-compatible geo_point field."""

    x: float
    y: float


class XYShapeType(enum.StrEnum):
    """The type in an OpenSearch-compatible geo_shape field."""

    POINT = "point"
    LINE_STRING = "line_string"
    POLYGON = "polygon"
    MULTI_POINT = "multi_point"
    MULTI_LINE_STRING = "multi_line_string"
    MULTI_POLYGON = "multi_polygon"
    GEOMETRY_COLLECTION = "geometry_collection"
    ENVELOPE = "envelope"


@dataclass
class XYShape:
    """The value of an OpenSearch-compatible geo_shape field."""

    type: XYShapeType
    coordinates: typing.Union[list[float], list[list[float]]]


class ExpectationModifier(enum.StrEnum):
    """A modifier to a Bench statement."""

    LIKE = "like"
    UNLIKE = "unlike"
    CHECK = "check"


class LookupBy(enum.StrEnum):
    Name = "name"
    PyIdent = "py_ident"


class RemoteObjectStatus(enum.StrEnum):
    PREPARED = "prepared"
    UPLOADING = "uploading"
    AVAILABLE = "available"


# identifier names can be escaped as '<name with space>'
# references can look like
# 1. <name>
# 2. .<path>.<name>
# 3. <module_owner>.<module_name>.<path>.<name>
StatementPath = NamedTuple("StatementPath", [("path", str), ("name", str)])


def statement_path_as_str(statement_path: StatementPath) -> str:
    return f"{statement_path.path}:{statement_path.name}"


def parse_statement_path(statement_path: str) -> StatementPath:
    if ":" not in statement_path:
        raise ValueError(f"invalid statement path: {statement_path}")
    path, name = statement_path.split(":")
    return StatementPath(path, name)


REFERENCE_REGEX = re.compile(
    r"^((?P<module_owner>[\w\- ]+)\.(?P<module_name>[\w\- ]+))?(\.(?P<path>[\w.\- ]+)\.)?(?P<name>[\w\- ]+)$"
)
# import source must either be in current module (.*) or absolute (<owner>.<name>.*)
RELATIVE_REFERENCE_REGEX = re.compile(r"^\.(?P<path>[\w.\- ]+)$")
ABSOLUTE_IMPORT_SOURCE_REGEX = re.compile(
    r"^(?P<module_owner>[\w\- ]+)\.(?P<module_name>[\w\- ]+)\.(?P<path>[\w.\- ]+)$"
)

TYPE_TAG_BY_TYPE_HINT = {
    # string
    TypeHint.NAME: TypeTag.STRING,
    TypeHint.UUID: TypeTag.STRING,
    TypeHint.DATE: TypeTag.STRING,
    TypeHint.DATETIME: TypeTag.STRING,
    TypeHint.TIME: TypeTag.STRING,
    TypeHint.DURATION: TypeTag.STRING,
    TypeHint.EMAIL: TypeTag.STRING,
    TypeHint.URL: TypeTag.STRING,
    TypeHint.MARKDOWN: TypeTag.STRING,
    TypeHint.RICH_TEXT: TypeTag.STRING,
    TypeHint.HTML: TypeTag.STRING,
    TypeHint.CODE: TypeTag.STRING,
    TypeHint.KEY: TypeTag.STRING,
    TypeHint.SECRET: TypeTag.STRING,
    # number
    TypeHint.INTEGER: TypeTag.NUMBER,
    TypeHint.FLOAT: TypeTag.NUMBER,
    TypeHint.SLIDER: TypeTag.NUMBER,
    TypeHint.PHONE: TypeTag.NUMBER,
    TypeHint.RATING: TypeTag.NUMBER,
    # boolean
    TypeHint.TOGGLE: TypeTag.BOOLEAN,
    TypeHint.CHECKBOX: TypeTag.BOOLEAN,
    TypeHint.THUMBS: TypeTag.BOOLEAN,
    # file
    TypeHint.IMAGE: TypeTag.FILE,
    TypeHint.VIDEO: TypeTag.FILE,
    TypeHint.AUDIO: TypeTag.FILE,
}

ModuleReference = typing.NamedTuple(
    "ModuleReference", [("name", str), ("version", str), ("id", typing.Optional[UUID])]
)

STATIC_BUILTINS = {
    # primitive type builtins
    "string": str,
    "text": str,
    "number": float,
    "boolean": bool,
    # library builtins
    "numpy": numpy,
    "np": numpy,
    "pandas": pandas,
    "pd": pandas,
    "asyncio": asyncio,
    # functional builtins
    "itertools": itertools,
    "more_itertools": itertools,
    "first": first,
    "last": last,
    "chain": itertools.chain,
}
DYNAMIC_BUILTINS = {
    "session",
    "context",
    "random",
}

LiteralValue = Union[dict[str, Any], list[Any], bool, int, float, str, None]
PRIMITIVE_TYPES = [
    TypeTag.ANY,
    TypeTag.NULL,
    TypeTag.BOOLEAN,
    TypeTag.NUMBER,
    TypeTag.STRING,
    TypeTag.FILE,
    TypeTag.VECTOR,
]
FIELD_KEY_LENGTH = 8


class ModuleOp(enum.StrEnum):
    READ = "read"
    SEARCH = "search"
    CREATE = "create"
    UPDATE = "update"
    DELETE = "delete"


class SessionMode(enum.StrEnum):
    READ_ONLY = "ro"
    WRITE_GLOBAL = "w"
    WRITE_ONLY = "wo"


class SessionTracingLevel(enum.IntFlag):
    NONE = 0
    EXECUTION = 1
    MUTATION = 2
    VALIDATION = 4
    ALL = EXECUTION | MUTATION | VALIDATION


class ExecutionTriggerType(enum.StrEnum):
    API = "rest"
    UI = "ui"
    REACTIVE = "reactive"
    SCHEDULED = "scheduled"


@dataclass(slots=True)
class SessionContext:
    module_id: UUID
    project_id: UUID
    worker_id: UUID
    tracing_level: SessionTracingLevel
    trigger_type: ExecutionTriggerType
    trigger_id: typing.Optional[UUID]
    root_id: typing.Optional[UUID] = None

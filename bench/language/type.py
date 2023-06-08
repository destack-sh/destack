from __future__ import annotations

import abc
import asyncio
import enum
import itertools
import random
import string
import typing
import uuid
from dataclasses import dataclass, field, fields
from functools import cached_property
from typing import Any, Literal, NamedTuple, Optional, Self, Union
from uuid import UUID

import numpy
import pandas
from asgiref.sync import async_to_sync
from more_itertools import first, last

from bench.language.build import XConsiderError, XGenerationError
from bench.language.dataset import Query, Sort
from bench.language.session import Session
from bench.runtime.common.inference import ModelInference
from bench.settings import logging
from bench.utils.fractional import INTEGER_ZERO, generate_key_between, generate_n_keys_between
from bench.utils.func import describe_type, dict_minus
from bench.utils.proxy import unproxy_value
from bench.utils.utils import required_field, to_pyidentifier


class InterpScope(enum.StrEnum):
    MODULE = "module"
    FILE = "file"
    STATEMENT = "statement"


class StatementType(enum.StrEnum):
    """The type of Bench statement."""

    # symbol statements
    IMPORT = "import"  # import
    DEFINITION = "def"  # :
    REFERENCE = "ref"  #
    REDEFINITION = "redef"  # =
    # non-symbol statements
    COMMENT = "comment"  # #
    BLANK = "blank"  # ...


class StatementModifier(enum.StrEnum):
    """A modifier to a Bench statement."""

    LIKE = "like"
    UNLIKE = "unlike"
    CHECK = "check"
    MAGIC = "magic"


class SymbolType(enum.StrEnum):
    """The type of symbol content."""

    TYPE = "type"
    CAPABILITY = "capability"
    TASK = "task"
    EXPECTATION = "expect"
    CODE = "code"
    MODEL = "model"
    VALUE = "value"
    DATASET = "dataset"
    REQUIREMENT = "require"
    BUILD = "build"
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


@dataclass(repr=False)
class Module:
    name: str
    files: list[File] = field(default_factory=list)
    id: UUID = field(default_factory=uuid.uuid4)

    def __str__(self):
        return f"{self.name} ({len(self.files)} files)"

    def __repr__(self):
        return f"<Module {str(self)}>"


MOCK_MODULE = Module("<mock>", [])


@dataclass(repr=False)
class File:
    module: Module
    path: str
    statements: list[Statement] = field(default_factory=list)
    id: UUID = field(default_factory=uuid.uuid4)
    generated: bool = False

    def __str__(self):
        return f"{self.module.name}/{self.path}"

    def __repr__(self):
        return f"<File {str(self)}>"

    @property
    def extension(self) -> str:
        if "." not in self.path:
            raise ValueError(f"{self} has no extension")
        return self.path.split(".")[-1]

    @property
    def path_without_extension(self) -> str:
        if "." in self.path:
            return self.path[: -len(self.extension) - 1]
        else:
            return self.path

    @property
    def root_statements(self) -> list[Statement]:
        return [statement for statement in self.statements if statement.parent is None]


StatementPath = NamedTuple("StatementPath", [("path", str), ("name", str)])
StatementReference = Union["Statement", StatementPath, UUID]


def statement_path_as_str(statement_path: StatementPath) -> str:
    return f"{statement_path.path}:{statement_path.name}"


def parse_statement_path(statement_path: str) -> StatementPath:
    if ":" not in statement_path:
        raise ValueError(f"invalid statement path: {statement_path}")
    path, name = statement_path.split(":")
    return StatementPath(path, name)


@dataclass(repr=False)
class Statement:
    """A parsed but not interpreted statement in Bench source."""

    file: File
    parent: Optional[Statement]
    order_key: str
    type: StatementType
    modifier: Optional[StatementModifier] = None
    name: Optional[str] = None
    text: Optional[str] = None
    symbol_type: Optional[SymbolType] = None
    reference: Optional[Statement | StatementPath | UUID] = None
    id: UUID = field(default_factory=uuid.uuid4)
    generated: bool = False

    _source: Optional[Any] = None

    def __str__(self):
        loc = self.file.path + ":" + str(self.infile_path)
        if self.type == StatementType.DEFINITION:
            content_str = "()"  # should have some nice __str__ here
        elif self.type in (StatementType.IMPORT, StatementType.REFERENCE):
            content_str = f"{self.reference}"
        elif self.type == StatementType.COMMENT:
            content_str = ""
        elif self.type == StatementType.BLANK:
            content_str = ""
        else:
            raise ValueError(f"unknown statement type {self.type}")
        modifier_str = f" {self.modifier}" if self.modifier else ""
        return f"{loc}{modifier_str} {self.type} {self.symbol_type} {self.name} {content_str}"

    def __repr__(self):
        return f"<Statement {self}>"

    @property
    def infile_path(self) -> str:
        parent = self.parent
        ancestor_parts = [self.name or "<anon>"]
        seen_ids = {self.id}
        while parent is not None:
            if parent.id in seen_ids:
                # :CircularAncestry
                # circuit breaker: ignore here because this is an error in indexing
                ancestor_parts.append("<!loop>")
                break
            ancestor_parts.append(parent.name or "<anon>")
            seen_ids.add(parent.id)
            parent = parent.parent
        return ".".join(reversed(ancestor_parts))

    @property
    def fqn(self) -> str:
        return f"{self.file.module.name}.{self.file.path.replace('/', '.')}.{self.name}"

    @property
    def ident(self) -> str:
        return to_pyidentifier(self.name)

    @property
    def parent_id(self) -> Optional[UUID]:
        return self.parent.id if self.parent else None

    @property
    def reference_id(self) -> Optional[UUID]:
        return self.reference.id if isinstance(self.reference, Statement) else None

    @property
    def underlying_definition(self) -> Optional[Statement]:
        if self.type == StatementType.DEFINITION:
            return self
        elif isinstance(self.reference, Statement):
            return self.reference.underlying_definition
        else:
            return None

    @property
    def ungrouped(self) -> bool:
        """Whether this statement shouldn't be grouped with its siblings."""
        return self.defines_symbol and self.symbol_type not in (SymbolType.REQUIREMENT,)

    @property
    def has_reference(self) -> bool:
        return self.type in (
            StatementType.REDEFINITION,
            StatementType.IMPORT,
            StatementType.REFERENCE,
        )

    @property
    def is_expectable(self) -> bool:
        return self.symbol_type in (SymbolType.TASK, SymbolType.CODE, SymbolType.DATASET)

    @property
    def is_expect(self) -> bool:
        has_expect_intent = self.modifier in (
            StatementModifier.LIKE,
            StatementModifier.UNLIKE,
            StatementModifier.CHECK,
        )
        return self.symbol_type == SymbolType.EXPECTATION or (
            self.is_expectable and has_expect_intent
        )

    @property
    def defines_symbol(self) -> bool:
        return self.type in (StatementType.DEFINITION, StatementType.REDEFINITION)

    @property
    def is_alias(self):
        if isinstance(self.reference, StatementPath):
            return self.reference[1] != self.name
        elif isinstance(self.reference, Statement):
            return self.reference.name != self.name
        else:
            return False


class LanguageObject(abc.ABC):
    session: "Session"
    id: UUID

    def __post_init__(self):
        from bench.language.session import active_session

        if self.session is None:
            self.session = active_session.get()
            if self.session is None:
                raise RuntimeError(f"no active session for {self}")
            # we pass in session on instantiate, so this must be new
            self.session.add(self, new=True)
        else:
            self.session.add(self, new=False)

    def __del__(self):
        if self.session is not None:
            self.session.remove(self)

    @property
    def logger(self) -> logging.Logger:
        return self.session.logger


@dataclass(repr=False)
class Symbol(LanguageObject):
    """An interpreted - fully resolved, templated and validated - symbol from Bench source."""

    name: str = field(default="")
    modifier: Optional[StatementModifier] = None
    reference: Optional[Symbol | StatementPath] = None
    definition: Optional[Symbol] = None
    source: Optional[Statement] = None
    id: UUID = field(default_factory=uuid.uuid4)

    def deepcopy(self, keep_id: bool = True, keep_reference: bool = True) -> "Symbol":
        kwargs = {**self.__dict__}
        kwargs["id"] = self.id if keep_id else uuid.uuid4()
        return self.__class__(**kwargs)

    @property
    def ident(self) -> str:
        return to_pyidentifier(self.name)

    @property
    def is_definition(self) -> bool:
        return self.definition is not None and self.definition.id == self.id

    @property
    def fqn(self) -> str | None:
        return self.definition.source.fqn if self.definition.source else None

    @property
    def is_root(self):
        return self.source is None or self.source.parent is None

    @property
    def symbol_type(self) -> SymbolType:
        return SYMBOL_TYPE_BY_CLASS[self.__class__]

    @property
    def is_generated(self):
        return self.source is None or self.source.generated

    def __str__(self):
        modifier_str = f"{self.modifier} " if self.modifier else ""
        return f"{modifier_str}{self.symbol_type} {self.name} (source={self.source or '<unknown>'})"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"


# Danger: the order of these types is important because it influences deserialization order.
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


class TypeFlag(enum.IntFlag):
    # :TypeFlags
    Zero = 0
    IsOutput = 2**0
    IsArray = 2**1
    IsNullable = 2**2
    IsUnionWith = 2**3
    IsSecret = 2**4


class TypeNode(abc.ABC):
    id: UUID
    name: Optional[str]
    key: Optional[str]
    tag: TypeTag
    hint: Optional[TypeHint]
    flags: TypeFlag
    description: Optional[str]
    fields: list["TypeNode"]
    self_fields: list["TypeNode"]  # original fields excluding resolved fields
    reference: Union[None, StatementReference, "Type"]

    @property
    def ident(self):
        return to_pyidentifier(self.name)

    @property
    def inputs(self) -> list["TypeNode"]:
        return [child for child in self.fields if not child.flags & TypeFlag.IsOutput]

    @property
    def outputs(self) -> list["TypeNode"]:
        return [child for child in self.fields if child.flags & TypeFlag.IsOutput]

    def __getitem__(self, item: str) -> "TypeNode":
        node = first(
            (
                child
                for child in self.fields
                if child.name == item or child.ident == item or child.key == item
            ),
            None,
        )
        if node is None:
            raise KeyError(item)
        return node

    def __contains__(self, item):
        return any(
            child
            for child in self.fields
            if child.name == item or child.ident == item or child.key == item
        )

    def walk(self, path: list[TypeNode] | None = None, include_references: bool = False):
        if path is None:
            path = [self]
        else:
            path = path + [self]
        yield self
        if include_references and self.reference:
            yield from self.reference.walk(path, include_references=include_references)
        if self.fields:
            for child in self.fields:
                if child in path:
                    continue  # break cycles (allowed, but we don't want to traverse them)
                yield from child.walk(path, include_references=include_references)

    def deepcopy(self, keep_id: bool = True, keep_reference: bool = True) -> "TypeNode":
        raise NotImplementedError

    def unkey(self, data: Any, is_output: bool = None, to_ident: bool = False) -> Any:
        """'Unkeys' data by replacing keys with the names of the type nodes."""
        from bench.language.typer import unkey_value

        return unkey_value(data, self, is_output=is_output, to_ident=to_ident)

    def rekey(self, data: Any, is_output: bool = None, via_ident: bool = False) -> Any:
        """'Keys' data by replacing names with the keys of the type nodes."""
        from bench.language.typer import rekey_value

        return rekey_value(data, self, is_output=is_output, from_ident=via_ident)


# :TypeNodeKeys
FIELD_KEY_LENGTH = 8


def new_field_key() -> str:
    """Gets a random alphabetic key as a persistent key for a type node."""
    # (upper and lower case letters only)
    return "".join(random.choices(string.ascii_letters, k=FIELD_KEY_LENGTH))


@dataclass(repr=False)
class Field(TypeNode):
    name: Optional[str]
    tag: TypeTag
    hint: Optional[TypeHint] = None
    order_key: str = INTEGER_ZERO
    id: UUID = field(default_factory=uuid.uuid4)
    key: str = field(default_factory=new_field_key)
    description: Optional[str] = None
    flags: TypeFlag = TypeFlag(0)
    reference: Union[None, StatementPath, Statement, UUID, "Type"] = None
    source_reference: Optional[StatementPath] = None

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{name_str}{self.tag}"

    def __repr__(self):
        return f"<Field {self}>"

    @property
    def fields(self) -> list[TypeNode]:
        if isinstance(self.reference, Type):
            return self.reference.fields
        return []

    self_fields = fields  # the same by default

    def deepcopy(
        self, keep_id: bool = True, keep_reference: bool = True, deepcopy_reference: bool = True
    ) -> "Field":
        if not keep_reference or self.reference is None:
            reference = self.source_reference
        elif deepcopy_reference and isinstance(self.reference, Type):
            reference = self.reference.deepcopy(
                keep_id=True, keep_reference=keep_reference, deepcopy_reference=False
            )
        else:
            reference = self.reference
        return Field(
            id=self.id if keep_id else uuid.uuid4(),
            name=self.name,
            key=self.key,
            tag=self.tag,
            hint=self.hint,
            order_key=self.order_key,
            description=self.description,
            reference=reference,
            source_reference=self.source_reference,
            flags=self.flags,
        )


Expectable = Union["Expectation", "Task", "Dataset", "Code"]


@dataclass(repr=False)
class HasExpectations:
    expectations: list[Expectable] = field(default_factory=list)

    def expect(self, expectation: Expectable) -> "Self":
        self.expectations.append(expectation)
        return self

    def check(self, code: Code) -> "Self":
        # turn into ref and add modifier
        raise NotImplementedError

    def like(self, expectation: Expectable) -> "Self":
        raise NotImplementedError

    def unlike(self, expectation: Expectable) -> "Self":
        raise NotImplementedError

    def walk_expectations(self, path: list[Symbol] = None):
        if path is None:
            path = [self]
        else:
            path = path + [self]
        yield self
        if self.expectations:
            for child in self.expectations:
                if child in path:
                    continue
                if isinstance(child, HasExpectations):
                    yield from child.walk_expectations(path)


@dataclass(repr=False)
class Type(Symbol, TypeNode, HasExpectations):
    name: Optional[str] = None
    tag: TypeTag = required_field()
    description: Optional[str] = None
    flags: TypeFlag = TypeFlag(0)
    fields: list[TypeNode] = field(default_factory=list)
    self_fields: list[TypeNode] = None
    expectations: list[Expectable] = field(default_factory=list)
    # not directly configurable for types
    hint = None
    key = None
    reference = None

    @cached_property
    def py_type(self) -> type:
        return self.session.instance.get_py_type(self)

    # mimic python type behavior
    def __instancecheck__(self, instance):
        return isinstance(instance, self.py_type)

    def __subclasscheck__(self, subclass):
        return issubclass(subclass, self.py_type)

    def __call__(self, *args, **kwargs):
        return self.py_type(*args, **kwargs)

    def __getattr__(self, item):
        if self.tag == TypeTag.ENUM:
            return self.py_type[item]
        if item in self:
            return self[item]
        raise AttributeError(f"{self} has no attribute {item}")

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{name_str}{self.tag}"

    def __repr__(self):
        return f"<Type {self}>"

    def deepcopy(
        self, keep_id: bool = True, keep_reference: bool = True, deepcopy_reference: bool = True
    ) -> "Self":
        fields = [
            field.deepcopy(
                keep_id=keep_id,
                keep_reference=keep_reference,
                deepcopy_reference=deepcopy_reference,
            )
            for field in self.fields
        ]
        return self.__class__(
            name=self.name,
            tag=self.tag,
            flags=self.flags,
            description=self.description,
            fields=fields,
            expectations=self.expectations,
            source=self.source,
        )


@dataclass(repr=False)
class Task(Symbol, HasExpectations):
    description: str = ""
    root_type_tag = TypeTag.FUNCTION
    expectations: list[Expectable] = field(default_factory=list)
    steps: list[Task | Code] = field(default_factory=list)
    is_async = True
    # should probably store last good implementation .. in redis?
    last_good_impl_idx: int = 0
    py_type = None  # doesn't have a python type

    @property
    def generated_expectations(self) -> list[Expectation]:
        return [e for e in self.expectations if e.is_generated]

    @property
    def is_minimally_specified(self) -> bool:
        return bool(self.name and self.inputs and self.outputs)

    async def __call__(
        self,
        *args,
        build: Build | str = None,
        model: Model | str = None,
        retries: int = None,
        cache: bool = None,
        timeout: float = None,
        **kwargs,
    ):
        implementations = self.session.instance.get_implementations(self, build=build, model=model)
        # TODO @Broken: sort/filter implementations with some smartness
        impl_idx = self.last_good_impl_idx
        retries = retries if retries is not None else self.session.inference_retries
        remaining_retries = retries

        self.session.tracer.code_enter(self, args, kwargs)
        semantic_errors = []
        while remaining_retries >= 0:
            remaining_retries -= 1
            impl = implementations[impl_idx]
            log = self.session.logger.bind(
                task=self, retries=remaining_retries, implementation=impl
            )
            try:
                ret = await impl(*args, **kwargs, cache=cache, timeout=timeout)
                self.last_good_impl_idx = impl_idx
                self.session.tracer.code_exit(self, args, kwargs, ret)
                return ret
            except XGenerationError as e:
                semantic_errors.append(e)
                log.warning("task.failed", exc_info=e)
                if len(semantic_errors) <= self.session.inference_retries / len(implementations):
                    # retry with error info a few times
                    implementations[impl_idx] = impl.copy().emit(XConsiderError(e))
                else:
                    # fail over
                    impl_idx = (impl_idx + 1) % len(implementations)
                    semantic_errors = []
            except TimeoutError as e:
                # fail over
                self.session.logger.warning("task.failed", exc_info=e)
                impl_idx = (impl_idx + 1) % len(implementations)

        # give up
        errors_repr = "\n".join(str(e) for e in semantic_errors) if semantic_errors else "<timeout>"
        e = RuntimeError(f"{self} failed after {retries} retries: {errors_repr}")
        self.session.tracer.code_exception(self, args, kwargs, e)
        if semantic_errors:
            raise e from semantic_errors[-1]
        else:
            raise e

    def to_sync(self) -> "Self":
        sync_task = self.__class__(**dict_minus(self.__dict__, ["is_async"]))
        sync_task.is_async = False
        sync_task.__call__ = async_to_sync(self.__call__)
        return sync_task


@dataclass(repr=False)
class CodeTransformation:
    original_code: str
    transformed_code: str
    method_name: str
    start_offset: int


@dataclass(slots=True)
class CodeParse:
    references: dict[str, "StatementPath"] = field(default_factory=dict)
    is_async: bool = False
    fake_line_numbers: list[int] = field(default_factory=list)


AsyncCodeCallable = typing.Callable[..., typing.Coroutine]
SyncCodeCallable = typing.Callable[..., Any]


@dataclass(repr=False)
class Code(Symbol):
    description: Optional[str] = None
    language: Literal["python"] | Literal["x"] = "python"
    code: Optional[str] = None
    parse: Optional[CodeParse] = None
    transform: Optional[CodeTransformation] = None
    references: dict[str, Statement] = field(default_factory=dict)
    context: dict[str, Symbol] = field(default_factory=dict)

    @cached_property
    def code_callable(self) -> AsyncCodeCallable | SyncCodeCallable:
        return self.session.instance.get_code_callable(self)

    async def __call__(self, *args, **kwargs):
        log = self.session.logger.bind(code=self, args=len(args), kwargs=describe_type(kwargs))
        try:
            self.session.tracer.code_enter(self, args, kwargs)
            log.debug("code.enter")
            result = await self.code_callable(*args, **kwargs)
            self.session.tracer.code_exit(self, args, kwargs, result)
            log.debug("code.exit", result=describe_type(result))
            return result
        except Exception as exception:
            self.session.tracer.code_exception(self, args, kwargs, exception)
            log.debug("code.exception", excinfo=True)
            raise

    def to_sync(self) -> "Code":
        sync_code = self.__class__(**dict_minus(self.__dict__, ["is_async"]))
        sync_code.is_async = False
        sync_code.__call__ = async_to_sync(self.__call__)
        return sync_code


@dataclass(repr=False)
class Expectation(Symbol):
    description: str = required_field()
    expectations: list[Expectable] = field(default_factory=list)

    def __init__(self, name: str = None, description: str = None, expectations: list = None):
        super().__init__(name=name, description=description, expectations=expectations)


@dataclass(repr=False, slots=True)
class RecordMeta:
    dataset: Dataset


@dataclass(repr=False)
class Record:
    _order_key: str
    _data: typing.Any = field(default_factory=dict)
    _id: UUID = field(default_factory=uuid.uuid4)
    _: RecordMeta = field(default_factory=RecordMeta)

    def __str__(self):
        return f"{self._order_key} {describe_type(self._data)}"

    def __repr__(self):
        return f"<Record {self}>"

    @property
    def keys(self):
        return self._data.keys

    def __getitem__(self, item: str):
        try:
            return self._data[item]
        except KeyError:
            raise KeyError(f"missing key '{item}' (available: {list(self._data.keys())})")

    def __setitem__(self, key, value):
        self._data[key] = value

    def __getattr__(self, item):
        try:
            return self._data[item]
        except KeyError:
            raise KeyError(f"missing key '{item}' (available: {list(self._data.keys())})")

    def __setattr__(self, key, value):
        if key in RECORD_FIELD_KEYS:
            super().__setattr__(key, value)
        else:
            self[key] = value


RECORD_FIELD_KEYS = {field.name for field in fields(Record)}


@dataclass(repr=False)
class DatasetView:
    name: str
    query: Optional[Query] = None
    sort: Optional[list[Sort]] = None
    order_key: str = field(default_factory=uuid.uuid4)
    id: UUID = field(default_factory=uuid.uuid4)


@dataclass(repr=False)
class Dataset(Symbol):
    records: Optional[list[Record]] = None
    inmemory: bool = True
    length: Optional[int] = None
    description: Optional[str] = None
    views: Optional[list[DatasetView]] = None
    type: Optional[Type] = field(default_factory=Type)

    def __post_init__(self):
        self.meta = RecordMeta(type=self.type, session=self.session, owner=self)

    def clear(self):
        self.session.tracer.dataset_clear(self)
        if self.inmemory:
            self.records = []

    def append(self, record: Record = None, **data):
        if record is not None:
            if data:
                raise ValueError("cannot pass both record and data")
            data = record._data
        data = unproxy_value(data)  # remove source proxy if any
        # insert at end
        last_ok = self.records[-1]._order_key if self.records else INTEGER_ZERO
        record = Record(
            _id=uuid.uuid4(),
            _=self.meta,
            _order_key=generate_key_between(last_ok, None),
            _data=data,
        )
        self.session.tracer.dataset_append(self, record)
        if self.inmemory:
            self.records.append(record)

    def extend(self, records: typing.Iterable[Record | dict]):
        datas = [  # remove source proxy if any
            unproxy_value(record._data) if isinstance(record, Record) else unproxy_value(record)
            for record in records
        ]
        last_ok = self.records[-1]._order_key if self.records else INTEGER_ZERO
        oks = generate_n_keys_between(last_ok, None, len(datas))
        records = [
            Record(
                _id=uuid.uuid4(),
                _=self.meta,
                _order_key=ok,
                _data=data,
            )
            for ok, data in zip(oks, datas)
        ]
        self.session.tracer.dataset_extend(self, records)
        self.records.extend(records)

    def __len__(self):
        return self.length

    def __getitem__(self, item: int | slice) -> Record | list[Record]:
        return self.records[item]

    def __iter__(self):
        return iter(self.records)


@dataclass(repr=False)
class Value(Symbol):
    value: Any = None

    def __post_init__(self):
        self.meta = RecordMeta(type=self.type, session=self.session, owner=self)

    @property
    def keys(self):
        return self.value.keys()

    def __getitem__(self, item):
        return self.value[item]

    def __setitem__(self, key, value):
        self.value[key] = value
        self.session.tracer.value_setitem(self, key, value)

    # proxy to record data if not in this class

    def __getattr__(self, item):
        if item in self.__dict__:
            return self.__dict__[item]
        elif item in self.records[0]._data:
            return self.records[0][item]
        else:
            raise AttributeError(item)

    def __setattr__(self, key, value):
        if key in VALUE_INSTANCE_FIELDS:
            super().__setattr__(key, value)
        else:
            setattr(self.records[0], key, value)

    def __str__(self):
        return f"{self.value}"


VALUE_INSTANCE_FIELDS = {field.name for field in fields(Value)}


@dataclass(repr=False)
class Model(Symbol):
    external_name: str = required_field()

    @cached_property
    def inference(self) -> ModelInference:
        return self.session.instance.get_inference(self)

    def __str__(self):
        return f"{self.external_name}"

    # forward inference methods
    def __getattr__(self, item: str):
        if item in self.inference.__dict__:
            return getattr(self.inference, item)
        else:
            raise AttributeError(item)


@dataclass(repr=False)
class Requirement(Symbol):
    module_name: Optional[str] = None
    module_id: Optional[UUID] = None
    version: Optional[str] = None

    def __str__(self):
        return f"{self.module_name or '<unspecified>'}@{self.version or '<any>'}"


@dataclass(repr=False)
class Build(Symbol):
    tasks: list[Task] = field(default_factory=list)
    models: list[Model] = field(default_factory=list)


@dataclass(repr=False)
class Block(Symbol):
    contents: list[Symbol] = field(default_factory=list)


SYMBOL_CLASS_BY_TYPE: dict[SymbolType, typing.Type[Symbol]] = {
    SymbolType.TYPE: Type,
    SymbolType.TASK: Task,
    SymbolType.EXPECTATION: Expectation,
    SymbolType.DATASET: Dataset,
    SymbolType.VALUE: Value,
    SymbolType.MODEL: Model,
    SymbolType.CODE: Code,
    SymbolType.REQUIREMENT: Requirement,
    SymbolType.BUILD: Build,
    SymbolType.BLOCK: Block,
}
SYMBOL_TYPE_BY_CLASS: dict[typing.Type[Symbol], SymbolType] = {
    v: k for k, v in SYMBOL_CLASS_BY_TYPE.items()
}
SYMBOL_FIELDS_BY_TYPE = {t: fields(c) for t, c in SYMBOL_CLASS_BY_TYPE.items()}
SYMBOL_FIELDS_NAMES_BY_TYPE = {
    t: {f.name for f in fields(c)} for t, c in SYMBOL_CLASS_BY_TYPE.items()
}

EMPTY_FUNC_TYPE = Type(name=None, tag=TypeTag.FUNCTION)
EMPTY_STRUCT_TYPE = Type(name=None, tag=TypeTag.STRUCT)


def make_func_type(inputs: list[TypeNode], outputs: list[TypeNode], name: str = None) -> Type:
    """Create a function type from input and output types."""

    return Type(
        name=name,
        tag=TypeTag.FUNCTION,
        children=[*inputs, *outputs],
    )


def make_struct_type(
    *children: TypeNode,
    name: str = None,
    description: str = None,
    is_array: bool = False,
    is_nullable: bool = False,
) -> Type:
    """Create a struct type from children types."""
    return Type(
        name=name,
        description=description,
        tag=TypeTag.STRUCT,
        children=list(children),
        is_array=is_array,
        is_nullable=is_nullable,
    )


def deepcopy_types(nodes: list[Field] | None, keep_id: bool = True) -> list[Field]:
    nodes = nodes or []
    return [node.deepcopy(keep_id=keep_id) for node in nodes]


def flatten_func_type(func_type: Type) -> Type:
    """Inline the input and output types into one struct."""
    # check that no input children are called output (hacky deluxe)
    return Type(
        name=func_type.name,
        tag=TypeTag.STRUCT,
        fields=[*deepcopy_types(func_type.fields, keep_id=False)],
    )


# :RemoteObjectType
class RemoteObjectStatus(enum.StrEnum):
    PREPARED = "prepared"
    UPLOADING = "uploading"
    AVAILABLE = "available"


@dataclass(repr=False, slots=True)
class RemoteObject(LanguageObject):
    """A proxy to a remotely stored object behaving like a Python file on demand."""

    id: UUID
    sha512: str
    content_length: int
    content_type: str
    name: str
    status: RemoteObjectStatus

    def __str__(self):
        return f"{self.id} {self.name} ({self.status}, {self.content_type}, {self.content_length} bytes)"

    def __repr__(self):
        return f"<RemoteObject {self}>"

    def __getitem__(self, item):
        return self.__dict__[item]

    async def aread(self, timeout: float = 1) -> bytes:
        """Read the object from the remote storage."""
        if self.status != RemoteObjectStatus.AVAILABLE:
            raise ValueError(f"unable to read {self}")
        return await self.session.instance.remote_object_aread(self, timeout=timeout)

    async def areadtext(self) -> str:
        return (await self.aread()).decode()

    async def areadlines(self) -> list[str]:
        return (await self.aread()).decode().splitlines()

    def read(self, timeout: float = 1) -> bytes:
        """Read the object from the remote storage."""
        return async_to_sync(self.aread)(timeout=timeout)

    def readtext(self) -> str:
        return self.read().decode()

    def readlines(self) -> list[str]:
        return self.read().decode().splitlines()


SecretValueT = typing.TypeVar("SecretValueT")


@dataclass(repr=False, slots=True)
class Secret(LanguageObject, typing.Generic[SecretValueT]):
    """A proxy to a remotely stored secret."""

    id: UUID
    sha512: str
    value: Optional[SecretValueT] = None

    def __str__(self):
        return f"{self.id} ({self.sha512})"

    def __repr__(self):
        return f"<Secret {self}>"

    async def areveal(self) -> SecretValueT:
        if self.value is not None:
            return self.value
        self.value = await self.session.instance.secret_areveal(self)
        return self.value

    def reveal(self) -> SecretValueT:
        return async_to_sync(self.areveal)()


STATIC_BUILTINS = {
    # primitive type builtins
    "string": str,
    "text": str,
    "number": float,
    "file": RemoteObject,
    "boolean": bool,
    "image": RemoteObject,
    "audio": RemoteObject,
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

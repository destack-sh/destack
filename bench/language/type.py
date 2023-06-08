from __future__ import annotations

import abc
import enum
import random
import string
import typing
import uuid
from collections import defaultdict
from dataclasses import dataclass, field, fields
from functools import cached_property
from typing import Any, Literal, Optional, Self, Union
from uuid import UUID

from asgiref.sync import async_to_sync
from more_itertools import first

from bench.language import IssueType
from bench.language.const import (
    FIELD_KEY_LENGTH,
    REFERENCE_REGEX,
    ErrorHandler,
    LookupBy,
    ModuleReference,
    RemoteObjectStatus,
    StatementModifier,
    StatementPath,
    StatementReference,
    StatementType,
    SymbolType,
    TypeFlag,
    TypeHint,
    TypeTag,
    parse_statement_path,
    raise_error,
)
from bench.language.dataset import Query, Sort
from bench.language.parse import parse_code
from bench.settings import logging
from bench.utils.fractional import INTEGER_ZERO, generate_key_between, generate_n_keys_between
from bench.utils.func import describe_type, dict_minus
from bench.utils.proxy import unproxy_value
from bench.utils.utils import required_field, to_pyidentifier

if typing.TYPE_CHECKING:
    from bench.language.inference import ModelInference
    from bench.language.session import Session


class LanguageObject(abc.ABC):
    session: "Session"
    id: UUID

    def __post_init__(self):
        if self.session is None:
            from bench.language.session import active_session

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


SymbolT = typing.TypeVar("SymbolT", bound="Symbol")


@dataclass(repr=False, slots=True)
class Scope:
    id: UUID
    name: str
    parent_scope: Optional[Scope]
    scopes_by_name: dict[str, Scope] = field(default_factory=dict)
    symbols_by_name: dict[str, Symbol] = field(default_factory=dict)
    symbols_by_id: dict[UUID, Symbol] = field(default_factory=dict)
    names_by_identifier: dict[str, str] = field(default_factory=dict)

    def find_symbol(self, name: str, by: LookupBy) -> Statement | None:
        """Find the statement recursively in this scope and its parents."""
        if by == LookupBy.Name:
            if name in self.symbols_by_name:
                return self.symbols_by_name[name]
        elif by == LookupBy.PyIdent:
            if name in self.names_by_identifier:
                name = self.names_by_identifier[name]
                if name in self.scopes_by_name:
                    return self.scopes_by_name[name]
        else:
            raise ValueError(f"unexpected lookup type: {by}")
        if self.parent_scope is not None:
            return self.parent_scope.find_symbol(name, by=by)
        return None

    def lookup_symbol(
        self,
        path: StatementPath | UUID | str,
        symbol_t: typing.Type[SymbolT] | None = None,
    ) -> SymbolT | None:
        """Lookup the symbol either by path or id. If path is a string, it can be
        it can be a name (lookup upwards) or a full relative/absolute path).
        """
        if isinstance(path, UUID):
            return self.symbols_by_id[path]
        elif isinstance(path, str):
            if ":" not in path and "." not in path:
                return self.find_symbol(path, by=LookupBy.Name)
            path = parse_statement_path(path)
        # strip leading . in path
        path = StatementPath(path.path[1:], path.name)
        first_part = path.path.split(".")[0]
        statement = self.find_symbol(first_part, by=LookupBy.Name)
        if statement is None:
            return None
        return statement.lookup_symbol(path, symbol_t=symbol_t)

    def add_scope(self, scope: Scope):
        self.scopes_by_name[scope.name] = scope

    def add_statement(self, statement: Statement):
        """Adds a statement and its symbol to this scope. Does not check for duplicates."""
        self.scopes_by_name[statement.name] = statement
        self.names_by_identifier[statement.ident] = statement.name
        self.symbols_by_name[statement.name] = statement.symbol
        self.symbols_by_id[statement.symbol.id] = statement.symbol

    def add_symbol(self, symbol: Symbol):
        """Adds a symbol to this scope. Does not check for duplicates."""
        self.symbols_by_name[symbol.name] = symbol
        self.symbols_by_id[symbol.id] = symbol

    def index(self):
        raise NotImplementedError

    def clear(self):
        """Resets this scope and all child scopes."""
        self.scopes_by_name.clear()
        self.symbols_by_name.clear()
        self.symbols_by_id.clear()
        self.names_by_identifier.clear()
        for scope in self.scopes_by_name.values():
            scope.clear()


@dataclass(repr=False)
class Module(Scope):
    name: str = required_field()
    files: list[File] = field(default_factory=list)
    id: UUID = field(default_factory=uuid.uuid4)
    dependencies: dict[str, Module | ModuleReference] = field(default_factory=dict)
    parent_scope = None

    def lookup_symbol(
        self,
        path: StatementPath | UUID | str,
        symbol_t: typing.Type[SymbolT] | None = None,
    ) -> SymbolT:
        if path.startswith("."):
            return super().lookup_symbol(path, symbol_t=symbol_t)
        else:
            match = REFERENCE_REGEX.match(path)
            module_name = match.group("module_owner") + "." + match.group("module_name")
            dependency = self.dependencies.get(module_name)
            if dependency is None:
                raise LookupError(f"could not find dependency {module_name}")
            return dependency.lookup_symbol("." + match.group("path") + ":" + match.group("name"))

    def sort(self):
        """Sorts the modules statements in-place according to parent & order keys."""
        for file in self.files:
            file.sort()

    def __str__(self):
        return f"{self.name} ({len(self.files)} files)"

    def __repr__(self):
        return f"<Module {str(self)}>"

    def index(self, on_error: ErrorHandler = raise_error):
        for file in self.files:
            file.index(on_error=on_error)

    def interp(self, on_error: ErrorHandler = raise_error):
        for file in self.files:
            file.interp(on_error=raise_error)


@dataclass(repr=False)
class File(LanguageObject, Scope):
    module: Module = required_field()
    path: str = required_field()
    statements: list[Statement] = field(default_factory=list)
    id: UUID = field(default_factory=uuid.uuid4)
    generated: bool = False

    def __str__(self):
        return f"{self.module.name}/{self.path}"

    def __repr__(self):
        return f"<File {str(self)}>"

    @property
    def parent_scope(self) -> Scope:
        return self.module

    @property
    def root_statements(self) -> list[Statement]:
        return [statement for statement in self.statements if statement.parent is None]

    def sort(self):
        """Sorts the files statements in-place according to parent & order keys."""
        # per parent (incl. root = None) sort by order key
        sorted_statements = []
        statements_by_parent_id: dict[UUID | None, list[Statement]] = defaultdict(list)
        for statement in self.statements:
            statements_by_parent_id[statement.parent_id].append(statement)

        def walk_dfs(statement: Statement):
            sorted_statements.append(statement)
            children = statements_by_parent_id.get(statement.id)
            if children is not None:
                for child in sorted(children, key=lambda s: s.order_key):
                    walk_dfs(child)

        roots = statements_by_parent_id.get(None, [])
        for statement in sorted(roots, key=lambda s: s.order_key):
            walk_dfs(statement)

        self.statements = sorted_statements

    def index(self, on_error: ErrorHandler = raise_error):
        for statement in self.statements:
            statement.index(on_error=on_error)

    def interp(self, on_error: ErrorHandler = raise_error):
        for statement in self.statements:
            statement.interp(on_error=on_error)


@dataclass(repr=False)
class Statement(LanguageObject, Scope):
    """A parsed but not interpreted statement in Bench source."""

    file: File = required_field()
    parent: Optional[Statement] = None
    order_key: str = required_field()
    type: StatementType = StatementType.SYMBOL
    modifier: Optional[StatementModifier] = None
    name: Optional[str] = None
    text: Optional[str] = None
    symbol: Optional[Symbol] = None
    symbol_type: Optional[SymbolType] = None
    reference: Optional[Statement | StatementPath | UUID] = None
    id: UUID = field(default_factory=uuid.uuid4)
    generated: bool = False

    _source: Optional[Any] = None

    def __str__(self):
        loc = self.file.path + ":" + str(self.infile_path)
        if self.type == StatementType.SYMBOL:
            content_str = "()"  # should have some nice __str__ here
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

    def is_expectable(self) -> bool:
        return self.symbol_type in (
            SymbolType.EXPECTATION,
            SymbolType.TASK,
            SymbolType.CODE,
            SymbolType.DATASET,
        )

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

    def interp(self, on_error: ErrorHandler = raise_error) -> None:
        self.symbol.interp(self, on_error)


@dataclass(repr=False)
class Symbol(LanguageObject):
    """An interpreted - fully resolved, templated and validated - symbol from Bench source."""

    name: str = field(default="")
    description: Optional[str] = None
    modifier: Optional[StatementModifier] = None
    reference: Optional[Symbol | StatementReference] = None
    definition: Optional[Symbol] = None
    source: Optional[Statement] = None
    id: UUID = field(default_factory=uuid.uuid4)

    def deepcopy(self, keep_id: bool = True, keep_reference: bool = True) -> "Symbol":
        kwargs = {**self.__dict__}
        kwargs["id"] = self.id if keep_id else uuid.uuid4()
        return self.__class__(**kwargs)

    def interp(self, scope: Scope, on_error: ErrorHandler = raise_error) -> None:
        """Updates, resolves and checks any derived/interpreted values on this symbol."""
        raise NotImplementedError

    def reinterp(self, scope: Scope = None, on_error: ErrorHandler = raise_error) -> None:
        if self.source is None:
            raise ValueError(f"cannot reinterp symbol {self} without source")
        self.interp(self.source, on_error=on_error)

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


class TypeNode(abc.ABC):
    id: UUID
    name: Optional[str]
    key: Optional[str]
    tag: TypeTag
    hint: Optional[TypeHint]
    flags: TypeFlag
    description: Optional[str]
    fields: list["TypeNode"]
    resolved_fields: list["TypeNode"]  # resolved fields with unions and such
    reference: Union[None, StatementReference, "HasType"]
    source: Optional[Statement]

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


def new_field_key() -> str:
    """Gets a random alphabetic key as a persistent key for a type node."""
    # (upper and lower case letters only)
    # :TypeNodeKeys
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

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{name_str}{self.tag}"

    def __repr__(self):
        return f"<Field {self}>"

    @property
    def resolved_fields(self) -> list[TypeNode]:
        if isinstance(self.reference, Type):
            return self.reference.fields
        return []

    fields = resolved_fields  # the same by default

    def deepcopy(self, keep_id: bool = True, deepcopy_reference: bool = True) -> "Field":
        if deepcopy_reference and isinstance(self.reference, Type):
            reference = self.reference.deepcopy(keep_id=True, deepcopy_reference=False)
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
            flags=self.flags,
        )


Expectable = Union["Expectation", "Task", "Dataset", "Code"]


@dataclass(repr=False)
class HasExpectations:
    """Symbols we can attach expectations to"""

    expectations: list[Expectable] = field(default_factory=list)
    resolved_expectations: list[Expectable] = None

    def expect(self, expectation: Expectable) -> "Self":
        self.expectations.append(expectation)
        return self

    def check(self, code: Code) -> "Self":
        # turn into ref and add modifier
        raise NotImplementedError

    def like(self, expectation: Expectable) -> "Self":
        # same
        raise NotImplementedError

    def unlike(self, expectation: Expectable) -> "Self":
        # same
        raise NotImplementedError

    def interp(self, scope: Scope, on_error: ErrorHandler) -> None:
        self.resolved_expectations = self.expectations


@dataclass(repr=False)
class HasType(TypeNode):
    """A symbol that has (but is not) a type"""

    tag: TypeTag = required_field()
    hint = None
    flags = TypeFlag.Zero
    fields: list[TypeNode] = field(default_factory=list)
    resolved_fields: list[TypeNode] = None
    key = None
    reference = None

    def extend(self, base: Type) -> "Self":
        """Adds the fields of another type to this one"""
        self.fields.append(
            Field(
                tag=TypeTag.TYPE_REFERENCE,
                reference=base,
                flags=TypeFlag.IsUnionWith,
            )
        )
        return self

    def append(self, field: Field) -> "Self":
        """Adds a field to this type"""
        self.fields.append(field)
        return self

    @property
    def type(self) -> TypeNode:
        """For clarity when explicitly referring to the type of a symbol"""
        return self

    def interp(self, scope: Scope, on_error: ErrorHandler) -> None:
        # resolve references
        for node in self.walk():
            if node.reference is None or isinstance(node.reference, Statement):
                continue  # nothing to resolve
            # normalize path to statement
            resolved_stmt = scope.resolve_reference(
                reference=node.reference, symbol_type=SymbolType.TYPE, on_error=on_error
            )
            if resolved_stmt is None:
                continue  # error already reported
            if not isinstance(resolved_stmt.symbol, TypeNode):
                on_error()
            node.reference = resolved_stmt.symbol
        # expand unions

        # inline union types (and extend expectations if they exist)
        def _resolve_unions(node: TypeNode, path: list[TypeNode]) -> list[TypeNode]:
            if any(n.id == node.id for n in path):
                path = "->".join(str(n) for n in path + [node])
                on_error(IssueType.CIRCULAR_UNION, node.source, path=path)
                return []
            if not isinstance(node, HasType) or node.resolved_fields is not None:
                return node.fields  # not a type or already resolved
            if not any(n.flags & TypeFlag.IsUnionWith for n in node.fields):
                node.fields = node.fields
                return node.fields  # skip, not a union
            path = path + [node]
            resolved_fields = []
            node.fields = deepcopy_types(node.fields)  # retain originals
            for child in node.fields:
                if not child.flags & TypeFlag.IsUnionWith:
                    resolved_fields.append(child)
                    continue
                if not isinstance(child.reference, Type):
                    continue  # ignore unresolved
                # inline child's type nodes
                for to_inline in _resolve_unions(child.reference, path):
                    existing = first((n for n in resolved_fields if n.name == to_inline.name), None)
                    # check if type is compatible if overlapping
                    if existing is not None and (
                        existing.tag != to_inline.tag
                        or existing.flags != to_inline.flags
                        or existing.hint != to_inline.hint
                    ):
                        # TODO @Robustness: check union type compatibility properly
                        path = "->".join(str(n) for n in path)
                        on_error(
                            IssueType.MISMATCHED_UNION,
                            node,
                            node=existing,
                            other=to_inline,
                            path=path,
                        )
                        continue
                    resolved_fields.append(to_inline)
                if isinstance(node, Type):  # extend expectations
                    node.expectations.extend(child.reference.expectations)
            node.resolved_fields = resolved_fields
            return node.fields

        _resolve_unions(self, [])


@dataclass(repr=False)
class Type(Symbol, TypeNode, HasExpectations):
    tag: TypeTag = required_field()
    flags: TypeFlag = TypeFlag(0)
    fields: list[TypeNode] = field(default_factory=list)
    resolved_fields: list[TypeNode] = None
    # not directly configurable for types
    hint = None
    key = None
    reference = None

    @cached_property
    def py_type(self) -> type | enum.Enum:
        return self.session.instance.get_py_type(self)

    def __instancecheck__(self, instance):
        # this used to mimic python type but probably want to check the value (duck typing)?
        raise NotImplementedError

    def __subclasscheck__(self, subclass):
        raise NotImplementedError

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
class Task(Symbol, HasExpectations, HasType):
    tag = TypeTag.FUNCTION
    is_async = True
    # should probably store last good implementation ... in redis?
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
        from bench.language.build import XConsiderError, XGenerationError

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
class Expectation(Symbol):
    pass

    def __init__(self, name: str = None, description: str = None, expectations: list = None):
        super().__init__(name=name, description=description, expectations=expectations)


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
class Code(Symbol, HasType):
    tag = TypeTag.FUNCTION
    language: Literal["python"] | Literal["x"] = "python"
    code: Optional[str] = None
    parse: Optional[CodeParse] = None
    transform: Optional[CodeTransformation] = None
    references: dict[str, Symbol] = field(default_factory=dict)
    context: dict[str, Symbol] = field(default_factory=dict)

    @cached_property
    def code_callable(self) -> AsyncCodeCallable | SyncCodeCallable:
        return self.session.instance.get_code_callable(self)

    def interp(self, scope: Scope, on_error: ErrorHandler = raise_error) -> None:
        input_keys = (input.ident for input in self.inputs)
        self.parse = parse_code(self.code)
        for key, reference in self.parse.references.items():
            if key in input_keys:
                continue  # input arguments are obviously not resolved
            reference = scope.resolve_reference(
                reference=reference, on_error=on_error, by=LookupBy.PyIdent
            )
            if reference is not None:
                self.references[key] = reference.symbol
            else:
                on_error(IssueType.MISSING_REFERENCE, reference=reference, symbol=self)

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
    reference: Optional[Dataset | StatementReference] = None
    order_key: str = field(default_factory=uuid.uuid4)
    id: UUID = field(default_factory=uuid.uuid4)


DEFAULT_VIEW = DatasetView(name="default")


@dataclass(repr=False)
class Dataset(Symbol, HasType):
    tag = TypeTag.STRUCT
    flags = TypeFlag.IsArray
    length: Optional[int] = None
    inmemory: bool = True
    records: Optional[list[Record]] = None
    views: Optional[list[DatasetView]] = None

    def __post_init__(self):
        self.meta = RecordMeta(type=self.type, session=self.session, owner=self)

    @property
    def default_view(self) -> DatasetView:
        return self.views[0] if self.views else DEFAULT_VIEW

    def clear(self):
        self.session.tracer.dataset_clear(self)
        if self.inmemory:
            self.records = []

    def append(self, record: Record = None, **data):
        if record is not None:
            if data:
                raise ValueError("cannot pass both record and data")
            data = record._data
        # nocheckin: broken if not in memory
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
class Value(Symbol, HasType):
    tag = TypeTag.STRUCT
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
    def inference(self) -> "ModelInference":
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

    def __iter__(self):
        return iter(self.contents)

    def __getattr__(self, item: str):
        content = first(self.contents, lambda c: c.name == item, None)
        if content is not None:
            return content
        else:
            raise AttributeError(item)


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


def deepcopy_types(nodes: list[Field] | None, keep_id: bool = True) -> list[Field]:
    nodes = nodes or []
    return [node.deepcopy(keep_id=keep_id) for node in nodes]


# :RemoteObjectType


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

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
from typing import Any, Optional, Self, Union
from uuid import UUID

from asgiref.sync import async_to_sync
from more_itertools import first

from bench.bench import IssueType
from bench.bench.const import (
    FIELD_KEY_LENGTH,
    REFERENCE_REGEX,
    ExpectationModifier,
    LookupBy,
    ModuleReference,
    RemoteObjectStatus,
    StatementPath,
    StatementType,
    TypeFlag,
    TypeHint,
    TypeTag,
    parse_statement_path,
)
from bench.bench.dataset import Query, Sort
from bench.bench.issue import IssueHandler, raise_if_error
from bench.bench.parse import parse_code
from bench.settings import logging
from bench.utils.fractional import INTEGER_ZERO, generate_key_between, generate_n_keys_between
from bench.utils.func import describe_type, dict_minus
from bench.utils.proxy import unproxy_value
from bench.utils.utils import required_field, to_pyidentifier

if typing.TYPE_CHECKING:
    from bench.bench.inference import ModelInference
    from bench.bench.session import Session


@dataclass(repr=False)
class ModuleNode(abc.ABC):
    id: UUID = field(default_factory=uuid.uuid4)
    parent: Optional[ModuleNode] = None
    revision: int = 0

    @property
    def attached(self) -> bool:
        return self.parent is not None


@dataclass(repr=False)
class HasSession(abc.ABC):
    id: UUID = field(default_factory=uuid.uuid4)
    _session: "Session" = None

    @property
    def session(self) -> "Session":
        """Access the session, error-ing if there is none."""
        if self._session is None:
            raise RuntimeError(f"no active session for {self}")
        return self._session

    def __post_init__(self):
        if self._session is None:
            from bench.bench.session import active_session

            self._session = active_session.get()
            if self._session is not None:
                # we pass in session on instantiate, so this must be new
                self._session.add(self, new=True)
        else:
            self._session.add(self, new=False)

    def __del__(self):
        if self._session is not None:
            self._session.remove(self)

    @property
    def logger(self) -> logging.Logger:
        return self.session.logger


SymbolT = typing.TypeVar("SymbolT", bound="Symbol")


@dataclass(repr=False)
class Scope:
    parent_scope: Optional[Scope] = None
    scopes_by_name: dict[str, Scope] = field(default_factory=dict)
    symbols_by_id: dict[UUID, Symbol] = field(default_factory=dict)
    names_by_identifier: dict[str, str] = field(default_factory=dict)

    @cached_property
    def root_scope(self) -> Scope:
        if self.parent_scope is None:
            return self
        return self.parent_scope.root_scope

    def find_symbol(self, name: str, by: LookupBy) -> SymbolT | None:
        """Find the statement recursively in this scope and its parents."""
        scope = None
        if by == LookupBy.Name:
            scope = self.scopes_by_name.get(name)
        elif by == LookupBy.PyIdent:
            if name in self.names_by_identifier:
                name = self.names_by_identifier[name]
                scope = self.scopes_by_name.get(name)
        else:
            raise ValueError(f"unexpected lookup type: {by}")
        if scope is not None:
            if not isinstance(scope, Symbol):
                raise TypeError(f"expected symbol, got {type(scope)}")
            return scope
        if self.parent_scope is not None:
            return self.parent_scope.find_symbol(name, by=by)
        return None

    def lookup_symbol(
        self,
        path: StatementPath | UUID | str,
        symbol_t: StatementType | typing.Type[SymbolT] | None = None,
        by: LookupBy = LookupBy.Name,
    ) -> SymbolT | None:
        """Lookup the symbol either by path or id. If path is a string, it can be
        it can be a name (lookup upwards) or a full relative/absolute path).
        """
        if isinstance(path, UUID):
            return self.root_scope.symbols_by_id.get(path)
        elif isinstance(path, str):
            if ":" not in path and "." not in path:
                return self.find_symbol(path, by=by)
            path = parse_statement_path(path)
        if path.path == ".":
            return self.find_symbol(path.name, by=by)
        # strip leading . in path
        path = StatementPath(path.path[1:], path.name)
        first_part = path.path.split(".")[0]
        scope = self.find_symbol(first_part, by=by)
        if scope is None:
            return None
        return scope.lookup_symbol(path, symbol_t=symbol_t)

    def add_statement(self, statement: Statement, by_name: bool, on_issue: IssueHandler) -> None:
        if statement.name is not None and by_name:
            if statement.name in self.scopes_by_name:
                on_issue(
                    type=IssueType.AMBIGUOUS_DEFINITION, subject=statement, path=statement.path
                )
            else:
                self.scopes_by_name[statement.name] = statement
                self.names_by_identifier[statement.ident] = statement.name

        self.symbols_by_id.update(statement.symbols_by_id)
        if isinstance(statement, Symbol):
            self.symbols_by_id[statement.id] = statement

    def clear(self):
        """Resets this scope and all child scopes."""
        self.scopes_by_name.clear()
        self.symbols_by_id.clear()
        self.names_by_identifier.clear()
        for scope in self.scopes_by_name.values():
            scope.clear()


@dataclass(repr=False)
class Module(ModuleNode, HasSession, Scope):
    name: str = required_field()
    files: list[File] = field(default_factory=list)
    dependencies: dict[str, Module | ModuleReference] = field(default_factory=dict)
    parent_scope: Scope = None
    committed: bool = False
    # index
    files_by_parent_id: dict[UUID | None, list[File]] | None = None

    @property
    def attached(self) -> bool:
        return True  # root is always "attached"

    def lookup_symbol(
        self,
        path: StatementPath | UUID | str,
        symbol_t: typing.Type[SymbolT] | None = None,
        by: LookupBy = LookupBy.Name,
    ) -> SymbolT:
        if path.startswith("."):
            return super().lookup_symbol(path, symbol_t=symbol_t, by=by)
        else:
            match = REFERENCE_REGEX.match(path)
            module_name = match.group("module_owner") + "." + match.group("module_name")
            dependency = self.dependencies.get(module_name)
            if dependency is None:
                raise LookupError(f"could not find dependency {module_name}")
            localized_path = "." + match.group("path") + ":" + match.group("name")
            return dependency.lookup_symbol(localized_path, symbol_t=symbol_t, by=by)

    def __str__(self):
        return f"{self.name} ({len(self.files)} files)"

    def __repr__(self):
        return f"<Module {str(self)}>"

    def index(self, on_issue: IssueHandler = raise_if_error):
        self.files_by_parent_id = {
            file.parent.id if file.parent else None: file for file in self.files
        }
        for file in self.files:
            file.index(on_issue=on_issue)
            if file.name in self.scopes_by_name:
                on_issue(type=IssueType.AMBIGUOUS_DEFINITION, subject=file, path=file.name)
            else:
                self.scopes_by_name[file.name] = file
            self.symbols_by_id.update(file.symbols_by_id)

    def interp(self, on_issue: IssueHandler = raise_if_error):
        for file in self.files:
            file.interp(on_issue=on_issue)


@dataclass(repr=False)
class File(ModuleNode, HasSession, Scope):
    module: Module = required_field()
    name: str = required_field()
    parent: File | None = None
    children: list[File] | None = None
    statements: list[Statement] = field(default_factory=list)
    # index
    statements_by_parent_id: dict[UUID | None, list[Statement]] | None = None

    def __post_init__(self):
        super().__post_init__()
        if self.parent_scope is None:
            self.parent_scope = self.module
        self._sort()

    def __str__(self):
        return f"{self.module.name}/{self.name}"

    def __repr__(self):
        return f"<File {str(self)}>"

    @property
    def path(self) -> str:
        if self.parent:
            return f"{self.parent.path}.{self.name}"
        else:
            return self.name

    def append(self, *statements: Statement):
        """Appends the statements to this file."""
        for statement in statements:
            statement.file = self
            statement.parent_scope = self
            self.statements.append(statement)

    def _sort(self):
        """Sorts the files statements in-place according to parent & order keys."""
        # per parent (incl. root = None) sort by order key
        sorted_statements = []
        self.statements_by_parent_id: dict[UUID | None, list[Statement]] = defaultdict(list)
        for statement in self.statements:
            self.statements_by_parent_id[statement.parent_id].append(statement)

        def walk_dfs(statement: Statement):
            sorted_statements.append(statement)
            children = self.statements_by_parent_id.get(statement.id)
            if children is not None:
                for child in sorted(children, key=lambda s: s.order_key):
                    walk_dfs(child)

        roots = self.statements_by_parent_id.get(None, [])
        for statement in sorted(roots, key=lambda s: s.order_key):
            walk_dfs(statement)

        self.statements = sorted_statements

    def index(self, on_issue: IssueHandler = raise_if_error):
        """Indexes all statements in this file into the scope."""
        self.clear()
        self._sort()
        for statement in self.statements:
            statement.index(on_issue=on_issue)
            is_root = statement.parent_id is None
            self.add_statement(statement, by_name=is_root, on_issue=on_issue)

    def interp(self, on_issue: IssueHandler = raise_if_error):
        for statement in self.statements:
            statement.interp(on_issue=on_issue)


@dataclass(repr=False)
class Statement(ModuleNode, HasSession, Scope):
    """A parsed but not interpreted statement in Bench source."""

    file: File | None = None
    parent: Optional[Statement] = None
    children: list[Statement] | None = None
    order_key: str | None = None
    type: StatementType = StatementType.BLANK
    name: Optional[str] = None
    id: UUID = field(default_factory=uuid.uuid4)

    def __post_init__(self):
        super().__post_init__()
        if self.file is None and self.parent is not None:
            self.file = self.parent.file
        if self.parent_scope is None:
            # default scope to parent or file if not set
            self.parent_scope = self.parent or self.file

    def __str__(self):
        return f"{self.path} {self.type} {self.name}"

    def __repr__(self):
        return f"<{self.__class__.name} {self}>"

    @property
    @property
    def path(self) -> str:
        if self.file is None:
            return f"<detached>:{self.infile_path}"
        else:
            return self.file.path + ":" + str(self.infile_path)

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
        if self.file is None:
            raise ValueError(f"cannot get fqn of detached statement {self}")
        return f"{self.file.module.name}.{self.file.name.replace('/', '.')}.{self.name}"

    @property
    def ident(self) -> str:
        return to_pyidentifier(self.name)

    @property
    def parent_id(self) -> Optional[UUID]:
        return self.parent.id if self.parent else None

    def index(self, on_issue: IssueHandler = raise_if_error):
        self.clear()
        self.children = self.file.statements_by_parent_id.get(self.id, [])
        for child in self.children:
            # only index self, not children
            # (unlike in file/module, statement nesting is only semantic, not structural)
            self.add_statement(child, by_name=True, on_issue=on_issue)

    def interp(self, scope: Scope, on_issue: IssueHandler = raise_if_error) -> None:
        pass


@dataclass(repr=False)
class Blank(Statement):
    """A blank statement."""

    type: StatementType = StatementType.BLANK


@dataclass(repr=False)
class Comment(Statement):
    """A comment that's not semantic/interpreted by default."""

    type: StatementType = StatementType.COMMENT
    html: str | None = None

    @property
    def text(self) -> str:
        """The rendered text of this comment."""
        return self.html or ""


StatementReference = typing.Union[Statement, StatementPath, UUID]


class SymbolBase(abc.ABC):
    """Base for interpretable symbols for type-checking."""

    session: Session

    def interp(self, scope: Scope, on_issue: IssueHandler = raise_if_error) -> None:
        raise NotImplementedError

    def clear_interp(self) -> None:
        raise NotImplementedError

    def reinterp(self, scope: Scope = None, on_issue: IssueHandler = raise_if_error) -> None:
        raise NotImplementedError


@dataclass(repr=False)
class Symbol(Statement, SymbolBase):
    """An interpretable and semantic statement (symbol) in Bench source."""

    def interp(self, scope: Scope, on_issue: IssueHandler = raise_if_error) -> None:
        """Updates, resolves and checks any derived/interpreted values on this symbol."""
        raise NotImplementedError

    def clear_interp(self) -> None:
        """Clears any derived/interpreted values on this symbol."""
        raise NotImplementedError

    def reinterp(self, scope: Scope = None, on_issue: IssueHandler = raise_if_error) -> None:
        self.clear_interp()
        self.interp(scope or self, on_issue=on_issue)


class TypeBase(abc.ABC):
    id: UUID
    name: Optional[str]
    key: Optional[str]
    tag: TypeTag
    hint: Optional[TypeHint]
    flags: TypeFlag
    description: Optional[str]
    fields: list["TypeBase"]
    resolved_fields: list["TypeBase"]  # resolved fields with unions and such
    reference: Union[None, StatementReference, "HasType"]
    source: Optional[Statement]

    @property
    def ident(self):
        return to_pyidentifier(self.name)

    @property
    def bases(self):
        return [field for field in self.fields if field.flags & TypeFlag.IsUnionWith]

    @property
    def inputs(self) -> list["TypeBase"]:
        return [child for child in self.fields if not child.flags & TypeFlag.IsOutput]

    @property
    def outputs(self) -> list["TypeBase"]:
        return [child for child in self.fields if child.flags & TypeFlag.IsOutput]

    def __getitem__(self, item: str) -> "TypeBase":
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

    def walk(self, path: list[TypeBase] | None = None, include_references: bool = False):
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

    def unkey(self, data: Any, is_output: bool = None, to_ident: bool = False) -> Any:
        """'Unkeys' data by replacing keys with the names of the type nodes."""
        from bench.bench.typer import unkey_value

        return unkey_value(data, self, is_output=is_output, to_ident=to_ident)

    def rekey(self, data: Any, is_output: bool = None, via_ident: bool = False) -> Any:
        """'Keys' data by replacing names with the keys of the type nodes."""
        from bench.bench.typer import rekey_value

        return rekey_value(data, self, is_output=is_output, from_ident=via_ident)


def new_field_key() -> str:
    """Gets a random alphabetic key as a persistent key for a type node."""
    # (upper and lower case letters only)
    # :TypeNodeKeys
    return "".join(random.choices(string.ascii_letters, k=FIELD_KEY_LENGTH))


@dataclass(repr=False)
class Field(ModuleNode, HasSession, TypeBase):
    name: Optional[str] = None
    tag: TypeTag = required_field()
    hint: Optional[TypeHint] = None
    order_key: str = INTEGER_ZERO
    key: str = field(default_factory=new_field_key)
    description: Optional[str] = None
    flags: TypeFlag = TypeFlag(0)
    metadata: dict[str, Any] = None
    reference: Union[None, StatementPath, Statement, UUID, "Type"] = None

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{name_str}{self.tag}"

    def __repr__(self):
        return f"<Field {self}>"

    @property
    def resolved_fields(self) -> list[TypeBase]:
        if isinstance(self.reference, Type):
            return self.reference.fields
        return []

    fields = resolved_fields  # the same by default


Expectable = Union["Expectation", "Task", "Dataset", "Code"]


@dataclass(repr=False)
class IsExpectable:
    """Symbols that can define expectations"""

    modifier: Optional[ExpectationModifier] = None


@dataclass(repr=False)
class HasExpectations(SymbolBase, IsExpectable):
    """Symbols we can attach expectations to"""

    description: Optional[str] = None
    expectations: list[Expectable] = field(default_factory=list)
    resolved_expectations: list[Expectable] | None = None

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

    def interp(self, scope: Scope, on_issue: IssueHandler = raise_if_error) -> None:
        if self.resolved_expectations is not None:
            return
        resolved_expectations = [*self.expectations]
        if isinstance(self, HasType):
            # inline union expectations
            for base in self.bases:
                if isinstance(base.reference, HasExpectations):
                    resolved_expectations.extend(base.reference.expectations)
        self.resolved_expectations = resolved_expectations

    def clear_interp(self) -> None:
        self.resolved_expectations = None


@dataclass(repr=False)
class HasType(TypeBase, SymbolBase):
    """A symbol that has (but is not) a type"""

    tag: TypeTag = required_field()
    hint: Optional[TypeHint] = None
    flags: TypeFlag = TypeFlag.Zero
    fields: list[TypeBase] = field(default_factory=list)
    resolved_fields: list[TypeBase] | None = None
    key: str = None
    reference = None

    def interp(self, scope: Scope, on_issue: IssueHandler = raise_if_error) -> None:
        # resolve references
        for node in self.walk():
            if node.reference is None or isinstance(node.reference, Symbol):
                continue  # nothing to resolve
            # normalize path to statement
            symbol = scope.lookup_symbol(node.reference, StatementType.TYPE)
            if symbol is None:
                continue  # error already reported
            if not isinstance(symbol, TypeBase):
                on_issue(type=IssueType.MISSING_REFERENCE, symbol=node, reference=node.reference)
            node.reference = symbol

        # expand unions (recursively)
        Type._resolve_unions(self, [], on_issue)

    def clear_interp(self) -> None:
        self.resolved_fields = None

    def extend(self, *bases: Type) -> "Self":
        """Adds the fields of another type to this one"""
        for base in bases:
            field = Field(
                name=None,
                tag=TypeTag.TYPE_REFERENCE,
                reference=base,
                flags=TypeFlag.IsUnionWith,
            )
            self.session.tracer.field_append(self, field)
            self.fields.append(field)
        self.reinterp()
        return self

    def append(self, *fields: Field) -> "Self":
        """Adds a field to this type"""
        for field in fields:  # noqa shadows dataclass.field
            self.session.tracer.field_append(self, field)
            self.fields.append(field)
        self.reinterp()
        return self

    @property
    def type(self) -> TypeBase:
        """For clarity when explicitly referring to the type of a symbol"""
        return self

    @staticmethod
    def _resolve_unions(
        node: TypeBase, path: list[TypeBase], on_issue: IssueHandler
    ) -> list[TypeBase]:
        if any(n.id == node.id for n in path):
            path = "->".join(str(n) for n in path + [node])
            on_issue(type=IssueType.CIRCULAR_UNION, subject=node, path=path)
            return []  # circle!
        if node.resolved_fields is not None:
            return node.resolved_fields  # already resolved
        if not any(n.flags & TypeFlag.IsUnionWith for n in node.fields):
            node.resolved_fields = node.fields
            return node.fields  # skip, not a union
        path = path + [node]

        resolved_fields = []
        for child in node.fields:
            if not child.flags & TypeFlag.IsUnionWith:
                resolved_fields.append(child)
                continue
            if not isinstance(child.reference, TypeBase):
                continue  # ignore unresolved
            # inline child's type nodes
            for to_inline in Type._resolve_unions(child.reference, path, on_issue):
                existing = first((n for n in resolved_fields if n.name == to_inline.name), None)
                # check if type is compatible if overlapping
                if existing is not None and (
                    existing.tag != to_inline.tag
                    or existing.flags != to_inline.flags
                    or existing.hint != to_inline.hint
                ):
                    # TODO @Robustness: check union type compatibility properly/deeply
                    path = "->".join(str(n) for n in path)
                    on_issue(type=IssueType.MISMATCHED_UNION, symbol=node, path=path)
                    continue
                resolved_fields.append(to_inline)
        node.resolved_fields = resolved_fields
        return node.fields


@dataclass(repr=False)
class Type(Symbol, HasType, HasExpectations):
    description: Optional[str] = None
    tag: TypeTag = required_field()
    flags: TypeFlag = TypeFlag(0)
    # not directly configurable for types
    hint = None
    key = None
    reference = None

    @cached_property
    def py_type(self) -> type | enum.Enum:
        return self.session.instance.get_py_type(self)

    def interp(self, scope: Scope, on_issue: IssueHandler = raise_if_error) -> None:
        HasType.interp(self, scope, on_issue)
        HasExpectations.interp(self, scope, on_issue)

    def clear_interp(self) -> None:
        HasType.clear_interp(self)
        HasExpectations.clear_interp(self)

    def __call__(self, *args, **kwargs):
        return self.py_type(*args, **kwargs)

    def __str__(self):
        name_str = f"{self.name} " if self.name else ""
        return f"{name_str}{self.tag}"

    def __repr__(self):
        return f"<Type {self}>"

    def __getattr__(self, item):
        if any(item == field.name for field in self.fields):
            return self.fields[item]
        return super(HasType).__getattr__(item)


TYPE_FIELD_KEYS = {field.name for field in fields(Type)}


@dataclass(repr=False)
class Task(Symbol, HasType, HasExpectations):
    description: Optional[str] = None
    tag: TypeTag = TypeTag.FUNCTION
    is_async: bool = True
    # should probably store last good implementation ... in redis?
    last_good_impl_idx: int = 0

    def interp(self, scope: Scope, on_issue: IssueHandler = raise_if_error) -> None:
        HasType.interp(self, scope, on_issue)
        HasExpectations.interp(self, scope, on_issue)

    def clear_interp(self) -> None:
        HasType.clear_interp(self)
        HasExpectations.clear_interp(self)

    async def __call__(
        self,
        *args,
        build: str = None,
        model: Model | str = None,
        retries: int = None,
        cache: bool = None,
        timeout: float = None,
        **kwargs,
    ):
        from bench.bench.build import XConsiderError, XGenerationError

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
class Expectation(Symbol, HasExpectations):
    reference: StatementReference | Statement | None = None
    description: Optional[str] = None

    def interp(self, scope: Scope, on_issue: IssueHandler = raise_if_error) -> None:
        # resolve reference
        if self.reference is not None:
            resolved = scope.lookup_symbol(self.reference)
            if resolved is None:
                on_issue(type=IssueType.MISSING_REFERENCE, reference=self.reference, subject=self)
            else:
                self.reference = resolved
        # interp
        HasExpectations.interp(self, scope, on_issue)
        if isinstance(self.reference, HasExpectations):
            self.resolved_expectations.extend(self.reference.expectations)

    def clear_interp(self) -> None:
        HasExpectations.clear_interp(self)


@dataclass(repr=False)
class CodeTransformation:
    original_code: str
    transformed_code: str
    method_name: str
    start_offset: int


@dataclass(repr=False)
class CodeParse:
    references: dict[str, "StatementPath"] = field(default_factory=dict)
    is_async: bool = False
    fake_line_numbers: list[int] = field(default_factory=list)


AsyncCodeCallable = typing.Callable[..., typing.Coroutine]
SyncCodeCallable = typing.Callable[..., Any]


@dataclass(repr=False)
class Code(Symbol, HasType, IsExpectable):
    tag: TypeTag = TypeTag.FUNCTION
    language: str = "python"
    code: Optional[str] = None
    parse: Optional[CodeParse] = None
    references: dict[str, Symbol] | None = field(default_factory=dict)
    transform: Optional[CodeTransformation] = None
    _code_callable: AsyncCodeCallable | SyncCodeCallable | None = None

    def interp(self, scope: Scope, on_issue: IssueHandler = raise_if_error) -> None:
        HasType.interp(self, scope, on_issue)

        # parse and resolve code references
        input_keys = (input.ident for input in self.inputs)
        self.parse = parse_code(self.code)
        for key, reference in self.parse.references.items():
            if key in input_keys:
                continue  # input arguments are not context
            resolved = scope.lookup_symbol(reference, by=LookupBy.PyIdent)
            if resolved is not None:
                self.references[key] = resolved
            else:
                on_issue(type=IssueType.MISSING_REFERENCE, reference=reference, subject=self)

    def clear_interp(self) -> None:
        self.parse = None
        self.references = None
        self.transform = None
        self._code_callable = None
        HasType.clear_interp(self)

    async def __call__(self, *args, **kwargs):
        if self._code_callable is None:
            self._code_callable = self.session.instance.get_code_callable(self)
        log = self.session.logger.bind(code=self, args=len(args), kwargs=describe_type(kwargs))
        try:
            self.session.tracer.code_enter(self, args, kwargs)
            log.debug("code.enter")
            result = await self._code_callable(*args, **kwargs)
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
class Record:
    _dataset: Dataset
    _order_key: str
    _data: typing.Any = field(default_factory=dict)
    _id: UUID = field(default_factory=uuid.uuid4)

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
    name: str = None
    query: Optional[Query] = None
    sort: Optional[list[Sort]] = None
    reference: Optional[Dataset | UUID] = None
    order_key: str = field(default_factory=uuid.uuid4)
    id: UUID = field(default_factory=uuid.uuid4)


DEFAULT_VIEW = DatasetView(name="default")


@dataclass(repr=False)
class Dataset(Symbol, HasType, IsExpectable):
    description: Optional[str] = None
    tag: TypeTag = TypeTag.STRUCT
    flags: TypeFlag = TypeFlag.IsArray
    length: Optional[int] = None
    inmemory: bool = True
    versioned: bool = True
    records: Optional[list[Record]] = None
    views: Optional[list[DatasetView]] = None

    def interp(self, scope: Scope, on_issue: IssueHandler = raise_if_error) -> None:
        # resolve references
        for view in self.views or []:
            if view.reference is not None:
                view.reference = scope.lookup_symbol(view.reference, by=LookupBy.PyIdent)
                if view.reference is None:
                    on_issue(type=IssueType.MISSING_REFERENCE, subject=self)

        HasType.interp(self, scope, on_issue)

    def clear_interp(self) -> None:
        HasType.clear_interp(self)

    @property
    def default_view(self) -> DatasetView:
        return self.views[0] if self.views else DEFAULT_VIEW

    def clear(self):
        self.session.tracer.dataset_clear(self)
        if self.inmemory:
            self.records = []

    def append(self, record: Record = None, **data):
        """Appends a record to the dataset."""
        if record is not None:
            if data:
                raise ValueError("cannot pass both record and data")
            data = record._data
        # nocheckin: remote datasets
        data = unproxy_value(data)  # remove source proxy if any
        # insert at end
        last_ok = self.records[-1]._order_key if self.records else INTEGER_ZERO
        record = Record(
            _id=uuid.uuid4(),
            _dataset=self,
            _order_key=generate_key_between(last_ok, None),
            _data=data,
        )
        self.session.tracer.dataset_append(self, record)
        if self.inmemory:
            if self.records is None:
                self.records = []
            self.records.append(record)

    def extend(self, records: typing.Iterable[Record | dict]):
        """Extends the dataset with the given records."""
        datas = [  # remove source proxy if any
            unproxy_value(record._data) if isinstance(record, Record) else unproxy_value(record)
            for record in records
        ]
        last_ok = self.records[-1]._order_key if self.records else INTEGER_ZERO
        # nocheckin: remote datasets
        oks = generate_n_keys_between(last_ok, None, len(datas))
        records = [
            Record(
                _id=uuid.uuid4(),
                _dataset=self,
                _order_key=ok,
                _data=data,
            )
            for ok, data in zip(oks, datas)
        ]
        self.session.tracer.dataset_extend(self, records)
        if self.inmemory:
            if self.records is None:
                self.records = []
            self.records.extend(records)

    async def asearch(self, query: Query, sort: list[Sort] = None) -> Dataset:
        """Searches this dataset remotely."""
        # nocheckin: remote datasets
        self.session.tracer.dataset_search(self, query, sort)
        return await self.session.instance.search(self, query, sort)

    def search(self, query: Query, sort: list[Sort] = None) -> Dataset:
        """Searches this dataset remotely."""
        return async_to_sync(self.asearch)(query, sort)

    def __getitem__(self, item: int | slice) -> Record | list[Record]:
        # nocheckin: proxy dataset access
        return self.records[item]

    def __len__(self):
        return self.length

    def __iter__(self):
        # nocheckin: remote datasets
        if self.inmemory:
            return iter(self.records)
        raise NotImplementedError("nocheckin: remote datasets")

    def __aiter__(self):
        # nocheckin: remote datasets
        if self.inmemory:
            yield from self.records
        raise NotImplementedError("nocheckin: remote datasets")


@dataclass(repr=False)
class Value(Symbol, HasType, IsExpectable):
    description: Optional[str] = None
    tag: TypeTag = TypeTag.STRUCT
    flags: TypeFlag = TypeFlag.Zero
    value: Any = None

    @property
    def keys(self):
        return self.value.keys()

    def interp(self, scope: Scope, on_issue: IssueHandler = raise_if_error) -> None:
        HasType.interp(self, scope, on_issue)

    def clear_interp(self) -> None:
        HasType.clear_interp(self)

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

    def interp(self, scope: Scope, on_issue: IssueHandler = raise_if_error) -> None:
        pass

    def clear_interp(self) -> None:
        pass

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

    def interp(self, scope: Scope, on_issue: IssueHandler = raise_if_error) -> None:
        pass


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

    def interp(self, scope: Scope, on_issue: IssueHandler = raise_if_error) -> None:
        pass


STATEMENT_CLASS_BY_TYPE: dict[StatementType, typing.Type[Symbol]] = {
    StatementType.TYPE: Type,
    StatementType.TASK: Task,
    StatementType.EXPECTATION: Expectation,
    StatementType.DATASET: Dataset,
    StatementType.VALUE: Value,
    StatementType.MODEL: Model,
    StatementType.CODE: Code,
    StatementType.REQUIREMENT: Requirement,
    StatementType.BLOCK: Block,
}
STATEMENT_TYPE_BY_CLASS: dict[typing.Type[Symbol], StatementType] = {
    v: k for k, v in STATEMENT_CLASS_BY_TYPE.items()
}


@dataclass(repr=False)
class RemoteObject(HasSession):
    """
    A proxy to a remotely stored object behaving like a Python file on demand.
    :RemoteObjectType
    """

    id: UUID = field(default_factory=uuid.uuid4)
    sha512: str = required_field()
    content_length: int = required_field()
    content_type: str = required_field()
    name: str = required_field()
    status: RemoteObjectStatus = required_field()

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


@dataclass(repr=False)
class Secret(HasSession, typing.Generic[SecretValueT]):
    """A proxy to a remotely stored secret."""

    id: UUID = field(default_factory=uuid.uuid4)
    sha512: str = required_field()
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

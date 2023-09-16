import abc
import typing
from dataclasses import field
from typing import TYPE_CHECKING, Any, Optional, Union
from uuid import UUID

from bench.language.const import (
    MNT,
    StatementReference,
    StatementType,
    TextHeadingLevel,
    TypeFlag,
    TypeTag,
)
from bench.language.field import HasFields
from bench.language.issue import BenchError, HasIssues, Issue, IssueHandler
from bench.language.module import Module, ModuleNode, ModuleVisitor, Scope, node
from bench.utils.fractional import generate_n_keys_between
from bench.utils.func import did_you_mean_str
from bench.utils.proxy import proxy_value
from bench.utils.utils import DotDict, IdentifierType, required_field, to_pyidentifier

if TYPE_CHECKING:
    from bench.language.file import File


@node(mnt=MNT.Statement, tracked=["name"])
class Statement(ModuleNode, ModuleNode, HasIssues, Scope):
    """A Bench statement."""

    file: File | None = None
    parent: Union["Statement", File] = None
    children: list["Statement"] | None = None
    order_key: str | None = None
    type: StatementType = required_field()  # set by subclasses
    name: Optional[str] = None
    issues: list[Issue] | None = None
    # content
    reference: Union["Statement", StatementReference, None] = None
    heading_level: Optional["TextHeadingLevel"] = None
    text: str | None = None
    key: str | None = None
    tag: Optional["TypeTag"] = None
    flags: Optional["TypeFlag"] = 0
    code: str | None = None
    value: Any | None = None
    versioned: bool | None = None
    # internal
    _unpacked: bool = False

    def __post_init__(self):
        super().__post_init__()
        if self.file is None and self.parent is not None:
            self.file = self.parent.file
        if self.parent is None:
            self.parent = self.file

    def __str__(self):
        return f"{self.path} '{self.name}'" if self.name else self.path

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    @property
    def reference_ck(self) -> Optional[UUID]:
        if isinstance(self.reference, Statement):
            return self.reference.ck
        elif isinstance(self.reference, UUID):
            return self.reference
        else:
            return None

    @property
    def module(self) -> Optional[Module]:
        if self.file is None:
            return None
        return self.file.module

    @property
    def path(self) -> str:
        if self.file is None:
            return f"<detached>:{self.infile_path}"
        else:
            return self.file.path + "." + str(self.infile_path)

    @property
    def infile_path(self) -> str:
        parent = self.parent
        ancestor_parts = [self.py_ident or "<anon>"]
        seen_ids = {self.id}
        while isinstance(parent, Statement):
            if parent.id in seen_ids:
                # :CircularAncestry
                # circuit breaker: ignore here because this is an error in indexing
                ancestor_parts.append("<!loop>")
                break
            ancestor_parts.append(parent.py_ident or "<anon>")
            seen_ids.add(parent.id)
            parent = parent.parent
        return ".".join(reversed(ancestor_parts))

    @property
    def py_ident(self) -> Optional[str]:
        if self.name is None:
            return None
        else:
            return to_pyidentifier(self.name, IdentifierType.VARIABLE)

    @property
    def parent_id(self) -> Optional[UUID]:
        return self.parent.id if self.parent is not None else None

    def append_statement(self, *statements: "Statement"):
        last_ok = self.children[-1].order_key if self.children else None
        oks = generate_n_keys_between(last_ok, None, len(statements))
        for ok, statement in zip(oks, statements):
            if statement.parent is not None and statement.parent != self:
                raise ValueError(f"statement {statement} belongs to {statement.parent}")
            statement.order_key = ok
            statement.file = self.file
            statement.parent = self
        if self.children is None:
            self.children = [*statements]
        else:
            self.children.extend(statements)

    def walk_descendants(self) -> typing.Iterator["Statement"]:
        """Yields all descendant statements in DFS order."""
        yield self
        if self.children is not None:
            for child in self.children:
                yield from child.walk_descendants()

    def __getattr__(self, item):
        if item in self._PROPERTIES:
            return super().__getattribute__(item)
        else:
            scope = self._scopes_by_name.get(item)
            if scope is not None:
                return scope
        candidates = {
            **{s: s for s in self._PROPERTIES},
            **{s.name: s for s in self._scopes_by_name.values()},
        }
        did_you_mean = did_you_mean_str(candidates, item)
        raise AttributeError(f"{self} has no attribute {item} ({did_you_mean})")

    def _index(self):
        self._clear()
        self.children = self.file._statements_by_parent_id.get(self.id, [])
        for child in self.children:
            # only index self, not children
            # (unlike in file/module, statement nesting is only semantic, not structural)
            self._add_child_scope(child, by_name=True)

    def _clear(self) -> None:
        """Clears any derived/interpreted values on this statement."""
        super()._clear()
        self.issues = None
        # nocheckin: clear all component classes

    def _interp(self, scope: Scope) -> None:
        """Updates, resolves and checks any derived/interpreted values on this statement."""
        # nocheckin: interp all component classes
        pass

    def _visit(self, visitor: ModuleVisitor) -> None:
        # nocheckin: visit all component classes
        for child in self.children or []:
            visitor.visit_child(child)

    def _reinterp(self, scope: Scope = None, raise_errors: bool = True) -> None:
        """Clears and re-interprets this statement in scope."""
        self._clear()
        self._index()
        self._interp(scope or self)
        if raise_errors and self.errors:
            raise BenchError(self.errors[0])


class StatementBase(abc.ABC):  # nocheckin: remove/replace StatementBase?
    """Base for statements for type-checking."""

    parent: Statement | File
    session: "Session"
    _scopes_by_name: dict[str, Scope] | None
    _names_by_ident: dict[str, str] | None

    def _clear(self) -> None:
        raise NotImplementedError

    def _index(self) -> None:
        raise NotImplementedError

    def _interp(self, scope: Scope) -> None:
        raise NotImplementedError

    def _reinterp(self, scope: Scope = None, raise_errors: bool = True) -> None:
        raise NotImplementedError

    _on_issue: IssueHandler


#
# Concrete statements
#


@node
class Type(HasFields, HasText, HasTags, Statement):
    tag: TypeTag = required_field()
    flags: TypeFlag = TypeFlag.Zero
    # not directly configurable for type statements
    hint = None
    reference = None

    def __call__(self, *args, **kwargs):
        combined_kwargs = {**kwargs}
        for i in range(len(args)):
            combined_kwargs[self.fields[i].name] = args[i]
        return DotDict(combined_kwargs)

    def __str__(self):
        path_str = f"{self.path} " if self.name else ""
        return f"{path_str}{self.tag}"

    def __repr__(self):
        return f"<Type {self}>"

    def __getattr__(self, item):
        if self._names_by_ident is not None and item in self._names_by_ident:
            item = self._names_by_ident.get(item)
            return self._scopes_by_name.get(item)
        else:
            return super().__getattr__(item)

    @property
    def py_ident(self) -> str:
        return to_pyidentifier(self.name, IdentifierType.TYPE)

    @staticmethod
    def from_py_type(py_type: Any):
        from bench.language.mapping import type_from_instance_type

        return type_from_instance_type(py_type)


@node(tracked=[])
class Blank(Statement):
    """A blank statement."""

    type: StatementType = StatementType.BLANK


@node(tracked=["text", "heading_level"])
class Text(HasText, HasTags, HasFields, Statement):
    heading_level: Optional[TextHeadingLevel] = None
    type: StatementType = StatementType.TEXT


@node(tracked=["text", "value"])
class Variable(HasFields, HasTags, HasText, Statement):
    type: StatementType = StatementType.VARIABLE
    tag: TypeTag = TypeTag.STRUCT
    flags: TypeFlag = TypeFlag.Zero
    value: Any = field(default_factory=dict)

    def __getattr__(self, item):
        if item in self._PROPERTIES:
            return self.__dict__[item]
        elif item in self.value:
            return self.value[item]
        elif self.has_field(item):
            return None
        elif not isinstance(item, str):
            raise TypeError(f"cannot index {self} with {type(item)}")
        else:
            candidates = {
                **{f: f for f in self._PROPERTIES},
                **{f.py_ident: f for f in self.fields},
            }
            raise AttributeError(
                f"{self} has no field {item} ({did_you_mean_str(candidates, item)}, available: {self.fields})"
            )

    def __setattr__(self, key, value):
        if key in self._PROPERTIES:
            super().__setattr__(key, value)
        else:
            assert self.value is not None, f"cannot set {key} on {self} without value"
            self.value[key] = value

    def __getitem__(self, item):
        if item in self.value:
            return self.value[item]
        elif self.has_field(item):
            return None
        elif not isinstance(item, str):
            raise TypeError(f"cannot index {self} with {type(item)}")
        else:
            candidates = {f.py_ident: f for f in self.fields}
            raise AttributeError(
                f"{self} has no field {item} ({did_you_mean_str(candidates, item)}, available: {self.fields})"
            )

    def __iter__(self):
        return iter(self.value)


@node
class Flow(HasFields, HasFlow, HasTags, HasText, Statement):
    """An orchestrated flow of triggered runs."""

    type: StatementType = StatementType.FLOW
    tag: TypeTag = TypeTag.FUNCTION


@node(tracked=["reference"])
class Reference(Statement, HasTags, HasText, HasTriggers):
    """A reference to another statement."""

    type: StatementType = StatementType.REFERENCE


@node(tracked=["name"])
class Tag(HasFields, HasTags, HasText, Statement):
    """A tag statement."""

    type: StatementType = StatementType.TAG
    tag: TypeTag = TypeTag.STRUCT

    def __post_init__(self):
        if self.key is None:
            self.key = new_field_key(self.ck)

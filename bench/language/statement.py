import typing
from dataclasses import field
from typing import TYPE_CHECKING, Any, Optional, Union
from uuid import UUID

from bench.language.code_ import HasCode
from bench.language.const import (
    MNT,
    StatementReference,
    StatementType,
    TextHeadingLevel,
    TypeFlag,
    TypeTag,
)
from bench.language.dataset import HasDataset
from bench.language.field import HasFields
from bench.language.issue import BenchError, Issue
from bench.language.model import HasModel
from bench.language.module import Module, ModuleNode, ModuleVisitor, Scope, node
from bench.language.reference import HasReference
from bench.language.tagging import HasTags
from bench.language.task import HasTask
from bench.language.text import HasText
from bench.language.trigger import HasTriggers
from bench.language.value import HasValue
from bench.utils.fractional import generate_n_keys_between
from bench.utils.func import did_you_mean_str
from bench.utils.utils import DotDict, IdentifierType, required_field, to_pyidentifier

if TYPE_CHECKING:
    from bench.language.file import File
    from bench.language.session import Session


@node(mnt=MNT.Statement, tracked=["name"])
class Statement(ModuleNode, Scope):
    """A Bench statement."""

    file: Optional["File"] = None
    parent: Union["Statement", "File"] = None
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

    def _clear(self) -> None:
        """Clears any derived/interpreted values on this statement."""
        Scope._clear(self)
        self.issues = None
        for cls in _STATEMENT_COMPONENTS_BY_TYPE[self.type]:
            cls._clear(self)

    def _index(self):
        self.children = self.file._statements_by_parent_id.get(self.id, [])
        for child in self.children:
            # only index self, not children
            # (unlike in file/module, statement nesting is only semantic, not structural)
            self._add_child_scope(child, by_name=True)
        for cls in _STATEMENT_COMPONENTS_BY_TYPE[self.type]:
            cls._index(self)

    def _interp(self, scope: Scope) -> None:
        """Updates, resolves and checks any derived/interpreted values on this statement."""
        for cls in _STATEMENT_COMPONENTS_BY_TYPE[self.type]:
            cls._interp(self, scope)

    def _visit(self, visitor: ModuleVisitor) -> None:
        for child in self.children or []:
            visitor.visit_child(child)
        for cls in _STATEMENT_COMPONENTS_BY_TYPE[self.type]:
            cls._visit(self, visitor)

    def _reinterp(self, scope: Scope = None, raise_errors: bool = True) -> None:
        """Clears and re-interprets this statement in scope."""
        self._clear()
        self._index()
        self._interp(scope or self)
        if raise_errors and self.errors:
            raise BenchError(self.errors[0])

    def _activate_in(self, session: "Session") -> None:
        """Activates this statement in the given session."""
        for cls in _STATEMENT_COMPONENTS_BY_TYPE[self.type]:
            if hasattr(cls, "_activate_in"):
                cls._activate_in(self, session)
        super()._activate_in(session)

    def _deactivate(self) -> None:
        """Deactivates this statement."""
        super()._deactivate()
        for cls in _STATEMENT_COMPONENTS_BY_TYPE[self.type]:
            if hasattr(cls, "_deactivate"):
                cls._deactivate(self)


#
# Concrete statements
#


@node
class Blank(Statement):
    """A blank statement."""

    type: StatementType = StatementType.BLANK


@node
class Text(Statement, HasText, HasTags, HasFields):
    heading_level: Optional[TextHeadingLevel] = None
    type: StatementType = StatementType.TEXT


@node
class Reference(Statement, HasFields, HasReference, HasTags, HasText):
    type: StatementType = StatementType.REFERENCE


@node
class Type(Statement, HasFields, HasTags, HasText):
    type: StatementType = StatementType.TYPE
    tag: TypeTag = TypeTag.STRUCT

    def __call__(self, *args, **kwargs):
        combined_kwargs = {**kwargs}
        for i in range(len(args)):
            combined_kwargs[self.fields[i].name] = args[i]
        return DotDict(combined_kwargs)  # this is not quite correct, should be a real type

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


@node
class Tag(Statement, HasFields, HasTags, HasText):
    type: StatementType = StatementType.TAG
    tag: TypeTag = TypeTag.STRUCT


@node
class Dataset(Statement, HasDataset, HasTags, HasText):
    type: StatementType = StatementType.DATASET
    tag: TypeTag = TypeTag.STRUCT


from bench.language.run import HasRun  # noqa: E402


@node
class Model(Statement, HasModel, HasRun):
    type: StatementType = StatementType.MODEL
    tag: TypeTag = TypeTag.FUNCTION
    flags: TypeFlag = TypeFlag.Zero


@node
class Code(Statement, HasCode, HasRun, HasTriggers, HasTags, HasText):
    type: StatementType = StatementType.CODE
    tag: TypeTag = TypeTag.FUNCTION
    flags: TypeFlag = TypeFlag.Zero


@node
class Task(Statement, HasTask, HasRun, HasFields, HasTags, HasText):
    type: StatementType = StatementType.TASK
    tag: TypeTag = TypeTag.FUNCTION
    flags: TypeFlag = TypeFlag.Zero


@node
class Flow(Statement, HasFields, HasTags, HasText):
    type: StatementType = StatementType.FLOW
    tag: TypeTag = TypeTag.FUNCTION


@node
class Variable(Statement, HasValue, HasTags, HasText):
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


_COMPONENT_CLASSES: list[type[ModuleNode]] = [
    HasCode,
    HasDataset,
    HasFields,
    HasModel,
    HasRun,
    HasTags,
    HasText,
    HasTask,
    HasTriggers,
    HasValue,
    HasReference,
]
_COMPONENT_METHODS = ["_clear", "_index", "_interp", "_visit", "_activate_in", "_deactivate"]
_MUST_OVERRIDE_METHODS = ["_clear", "_index", "_interp", "_visit"]
_seen_methods: dict[object, type] = {getattr(ModuleNode, m): ModuleNode for m in _COMPONENT_METHODS}
for c in _COMPONENT_CLASSES:
    # check that they implement _clear, _index, _interp, _visit (in their own class)
    for m in _COMPONENT_METHODS:
        assert hasattr(c, m), f"{c} does not implement {m}"
        seen = _seen_methods.get(getattr(c, m))
        if m in _MUST_OVERRIDE_METHODS:
            assert seen is None, f"{c} must override {m}"
        _seen_methods[getattr(c, m)] = c

STATEMENT_CLASS_BY_TYPE: dict[StatementType, typing.Type[Statement]] = {
    StatementType.BLANK: Blank,
    StatementType.TEXT: Text,
    StatementType.TASK: Task,
    StatementType.TYPE: Type,
    StatementType.TAG: Tag,
    StatementType.CODE: Code,
    StatementType.DATASET: Dataset,
    StatementType.VARIABLE: Variable,
    StatementType.FLOW: Flow,
    StatementType.MODEL: Model,
    StatementType.REFERENCE: Reference,
}
_missing_statement_types = set(StatementType) - set(STATEMENT_CLASS_BY_TYPE)
assert not _missing_statement_types, f"missing statement types: {_missing_statement_types}"

_STATEMENT_COMPONENTS_BY_TYPE: dict[StatementType, list[typing.Type[ModuleNode]]] = {}
for type, cls in STATEMENT_CLASS_BY_TYPE.items():
    assert cls.type == type, f"{cls} has wrong type {cls.type} (expected: {type})"
    # check that only Statement and Component classes are immediate parent of cls
    parents = [c for c in cls.__bases__ if c != Statement]
    illegal_parents = [c for c in parents if c not in _COMPONENT_CLASSES]
    assert (
        not illegal_parents
    ), f"unexpected parents of {cls}: {illegal_parents} (allowed: {_COMPONENT_CLASSES})"
    # check that Statement is first parent of cls (for MRO)
    assert (
        cls.__bases__[0] == Statement
    ), f"unexpected first parent of {cls}: {parents[0]} (expected: Statement)"
    _STATEMENT_COMPONENTS_BY_TYPE[type] = parents
    # expand parents into their parent components
    parents = parents[:]
    while parents:
        parent = parents.pop()
        if parent not in _STATEMENT_COMPONENTS_BY_TYPE[type]:
            _STATEMENT_COMPONENTS_BY_TYPE[type].append(parent)
        parents.extend([c for c in parent.__bases__ if c in _COMPONENT_CLASSES])

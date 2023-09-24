import typing
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
from bench.language.issue import Issue
from bench.language.module import (
    Module,
    ModuleNode,
    NodeVisitor,
    Scope,
    node,
    nproperty,
    nancestor,
    nparent,
    nchildren,
    NRel,
    NodeList,
)
from bench.language.field import HasFields
from bench.language.text import HasText
from bench.utils.func import did_you_mean_str
from bench.utils.utils import IdentifierType, to_pyidentifier

if TYPE_CHECKING:
    from bench.language import (
        File,
        Statement,
        Field,
        Trigger,
        Tagging,
        DatabaseView,
        Record,
        ResolvedField,
        Session,
    )


@node(MNT.Statement)
class Statement(ModuleNode, Scope):
    """A Bench statement."""

    file: Optional["File"] = nancestor(MNT.File)
    parent: Union["Statement", "File"] = nparent(MNT.Statement, MNT.File)
    children: NodeList["Statement"] = nchildren(
        MNT.Statement, NRel.INLINE | NRel.ORDERED | NRel.NAMED
    )

    name: Optional[str] = nproperty(default=None)
    order_key: str | None = nproperty(default=None)
    type: StatementType = nproperty(default=StatementType.BLANK)

    reference: Union["Statement", StatementReference, None] = nproperty(default=None)
    heading_level: Optional["TextHeadingLevel"] = nproperty(default=None)
    text: str | None = nproperty(default=None)
    key: str | None = nproperty(default=None)
    tag: Optional["TypeTag"] = nproperty(default=None)
    flags: Optional["TypeFlag"] = nproperty(default=0)
    code: str | None = nproperty(default=None)
    value: Any | None = nproperty(default=None)
    versioned: bool = nproperty(default=True)

    tags: NodeList["Tagging"] = nchildren(MNT.Tagging, NRel.INLINE)
    fields: NodeList["Field"] = nchildren(MNT.Field, NRel.INLINE | NRel.NAMED | NRel.ORDERED)
    resolved_fields: NodeList["ResolvedField"] = nchildren(
        MNT.ResolvedField, NRel.INLINE | NRel.ORDERED
    )
    triggers: NodeList["Trigger"] = nchildren(MNT.Trigger, NRel.INLINE)
    views: NodeList["DatabaseView"] = nchildren(
        MNT.DatabaseView, NRel.INLINE | NRel.NAMED | NRel.ORDERED
    )
    records: NodeList["Record"] = nchildren(MNT.Record, NRel.ZERO)
    issues: NodeList[Issue] | None = nchildren(MNT.Issue, NRel.INLINE | NRel.CUMULATIVE)

    def __post_init__(self):
        super().__post_init__()
        if self.file is None and self.parent is not None:
            self.file = self.parent.file
        if self.parent is None:
            self.parent = self.file
        # nocheckin: init components

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
    def inputs(self):
        return [f for f in self.resolved_fields if not (f.flags & TypeFlag.IsOutput)]

    @property
    def outputs(self):
        return [f for f in self.resolved_fields if f.flags & TypeFlag.IsOutput]

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

    def walk_descendants(self) -> typing.Iterator["Statement"]:
        """Yields all descendant statements in DFS order."""
        yield self
        if self.children is not None:
            for child in self.children:
                yield from child.walk_descendants()

    def __getattr__(self, item):
        if not self._tracked or item in self._PROPERTIES:
            return super().__getattribute__(item)
        if item in self._names_by_ident:
            item = self._names_by_ident.get(item)
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

    def _visit(self, visitor: NodeVisitor) -> None:
        for child in self.children or []:
            visitor.visit_child(child)
        for cls in _STATEMENT_COMPONENTS_BY_TYPE[self.type]:
            cls._visit(self, visitor)

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


_STATEMENT_COMPONENTS_BY_TYPE: dict[StatementType, list[typing.Type[ModuleNode]]] = {
    StatementType.TYPE: [HasFields, HasText],
    StatementType.MODEL: [HasFields, HasText],
}

_missing_types = set(StatementType) - set(_STATEMENT_COMPONENTS_BY_TYPE)
assert not _missing_types, f"missing statement components for {_missing_types}"


#
# 'Concrete' statements are a mirage, we just have a custom metaclass
#  where for e.g. statement.type == 'X', the 'concrete' class X
#  works for isinstance(x, Type) and Type(**kwargs) works like Statement(type=X, **kwargs)
#


class _StatementProxy(type):
    def __init__(self, _type: StatementType):
        super().__init__()
        self._type = _type

    def __call__(self, *args, **kwargs):
        return Statement(type=self._type, *args, **kwargs)

    def __instancecheck__(self, instance):
        return isinstance(instance, Statement) and instance.type == self._type

    def __subclasscheck__(self, subclass):
        return issubclass(subclass, Statement) and subclass.type == self._type


def _make_statement_proxy(_type: StatementType):
    return _StatementProxy(_type)


Blank = _make_statement_proxy(StatementType.BLANK)
Text = _make_statement_proxy(StatementType.TEXT)
Reference = _make_statement_proxy(StatementType.REFERENCE)
Type = _make_statement_proxy(StatementType.TYPE)
Tag = _make_statement_proxy(StatementType.TAG)
Database = _make_statement_proxy(StatementType.DATABASE)
Model = _make_statement_proxy(StatementType.MODEL)
Code = _make_statement_proxy(StatementType.CODE)
Task = _make_statement_proxy(StatementType.TASK)
Flow = _make_statement_proxy(StatementType.FLOW)
Variable = _make_statement_proxy(StatementType.VARIABLE)

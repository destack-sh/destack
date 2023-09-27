import typing
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
from bench.language.database import HasDatabase
from bench.language.field import HasFields
from bench.language.model import HasModel
from bench.language.module import (
    ModuleNode,
    NodeList,
    NRel,
    ScopeNode,
    nancestor,
    nchildren,
    ninternal,
    node,
    nparent,
    nproperty,
)
from bench.language.reference import HasReference
from bench.language.run import HasRun
from bench.language.task import HasTask
from bench.language.text import HasText
from bench.language.value import HasValue
from bench.utils.func import did_you_mean_str
from bench.utils.utils import IdentifierType, to_pyidentifier

if TYPE_CHECKING:
    from bench.language import (
        DatabaseView,
        Field,
        File,
        Record,
        ResolvedField,
        Tagging,
        Trigger,
        TypeHint,
    )


@node(MNT.Statement)
class Statement(ScopeNode):
    """A Bench statement."""

    file: Optional["File"] = nancestor(MNT.File)
    parent: Union["Statement", "File"] = nparent(MNT.Statement, MNT.File)
    children: NodeList["Statement"] = nchildren(
        MNT.Statement, NRel.Ordered | NRel.Named | NRel.Scoped
    )

    type: StatementType = nproperty(default=StatementType.BLANK)
    name: Optional[str] = nproperty(default=None)
    order_key: str | None = ninternal(default=None)

    reference: Union["Statement", StatementReference, None] = nproperty(default=None)
    heading_level: Optional["TextHeadingLevel"] = nproperty(default=None)
    text: str | None = nproperty(default=None)
    key: str | None = nproperty(default=None)
    tag: Optional["TypeTag"] = nproperty(default=None)
    hint: Optional["TypeHint"] = nproperty(default=None)
    flags: Optional["TypeFlag"] = nproperty(default=0)
    code: str | None = nproperty(default=None)
    value: Any | None = nproperty(default=None)
    versioned: bool = nproperty(default=True)
    external_name: str | None = ninternal(default=None)  # for model, to be moved to value

    # all possible child relations inlined (dynamic components can't have non-runtime
    #  properties, primarily because that would be confusing, and we want to edit all of them)
    tags: NodeList["Tagging"] = nchildren(MNT.Tagging)
    fields: NodeList["Field"] = nchildren(MNT.Field, NRel.Named | NRel.Scoped | NRel.Ordered)
    resolved_fields: NodeList["ResolvedField"] = nchildren(
        MNT.ResolvedField, NRel.Named | NRel.Ordered
    )
    triggers: NodeList["Trigger"] = nchildren(MNT.Trigger)
    views: NodeList["DatabaseView"] = nchildren(MNT.DatabaseView, NRel.Named | NRel.Ordered)
    records: NodeList["Record"] = nchildren(MNT.Record, NRel.Default)

    @staticmethod
    def _coerce_from(type: StatementType = None, name: str = None, *args, **kwargs) -> "Statement":
        proxy = STATEMENT_CLASS_BY_TYPE[type]
        if proxy.tag:
            kwargs["tag"] = proxy.tag
        if proxy.flags:
            kwargs["flags"] = proxy.flags
        return Statement(type=type, name=name, *args, **kwargs)

    @property
    def _components(self):
        return _ALL_COMPONENTS_BY_TYPE[self.type]

    def __str__(self):
        return f"{self.path} '{self.name}'" if self.name else self.path

    def __repr__(self):
        return f"<{self.type.camel_name} {self}>"

    def _init_inner(self) -> None:
        # add runtime properties from dynamic components
        for component in _DYNAMIC_COMPONENTS_BY_TYPE[self.type]:
            for prop in component.__properties__.values():
                if prop.is_runtime:
                    setattr(self, prop.name, prop.new())

    @property
    def reference_ck(self) -> Optional[UUID]:
        if isinstance(self.reference, Statement):
            return self.reference.ck
        elif isinstance(self.reference, UUID):
            return self.reference
        else:
            return None

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
        seen_ids = [self.id]
        while isinstance(parent, Statement):
            if parent.id in seen_ids:
                # :CircularAncestry
                # circuit breaker: ignore here because this is an error in indexing
                ancestor_parts.append("<!loop>")
                break
            ancestor_parts.append(parent.py_ident or "<anon>")
            seen_ids.append(parent.id)
            parent = parent.parent
        return ".".join(reversed(ancestor_parts))

    @property
    def py_ident(self) -> Optional[str]:
        if self.name is None:
            return None
        else:
            return to_pyidentifier(self.name, _IDENTIFIER_BY_TYPE[self.type])

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
        if item in self.__properties__:
            return super().__getattribute__(item)
        for component in self._components:
            if hasattr(component, item):
                return getattr(component, item).__get__(self)

        # report lookup error
        if item in self._names_by_ident:
            item = self._names_by_ident.get(item)
        scope = self._scopes_by_name.get(item)
        if scope is not None:
            return scope
        candidates = {
            **{s: s for s in self.__properties__.keys()},
            **{s.name: s for s in self._scopes_by_name.values()},
        }
        did_you_mean = did_you_mean_str(candidates, item)
        raise AttributeError(f"{self} has no attribute {item} ({did_you_mean})")


# Note that order matters as components are called in order.
_DYNAMIC_COMPONENTS_BY_TYPE: dict[StatementType, tuple[typing.Type[ModuleNode]]] = {
    StatementType.TYPE: (HasFields, HasText),
    StatementType.CODE: (HasCode, HasRun, HasFields, HasText),
    StatementType.MODEL: (HasModel, HasRun, HasFields, HasText),
    StatementType.TASK: (HasTask, HasRun, HasFields, HasText),
    StatementType.FLOW: (HasRun, HasFields, HasText),
    # Order matters for Database because HasDatabase _init
    StatementType.DATABASE: (HasDatabase, HasFields, HasText),
    StatementType.TAG: (HasFields, HasText),
    StatementType.VARIABLE: (HasValue, HasFields, HasText),
    StatementType.REFERENCE: (HasReference, HasText),
    StatementType.TEXT: (HasText,),
    StatementType.BLANK: tuple(),
}
_ALL_COMPONENTS_BY_TYPE: dict[StatementType, tuple[typing.Type[ModuleNode]]] = {
    t: _DYNAMIC_COMPONENTS_BY_TYPE[t] + Statement.__static_components__ for t in StatementType
}
_IDENTIFIER_BY_TYPE: dict[StatementType, IdentifierType] = {
    StatementType.TYPE: IdentifierType.TYPE,
    StatementType.MODEL: IdentifierType.METHOD,
    StatementType.TASK: IdentifierType.METHOD,
    StatementType.FLOW: IdentifierType.METHOD,
    StatementType.CODE: IdentifierType.METHOD,
    StatementType.DATABASE: IdentifierType.VARIABLE,
    StatementType.VARIABLE: IdentifierType.VARIABLE,
    StatementType.TAG: IdentifierType.VARIABLE,
    StatementType.REFERENCE: IdentifierType.VARIABLE,
    StatementType.TEXT: IdentifierType.VARIABLE,
    StatementType.BLANK: IdentifierType.VARIABLE,
}

_missing_types = set(StatementType) - set(_DYNAMIC_COMPONENTS_BY_TYPE)
assert not _missing_types, f"missing statement components for {_missing_types}"

#
# 'Concrete' statements are a mirage, we just have a custom class
#  where for e.g. statement.type == 'X', the 'concrete' class X
#  works for isinstance(x, Type) and Type(**kwargs) works like Statement(type=X, **kwargs)
#


STATEMENT_CLASS_BY_TYPE: dict[StatementType, "_StatementProxy"] = {}


class _StatementProxy:
    def __init__(
        self,
        _type: StatementType,
        tag: Optional[TypeTag] = None,
        flags: Optional[TypeFlag] = None,
        register: bool = True,
    ):
        self.type = _type
        self.tag = tag
        self.flags = flags
        if register:
            if _type in STATEMENT_CLASS_BY_TYPE:
                raise ValueError(f"statement type {_type} already registered")
            STATEMENT_CLASS_BY_TYPE[_type] = self

    def __call__(self, tag: TypeTag = None, flags: TypeFlag = None, *args, **kwargs):
        kwargs["type"] = self.type
        kwargs["tag"] = tag if tag is not None else self.tag
        kwargs["flags"] = (flags if flags is not None else self.flags) or 0
        return Statement(**kwargs)

    def __instancecheck__(self, instance):
        return isinstance(instance, Statement) and instance.type == self.type

    def __subclasscheck__(self, subclass):
        return issubclass(subclass, Statement) and subclass.type == self.type


def _make_statement_proxy(
    _type: StatementType,
    tag: Optional[TypeTag] = None,
    flags: Optional[TypeFlag] = None,
    register=True,
):
    return _StatementProxy(_type, tag=tag, flags=flags, register=register)


Blank = _make_statement_proxy(StatementType.BLANK)
Text = _make_statement_proxy(StatementType.TEXT)
Reference = _make_statement_proxy(StatementType.REFERENCE)
Struct = _make_statement_proxy(StatementType.TYPE, tag=TypeTag.STRUCT)
Choice = _make_statement_proxy(StatementType.TYPE, tag=TypeTag.ENUM, register=False)
Type = Struct
Tag = _make_statement_proxy(StatementType.TAG, tag=TypeTag.STRUCT)
Database = _make_statement_proxy(StatementType.DATABASE, tag=TypeTag.STRUCT, flags=TypeFlag.IsArray)
Model = _make_statement_proxy(StatementType.MODEL, tag=TypeTag.FUNCTION)
Code = _make_statement_proxy(StatementType.CODE, tag=TypeTag.FUNCTION)
Task = _make_statement_proxy(StatementType.TASK, tag=TypeTag.FUNCTION)
Flow = _make_statement_proxy(StatementType.FLOW, tag=TypeTag.FUNCTION)
Variable = _make_statement_proxy(StatementType.VARIABLE, tag=TypeTag.STRUCT)

_missing_proxies = set(StatementType) - set(STATEMENT_CLASS_BY_TYPE)
assert not _missing_proxies, f"missing statement proxies for {_missing_proxies}"

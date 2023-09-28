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
    Passthrough,
)
from bench.language.reference import HasReference
from bench.language.run import HasRun
from bench.language.task import HasTask
from bench.language.text import HasText
from bench.language.value import HasValue
from bench.language.tagging import HasTags
from bench.utils.utils import IdentifierType, to_pyidentifier

if TYPE_CHECKING:
    from bench.language import File, TypeHint

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
_missing_types = set(StatementType) - set(_DYNAMIC_COMPONENTS_BY_TYPE)
assert not _missing_types, f"missing statement components for {_missing_types}"
_ALL_DYNAMIC_COMPONENTS: tuple[typing.Type[ModuleNode]] = tuple(
    {c for cs in _DYNAMIC_COMPONENTS_BY_TYPE.values() for c in cs}
)

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
_PASSTHROUGH_BY_TYPE: dict[StatementType, tuple[tuple[str, Passthrough]]] = {
    StatementType.VARIABLE: (("value", Passthrough.Full),),
    StatementType.DATABASE: (("records", Passthrough.Full), ("fields", Passthrough.Scope)),
    StatementType.TAG: (("fields", Passthrough.Scope),),
    StatementType.TYPE: (("fields", Passthrough.Full),),
}
_STATIC_PASSTHROUGH: tuple[tuple[str, Passthrough]] = (("children", Passthrough.Scope),)


@node(
    MNT.Statement,
    passthrough=(("children", Passthrough.Scope),),
    dynamic_components=_ALL_DYNAMIC_COMPONENTS,
)
class Statement(ScopeNode, HasTags):
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
    external_name: str | None = ninternal(default=None)  # for model, to be moved into value

    @staticmethod
    def new(type: StatementType = None, name: str = None, *args, **kwargs) -> "Statement":
        if type is None:
            raise ValueError("type must be specified")
        proxy = STATEMENT_CLASS_BY_TYPE[type]
        if proxy.tag:
            kwargs["tag"] = proxy.tag
        if proxy.flags:
            kwargs["flags"] = proxy.flags
        return Statement(type=type, name=name, *args, **kwargs)

    @property
    def _components(self) -> tuple[typing.Type[ModuleNode]]:
        return _ALL_COMPONENTS_BY_TYPE[self.type]

    @property
    def _dynamic_components(self) -> tuple[typing.Type[ModuleNode]]:
        return _DYNAMIC_COMPONENTS_BY_TYPE[self.type]

    @property
    def _concrete_cache_key(self) -> str:
        return self.type

    @property
    def _passthrough_targets(self) -> tuple[tuple[str, Passthrough]] | None:
        return _ALL_PASSTHROUGH_BY_TYPE[self.type]

    def __str__(self):
        return f"{self.path} '{self.name}'" if self.name else self.path

    def __repr__(self):
        return f"<{self.type.camel_name} {self}>"

    def _init_inner(self) -> None:
        # add runtime properties from dynamic components
        for component in self._dynamic_components:
            for prop in component.__properties__.values():
                if prop.is_runtime and not hasattr(self, prop.name):
                    setattr(self, prop.name, prop.new())

    def morph(self, to_type: StatementType, **kwargs):
        self.type = to_type
        Statement._init_inner(self)
        # what else to do?
        raise NotImplementedError(f"{self!r} does not support morphing yet")

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


#
# 'Concrete' statements are a mirage, we just have a custom class
#  where for e.g. statement.type == 'X', the 'concrete' class X
#

_ALL_COMPONENTS_BY_TYPE: dict[StatementType, tuple[typing.Type[ModuleNode]]] = {
    t: _DYNAMIC_COMPONENTS_BY_TYPE[t] + Statement.__static_components__ for t in StatementType
}
_ALL_PASSTHROUGH_BY_TYPE: dict[StatementType, tuple[tuple[str, Passthrough]]] = {
    # custom passthrough + default passthrough
    t: _PASSTHROUGH_BY_TYPE.get(t, tuple()) + Statement.__static_passthrough__
    for t in StatementType
}
STATEMENT_CLASS_BY_TYPE: dict[StatementType, "_StatementProxy"] = {}


class _StatementProxy:
    def __init__(
        self,
        _type: StatementType,
        tag: Optional[TypeTag] = None,
        flags: TypeFlag = 0,
        register: bool = True,
    ):
        self.type = _type
        self.tag = tag
        self.flags = flags
        if register:
            if _type in STATEMENT_CLASS_BY_TYPE:
                raise ValueError(f"statement type {_type} already registered")
            STATEMENT_CLASS_BY_TYPE[_type] = self

    def __call__(self, tag: TypeTag = None, flags: TypeFlag = 0, *args, **kwargs):
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
    flags: TypeFlag = 0,
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

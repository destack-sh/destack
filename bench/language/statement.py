import typing
from copy import deepcopy
from typing import TYPE_CHECKING, Any, Optional, Union
from uuid import UUID

from bench.language.code_ import HasCode
from bench.language.const import (
    MNT,
    StatementReference,
    StatementType,
    TextHeadingLevel,
    TypeFlag,
    TypeHint,
    TypeTag,
)
from bench.language.database import HasDatabase
from bench.language.field import HasFields, IsType, IsTyped
from bench.language.model import HasModel
from bench.language.module import (
    Node,
    NodeList,
    NRel,
    ScopeNode,
    _Passthrough,
    nancestor,
    nchildren,
    ninternal,
    node,
    nparent,
    nproperty,
)
from bench.language.reference import HasReference
from bench.language.run import HasRun
from bench.language.tagging import HasTags
from bench.language.task import HasTask
from bench.language.text import HasText
from bench.language.trigger import HasTriggers
from bench.language.validation import enum_validator, flag_validator, validate_name
from bench.language.value import HasValue
from bench.utils.func import dict_minus
from bench.utils.utils import IdentifierType, identity, to_pyidentifier

if TYPE_CHECKING:
    from bench.language import File

# Note that order matters as components are called in order.
_DYNAMIC_COMPONENTS_BY_TYPE: dict[StatementType, tuple[typing.Type[Node]]] = {
    StatementType.TYPE: (IsType, HasFields, HasText),
    StatementType.CODE: (HasCode, HasRun, HasTriggers, HasFields, HasText),
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
_ALL_DYNAMIC_COMPONENTS: tuple[typing.Type[Node]] = tuple(
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
_PASSTHROUGH_BY_TYPE: dict[StatementType, tuple[tuple[str, _Passthrough]]] = {
    StatementType.VARIABLE: (("value", _Passthrough.Full),),
    StatementType.DATABASE: (("records", _Passthrough.Full), ("fields", _Passthrough.Scope)),
    StatementType.TAG: (("fields", _Passthrough.Scope),),
    StatementType.TYPE: (("fields", _Passthrough.Full),),
}
_STATIC_PASSTHROUGH: tuple[tuple[str, _Passthrough]] = (("children", _Passthrough.Scope),)


@node(
    MNT.STATEMENT,
    passthrough=(("children", _Passthrough.Scope),),
    dynamic_components=_ALL_DYNAMIC_COMPONENTS,
)
class Statement(ScopeNode, HasTags):
    """A Bench statement."""

    file: Optional["File"] = nancestor(MNT.FILE)
    parent: Union["Statement", "File"] = nparent(MNT.STATEMENT, MNT.FILE)
    children: NodeList["Statement"] = nchildren(
        MNT.STATEMENT, NRel.Ordered | NRel.Named | NRel.Scoped
    )

    type: StatementType = ninternal(default=StatementType.BLANK)
    name: str | None = nproperty(default=None, validate=validate_name)
    order_key: str | None = ninternal(default=None)

    reference: Union["Statement", StatementReference, None] = nproperty(default=None, copy=identity)
    heading_level: Optional["TextHeadingLevel"] = nproperty(
        default=None, validate=enum_validator(TextHeadingLevel)
    )
    text: str | None = nproperty(default=None)
    key: str | None = ninternal(default=None)
    # Statement.tag is optional, but IsTyped.tag is not - we validate this manually in init/morph.
    tag: Optional[TypeTag] = nproperty(
        default=None, validate=enum_validator(TypeTag), ignore_conflicts_with=(IsTyped,)
    )
    hint: Optional[TypeHint] = nproperty(default=None, validate=enum_validator(TypeHint))
    flags: Optional[TypeFlag] = nproperty(default=0, validate=flag_validator(TypeFlag))
    code: str | None = nproperty(default=None)
    value: Any | None = nproperty(default_factory=dict, copy=deepcopy)
    versioned: bool = nproperty(default=True)
    external_name: str | None = ninternal(default=None)  # for model, to be moved into value

    @staticmethod
    def new(
        type: Union[str, StatementType, "_StatementProxy"] = None,
        name: str = None,
        tag: TypeTag = None,
        *args,
        for_parent: Union["Statement", "File", None] = None,
        **kwargs,
    ) -> "Statement":
        if type is None:
            raise ValueError("type must be specified")
        if isinstance(type, _StatementProxy):
            type = type.type
        if not isinstance(type, StatementType):
            type = StatementType(type.lower())
        proxy = STATEMENT_CLASS_BY_TYPE[type]
        kwargs["tag"] = tag or proxy.tag
        if proxy.flags and "flags" not in kwargs:
            kwargs["flags"] = proxy.flags
        return Statement(type=type, name=name, *args, **kwargs)

    @staticmethod
    def text_(text: str, *args, **kwargs):
        return Statement.new(type=StatementType.TEXT, text=text, *args, **kwargs)

    # the others are defined below

    @staticmethod
    def to_python(
        node: "Statement", props: dict, for_parent: Union["Statement", "File", None] = None
    ) -> tuple[str, dict, dict]:
        if node.type == StatementType.TYPE:
            extra_kwargs = {"tag": node.tag}
        else:
            extra_kwargs = {}

        init_name = f"Statement.{node.type.lower()}"
        if node.type == StatementType.BLANK:
            init_args = {}
        elif node.type == StatementType.TEXT:
            init_args = {"text": node.text}
            if node.name:
                init_args = {"name": node.name, **init_args}
            props = dict_minus(props, "text")
        else:
            init_args = {"name": node.name, **extra_kwargs}

        return init_name, init_args, dict_minus(props, "name", "type", "tag", "flags")

    @property
    def _components(self) -> tuple[typing.Type[Node]]:
        return _ALL_COMPONENTS_BY_TYPE[self.type]

    @property
    def _dynamic_components(self) -> tuple[typing.Type[Node]]:
        return _DYNAMIC_COMPONENTS_BY_TYPE[self.type]

    @property
    def _concrete_cache_key(self) -> str:
        return self.type

    @property
    def _passthrough_targets(self) -> tuple[tuple[str, _Passthrough]] | None:
        return _ALL_PASSTHROUGH_BY_TYPE[self.type]

    def __str__(self):
        return f"{self.path} '{self.name}'" if self.name else self.path

    def __repr__(self):
        return f"<Statement.{self.type.camel_name} {self}>"

    def _init_inner(self) -> None:
        # add runtime properties from dynamic components
        for component in self._dynamic_components:
            for prop in component.__properties__.values():
                if prop.is_runtime and prop.name not in self.__dict__:
                    setattr(self, prop.name, prop.new())

    def morph(self, to_type: StatementType):
        self.type = to_type
        Statement._init_inner(self)
        if self.attached:
            self._session.tracer.node_update(self, ["type"])
        # what else to do? trigger re-interp of everything?
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
            return f"<detached>.{self.infile_path}"
        else:
            return self.file.path + "." + str(self.infile_path)

    @property
    def infile_path(self) -> str:
        parent = self.parent
        ancestor_parts = [self.py_ident or "<anon>"]
        seen_ids = [self.ck]
        while isinstance(parent, Statement):
            if parent.id in seen_ids:
                ancestor_parts.append("<!loop>")
                break
            ancestor_parts.append(parent.py_ident or "<anon>")
            seen_ids.append(parent.ck)
            parent = parent.parent
        return ".".join(reversed(ancestor_parts))

    @property
    def py_ident(self) -> Optional[str]:
        if self.name is None:
            return None
        else:
            return to_pyidentifier(self.name, _IDENTIFIER_BY_TYPE[self.type])


# Statement.<type> convenience constructors
Statement.text = Statement.text_
for _type in StatementType:
    if _type in StatementType.TEXT:
        continue
    method = staticmethod(
        lambda name=None, _type=_type, *args, **kwargs: Statement.new(
            _type, name=name, *args, **kwargs
        )
    )
    method_name = _type.lower()
    if method_name not in locals():
        setattr(Statement, method_name, method)

#
# 'Concrete' statements are a mirage, we just have a custom class
#  where for e.g. statement.type == 'X', the 'concrete' class X
#

_ALL_COMPONENTS_BY_TYPE: dict[StatementType, tuple[typing.Type[Node]]] = {
    t: _DYNAMIC_COMPONENTS_BY_TYPE[t] + Statement.__static_components__ for t in StatementType
}
_ALL_PASSTHROUGH_BY_TYPE: dict[StatementType, tuple[tuple[str, _Passthrough]]] = {
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

    def new(self, *args, **kwargs):
        return Statement.new(self.type, *args, tag=self.tag, flags=self.flags, **kwargs)

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
Class = _make_statement_proxy(StatementType.TYPE, tag=TypeTag.STRUCT)
Choice = _make_statement_proxy(StatementType.TYPE, tag=TypeTag.ENUM, register=False)
Type = Class
Tag = _make_statement_proxy(StatementType.TAG, tag=TypeTag.STRUCT)
Database = _make_statement_proxy(StatementType.DATABASE, tag=TypeTag.STRUCT, flags=TypeFlag.IsArray)
Model = _make_statement_proxy(StatementType.MODEL, tag=TypeTag.FUNCTION)
Code = _make_statement_proxy(StatementType.CODE, tag=TypeTag.FUNCTION)
Task = _make_statement_proxy(StatementType.TASK, tag=TypeTag.FUNCTION)
Flow = _make_statement_proxy(StatementType.FLOW, tag=TypeTag.FUNCTION)
Variable = _make_statement_proxy(StatementType.VARIABLE, tag=TypeTag.STRUCT)

_missing_proxies = set(StatementType) - set(STATEMENT_CLASS_BY_TYPE)
assert not _missing_proxies, f"missing statement proxies for {_missing_proxies}"

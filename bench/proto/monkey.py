"""
Patch code for betterproto to make it behave like we want.
Auto-pasted into the generated wire files.
"""

import dataclasses
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import Any, Collection, Iterable, Mapping, Self, Union

import betterproto
from betterproto import Message as ProtoMessage
from betterproto import _Duration as ProtoDuration
from betterproto.lib.google.protobuf import ListValue, NullValue
from betterproto.lib.google.protobuf import Struct as ProtoStruct
from betterproto.lib.google.protobuf import Value as ProtoValue
from betterproto.utils import hybridmethod

from bench.utils.utils import frozendict

# monkey-patch betterproto 'default generator' to initialize unspecified enums as None

_default_gen_none = type(None)

_EMPTY_DICT = frozendict()


def _default_gen_dict() -> dict:
    return _EMPTY_DICT


def _get_field_default_gen(cls: type["betterproto.Message"], field: dataclasses.Field) -> Any:
    # adapted from betterproto source
    t = cls._type_hint(field.name)

    if hasattr(t, "__origin__"):
        if t.__origin__ is dict:
            return dict
        elif t.__origin__ is list:
            return list
        elif t.__origin__ is Union and t.__args__[1] is type(None):
            return _default_gen_none
        else:
            return t
    elif issubclass(t, betterproto.Enum):
        # default to None instead of UNSPECIFIED
        if t.__members__.get("UNSPECIFIED") is not None:
            return _default_gen_none
        else:
            return t.try_value
    elif t is datetime:
        return _default_gen_none
    else:
        return t


class _PatchedProtoClassMetadata(betterproto.ProtoClassMetadata):
    def _get_default_gen(  # type: ignore
        self, cls: type[betterproto.Message], fields: Iterable[dataclasses.Field]
    ) -> Any:
        return {field.name: _get_field_default_gen(cls, field) for field in fields}


betterproto.ProtoClassMetadata._get_default_gen = _PatchedProtoClassMetadata._get_default_gen  # type: ignore


# monkey-patch betterproto Messages for better __str__/__repr__ on messages


class _PatchedMessage(ProtoMessage):
    def __str__(self):
        str_parts = []
        for field_name in (
            "id",
            "ck",
            "revision",
            "type",
            "name",
            "slug",
            "node_type",
            "node_ptr",
            "type_ptr",
            "epoch",
            "status",
            "bench_id",
            "package_id",
            "transaction_id",
        ):
            value = getattr(self, field_name, None)
            if value is not None:
                str_parts.append(f"{field_name}={value!r}")
        return ", ".join(str_parts)

    def __repr__(self):
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str}>"
        else:
            return f"<{self.__class__.__name__}>"


# NOTE: betterproto has a very annoying default __bool__ where it checks for non-default fields
#  (recursively!, which is very flow and leads to weird performance regressions)
betterproto.Message.__bool__ = lambda self: True  # type: ignore
betterproto.Message.__str__ = _PatchedMessage.__str__  # type: ignore
betterproto.Message.__repr__ = _PatchedMessage.__repr__  # type: ignore


def _wrap_value(value: Any) -> ProtoValue:
    """Wrap a JSON-able Python value in a betterproto Value."""
    if value is None:
        return ProtoValue(null_value=NullValue.NULL_VALUE)
    elif type(value) is bool:
        return ProtoValue(bool_value=value)
    elif type(value) is int:
        return ProtoValue(number_value=float(value))
    elif type(value) is float:
        return ProtoValue(number_value=value)
    elif type(value) is str:
        return ProtoValue(string_value=value)
    elif type(value) is dict:
        return ProtoValue(struct_value=_PatchedProtoStruct.from_dict(value))
    elif type(value) is list:
        return ProtoValue(list_value=ListValue([_wrap_value(v) for v in value]))
    else:
        raise ValueError(f"cannot wrap non-JSON value: {value!r} ({type(value)!r})")


def _unwrap_value(value: ProtoValue) -> Any:
    """Unwrap a betterproto Value into a JSON-able Python value."""
    _, v = betterproto.which_one_of(value, "kind")
    if v is None or isinstance(v, NullValue):
        return None
    elif isinstance(v, ProtoStruct):
        return v.to_dict()
    elif isinstance(v, ListValue):
        return [_unwrap_value(e) for e in v.values]
    else:
        return v


# monkey-patch '_Duration' to fix floating preicion loss


class _PatchedDuration(ProtoDuration):
    @classmethod
    def from_timedelta(
        cls, delta: timedelta, *, _1_microsecond: timedelta | None = None
    ) -> ProtoDuration:
        delta_us = (delta.days * 24 * 60 * 60 + delta.seconds) * 10**6 + delta.microseconds
        seconds, us = divmod(delta_us, 10**6)
        return cls(seconds, us * 10**3)


ProtoDuration.from_timedelta = _PatchedDuration.from_timedelta  # type: ignore


# monkey-patch betterproto 'Struct' to fix from_dict/to_dict for nested messages
#  pulls ahead changes from https://github.com/danielgtaylor/python-betterproto/pull/551
#  see https://github.com/danielgtaylor/python-betterproto/issues/332


@dataclass(eq=False, repr=False)
class _PatchedProtoStruct(ProtoStruct):
    @hybridmethod
    def from_dict(self: type[Self], mapping: Mapping[str, Any]) -> Self:  # type: ignore
        self = self()  # type: ignore
        return self.from_dict(mapping)

    @from_dict.instancemethod
    def from_dict(self, mapping: Mapping[str, Any]) -> Self:  # type: ignore
        fields = {**mapping}
        for k, v in fields.items():
            if not isinstance(v, ProtoValue):
                fields[k] = _wrap_value(v)
        self.fields = fields
        return self

    def to_dict(
        self,
        casing: betterproto.Casing | None = None,
        include_default_values: bool = False,
        only: Collection[str] | None = None,
    ) -> dict[str, Any]:
        output = {}
        for k, v in self.fields.items():
            if only is not None and k not in only:
                continue
            output[k] = _unwrap_value(v)
        return output


# ensure 'Value' is in namespace the first time a Struct-like class is created
# if we don't do this here calls will fail mysteriously later
from betterproto.lib.google.protobuf import Value  # noqa

_PatchedProtoStruct()

ProtoStruct.from_dict = _PatchedProtoStruct.from_dict  # type: ignore
ProtoStruct.to_dict = _PatchedProtoStruct.to_dict  # type: ignore

# monkey-patch RpcMetadata to print it nicely
from bench.proto.wire import RpcMetadata, RpcMetadataBadgeInfo  # noqa


class _PatchedRpcMetadata(RpcMetadata):
    def __repr__(self):
        # print all non-default values
        str_parts = []
        for field in dataclasses.fields(self):
            value = getattr(self, field.name)
            if value:
                str_parts.append(f"{field.name}={value!r}")
        return f"{self.__class__.__name__}({', '.join(str_parts)})"


RpcMetadata.__repr__ = _PatchedRpcMetadata.__repr__  # type: ignore

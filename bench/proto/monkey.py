"""
Patch code for betterproto to make it behave like we want.
Auto-pasted into the generated wire files.
"""

import dataclasses
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Any, Collection, Iterable, Mapping, Self, Union

import betterproto
from betterproto import PLACEHOLDER
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

    if not TYPE_CHECKING:

        def __getattribute__(self, name: str) -> Any:
            """
            Lazily initialize default values to avoid infinite recursion for recursive
            message types.
            Return None on attempts to access unset ``oneof`` fields,
             and proxy oneof group get to the currently set field.
            """
            try:
                group_current = object.__getattribute__(self, "_group_current")
            except AttributeError:
                pass
            else:
                if name not in ("__class__", "_betterproto"):
                    if name in self._betterproto.oneof_group_by_field:
                        # union: get field via specific field name (None if not current instead of default)
                        current_field = group_current[self._betterproto.oneof_group_by_field[name]]
                        if current_field != name:
                            return None
                    elif name in self._betterproto.oneof_field_by_group:
                        # union: get field via group name
                        current_field = group_current[name]
                        if not current_field:
                            return None
                        else:
                            return getattr(self, current_field)

            value = object.__getattribute__(self, name)
            if value is not PLACEHOLDER:
                return value

            value = self._get_field_default(name)
            object.__setattr__(self, name, value)
            return value

    def __setattr__(self, attr: str, value: Any) -> None:
        if (
            isinstance(value, betterproto.Message)
            and hasattr(value, "_betterproto")
            and not value._betterproto.meta_by_field_name
        ):
            value._serialized_on_wire = True

        if attr != "_serialized_on_wire":
            # Track when a field has been set.
            self.__dict__["_serialized_on_wire"] = True

        if hasattr(self, "_group_current"):  # __post_init__ had already run
            if attr in self._betterproto.oneof_group_by_field:
                # union: set specific field
                group = self._betterproto.oneof_group_by_field[attr]
                for field in self._betterproto.oneof_field_by_group[group]:
                    if field.name == attr:
                        self._group_current[group] = field.name
                    else:
                        object.__setattr__(self, field.name, PLACEHOLDER)
            elif attr in self._betterproto.oneof_field_by_group:
                # union: set directly via group
                # clear all fields
                group = self._betterproto.oneof_field_by_group[attr]
                for field in group:
                    object.__setattr__(self, field.name, PLACEHOLDER)
                if value is not None:
                    # figure out which field to set (simple type check)
                    value_type_name = type(value).__name__
                    for field in group:
                        if field.type == value_type_name:
                            object.__setattr__(self, field.name, value)
                            self._group_current[attr] = field.name
                            break
                    else:
                        raise AttributeError(
                            f"cannot set {value!r} to {attr!r} in {self.__class__.__name__}"
                        )
                else:
                    self._group_current[attr] = ""

        object.__setattr__(self, attr, value)


# NOTE: betterproto has a very annoying default __bool__ where it checks for non-default fields
#  (recursively!, which is very flow and leads to weird performance regressions)
betterproto.Message.__bool__ = lambda self: True  # type: ignore
betterproto.Message.__str__ = _PatchedMessage.__str__  # type: ignore
betterproto.Message.__repr__ = _PatchedMessage.__repr__  # type: ignore
# override get/set attr to enable direct setting of oneof fields
betterproto.Message.__getattribute__ = _PatchedMessage.__getattribute__  # type: ignore
betterproto.Message.__setattr__ = _PatchedMessage.__setattr__  # type: ignore


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

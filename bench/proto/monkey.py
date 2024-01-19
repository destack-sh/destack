"""
Patch code for betterproto to make it behave like we want.
Auto-pasted into the generated wire files.
"""

import dataclasses
from dataclasses import dataclass
from datetime import datetime
from typing import Mapping, Any, Self, Union, Iterable

import betterproto
from betterproto import hybridmethod
from betterproto.lib.google.protobuf import Struct as BetterprotoStruct

from bench.utils.utils import frozendict

# monkey-patch default generator to initialize all *optional* fields as None
# (including nested messages, primitives, and repeated fields - proto3 can only 'optional' primitives,
#  which is a newer addition for field presence tracking)

_default_gen_none = type(None)

_EMPTY_DICT = frozendict()


def _default_gen_dict() -> dict:
    return _EMPTY_DICT


def _get_field_default_gen(cls: type["betterproto.Message"], field: dataclasses.Field) -> Any:
    t = cls._type_hint(field.name)

    if hasattr(t, "__origin__"):
        if t.__origin__ is dict:  # map (make const since we never modify it directly)
            return _default_gen_dict
        elif t.__origin__ is list:  # repeated field (also make const)
            return tuple
        elif t.__origin__ is Union and t.__args__[1] is type(None):
            return _default_gen_none
        else:
            return t
    elif issubclass(t, betterproto.Enum):
        # Enums always default to zero.
        return _default_gen_none
    elif t is datetime:
        # Offsets are relative to 1970-01-01T00:00:00Z
        return _default_gen_none
    else:
        # This is either a primitive scalar or another message type. Calling
        # it should result in its zero value.
        return t()


class _PatchedProtoClassMetadata(betterproto.ProtoClassMetadata):
    def _get_default_gen(
        self, cls: type[betterproto.Message], fields: Iterable[dataclasses.Field]
    ) -> Any:
        return {field.name: _get_field_default_gen(cls, field) for field in fields}


betterproto.ProtoClassMetadata._get_default_gen = _PatchedProtoClassMetadata._get_default_gen


# monkey-patch betterproto 'Struct' to fix from_dict/to_dict
#  pulls ahead changes from https://github.com/danielgtaylor/python-betterproto/pull/551
#  see https://github.com/danielgtaylor/python-betterproto/issues/332


@dataclass(eq=False, repr=False)
class _PatchedStruct(BetterprotoStruct):
    @hybridmethod
    def from_dict(cls: type[Self], value: Mapping[str, Any]) -> Self:  # noqa
        self = cls()
        return self.from_dict(value)

    @from_dict.instancemethod
    def from_dict(self, value: Mapping[str, Any]) -> Self:
        fields = {**value}
        for k in fields:
            if hasattr(fields[k], "from_dict"):
                fields[k] = fields[k].from_dict()

        self.fields = fields
        return self

    def to_dict(
        self,
        casing: betterproto.Casing = betterproto.Casing.CAMEL,
        include_default_values: bool = False,
    ) -> dict[str, Any]:
        output = {**self.fields}
        for k in self.fields:
            if hasattr(self.fields[k], "to_dict"):
                output[k] = self.fields[k].to_dict(casing, include_default_values)
        return output


# ensure 'Value' is in namespace the first time a Struct-like class is created
# if we don't do this here calls will fail mysteriously later
from betterproto.lib.google.protobuf import Value  # noqa

_PatchedStruct()

BetterprotoStruct.from_dict = _PatchedStruct.from_dict
BetterprotoStruct.to_dict = _PatchedStruct.to_dict

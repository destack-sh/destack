"""
Patch code for betterproto to make it behave like we want.
Auto-pasted into the generated wire files.
"""
import dataclasses
import json
from base64 import b64decode, b64encode
from dataclasses import dataclass
from datetime import datetime, timedelta
from typing import Any, Iterable, Mapping, Self, Union

import betterproto
from betterproto import Message as BetterprotoMessage
from betterproto import hybridmethod
from betterproto.lib.google.protobuf import ListValue, NullValue
from betterproto.lib.google.protobuf import Struct as BetterprotoStruct
from betterproto.lib.google.protobuf import Value as BetterprotoValue
from dateutil.parser import isoparse

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
    def _get_default_gen(
        self, cls: type[betterproto.Message], fields: Iterable[dataclasses.Field]
    ) -> Any:
        return {field.name: _get_field_default_gen(cls, field) for field in fields}


betterproto.ProtoClassMetadata._get_default_gen = _PatchedProtoClassMetadata._get_default_gen


# monkey-patch betterproto to provide to_robust_dict/from_robust_dict serialization :RobustJson


class _PatchedMessage(BetterprotoMessage):
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
            "epoch",
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

    def to_robust_dict(self):
        """Patched betterproto.Message.to_dict that handles RobustJson for Structs."""

        from bench.proto.wiring import BENCH_CLASS_BY_PROTO_CLASS

        # we assume this is only called for Bench types, so a bench class must exist
        struct_cls = BENCH_CLASS_BY_PROTO_CLASS[self.__class__]
        output: dict[str, Any] = {}
        self._type_hints()
        defaults = self._betterproto.default_gen
        for field_name, meta in self._betterproto.meta_by_field_name.items():
            field_is_repeated = defaults[field_name] is list
            try:
                value = getattr(self, field_name)
            except AttributeError:
                value = self._get_field_default(field_name)
            # prop lookup here error here -> proto schema locally out of sync (tested against)
            prop = struct_cls.__properties__[field_name]
            key = str(prop.id)
            if meta.proto_type == betterproto.TYPE_MESSAGE:
                if isinstance(value, datetime):
                    if value != betterproto.DATETIME_ZERO or self._include_default_value_for_oneof(
                        field_name=field_name, meta=meta
                    ):
                        output[key] = betterproto._Timestamp.timestamp_to_json(value)
                elif isinstance(value, timedelta):
                    if value != timedelta(0) or self._include_default_value_for_oneof(
                        field_name=field_name, meta=meta
                    ):
                        output[key] = betterproto._Duration.delta_to_json(value)
                elif isinstance(value, BetterprotoStruct):
                    if len(value) > 0:
                        output[key] = value.to_dict()
                elif meta.wraps:
                    if value is not None:
                        output[key] = value
                elif field_is_repeated:
                    # Convert each item.
                    cls = self._betterproto.cls_by_field[field_name]
                    if cls == datetime:
                        value = [betterproto._Timestamp.timestamp_to_json(i) for i in value]
                    elif cls == timedelta:
                        value = [betterproto._Duration.delta_to_json(i) for i in value]
                    elif cls == BetterprotoStruct:
                        value = [i.to_dict() for i in value]
                    else:
                        value = [i.to_robust_dict() for i in value]
                    if value:
                        output[key] = value
                elif value is None:
                    pass
                elif value._serialized_on_wire or self._include_default_value_for_oneof(
                    field_name=field_name, meta=meta
                ):
                    output[key] = value.to_robust_dict()
            elif meta.proto_type == betterproto.TYPE_MAP:
                raise NotImplementedError("proto maps are not yet supported")
            elif value != self._get_field_default(
                field_name
            ) or self._include_default_value_for_oneof(field_name=field_name, meta=meta):
                if meta.proto_type in betterproto.INT_64_TYPES:
                    if field_is_repeated:
                        output[key] = [str(n) for n in value]
                    elif value is not None:
                        output[key] = str(value)
                elif meta.proto_type == betterproto.TYPE_BYTES:
                    if field_is_repeated:
                        output[key] = [b64encode(b).decode("utf8") for b in value]
                    elif value is not None:
                        output[key] = b64encode(value).decode("utf8")
                elif meta.proto_type == betterproto.TYPE_ENUM:
                    if field_is_repeated:
                        if isinstance(value, Iterable) and not isinstance(value, int):
                            output[key] = [*value]
                        else:
                            # transparently upgrade single value to repeated
                            output[key] = [value]
                    elif value is not None:
                        output[key] = value
                elif meta.proto_type in (betterproto.TYPE_FLOAT, betterproto.TYPE_DOUBLE):
                    if field_is_repeated:
                        output[key] = [betterproto._dump_float(n) for n in value]
                    else:
                        output[key] = betterproto._dump_float(value)
                else:
                    output[key] = value
        return output

    def from_robust_dict(self, mapping: dict):
        from bench.proto.wiring import BENCH_CLASS_BY_PROTO_CLASS

        cls = self.__class__
        struct_cls = BENCH_CLASS_BY_PROTO_CLASS[self.__class__]
        for key, value in mapping.items():
            prop = struct_cls.__properties_by_id__.get(int(key))
            if prop is None or value is None:
                continue
            if prop.reference_wired_ptr:
                prop = prop.reference_wired_ptr
            try:
                meta = cls._betterproto.meta_by_field_name[prop.name]
            except KeyError:
                continue

            if meta.proto_type == betterproto.TYPE_MESSAGE:
                sub_cls = cls._betterproto.cls_by_field[prop.name]
                if sub_cls == datetime:
                    value = (
                        [isoparse(item) for item in value]
                        if isinstance(value, list)
                        else isoparse(value)
                    )
                elif sub_cls == timedelta:
                    value = (
                        [timedelta(seconds=float(item[:-1])) for item in value]
                        if isinstance(value, list)
                        else timedelta(seconds=float(value[:-1]))
                    )
                elif sub_cls == BetterprotoStruct:
                    if not value:
                        value = None
                    else:
                        value = (
                            [sub_cls().from_dict(item) for item in value]
                            if isinstance(value, list)
                            else sub_cls().from_dict(value)
                        )
                elif not meta.wraps:
                    value = (
                        [sub_cls().from_robust_dict(item) for item in value]
                        if isinstance(value, list)
                        else sub_cls().from_robust_dict(value)
                    )
            elif meta.map_types and meta.map_types[1] == betterproto.TYPE_MESSAGE:
                raise NotImplementedError("proto maps are not yet supported")
            else:
                if meta.proto_type in betterproto.INT_64_TYPES:
                    value = [int(n) for n in value] if isinstance(value, list) else int(value)
                elif meta.proto_type == betterproto.TYPE_BYTES:
                    value = (
                        [b64decode(n) for n in value]
                        if isinstance(value, list)
                        else b64decode(value)
                    )
                elif meta.proto_type == betterproto.TYPE_ENUM:
                    enum_cls = cls._betterproto.cls_by_field[prop.name]
                    if isinstance(value, list):
                        value = [enum_cls(e) for e in value]
                    elif isinstance(value, int):
                        value = enum_cls(value)
                elif meta.proto_type in (betterproto.TYPE_FLOAT, betterproto.TYPE_DOUBLE):
                    value = (
                        [betterproto._parse_float(n) for n in value]
                        if isinstance(value, list)
                        else betterproto._parse_float(value)
                    )

            setattr(self, prop.name, value)
        self._serialized_on_wire = True
        return self

    def to_robust_json(self, indent: int = 2):
        """Patched betterproto.Message.to_json that handles RobustJson for Structs."""
        return json.dumps(self.to_robust_dict(), indent=indent)

    def from_robust_json(self, json_string: str):
        """Patched betterproto.Message.from_json that handles RobustJson for Structs."""
        return self.from_robust_dict(json.loads(json_string))


betterproto.Message.__str__ = _PatchedMessage.__str__
betterproto.Message.__repr__ = _PatchedMessage.__repr__
betterproto.Message.to_robust_dict = _PatchedMessage.to_robust_dict
betterproto.Message.from_robust_dict = _PatchedMessage.from_robust_dict
betterproto.Message.to_robust_json = _PatchedMessage.to_robust_json
betterproto.Message.from_robust_json = _PatchedMessage.from_robust_json


# monkey-patch betterproto 'Struct' to fix from_dict/to_dict for nested messages
#  pulls ahead changes from https://github.com/danielgtaylor/python-betterproto/pull/551
#  see https://github.com/danielgtaylor/python-betterproto/issues/332

# @dataclass(eq=False, repr=False)
# class Value(betterproto.Message):
#     """
#     `Value` represents a dynamically typed value which can be either null, a
#     number, a string, a boolean, a recursive struct value, or a list of values.
#     A producer of value is expected to set one of these variants. Absence of
#     any variant indicates an error. The JSON representation for `Value` is JSON
#     value.
#     """
#
#     null_value: "NullValue" = betterproto.enum_field(1, group="kind")
#     """Represents a null value."""
#
#     number_value: float = betterproto.double_field(2, group="kind")
#     """Represents a double value."""
#
#     string_value: str = betterproto.string_field(3, group="kind")
#     """Represents a string value."""
#
#     bool_value: bool = betterproto.bool_field(4, group="kind")
#     """Represents a boolean value."""
#
#     struct_value: "Struct" = betterproto.message_field(5, group="kind")
#     """Represents a structured value."""
#
#     list_value: "ListValue" = betterproto.message_field(6, group="kind")
#     """Represents a repeated `Value`."""


def _wrap_value(value: Any) -> BetterprotoValue:
    """Wrap a JSON-able Python value in a betterproto Value."""
    if value is None:
        return BetterprotoValue(null_value=NullValue.NULL_VALUE)
    elif isinstance(value, bool):
        return BetterprotoValue(bool_value=value)
    elif isinstance(value, int):
        return BetterprotoValue(number_value=float(value))
    elif isinstance(value, float):
        return BetterprotoValue(number_value=value)
    elif isinstance(value, str):
        return BetterprotoValue(string_value=value)
    elif isinstance(value, dict):
        return BetterprotoValue(struct_value=_PatchedStruct.from_dict(value))
    elif isinstance(value, list):
        return BetterprotoValue(list_value=ListValue([_wrap_value(v) for v in value]))
    else:
        raise ValueError(f"cannot wrap value: {value!r}")


def _unwrap_value(value: BetterprotoValue) -> Any:
    """Unwrap a betterproto Value into a JSON-able Python value."""
    _, v = betterproto.which_one_of(value, "kind")
    if v is None:
        return None
    elif isinstance(v, NullValue):
        return None
    elif isinstance(v, BetterprotoStruct):
        return v.to_dict()
    elif isinstance(v, ListValue):
        return [_unwrap_value(e) for e in v.values]
    else:
        return v


@dataclass(eq=False, repr=False)
class _PatchedStruct(BetterprotoStruct):
    @hybridmethod
    def from_dict(cls: type[Self], mapping: Mapping[str, Any]) -> Self:  # noqa
        self = cls()
        return self.from_dict(mapping)

    @from_dict.instancemethod
    def from_dict(self, mapping: Mapping[str, Any]) -> Self:
        fields = {**mapping}
        for k, v in fields.items():
            if not isinstance(v, BetterprotoValue):
                fields[k] = _wrap_value(v)
        self.fields = fields
        return self

    def to_dict(
        self,
        casing: betterproto.Casing = betterproto.Casing.CAMEL,
        include_default_values: bool = False,
    ) -> dict[str, Any]:
        output = {}
        for k, v in self.fields.items():
            output[k] = _unwrap_value(v)
        return output


# ensure 'Value' is in namespace the first time a Struct-like class is created
# if we don't do this here calls will fail mysteriously later
from betterproto.lib.google.protobuf import Value  # noqa

_PatchedStruct()

BetterprotoStruct.from_dict = _PatchedStruct.from_dict
BetterprotoStruct.to_dict = _PatchedStruct.to_dict

# add custom encode/decode methods for headers to RpcMetadata
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

    def to_headers(self) -> dict:
        # flat encoding with prefix, messages as base64 :RpcMetadataEncoding
        packed = {
            "2": self.client_id,
            "3": self.client_nonce,
            "4": self.client_access_token,
        }
        packed_badges = [
            {
                "2": badge.id,
                "3": badge.key,
                "4": badge.password,
            }
            for badge in self.badges
        ]
        if packed_badges:
            packed["5"] = b64encode(json.dumps(packed_badges).encode("utf-8")).decode("utf-8")
        return {"x-bench-" + k: v for k, v in packed.items() if v is not None}

    def from_headers(self, headers: Mapping) -> RpcMetadata:
        # flat encoding with prefixy, messages as base64 :RpcMetadataEncoding
        self.client_id = headers.get("x-bench-2")
        self.client_nonce = headers.get("x-bench-3")
        self.client_access_token = headers.get("x-bench-4")
        if headers.get("5"):
            unpacked_badges = json.loads(b64decode(headers.get("x-bench-5")).decode("utf-8"))
            self.badges = [
                RpcMetadataBadgeInfo(
                    id=badge.get("2"),
                    key=badge.get("3"),
                    password=badge.get("4"),
                )
                for badge in unpacked_badges
            ]
        return self


RpcMetadata.__repr__ = _PatchedRpcMetadata.__repr__
RpcMetadata.to_headers = _PatchedRpcMetadata.to_headers
RpcMetadata.from_headers = _PatchedRpcMetadata.from_headers

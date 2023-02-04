from __future__ import annotations

import copy
import dataclasses
import enum
import typing
from datetime import datetime
from functools import cache
from typing import get_type_hints
from uuid import UUID


@cache
def _prep_dataclass_fields(cls: typing.Type) -> dict[str, dataclasses.Field]:
    data_class_hints = get_type_hints(cls)
    fields = {}
    for f in dataclasses.fields(cls):  # noqa
        field = copy.copy(f)
        field.type = data_class_hints[field.name]
        fields[field.name] = field

    if "id" in fields:
        # ids must be UUID
        if fields["id"].type != UUID:
            raise TypeError(f"id field must be UUID, got {fields['id'].type} in {cls}")
    return fields


def deepcopy(obj: typing.Any) -> typing.Any:
    """Stupid simple deepcopy that serializes and deserializes."""
    cls = type(obj)
    return from_dict(cls, to_dict(obj))


def to_dict(obj: typing.Any, omit_empty: bool = False) -> typing.Any:
    """Convert any "reasonable" object to dict-able representation."""
    if dataclasses.is_dataclass(obj):
        fields = _prep_dataclass_fields(obj.__class__)
        return {
            f.name: to_dict(getattr(obj, f.name), omit_empty)
            for f in fields.values()
            if not omit_empty or getattr(obj, f.name) is not None
        }
    elif isinstance(obj, (list, tuple)):
        return [to_dict(item, omit_empty) for item in obj]
    elif isinstance(obj, dict):
        return {
            key: to_dict(value, omit_empty)
            for key, value in obj.items()
            if not omit_empty or value is not None
        }
    elif isinstance(obj, (datetime, UUID)):
        return str(obj)
    elif isinstance(obj, (int, float, str, bool)):
        return obj
    elif isinstance(obj, enum.Enum):
        return obj.value
    elif obj is None:
        return None
    else:
        raise TypeError(f"unexpected type {type(obj)} in {obj}")


def from_dict(
    cls: typing.Type | None, data: typing.Any, _path: list[str] | None = None
) -> typing.Any:
    """Convert data back to its value from a dict-able representation."""
    if _path is not None and len(_path) == 0:
        _path = ["<root>"]
    if not cls or not data:
        return data
    if dataclasses.is_dataclass(cls):
        if not isinstance(data, dict):
            raise TypeError(f"expected dict, got {type(data)} in {data}")
        fields = _prep_dataclass_fields(cls)
        # first pass: create object while skipping not required fields
        deserialized = {}
        for key, field in fields.items():
            is_primitive = field.type in (int, float, str, bool, datetime, UUID)
            has_default = (
                field.default is not dataclasses.MISSING
                or field.default_factory is not dataclasses.MISSING
            )
            if has_default and not is_primitive:
                continue
            if key in data:
                deserialized[key] = from_dict(
                    field.type, data[key], _path + [key] if _path else None
                )
            else:
                deserialized[key] = None
        obj = cls(**deserialized)
        # second pass: fill in missing fields
        for key, field in fields.items():
            if key in data and key not in deserialized:
                setattr(
                    obj, key, from_dict(field.type, data[key], _path + [key] if _path else None)
                )
        return obj
    elif cls in (str, int, float, bool, UUID, datetime):
        return cls(data)
    elif isinstance(cls, type) and issubclass(cls, enum.Enum):
        return cls(data)
    elif typing.get_origin(cls) is typing.Union:
        args = typing.get_args(cls)
        if len(args) == 2 and args[1] is type(None):  # noqa
            # optional
            return from_dict(args[0], data, _path)
        else:  # generic union
            # try to deserialize as each type until one works
            # (this isn't ideal, but we want to use a proper message format later anyway)
            # prefer classes to primitives to prevent trivial deserialization errors (like dict to str)
            args = sorted(args, key=lambda x: issubclass(x, (int, float, str, bool)))
            for arg in args:
                if arg is type(None):  # noqa
                    continue
                try:
                    return from_dict(arg, data, _path)
                except (TypeError, ValueError, AttributeError):
                    pass
    elif isinstance(data, list):
        args = typing.get_args(cls)
        origin_cls = typing.get_origin(cls)
        if origin_cls is list:
            inner_type = args[0] if args else None
            if _path is not None:
                return [
                    from_dict(inner_type, item, _path + [str(i)]) for i, item in enumerate(data)
                ]
            else:
                return [from_dict(inner_type, item, None) for item in data]
        elif origin_cls is tuple:
            if len(args) != len(data):
                raise TypeError(f"tuple length mismatch: {args} vs {data}")
            if _path is None:
                return tuple(
                    from_dict(inner_type, item, None) for inner_type, item in zip(args, data)
                )
            else:
                return tuple(
                    from_dict(inner_type, item, _path + [str(i)])
                    for i, (inner_type, item) in enumerate(zip(args, data))
                )
        elif hasattr(cls, "_fields"):  # namedtuple
            # get types from annotations
            fields = [
                from_dict(cls.__annotations__[field], item, _path + [field] if _path else None)
                for field, item in zip(cls._fields, data)
            ]
            return cls(*fields)
    elif isinstance(data, dict):
        args = typing.get_args(cls)
        key_type = args[0] if args else None
        value_type = args[1] if args else None
        return {
            from_dict(key_type, key, _path): from_dict(
                value_type, value, _path + [str(key)] if _path else None
            )
            for key, value in data.items()
        }

    if _path is not None:
        path_str = ".".join(_path) if _path else "<root>"
    else:
        path_str = "<unknown>"
    raise TypeError(f"unexpected type {cls} for {data} at {path_str}")

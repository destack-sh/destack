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
    for f in dataclasses.fields(cls):
        field = copy.copy(f)
        field.type = data_class_hints[field.name]
        fields[field.name] = field

    if "id" in fields:
        # ids must be UUID
        if fields["id"].type != UUID:
            raise TypeError(f"id field must be UUID, got {fields['id'].type} in {cls}")
    return fields


def to_dict(obj: typing.Any, refs: set[(str, UUID)] | None):
    """Convert dataclass to dict, storing repeated objects in refs."""
    if dataclasses.is_dataclass(obj):
        fields = _prep_dataclass_fields(obj.__class__)
        if refs is not None and "id" in fields:
            node_id = f"{type(obj).__name__}:{getattr(obj, 'id')}"
            if node_id in refs:
                return {"__ref__": node_id}
            refs.add(node_id)
        return {f.name: to_dict(getattr(obj, f.name), refs) for f in fields.values()}
    elif isinstance(obj, (list, tuple)):
        return [to_dict(item, refs) for item in obj]
    elif isinstance(obj, dict):
        return {key: to_dict(value, refs) for key, value in obj.items()}
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
    cls: typing.Type | None, data: typing.Any, refs: dict[(str, UUID), typing.Any]
) -> typing.Any:
    """Convert dict to dataclass, reusing repeated objects from refs."""
    if not cls or not data:
        return data
    if dataclasses.is_dataclass(cls):
        if "__ref__" in data:  # must be previously seen
            return refs[data["__ref__"]]
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
                deserialized[key] = from_dict(field.type, data[key], refs)
            else:
                deserialized[key] = None
        obj = cls(**deserialized)
        if "id" in fields:
            node_id = f"{type(obj).__name__}:{getattr(obj, 'id')}"
            refs[node_id] = obj
        # second pass: fill in missing fields
        for key, field in fields.items():
            if key in data and key not in deserialized:
                setattr(obj, key, from_dict(field.type, data[key], refs))
        return obj
    elif cls in (str, int, float, bool, UUID, datetime):
        return cls(data)
    elif isinstance(cls, type) and issubclass(cls, enum.Enum):
        return cls(data)
    elif typing.get_origin(cls) is typing.Union:
        args = typing.get_args(cls)
        if len(args) == 2 and args[1] is type(None):  # noqa
            # optional
            return from_dict(args[0], data, refs)
        else:  # generic union
            # try to deserialize as each type until one works
            # (this isn't ideal, but we want to use a proper message format later anyway)
            for arg in args:
                if arg is type(None):  # noqa
                    continue
                try:
                    return from_dict(arg, data, refs)
                except (TypeError, ValueError, AttributeError):
                    pass
    elif isinstance(data, list):
        args = typing.get_args(cls)
        inner_type = args[0] if args else None
        origin_cls = typing.get_origin(cls)
        if origin_cls is list:
            return [from_dict(inner_type, item, refs) for item in data]
        elif origin_cls is tuple:
            return tuple(from_dict(inner_type, item, refs) for item in data)
        elif hasattr(cls, "_fields"):  # namedtuple
            return cls(*[from_dict(inner_type, item, refs) for item in data])
    elif isinstance(data, dict):
        args = typing.get_args(cls)
        key_type = args[0] if args else None
        value_type = args[1] if args else None
        return {
            from_dict(key_type, key, refs): from_dict(value_type, value, refs)
            for key, value in data.items()
        }

    raise TypeError(f"unexpected type {cls} for {data}")

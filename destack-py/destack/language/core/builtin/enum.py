import enum
import functools
import typing
from typing import (
    TYPE_CHECKING,
)

from more_itertools import first

from destack.utils.string import Casing, to_casing

if TYPE_CHECKING:
    from destack.language import EnumType


_MIN_ID_BY_ENUM: dict[type, int] = {}
_MAX_ID_BY_ENUM: dict[type, int] = {}


class Enum(enum.IntEnum):
    ord: int
    id: int
    title: str | None
    text: str | None
    icon: str | None

    def __new__(
        cls,
        id: int,
        title: str | None = None,
        text: str | None = None,
        icon: str | None = None,
    ):
        obj = int.__new__(cls, id)
        obj._value_ = id
        obj.ord = len(cls)
        obj.id = id
        obj.text = text
        obj.title = title
        obj.icon = icon
        obj.__doc__ = text

        # check id
        assert id > 0, f"invalid id {id}"
        existing = first((v for v in cls if v.id == id), None)
        assert existing is None, f"{cls} has duplicate id {id} for {id} and {existing}"

        return obj

    @functools.cached_property
    def camel_name(self):
        from destack.utils.string import Casing, to_casing

        return to_casing(self.name, Casing.CAMEL)

    @classmethod
    def get_min_id(cls) -> int:
        """Get the minimum id."""
        if cls not in _MIN_ID_BY_ENUM:
            _MIN_ID_BY_ENUM[cls] = min(v.id for v in cls)
        return _MIN_ID_BY_ENUM[cls]

    @classmethod
    def get_max_id(cls) -> int:
        """Get the maximum id."""
        if cls not in _MAX_ID_BY_ENUM:
            _MAX_ID_BY_ENUM[cls] = max(v.id for v in cls)
        return _MAX_ID_BY_ENUM[cls]


# NOTE: we have the enum registry here to avoid circular imports
_ENUM_CLASS_BY_TYPE: dict["EnumType", type[Enum]] = {}
_ENUM_TYPE_BY_CLASS: dict[type[Enum], "EnumType"] = {}

BuiltinEnumT = typing.TypeVar("BuiltinEnumT", bound=Enum)


def builtin_enum(enum_type: "EnumType"):
    """Register a builtin Enum."""

    def register_enum(cls: type[BuiltinEnumT]) -> type[BuiltinEnumT]:
        if (existing_enum_type := _ENUM_CLASS_BY_TYPE.get(enum_type)) is not None:
            raise ValueError(
                f"enum {enum_type} duplicate: {existing_enum_type} ({cls.__module__}.{cls.__name__} != {existing_enum_type.__module__}.{existing_enum_type.__name__})"
            )
        _ENUM_CLASS_BY_TYPE[enum_type] = cls
        _ENUM_TYPE_BY_CLASS[cls] = enum_type
        enum_name = to_casing(cls.__name__, Casing.ALL_CAPS)
        assert enum_type.name == enum_name, f"enum name mismatch: {enum_type.name} != {enum_name}"
        return cls

    return register_enum

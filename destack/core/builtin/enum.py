import enum
import inspect
from dataclasses import dataclass
from sys import intern
from typing import TYPE_CHECKING, ClassVar, cast

from destack.registry import ENUM_CLASS_BY_TYPE, ENUM_TYPE_BY_CLASS

from ..utils.env import IS_DEV, IS_TEST
from ..utils.string import Casing, to_casing
from .const import UNSET

if TYPE_CHECKING:
    from destack import EnumType

# pyright: reportIncompatibleVariableOverride=false

type_ = type


@dataclass(slots=True)
class EnumDeclaration:
    # meta
    id: int
    type: EnumType
    name: str
    description: str | None
    is_flag: bool

    # content
    options: list["OptionDeclaration"]


@dataclass(slots=True)
class OptionDeclaration:
    # meta
    id: int
    name: str = UNSET
    description: str | None = None


def _process_enum_cls(
    cls: type_["Enum"], enum_type: EnumType
) -> tuple[type["Enum"], EnumDeclaration]:
    # walk the class defiition and collect options
    options: list[OptionDeclaration] = []
    for name, attribute in list(cls.__dict__.items()):
        if (
            name.startswith("__")
            or type(attribute).__name__.startswith("_")
            or inspect.ismethod(attribute)
            or inspect.isfunction(attribute)
            or isinstance(attribute, (property, classmethod, staticmethod))
        ):
            continue  # ignore reserved names and non-fields
        elif isinstance(attribute, OptionDeclaration):
            if attribute.name is UNSET:
                attribute.name = intern(name)
            options.append(attribute)
        else:
            raise TypeError(
                f"{cls.__name__}.{name} is not a OptionDeclaration: {attribute} ({type(attribute)})"
            )

    # enum declaration
    declaration = EnumDeclaration(
        id=enum_type.value,
        type=enum_type,
        name=cls.__name__,
        description=cls.__doc__,
        is_flag=isinstance(cls, FlagEnum),
        options=options,
    )
    cls.__declaration__ = declaration

    return cls, declaration


def declare_option(id: int, title: str | None = None, *, description: str | None = None) -> int:
    """Declare an option in an Enum."""

    declaration = OptionDeclaration(
        id=id,
        name=title or UNSET,
        description=description,
    )

    return cast(int, declaration)  # pretend it's an int for enum type annotation


def declare_enum(enum_type: "EnumType"):
    """Register a builtin Enum."""

    def decorate[T: type_["OptionEnum | FlagEnum"]](cls: T) -> T:
        cls, _ = _process_enum_cls(cls, enum_type)  # type: ignore
        cls.metatype = enum_type

        # register
        if (existing_enum_type := ENUM_CLASS_BY_TYPE.get(enum_type)) is not None:
            raise ValueError(
                f"enum {enum_type} duplicate: {existing_enum_type} ({cls.__module__}.{cls.__name__} != {existing_enum_type.__module__}.{existing_enum_type.__name__})"
            )
        ENUM_CLASS_BY_TYPE[enum_type] = cls
        ENUM_TYPE_BY_CLASS[cls] = enum_type

        # validate
        if IS_DEV or IS_TEST:
            # check name
            enum_name = to_casing(cls.__name__, Casing.ALL_CAPS)
            assert enum_type.name == enum_name, (
                f"enum name mismatch: {enum_type.name} != {enum_name}"
            )
            # check options
            options_by_id: dict[int, OptionDeclaration] = {}
            if issubclass(cls, OptionEnum):
                # options must be unique and in range (0 is forbidden)
                for option in cls.__declaration__.options:
                    assert 0 < option.id < 2**32, (
                        f"option {option.name}: {option.id} is out of range for {cls.__name__}"
                    )
                    assert option.id not in options_by_id, (
                        f"option {option.name}: {option.id} is a duplicate for {cls.__name__}"
                    )
                    options_by_id[option.id] = option
            elif issubclass(cls, FlagEnum):
                # flags must be unique and powers of 2 (0 is allowed)
                for option in cls.__declaration__.options:
                    assert 0 <= option.id < 2**32, (
                        f"option {option.name}: {option.id} is out of range for {cls.__name__}"
                    )
                    assert option.id & (option.id - 1) == 0, (
                        f"option {option.name}: {option.id} is not a power of 2 for {cls.__name__}"
                    )
                    assert option.id not in options_by_id, (
                        f"option {option.name}: {option.id} is a duplicate for {cls.__name__}"
                    )
                    options_by_id[option.id] = option
            else:
                raise ValueError(f"enum {cls.__name__} is not a OptionEnum or FlagEnum")

        return cls

    return decorate


class Enum(enum.IntEnum if TYPE_CHECKING else object):
    metatype: ClassVar[EnumType]  # type: ignore
    __declaration__: ClassVar[EnumDeclaration]  # type: ignore


class OptionEnum(Enum):
    pass


class FlagEnum(Enum):
    pass

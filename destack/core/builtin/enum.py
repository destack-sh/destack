import enum
import inspect
from collections.abc import Iterator
from dataclasses import dataclass
from sys import intern
from typing import TYPE_CHECKING, ClassVar, Self, assert_never, cast

from destack.registry import ENUM_CLASS_BY_TYPE, ENUM_TYPE_BY_CLASS

from ..utils.string import Casing, to_casing
from .const import UNSET

if TYPE_CHECKING:
    from destack import EnumType


type_ = type


@dataclass(slots=True)
class EnumDeclaration:
    # meta
    id: int
    type: "EnumType"
    name: str
    description: str | None
    is_flag: bool

    # content
    options: list["OptionDeclaration"]


class OptionDeclaration(int):
    # meta
    id: int
    name: str = UNSET
    component: type_["Enum"] = UNSET
    title: str | None = None
    description: str | None = None

    def __new__(cls, id: int, name: str, title: str, description: str | None = None):
        obj = int.__new__(cls, id)  # create the int part
        obj.id = id
        obj.name = name
        obj.component = UNSET
        obj.title = title
        obj.description = description
        return obj

    def __repr__(self) -> str:
        return f"<{self.component.__name__} {self.name} ({self.id})>"

    @property
    def value(self) -> int:
        return self.id  # aliased for enum compatibility


def _process_enum_cls(
    cls: type_["Enum"], enum_type: "EnumType"
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
            attribute.name = intern(name)
            if attribute.title is UNSET:
                attribute.title = to_casing(attribute.name, Casing.CAMEL, allow_whitespace=True)
            attribute.component = cls
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

    # register options
    options_by_id: dict[int, OptionDeclaration] = {option.id: option for option in options}
    options_by_name: dict[str, OptionDeclaration] = {option.name: option for option in options}
    cls.__options__ = options  # type: ignore
    cls.__options_by_id__ = options_by_id  # type: ignore
    cls.__options_by_name__ = options_by_name  # type: ignore

    return cls, declaration


def declare_option(id: int, title: str | None = None, *, description: str | None = None) -> int:
    """Declare an option in an Enum."""

    declaration = OptionDeclaration(
        id=id,
        name=UNSET,
        title=title or UNSET,
        description=description,
    )

    return cast(int, declaration)  # pretend it's an int for enum type annotation


def declare_enum(enum_type: "EnumType"):
    """Register a builtin Enum."""

    def decorate[T: type_["OptionEnum | FlagEnum"]](cls: T) -> T:
        processed_cls, declaration = _process_enum_cls(cast(type_["Enum"], cls), enum_type)
        cls = cast(T, processed_cls)
        cls.metatype = enum_type  # type: ignore

        # register
        if (existing_enum_type := ENUM_CLASS_BY_TYPE.get(enum_type)) is not None:
            raise ValueError(
                f"enum {enum_type} duplicate: {existing_enum_type} ({cls.__module__}.{cls.__name__} != {existing_enum_type.__module__}.{existing_enum_type.__name__})"
            )
        ENUM_CLASS_BY_TYPE[enum_type] = cls  # type: ignore
        ENUM_TYPE_BY_CLASS[cls] = enum_type  # type: ignore

        # validate
        # check name
        enum_name = to_casing(cls.__name__, Casing.ALL_CAPS)
        assert enum_type.name == enum_name, f"enum name mismatch: {enum_type.name} != {enum_name}"
        # check options
        options_by_id: dict[int, OptionDeclaration] = {}
        if issubclass(cls, OptionEnum):
            # options must be unique and in range (0 is forbidden)
            for option in declaration.options:
                assert 0 < option.id < 2**32, (
                    f"option {option.name}: {option.id} is out of range for {cls.__name__}"
                )
                assert option.id not in options_by_id, (
                    f"option {option.name}: {option.id} is a duplicate for {cls.__name__}"
                )
                options_by_id[option.id] = option
        elif issubclass(cls, FlagEnum):
            # flags must be unique and powers of 2 (0 is allowed)
            for option in declaration.options:
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
            assert_never(cls)

        return cls

    return decorate


class _EnumMeta(type):  # type: ignore
    """Metaclass for Enum that adds __len__ and __iter__."""

    def __len__(cls) -> int:
        return len(cls.__declaration__.options)  # type: ignore

    def __iter__(cls) -> Iterator[OptionDeclaration]:
        return iter(cls.__declaration__.options)  # type: ignore


class Enum(
    # pretend this is an IntEnum for regular use
    enum.IntEnum if TYPE_CHECKING else object,
    metaclass=type if TYPE_CHECKING else _EnumMeta,
):
    metatype: ClassVar["EnumType"]  # type: ignore
    __declaration__: ClassVar[EnumDeclaration]  # type: ignore

    __options__: ClassVar[list[Self]] = []
    __options_by_id__: ClassVar[dict[int, Self]] = {}
    __options_by_name__: ClassVar[dict[str, Self]] = {}


class OptionEnum(Enum):
    pass


class FlagEnum(enum.IntFlag if TYPE_CHECKING else Enum):
    pass

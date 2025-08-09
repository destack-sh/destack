import enum
import inspect
import re
from collections.abc import Iterator
from dataclasses import dataclass
from sys import intern
from typing import TYPE_CHECKING, ClassVar, Self, assert_never, cast

from destack.registry import ENUM_CLASS_BY_TYPE, ENUM_TYPE_BY_CLASS

from ._const import UNSET

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


class OptionDeclaration(int):  # pretend to be an enum member
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

    def __str__(self) -> str:
        return f"{self.name}"

    def __repr__(self) -> str:
        return f"{self.component.__name__}.{self.name}"

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
                attribute.title = _to_casing(
                    attribute.name, _Casing.UPPER_CAMEL, allow_whitespace=True
                )
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
    options_by_id: dict[int, OptionDeclaration] = {}
    options_by_alias: dict[str, OptionDeclaration] = {}
    for option in options:
        options_by_id[option.id] = option
        upper_name = option.name.upper()
        options_by_alias[upper_name] = option
        camel_name = _to_casing(option.name, _Casing.UPPER_CAMEL)
        options_by_alias[camel_name] = option
        snake_name = _to_casing(option.name, _Casing.SNAKE)
        options_by_alias[snake_name] = option
    cls.__options__ = options  # type: ignore
    cls.__options_by_id__ = options_by_id  # type: ignore
    cls.__options_by_alias__ = options_by_alias  # type: ignore
    cls.__members__ = {**options_by_id, **options_by_alias}  # type: ignore

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


_ALLOWED_FLAG_POSTFIXES = ("FLAG", "OPTION")


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
        enum_name = _to_casing(cls.__name__, _Casing.ALL_CAPS)
        assert enum_type.name == enum_name, f"enum name mismatch: {enum_type.name} != {enum_name}"
        # flag enum must end in Flag or Option
        if issubclass(cls, FlagEnum):
            assert enum_name.endswith(tuple(_ALLOWED_FLAG_POSTFIXES)), (
                f"flag enum {cls.__name__} must end with one of {_ALLOWED_FLAG_POSTFIXES}"
            )

        # check options
        options_by_id: dict[int, OptionDeclaration] = {}
        if issubclass(cls, OptionEnum):
            # options must be unique and in range (0 is forbidden)
            for option in declaration.options:
                assert 0 < option.id < 2**32, (
                    f"option {option.name}: {option.id} is out of range for {cls.__name__}"
                )
                assert option.id not in options_by_id, (
                    f"option {option.name}: {option.id} is a duplicate for {cls.__name__}: {options_by_id[option.id]}"
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
                    f"option {option.name}: {option.id} is a duplicate for {cls.__name__}: {options_by_id[option.id]}"
                )
                options_by_id[option.id] = option
        else:
            raise ValueError(f"unexpected enum type: {cls.__name__}")

        return cls

    return decorate


class _EnumMeta(type):  # type: ignore
    """Metaclass for Enum that adds __len__ and __iter__."""

    def __len__(cls) -> int:
        return len(cls.__declaration__.options)  # type: ignore

    def __iter__(cls) -> Iterator[OptionDeclaration]:
        return iter(cls.__declaration__.options)  # type: ignore

    def __getitem__(cls, key: int | str) -> OptionDeclaration:
        if isinstance(key, int):
            value = cls.__options_by_id__.get(key)  # type: ignore
        elif isinstance(key, str):
            value = cls.__options_by_alias__.get(key)  # type: ignore
        else:
            assert_never(key)
        if value is None:
            raise KeyError(f"option '{key}' not found in {cls.__name__}")
        return value

    def __call__(cls, value: int | str) -> OptionDeclaration:
        if isinstance(value, int):
            return cls.__options_by_id__[value]  # type: ignore
        elif isinstance(value, str):
            return cls.__options_by_alias__[value]  # type: ignore
        else:
            assert_never(value)


class Enum(  # type: ignore (we're just pretending to be an IntEnum, so metaclass conflict is fine)
    enum.IntEnum if TYPE_CHECKING else object,
    metaclass=enum.EnumMeta if TYPE_CHECKING else _EnumMeta,
):
    metatype: ClassVar["EnumType"]  # type: ignore
    __declaration__: ClassVar[EnumDeclaration]

    __options__: ClassVar[list[Self]] = []
    __options_by_id__: ClassVar[dict[int, Self]] = {}
    __options_by_alias__: ClassVar[dict[str, Self]] = {}

    __members__: ClassVar[dict[str | int, Self]] = {}  # alias to __options_by_alias__


class OptionEnum(Enum):
    pass


class FlagEnum(enum.IntFlag if TYPE_CHECKING else Enum):
    pass


#
# Casing
#

# NOTE: we define :Casing here because we use it for declaring enums
#  (we declare it for export in casing.py)


class _Casing(enum.IntEnum):  # see :Casing
    SNAKE = 1
    UPPER_CAMEL = 2
    LOWER_CAMEL = 3
    ALL_CAPS = 4


def _strip_alpha_num(name: str) -> str:
    # remove leading underscores
    name = re.sub(r"^_+", "", name)
    # remove trailing underscores
    name = re.sub(r"_+$", "", name)
    # remove double underscores
    name = re.sub(r"__+", "_", name)
    # remove leading digits
    name = re.sub(r"^[0-9]+", "", name)
    return name


_CACHED_CASING: dict[tuple[str, _Casing, bool], str] = {}


def _to_casing(name: str, casing: _Casing, allow_whitespace: bool = False) -> str:
    """Turns a string into a valid Python identifier."""
    key = (name, casing, allow_whitespace)
    if key in _CACHED_CASING:
        return _CACHED_CASING[key]

    if casing == _Casing.SNAKE:  # snake_case
        # first transform lowerUpper transitions into lower_upper
        name = re.sub(r"(?<=[a-z])(?=[A-Z])", "_", name)
        # turn non-alphanumeric characters into underscores
        name = re.sub(r"[^a-zA-Z0-9_]", "_", name)
        name = _strip_alpha_num(name)
        name = name.lower()
        if allow_whitespace:
            name = name.replace("_", " ").strip()
    elif (
        casing == _Casing.UPPER_CAMEL or casing == _Casing.LOWER_CAMEL
    ):  # CamelCase or lowerCamelCase
        # if it's already a mix of uppercase and lowercase starting with uppercase, leave it alone
        if re.match(r"^[A-Z][a-z0-9]+([A-Z]+[a-z0-9]+)+", name):
            _CACHED_CASING[key] = name
            return name
        # ignore non-alphanumeric characters and capitalize the next character
        name = re.sub(r"[^a-zA-Z0-9]", " ", name)
        # split on existing uppercase characters and spaces
        name = " ".join(re.split(r"(?<=[a-z])(?=[A-Z])", name))
        name = _strip_alpha_num(name).title()
        name = name.replace("_", " ").strip() if allow_whitespace else name.replace(" ", "")
        if casing == _Casing.LOWER_CAMEL:
            name = name[0].lower() + name[1:]
    elif casing == _Casing.ALL_CAPS:  # ALL_CAPS
        # ALL_CAPS, ignore non-alphanumeric characters and capitalize the next character
        name = re.sub(r"[^a-zA-Z0-9]", " ", name)
        # split on existing uppercase characters and spaces
        name = " ".join(re.split(r"(?<=[a-z])(?=[A-Z])", name))
        name = _strip_alpha_num(name).upper().replace(" ", "_")
        if allow_whitespace:
            name = name.replace("_", " ").strip()
    else:
        assert_never(casing)

    _CACHED_CASING[key] = name
    return name

import abc
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Self,
    cast,
    dataclass_transform,
)

from destack.language.registry import STRUCT_CLASS_BY_TYPE, STRUCT_TYPE_BY_CLASS
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

from .builtin import EnumType, ObjectKind, ObjectStability, StructType
from .declaration import StructDeclaration, TagDeclaration, builtin_method
from .object import Object, _process_object_cls
from .property import _PROPERTY_SPECIFIERS, builtin_property_runtime

if TYPE_CHECKING:
    from destack.language import StructDefinition

# pyright: reportIncompatibleVariableOverride=false

logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def builtin_struct(
    struct_type: StructType,
    *,
    frozen: bool = False,
    is_abstract: bool = False,
    stability: ObjectStability = ObjectStability.DYNAMIC,
    enum_types: tuple[EnumType, ...] = (),
    tags: tuple["TagDeclaration", ...] = (),
):
    """Register a class as a concrete struct for the given struct type."""

    def decorate(cls: type) -> type:
        # bases
        inherits: list[StructType] = []
        all_enum_types: list[EnumType] = []
        if cls.__name__ != "Struct" and cls.__name__ != "StructFrozen":
            for base in cls.__mro__:
                if issubclass(base, Struct):
                    if base.metatype not in inherits:
                        inherits.append(base.metatype)
                    for enum_type in base.__declaration__.self_enum_types:
                        if enum_type not in all_enum_types:
                            all_enum_types.append(enum_type)

        # declaration
        declaration = StructDeclaration(
            # meta
            cls=cls,
            type=struct_type,
            id=struct_type.value,
            kind=ObjectKind.STRUCT,
            stability=stability,
            is_abstract=is_abstract,
            is_frozen=frozen,
            is_final=False,
            # inherits
            base_type=inherits[0] if inherits else None,
            inherits=list(reversed(inherits)),
            inherited_by=[],
            extended_by=[],
            # content
            properties=[],
            methods=[],
            actions=[],
            constants=[],
            tags=list(tags),
            # associations
            enum_types=list(all_enum_types),
            self_enum_types=list(all_enum_types),
        )

        # abstract nodes cannot extend non-abstract nodes
        if is_abstract and cls.__bases__ and not cls.__bases__[0].__is_abstract__:
            raise ValueError(
                f"{cls.__name__} is abstract but extends non-abstract {cls.__bases__[0].__name__}"
            )

        # process class
        cls, _ = _process_object_cls(cast(type["Struct"], cls), declaration)
        if struct_type is not None:
            cls.metatype = struct_type

        # register struct
        if struct_type is not None and cls.__name__ != "StructFrozen":
            assert cls.__name__ == "Struct" or issubclass(cls, Struct), (
                f"struct class {cls} is not a Struct"
            )
            if struct_type in STRUCT_CLASS_BY_TYPE:
                raise ValueError(
                    f"struct class conflict for {struct_type}: {cls}, {STRUCT_CLASS_BY_TYPE[struct_type]}"
                )
            STRUCT_CLASS_BY_TYPE[struct_type] = cls
            STRUCT_TYPE_BY_CLASS[cls] = struct_type

        return cast(type["Struct"], cls)

    return decorate


@builtin_struct(StructType.STRUCT, is_abstract=True)
class Struct(Object, abc.ABC):
    """A Struct is an ordered collection of Properties."""

    # meta
    metatype: ClassVar[StructType]
    __kind__: ClassVar[ObjectKind] = ObjectKind.STRUCT
    __declaration__: ClassVar["StructDeclaration"]
    __definition__: ClassVar["StructDefinition"]

    def __eq__(self, other: Any):
        """Equals the Struct contents."""
        raise NotImplementedError  # generated


@builtin_struct(
    StructType.STRUCT,
    frozen=True,  # type: ignore (frozen can't inherit from non-frozen usually, but it's fine for us)
    is_abstract=True,
)
class StructFrozen(Struct):
    """An immutable Struct."""

    """Cached hash of the Struct."""
    _hash: "int | None" = builtin_property_runtime()
    """Cached repr of the Struct."""
    _repr: "str | None" = builtin_property_runtime()

    def _invalidate_frozen_cache(self) -> None:
        # frozen Structs should be immutable, but sometimes we need to break out of that
        object.__setattr__(self, "_hash", None)
        object.__setattr__(self, "_repr", None)

    @builtin_method(60)
    def clone(self, **override: Any) -> Self:
        """Clone the Struct with new values."""
        kwargs: dict[str, Any] = {}
        for prop in self.__properties__.values():
            if not prop.is_runtime_only:
                kwargs[prop.name] = getattr(self, prop.name)
        kwargs.update(override)
        return self.__class__(**kwargs)

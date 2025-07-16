import abc
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    cast,
    dataclass_transform,
)

import structlog
from opentelemetry import trace

from destack.language.registry import STRUCT_CLASS_BY_TYPE, STRUCT_TYPE_BY_CLASS
from destack.proto import AnyStructProto
from destack.utils.func import get_superclasses

from .common import StructType
from .object import (
    BuiltinObject,
    _process_object_cls,
)
from .property import _PROPERTY_SPECIFIERS, builtin_property_runtime

if TYPE_CHECKING:
    from destack.language import Json, StructDefinition

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)
type_ = type


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def builtin_struct(
    struct_type: StructType | None,
    frozen: bool = False,
    is_abstract: bool = False,
    is_extensible: bool = False,
):
    """Register a class as a concrete struct for the given struct type."""

    def decorate(cls: type) -> type:
        # bases
        inherits: list[StructType] = []
        for superclass in get_superclasses(cls):
            if isinstance(base_type := getattr(superclass, "metatype", None), StructType):
                if base_type not in inherits:
                    inherits.append(base_type)
        for base in cls.__bases__:
            if isinstance(base_type := getattr(base, "metatype", None), StructType):
                if base_type not in inherits:
                    inherits.append(base_type)
        cls.__inherits__ = tuple(reversed(inherits))
        cls.__base_type__ = cls.__inherits__[-1] if cls.__inherits__ else None
        cls.__is_abstract__ = is_abstract
        cls.__is_extensible__ = is_extensible

        # abstract nodes cannot extend non-abstract nodes
        if is_abstract and cls.__bases__ and not cls.__bases__[0].__is_abstract__:
            raise ValueError(
                f"{cls.__name__} is abstract but extends non-abstract {cls.__bases__[0].__name__}"
            )

        cls, _ = _process_object_cls(
            cls=cast(type["Struct"], cls),
            object_type=struct_type,
            is_frozen=frozen,
            is_concrete=struct_type is not None,
            is_struct=True,
            is_node=False,
            is_entity=False,
            is_root_node=False,
            is_abstract=is_abstract,
            base_type=cls.__base_type__,
            inherits=cls.__inherits__,
            traits=(),
        )
        if struct_type is not None:
            cls.metatype = struct_type

        # register struct
        if struct_type is not None:
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


@builtin_struct(StructType.STRUCT, is_abstract=True, is_extensible=True)
class Struct[StructProtoT: AnyStructProto](BuiltinObject[StructProtoT], abc.ABC):
    """A Struct is an ordered collection of Properties."""

    metatype: ClassVar[StructType]
    __is_struct__: ClassVar[bool] = True

    __definition__: ClassVar["StructDefinition"]

    """Whether this class is abstract (not concrete)."""
    __is_abstract__: ClassVar[bool] = False
    """Whether this Struct can be extended by custom Structs."""
    __is_extensible__: ClassVar[bool] = False
    """The base type this Struct extends (directly)."""
    __base_type__: ClassVar[StructType | None] = None
    """Structs that extend this Struct type (directly)."""
    __extended_by__: ClassVar[tuple[StructType, ...]] = ()
    """Structs that this Struct extends (directly and indirectly)."""
    __inherits__: ClassVar[tuple[StructType, ...]] = ()
    """Structs that extend this Struct type (directly and indirectly)."""
    __inherited_by__: ClassVar[tuple[StructType, ...]] = ()

    def __eq__(self, other: Any):
        """Equals the Struct contents."""
        raise NotImplementedError  # generated


@builtin_struct(None, is_abstract=True, is_extensible=True)
class StructMutable[StructProtoT: AnyStructProto](Struct[StructProtoT]):
    """A mutable Struct."""

    pass


@builtin_struct(
    None,
    frozen=True,  # type: ignore (frozen can't inherit from non-frozen usually, but it's fine for us)
    is_abstract=True,
    is_extensible=True,
)
class StructFrozen[StructProtoT: AnyStructProto](Struct[StructProtoT]):
    """An immutable Struct."""

    """Cached hash of the Struct."""
    _hash: "int | None" = builtin_property_runtime()
    """Cached repr of the Struct."""
    _repr: "str | None" = builtin_property_runtime()
    """Cached proto representation of the Struct."""
    _proto: "StructProtoT | None" = builtin_property_runtime()
    """Cached value representation of the Struct."""
    _value: "Json | None" = builtin_property_runtime()

    def _invalidate_frozen_cache(self) -> None:
        # frozen Structs should be immutable, but sometimes we need to break out of that
        object.__setattr__(self, "_hash", None)
        object.__setattr__(self, "_repr", None)
        object.__setattr__(self, "_proto", None)
        object.__setattr__(self, "_value", None)

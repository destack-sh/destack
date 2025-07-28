import abc
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Self,
    cast,
    dataclass_transform,
    override,
)

from destack.language.registry import STRUCT_CLASS_BY_TYPE, STRUCT_TYPE_BY_CLASS
from destack.utils.func import get_superclasses
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

from .builtin import EnumType, ObjectKind, StructType
from .common import Encoding, PackedObjectCache
from .const import ENCODERS
from .meta import TagDeclaration, builtin_method
from .object import (
    BuiltinObject,
    _process_object_cls,
)
from .property import _PROPERTY_SPECIFIERS, builtin_property_runtime

if TYPE_CHECKING:
    from destack.language import (
        ActionDefinition,
        BinaryWriter,
        ConstantDefinition,
        EncoderOptions,
        MethodDefinition,
        StructDefinition,
        TagDefinition,
    )

# pyright: reportIncompatibleVariableOverride=false

logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def builtin_struct(
    struct_type: StructType | None,
    *,
    frozen: bool = False,
    is_abstract: bool = False,
    is_stable: bool = False,
    enum_types: tuple[EnumType, ...] = (),
    tags: tuple["TagDeclaration", ...] = (),
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
        cls.__is_stable__ = is_stable

        # enum types
        cls.__self_enum_types__ = tuple(enum_types)

        # abstract nodes cannot extend non-abstract nodes
        if is_abstract and cls.__bases__ and not cls.__bases__[0].__is_abstract__:
            raise ValueError(
                f"{cls.__name__} is abstract but extends non-abstract {cls.__bases__[0].__name__}"
            )

        # process class
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

        # tags
        cls.__declared_tags__ = tags

        return cast(type["Struct"], cls)

    return decorate


@builtin_struct(StructType.STRUCT, is_abstract=True)
class Struct(BuiltinObject, abc.ABC):
    """A Struct is an ordered collection of Properties."""

    # meta
    metatype: ClassVar[StructType]
    __definition__: ClassVar["StructDefinition"]
    __kind__: ClassVar[ObjectKind] = ObjectKind.STRUCT
    __is_struct__: ClassVar[bool] = True
    """Whether this class is abstract (not concrete)."""
    __is_abstract__: ClassVar[bool] = False
    """Whether this Struct is stable (cannot be redefined by the system)."""
    __is_stable__: ClassVar[bool] = False

    # inheritance
    """The base type this Struct extends (directly)."""
    __base_type__: ClassVar[StructType | None] = None
    """Structs that extend this Struct type (directly)."""
    __extended_by__: ClassVar[tuple[StructType, ...]] = ()
    """Structs that this Struct extends (directly and indirectly)."""
    __inherits__: ClassVar[tuple[StructType, ...]] = ()
    """Structs that extend this Struct type (directly and indirectly)."""
    __inherited_by__: ClassVar[tuple[StructType, ...]] = ()

    # content
    """The methods for this Struct type."""
    __methods__: ClassVar[tuple["MethodDefinition", ...]] = ()
    """The actions for this Struct type."""
    __actions__: ClassVar[tuple["ActionDefinition", ...]] = ()
    """The constants for this Struct type."""
    __constants__: ClassVar[tuple["ConstantDefinition", ...]] = ()
    """The tags for this Struct type."""
    __tags__: ClassVar[tuple["TagDefinition", ...]] = ()
    """The tags for this Struct type (declarations for during construction)."""
    __declared_tags__: ClassVar[tuple["TagDeclaration", ...]] = ()

    # associations
    __enum_types__: ClassVar[tuple[EnumType, ...]] = ()
    __self_enum_types__: ClassVar[tuple[EnumType, ...]] = ()

    def __eq__(self, other: Any):
        """Equals the Struct contents."""
        raise NotImplementedError  # generated


@builtin_struct(
    None,
    frozen=True,  # type: ignore (frozen can't inherit from non-frozen usually, but it's fine for us)
    is_abstract=True,
)
class StructFrozen(Struct):
    """An immutable Struct."""

    """Cached hash of the Struct."""
    _hash: "int | None" = builtin_property_runtime()
    """Cached repr of the Struct."""
    _repr: "str | None" = builtin_property_runtime()
    """Cached packed representations (first N = each Encoding, next N = each Encoding as bytes)."""
    _packed_cache: "tuple[PackedObjectCache, ...] | None" = builtin_property_runtime()

    def _invalidate_frozen_cache(self) -> None:
        # frozen Structs should be immutable, but sometimes we need to break out of that
        object.__setattr__(self, "_hash", None)
        object.__setattr__(self, "_repr", None)

    @builtin_method(30)
    @override
    def pack(self, encoding: Encoding, options: "EncoderOptions | None" = None) -> Any:
        from ..runtime.encoder import Encoder

        options = options or Encoder.TAGGED
        # check if we have a cached packed representation
        if self._packed_cache is not None:
            for cached in self._packed_cache:
                if cached.encoding == encoding and not cached.is_bytes:
                    return cached.packed
        # pack the object
        encoder = ENCODERS[encoding]
        packed_object = encoder.pack_object(self.__kind__, self.metatype, self, options)
        # cache the result
        new_cache = PackedObjectCache(encoding=encoding, is_bytes=False, packed=packed_object)
        if self._packed_cache is None:
            object.__setattr__(self, "_packed_cache", (new_cache,))
        else:
            object.__setattr__(self, "_packed_cache", (*self._packed_cache, new_cache))
        return packed_object

    @builtin_method(31)
    @override
    def pack_binary(
        self, encoding: Encoding, writer: "BinaryWriter", options: "EncoderOptions | None" = None
    ) -> None:
        from ..runtime.encoder import Encoder

        options = options or Encoder.TAGGED
        # check if we have a cached packed bytes representation
        if self._packed_cache is not None:
            for cached in self._packed_cache:
                if cached.encoding == encoding and cached.is_bytes:
                    writer.write_bytes(cached.packed)
                    return
        # pack the object as bytes
        encoder = ENCODERS[encoding]
        encoder.pack_object_binary(self.__kind__, self.metatype, self, writer, options)
        # cache the result
        new_cache = PackedObjectCache(encoding=encoding, is_bytes=True, packed=writer.to_bytes())
        if self._packed_cache is None:
            object.__setattr__(self, "_packed_cache", (new_cache,))
        else:
            object.__setattr__(self, "_packed_cache", (*self._packed_cache, new_cache))

    @builtin_method(60)
    def clone(self, **override: Any) -> Self:
        """Clone the Struct with new values."""
        kwargs: dict[str, Any] = {}
        for prop in self.__properties__.values():
            if prop.name != "metatype" and prop.is_wired:
                kwargs[prop.name] = getattr(self, prop.name)
        kwargs.update(override)
        return self.__class__(**kwargs)

from typing import (
    TYPE_CHECKING,
    ClassVar,
    cast,
    dataclass_transform,
)

from destack.registry import STRUCT_CLASS_BY_TYPE, STRUCT_TYPE_BY_CLASS

from ._hoisted import EncoderStability
from .declaration import StructDeclaration, TagDeclaration
from .object import _get_universe_domain, _process_object_cls
from .property import _PROPERTY_SPECIFIERS, PropertyDeclaration
from .universe import NodeType, StructType

if TYPE_CHECKING:
    from destack import StructDefinition


type_ = type


def _process_struct_cls(
    # meta
    cls: type["Struct"],
    struct_type: StructType,
    stability: EncoderStability,
    is_immutable: bool,
    is_abstract: bool,
    is_final: bool,
    is_interned: bool,
    # associations
    tags: tuple["TagDeclaration", ...],
    into_node_types: tuple[NodeType, ...],
) -> type["Struct"]:
    """Process a Struct class and return the processed class and its properties."""

    # inheritance
    inherits: list[StructType] = []
    if cls.__name__ != "Struct" and cls.__name__ != "ImmutableStruct":
        for base in cls.__mro__:
            if issubclass(base, Struct):
                if base.metatype not in inherits:
                    inherits.append(base.metatype)

    # declaration
    domain, category = _get_universe_domain(struct_type)
    declaration = StructDeclaration(
        # meta
        cls=cls,
        type=struct_type,
        id=struct_type.value,
        name=cls.__name__,
        description=cls.__doc__ or "",
        stability=stability,
        domain=domain,
        category=category,
        is_abstract=is_abstract,
        is_immutable=is_immutable,
        is_final=is_final,
        is_interned=is_interned,
        # inherits
        base_type=inherits[0] if inherits else None,
        inherits=list(reversed(inherits)),
        inherited_by=[],
        extended_by=[],
        # content
        properties=[],  # set in _process_object_cls
        methods=[],  # set in finalize
        constants=[],  # set in finalize
        tags=list(tags),
        # associations
        into_node_types=list(into_node_types),
    )

    # process object class
    cls, _ = _process_object_cls(cast(type["Struct"], cls), declaration)
    cls.metatype = struct_type

    # register struct
    assert cls.__name__ == "Struct" or issubclass(cls, Struct), (
        f"struct class {cls} is not a Struct"
    )
    if struct_type in STRUCT_CLASS_BY_TYPE:
        raise ValueError(
            f"struct class conflict for {struct_type}: {cls}, {STRUCT_CLASS_BY_TYPE[struct_type]}"
        )
    STRUCT_CLASS_BY_TYPE[struct_type] = cls
    STRUCT_TYPE_BY_CLASS[cls] = struct_type

    # validate
    # interned structs must be immutable
    if is_interned and not is_immutable:
        raise ValueError(f"{cls.__name__} is interned but not immutable")
    # interned structs cannot have interned properties
    if is_interned and any(prop.is_interned for prop in cls.__declaration__.properties):
        interned_props = [prop for prop in cls.__declaration__.properties if prop.is_interned]
        raise ValueError(
            f"{cls.__name__} is interned but also has interned properties: {interned_props}"
        )
    # non-abstract structs must have properties
    if not is_abstract and not any(
        not prop.is_runtime_only for prop in cls.__declaration__.properties
    ):
        raise ValueError(f"{cls.__name__} is not abstract but has no properties")
    # abstract objects cannot extend non-abstract objects
    if is_abstract and len(cls.__bases__) > 1 and not cls.__bases__[1].__is_abstract__:
        raise ValueError(
            f"{cls.__name__} is abstract but extends non-abstract {cls.__bases__[1].__name__}"
        )
    # cannot be both abstract and final
    if is_abstract and is_final:
        raise ValueError(f"{cls.__name__} cannot be both abstract and final")
    # final classes must be annotated with @final
    if is_final != getattr(cls, "__final__", False):
        raise ValueError(
            f"{cls.__name__} has @final={getattr(cls, '__final__', False)} but is_final={is_final}"
        )
    # final objects cannot be extended
    if any(
        hasattr(base, "__declaration__") and base.__declaration__.is_final for base in cls.__bases__
    ):
        bad_base = next(
            base
            for base in cls.__bases__
            if hasattr(base, "__declaration__") and base.__declaration__.is_final
        )
        raise ValueError(f"{cls.__name__} extends final {bad_base.__name__}")

    return cast(type["Struct"], cls)


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def declare_struct(
    # meta
    struct_type: StructType,
    *,
    stability: EncoderStability = EncoderStability.DYNAMIC,
    is_immutable: bool = False,
    is_abstract: bool = False,
    is_final: bool = False,
    is_interned: bool = False,
    # associations
    tags: tuple["TagDeclaration", ...] = (),
    into_node_types: tuple[NodeType, ...] = (),
):
    """Register a class as a concrete struct for the given struct type."""

    def decorate(cls: type) -> type:
        cls = _process_struct_cls(
            cls=cast(type["Struct"], cls),
            struct_type=struct_type,
            stability=stability,
            is_immutable=is_immutable,
            is_abstract=is_abstract,
            is_final=is_final,
            is_interned=is_interned,
            tags=tags,
            into_node_types=into_node_types,
        )
        return cls

    return decorate


@declare_struct(StructType.STRUCT, is_abstract=True)
class Struct:
    """A Struct is a collection of Properties."""

    """The type of Struct this is (static)."""
    metatype: ClassVar[StructType]
    """The declaration of this Struct (static)."""
    __declaration__: ClassVar["StructDeclaration"]
    """The definition of this Struct (static)."""
    __definition__: ClassVar["StructDefinition"]

    """The properties of this Object (runtime)."""
    __properties__: ClassVar[dict[str, PropertyDeclaration]] = {}
    """The properties of this Object by alias (runtime)."""
    __properties_by_alias__: ClassVar[dict[str, PropertyDeclaration]] = {}
    """The properties of this Object by id (runtime)."""
    __properties_by_id__: ClassVar[dict[int, PropertyDeclaration]] = {}

    @classmethod
    def property(cls, name: str) -> PropertyDeclaration:
        """Get a Property by name."""
        prop = cls.__properties_by_alias__.get(name)
        if prop is not None:
            return prop
        raise ValueError(f"no property '{name}' in {cls.__name__}")

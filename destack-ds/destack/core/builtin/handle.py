from typing import (
    TYPE_CHECKING,
    ClassVar,
    cast,
    dataclass_transform,
)

from destack.registry import HANDLE_CLASS_BY_TYPE, HANDLE_TYPE_BY_CLASS

from ._hoisted import EncoderStability
from .declaration import HandleDeclaration, TagDeclaration
from .object import _get_universe_domain
from .property import _PROPERTY_SPECIFIERS
from .universe import HandleType, NodeType

if TYPE_CHECKING:
    from destack import HandleDefinition


type_ = type


def _process_handle_cls(
    cls: type["Handle"],
    handle_type: HandleType,
    stability: EncoderStability,
    is_abstract: bool,
    is_final: bool,
    tags: tuple["TagDeclaration", ...],
    event_types: tuple[NodeType, ...],
) -> type["Handle"]:
    # inheritance
    inherits: list[HandleType] = []
    all_event_types: list[NodeType] = []
    if cls.__name__ != "Handle":
        for base in cls.__mro__:
            if issubclass(base, Handle):
                if base.metatype not in inherits:
                    inherits.append(base.metatype)
                for event_type in base.__declaration__.event_types:
                    if event_type not in all_event_types:
                        all_event_types.append(event_type)

    # declaration
    domain, category = _get_universe_domain(handle_type.value)
    declaration = HandleDeclaration(
        # meta
        cls=cls,
        type=handle_type,
        id=handle_type.value,
        name=cls.__name__,
        description=cls.__doc__ or "",
        stability=stability,
        domain=domain,
        category=category,
        is_abstract=is_abstract,
        is_immutable=False,
        is_final=is_final,
        # inherits
        base_type=inherits[0] if inherits else None,
        inherits=list(reversed(inherits)),
        inherited_by=[],
        extended_by=[],
        # content
        properties=[],
        methods=[],
        constants=[],
        tags=list(tags),
        # associations
        event_types=list(event_types),
        self_event_types=list(all_event_types),
    )

    # process object class
    cls.__declaration__ = declaration
    cls.metatype = handle_type

    # register handle
    assert cls.__name__ == "Handle" or issubclass(cls, Handle), (
        f"handle class {cls} is not a Handle"
    )
    if handle_type in HANDLE_CLASS_BY_TYPE:
        raise ValueError(
            f"handle class conflict for {handle_type}: {cls}, {HANDLE_CLASS_BY_TYPE[handle_type]}"
        )
    HANDLE_CLASS_BY_TYPE[handle_type] = cls
    HANDLE_TYPE_BY_CLASS[cls] = handle_type

    # validate
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

    return cast(type["Handle"], cls)


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def declare_handle(
    handle_type: HandleType,
    *,
    is_abstract: bool = False,
    is_final: bool = False,
    stability: EncoderStability = EncoderStability.DYNAMIC,
    tags: tuple["TagDeclaration", ...] = (),
    event_types: tuple[NodeType, ...] = (),
):
    """Register a class as a concrete handle for the given handle type."""

    def decorate(cls: type) -> type:
        cls = _process_handle_cls(
            cls=cast(type["Handle"], cls),
            handle_type=handle_type,
            stability=stability,
            is_abstract=is_abstract,
            is_final=is_final,
            tags=tags,
            event_types=event_types,
        )
        return cls

    return decorate


@declare_handle(HandleType.HANDLE, is_abstract=True)
class Handle:
    """
    A Handle is a (runtime-only) Object for interacting with the runtime.

    NOTE: Runtime specific Handles calling conventions may deviate slightly.
     See the language-specific documentation for more information.
    """

    # NOTE :Architecture: runtime Handle implementations deviate from their Handle declaration
    #  Unlike with other Objects (Nodes/Structs), Handles are opaque and declaration-only,
    #   we don't actually generate anything from them directly since they're wildly different
    #   for every runtime and forcing a common definition would be meaningless and cumbersome.

    # meta
    metatype: ClassVar[HandleType]
    __declaration__: ClassVar["HandleDeclaration"]
    __definition__: ClassVar["HandleDefinition"]

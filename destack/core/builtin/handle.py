from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    cast,
    dataclass_transform,
)

from destack.registry import HANDLE_CLASS_BY_TYPE, HANDLE_TYPE_BY_CLASS

from ..utils.env import IS_DEV, IS_TEST
from .builtin import EnumType, HandleType, NodeType, ObjectKind, ObjectStability
from .declaration import HandleDeclaration, TagDeclaration
from .object import Object, _process_object_cls
from .property import _PROPERTY_SPECIFIERS

if TYPE_CHECKING:
    from destack import HandleDefinition

# pyright: reportIncompatibleVariableOverride=false

type_ = type


def _process_handle_cls(
    cls: type["Handle"],
    handle_type: HandleType,
    stability: ObjectStability,
    is_abstract: bool,
    is_final: bool,
    tags: tuple["TagDeclaration", ...],
    enum_types: tuple[EnumType, ...],
    event_types: tuple[NodeType, ...],
) -> type["Handle"]:
    # inheritance
    inherits: list[HandleType] = []
    all_enum_types: list[EnumType] = []
    all_event_types: list[NodeType] = []
    if cls.__name__ != "Handle":
        for base in cls.__mro__:
            if issubclass(base, Handle):
                if base.metatype not in inherits:
                    inherits.append(base.metatype)
                for enum_type in base.__declaration__.enum_types:
                    if enum_type not in all_enum_types:
                        all_enum_types.append(enum_type)
                for event_type in base.__declaration__.event_types:
                    if event_type not in all_event_types:
                        all_event_types.append(event_type)

    # declaration
    declaration = HandleDeclaration(
        # meta
        cls=cls,
        type=handle_type,
        id=handle_type.value,
        kind=ObjectKind.HANDLE,
        stability=stability,
        is_abstract=is_abstract,
        is_frozen=False,
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
        enum_types=list(enum_types),
        self_enum_types=list(all_enum_types),
        event_types=list(event_types),
        self_event_types=list(all_event_types),
    )

    # process object class
    cls, _ = _process_object_cls(cast(type["Handle"], cls), declaration)
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

    # sanity check
    if IS_DEV or IS_TEST:
        if any(not prop.is_runtime_only for prop in cls.__properties__.values()):
            non_runtime_properties = [
                prop for prop in cls.__properties__.values() if not prop.is_runtime_only
            ]
            raise ValueError(f"{cls.__name__} has non-runtime properties: {non_runtime_properties}")
        # abstract objects cannot extend non-abstract objects
        if is_abstract and cls.__bases__ and not cls.__bases__[0].__is_abstract__:
            raise ValueError(
                f"{cls.__name__} is abstract but extends non-abstract {cls.__bases__[0].__name__}"
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
            hasattr(base, "__declaration__") and base.__declaration__.is_final
            for base in cls.__bases__
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
    stability: ObjectStability = ObjectStability.DYNAMIC,
    tags: tuple["TagDeclaration", ...] = (),
    enum_types: tuple[EnumType, ...] = (),
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
            enum_types=enum_types,
            event_types=event_types,
        )
        return cls

    return decorate


@declare_handle(HandleType.HANDLE, is_abstract=True)
class Handle(Object):
    """A Handle is a (runtime-only) Object for interacting with the runtime."""

    # meta
    metakind: ClassVar[ObjectKind] = ObjectKind.HANDLE
    metatype: ClassVar[HandleType]
    __declaration__: ClassVar["HandleDeclaration"]
    __definition__: ClassVar["HandleDefinition"]

    def __eq__(self, other: Any):
        """Equals the Handle contents."""
        raise NotImplementedError  # generated

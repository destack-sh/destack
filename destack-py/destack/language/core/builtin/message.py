from typing import (
    TYPE_CHECKING,
    cast,
    dataclass_transform,
)

from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

from .builtin import ObjectStability, StructType
from .declaration import TagDeclaration
from .property import _PROPERTY_SPECIFIERS
from .struct import StructFrozen, _process_struct_cls

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false

logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type

_ALLOWED_POSTFIXES = ("MESSAGE", "REQUEST", "RESPONSE")


@dataclass_transform(
    kw_only_default=True,
    field_specifiers=_PROPERTY_SPECIFIERS,
    frozen_default=True,
)
def builtin_message(
    message_type: StructType,
    *,
    is_abstract: bool = False,
    is_final: bool = False,
    stability: ObjectStability = ObjectStability.DYNAMIC,
    tags: tuple["TagDeclaration", ...] = (),
):
    """Register a class as a concrete Message for the given Message type."""

    def decorate(cls: type) -> type:
        cls = _process_struct_cls(
            cls=cast(type["StructFrozen"], cls),
            struct_type=message_type,
            stability=stability,
            is_frozen=True,
            is_abstract=is_abstract,
            is_final=is_final,
            tags=tags,
        )
        assert (
            message_type == StructType.MESSAGE or StructType.MESSAGE in cls.__declaration__.inherits
        ), f"Message {cls.__name__} must inherit from Message"
        assert any(message_type.name.endswith(suffix) for suffix in _ALLOWED_POSTFIXES), (
            f"Message {cls.__name__} must end with one of {_ALLOWED_POSTFIXES}"
        )
        return cls

    return decorate


@builtin_message(StructType.MESSAGE, is_abstract=True)
class Message(StructFrozen):
    """A Message is an object containing data for communication with Actions."""

    ...

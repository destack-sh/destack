from datetime import datetime
from typing import TYPE_CHECKING, cast, dataclass_transform

from ..utility import UUID
from .builtin import EnumType, ObjectStability, StructType
from .declaration import TagDeclaration
from .hoisted import ValueFactory
from .property import _PROPERTY_SPECIFIERS, declare_property
from .struct import StructFrozen, _process_struct_cls
from .types import UInt128

if TYPE_CHECKING:
    from destack import Client


type_ = type

_ALLOWED_POSTFIXES = ("MESSAGE", "REQUEST", "RESPONSE")


@dataclass_transform(
    kw_only_default=True,
    field_specifiers=_PROPERTY_SPECIFIERS,
    frozen_default=True,
)
def declare_message(
    message_type: StructType,
    *,
    is_abstract: bool = False,
    is_final: bool = False,
    stability: ObjectStability = ObjectStability.DYNAMIC,
    tags: tuple["TagDeclaration", ...] = (),
    enum_types: tuple[EnumType, ...] = (),
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
            enum_types=enum_types,
        )

        # validate
        assert (
            message_type == StructType.MESSAGE or StructType.MESSAGE in cls.__declaration__.inherits
        ), f"Message {cls.__name__} must inherit from Message"
        assert any(message_type.name.endswith(suffix) for suffix in _ALLOWED_POSTFIXES), (
            f"Message {cls.__name__} must end with one of {_ALLOWED_POSTFIXES}"
        )

        return cls

    return decorate


@declare_message(
    StructType.MESSAGE,
    is_abstract=True,
    tags=(
        TagDeclaration(id=20, name="identity", description="Message identity"),
        TagDeclaration(id=21, name="tracking", description="Message tracking"),
    ),
)
class Message(StructFrozen):
    """
    A Message contains data for communicating with Nodes via Actions.

    Because Message are as-is provided by Clients, they only contain client-authority data.
    """

    # 1-20: Message identity
    id: UUID = declare_property(
        2,
        default_factory=ValueFactory.UUID7,
        description="The universally unique identifier of this Message.",
        tags=("identity",),
    )

    # 20-40: Message tracking
    client: "Client" = declare_property(
        23,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.CLIENT,
        description="The Client that created this Message (client, but verified).",
        tags=("tracking",),
    )
    client_nonce: UUID = declare_property(
        24,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.CLIENT_NONCE,
        description="The nonce of the Client that created this Message (client).",
        tags=("tracking",),
    )
    client_created_at: datetime = declare_property(
        25,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.NOW,
        description="The time in the Client when it created this Message (client).",
        tags=("tracking",),
    )
    client_remote_epoch: UInt128 = declare_property(
        26,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.REMOTE_EPOCH,
        description="The logical time last seen from the system in the Client for this space (client).",
        tags=("tracking",),
    )
    client_local_epoch: UInt128 = declare_property(
        27,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.LOCAL_EPOCH,
        description="The logical time in the Client when it created this Message (client).",
        tags=("tracking",),
    )

from datetime import datetime
from typing import TYPE_CHECKING, cast, dataclass_transform

from ._hoisted import ValueFactory
from .declaration import TagDeclaration
from .property import _PROPERTY_SPECIFIERS, declare_property
from .struct import Struct, _process_struct_cls
from .types import UInt128
from .universe import ObjectStability, StructType
from .uuid import UUID

if TYPE_CHECKING:
    from destack import Client


type_ = type

_ALLOWED_POSTFIXES = ("ERROR",)


@dataclass_transform(
    kw_only_default=True,
    field_specifiers=_PROPERTY_SPECIFIERS,
)
def declare_error(
    # meta
    error_type: StructType,
    *,
    is_abstract: bool = False,
    is_final: bool = False,
    stability: ObjectStability = ObjectStability.DYNAMIC,
    # associations
    tags: tuple["TagDeclaration", ...] = (),
):
    """Register a class as a concrete Error for the given Error type."""

    def decorate(cls: type) -> type:
        cls = _process_struct_cls(
            # meta
            cls=cast(type["Struct"], cls),
            struct_type=error_type,
            stability=stability,
            is_immutable=True,
            is_abstract=is_abstract,
            is_final=is_final,
            is_interned=False,
            # associations
            tags=tags,
            into_node_types=(),
        )

        # validate
        assert error_type == StructType.ERROR or StructType.ERROR in cls.__declaration__.inherits, (
            f"Error {cls.__name__} must inherit from Error"
        )
        assert any(error_type.name.endswith(suffix) for suffix in _ALLOWED_POSTFIXES), (
            f"Error {cls.__name__} must end with one of {_ALLOWED_POSTFIXES}"
        )

        return cls

    return decorate


@declare_error(
    StructType.ERROR,
    is_abstract=True,
    tags=(
        TagDeclaration(id=20, name="identity", description="Error identity"),
        TagDeclaration(id=21, name="tracking", description="Error tracking"),
    ),
)
class Error(Struct, Exception):
    """
    An Error is a structured error message.

    Errors are used to communicate failure states.
    Like Messages, Errors are as-is provided by Clients and tagged with client-authority tracking.
    """

    # 1-20: Error identity
    id: UUID = declare_property(
        2,
        default_factory=ValueFactory.UUID7,
        description="The universally unique identifier of this Error.",
        tags=("identity",),
    )

    # 20-40: Error tracking
    client: "Client" = declare_property(
        23,
        is_managed=True,
        is_readonly=True,
        default_factory=ValueFactory.CLIENT,
        description="The Client that created this Error (client, but verified).",
        tags=("tracking",),
    )
    client_nonce: UUID = declare_property(
        24,
        is_managed=True,
        is_readonly=True,
        default_factory=ValueFactory.CLIENT_NONCE,
        description="The nonce of the Client that created this Error (client).",
        tags=("tracking",),
    )
    client_created_at: datetime = declare_property(
        25,
        is_managed=True,
        is_readonly=True,
        default_factory=ValueFactory.NOW,
        description="The time in the Client when it created this Error (client).",
        tags=("tracking",),
    )
    client_remote_epoch: UInt128 = declare_property(
        26,
        is_managed=True,
        is_readonly=True,
        default_factory=ValueFactory.REMOTE_EPOCH,
        description="The logical time last seen from the system in the Client for this space (client).",
        tags=("tracking",),
    )
    client_local_epoch: UInt128 = declare_property(
        27,
        is_managed=True,
        is_readonly=True,
        default_factory=ValueFactory.LOCAL_EPOCH,
        description="The logical time in the Client when it created this Error (client).",
        tags=("tracking",),
    )

    description: str | None = declare_property(103, is_repr=True)

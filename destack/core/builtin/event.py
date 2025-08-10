from datetime import datetime
from typing import TYPE_CHECKING, Optional, dataclass_transform

from .declaration import TagDeclaration, declare_method
from .entity import Entity
from .enum import OptionEnum, declare_enum, declare_option
from .node import Node, _process_node_cls
from .property import _PROPERTY_SPECIFIERS, ValueFactory, declare_property
from .types import UInt128
from .universe import EnumType, NodeType, StructType
from .uuid import UUID

if TYPE_CHECKING:
    from destack import Client, NodeReference


@declare_enum(EnumType.EVENT_STATUS)
class EventStatus(OptionEnum):
    """The (forever) status of an Event."""

    # client
    PENDING = declare_option(1, description="Pending application on client")
    STAGED = declare_option(2, description="Optimistically staged on client")
    # PREDICTED, SUPERSEDED, ...
    # system hot
    APPROVED = declare_option(10, description="Successfully applied in system")
    SKIPPED = declare_option(11, description="Skipped and ignored in system")
    FAILED = declare_option(12, description="Could not apply in system")
    REJECTED = declare_option(13, description="Denied by the system")
    # system cold
    # COMPACTED, ...


@dataclass_transform(
    kw_only_default=True,
    field_specifiers=_PROPERTY_SPECIFIERS,
)
def declare_event(
    # meta
    event_type: NodeType,
    *,
    is_abstract: bool = False,
    is_final: bool = False,
    # associations
    tags: tuple["TagDeclaration", ...] = (),
    event_types: tuple[NodeType, ...] = (),
    enum_types: tuple[EnumType, ...] = (),
    message_types: tuple[StructType, ...] = (),
):
    """Register a class as a concrete Event for the given Event type."""

    def decorate(cls: type) -> type:
        cls = _process_node_cls(
            # meta
            cls=cls,
            node_type=event_type,
            is_abstract=is_abstract,
            is_final=is_final,
            is_singleton=False,
            is_immutable=True,
            # inheritance
            traits=(),
            # content
            indexes=(),
            constraints=(),
            permissions=(),
            tags=tags,
            # tree
            expected_parent_types=(),
            expected_child_types=(),
            expected_ancestor_types=(),
            expected_descendant_types=(),
            # associations
            event_types=event_types,
            enum_types=enum_types,
            message_types=message_types,
            base_struct_type=None,
        )

        # validate
        assert event_type.name.endswith("EVENT"), f"Event {cls.__name__} must end with 'EVENT'"
        assert event_type == NodeType.EVENT or NodeType.EVENT in cls.__declaration__.inherits, (
            f"Event {cls.__name__} must inherit from Event"
        )

        return cls

    return decorate


@declare_event(
    NodeType.EVENT,
    is_abstract=True,
    tags=(
        TagDeclaration(id=20, name="system", description="System authority"),
        TagDeclaration(id=21, name="client", description="Client authority"),
    ),
)
class Event(Node):
    """
    An Event is an immutable* datum of something happening to an Entity.

    Events are proposed by Clients, then approved or discarded by the system.
    All Events are stored in an append-only immutable* log, even rejected ones.

    The client_* data is as-is provided by Clients, and can not be verified by the system.

    *=immutable except for Event.status, which is set by the authoritative system
       and propagated to all Clients (who then restate their own Events and derived Entities).
    """

    # 10-20: Event identity
    definition: Optional["Entity"] = declare_property(
        10,
        is_internal=True,
        is_readonly=True,
        is_identity=True,
        description="The definition this Event is an instance of.",
        tags=("identity",),
    )

    # 20-40: Event tracking
    created_at: datetime = declare_property(
        20,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_readonly=True,
        default_factory=ValueFactory.NOW,
        description="The time this Event was created (system).",
        tags=("tracking", "system"),
    )
    created_epoch: UInt128 = declare_property(
        21,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_repr=True,
        is_readonly=True,
        default_factory=ValueFactory.REMOTE_EPOCH,
        description="The logical time this Event was created (system).",
        tags=("tracking", "system"),
    )
    created_by: "Entity" = declare_property(
        22,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_repr=False,
        is_readonly=True,
        default_factory=ValueFactory.ACTOR,
        description="The Actor that created this Event.",
        tags=("tracking",),
    )
    client: "Client" = declare_property(
        23,
        is_internal=True,
        is_eq=False,
        is_hash=False,
        is_repr=True,
        is_readonly=True,
        default_factory=ValueFactory.CLIENT,
        description="The Client that created this Event (client, but verified).",
        tags=("tracking", "client"),
    )
    client_nonce: UUID = declare_property(
        24,
        is_eq=False,
        is_hash=False,
        is_repr=False,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.CLIENT_NONCE,
        description="The nonce of the Client that created this Event (client).",
        tags=("tracking", "client"),
    )
    client_created_at: datetime = declare_property(
        25,
        is_eq=False,
        is_hash=False,
        is_repr=False,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.NOW,
        description="The time in the Client when it created this Event (client).",
        tags=("tracking", "client"),
    )
    client_remote_epoch: UInt128 = declare_property(
        26,
        is_eq=False,
        is_hash=False,
        is_repr=True,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.REMOTE_EPOCH,
        description="The logical time last seen from the system in the Client for this space (client).",
        tags=("tracking", "client"),
    )
    client_local_epoch: UInt128 = declare_property(
        27,
        is_eq=False,
        is_hash=False,
        is_repr=True,
        is_internal=True,
        is_readonly=True,
        default_factory=ValueFactory.LOCAL_EPOCH,
        description="The logical time in the Client when it created this Event (client).",
        tags=("tracking", "client"),
    )
    status: "EventStatus" = declare_property(
        30,
        is_eq=False,
        is_hash=False,
        is_repr=True,
        default=EventStatus.PENDING,
        description="The status of the Event (system).",
        tags=("tracking", "system"),
    )
    caused_by: Optional["Event"] = declare_property(
        31,
        is_eq=False,
        is_hash=False,
        is_readonly=True,
        is_internal=True,
        is_identity=True,
        description="The Event that caused this Event (if any).",
        tags=("identity",),
    )
    # 100+: content

    @declare_method(2)
    def to_ref(self) -> "NodeReference":
        """Gets a reference to this Node."""
        raise NotImplementedError

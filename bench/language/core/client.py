from datetime import datetime
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core.const import ClientType, NodeType
from bench.language.core.node import BenchNode, ClientOrigin, node_
from bench.language.core.property import p_kernel, p_node_parent, p_regular, p_system
from bench.language.core.validation import TITLE_CONSTRAINT
from bench.proto.wire import ClientData

if TYPE_CHECKING:
    from bench.language import (
        Bench,
        Machine,
        Space,
        User,
    )

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.CLIENT, roots=(NodeType.USER, NodeType.BENCH))
class Client(BenchNode[ClientData]):
    """A client to a Bench."""

    parent: Union["User", "Bench", None] = p_node_parent(4, NodeType.USER, NodeType.BENCH)
    type: ClientType = p_regular(30)
    name: str = p_regular(32, constraint=TITLE_CONSTRAINT)

    device_type: Optional[str] = p_regular(40, default=None)
    device_name: Optional[str] = p_regular(41, default=None)
    operating_system: Optional[str] = p_regular(42, default=None)
    browser_name: Optional[str] = p_regular(43, default=None)
    browser_version: Optional[str] = p_regular(44, default=None)
    place_id: Optional[str] = p_regular(45, default=None)

    access_token: Optional[str] = p_kernel(
        50, default=None, defer=True, unique=True, sensitive=True
    )
    seen_at: Optional[datetime] = p_system(51, default=None)
    logged_in_at: Optional[datetime] = p_system(52, default=None)

    space: Optional["Space"] = p_system(
        60, array=False, require=False, references=NodeType.SPACE, fk=True
    )
    machine: Optional["Machine"] = p_system(
        62, array=False, require=False, references=NodeType.MACHINE, fk=True
    )

    @property
    def is_attached(self) -> bool:
        parent = self.parent
        if parent is None:
            return False
        return parent.is_attached

    def __content_str__(self) -> str:
        value_parts = []
        for prop in (
            "device_type",
            "device_name",
            "operating_system",
            "browser_name",
            "browser_version",
        ):
            value = getattr(self, prop)
            if value is not None:
                value_parts.append(value)
        return ", ".join(value_parts)

    def to_origin(self, *, nonce: UUID | None) -> ClientOrigin:
        return ClientOrigin(type=self.type, id=self.id, nonce=nonce or self.id)

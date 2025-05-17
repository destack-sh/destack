from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    NodeReference,
    Struct,
    StructType,
    new_struct_id,
    p_system,
    struct_,
)

if TYPE_CHECKING:
    from bench.language import Bench, Client, Computer, Organization, User


@struct_(StructType.POLICY_SUBJECT)
class PolicySubject(Struct):
    """
    TODO :Architecture: remove legacy PolicySubject, implement new access control system
    The <whoever/whatever> issuing a request. Unknown/ignored attributes are unset.
    (We unset various combinations of attributes to evaluate the access of acting subjects independently.)
    """

    id: int = p_system(2, default_factory=new_struct_id)

    # flags
    is_authenticated: Optional[bool] = p_system(30, default=None)
    is_staff: Optional[bool] = p_system(31, default=None)
    is_system: Optional[bool] = p_system(32, default=None)
    # (Client isn't a separate subject but useful to know)

    # who
    client: Optional["Client"] = p_system(40)
    user: Optional["User"] = p_system(41)
    computer: Optional["Computer"] = p_system(43)
    if TYPE_CHECKING:
        client_ptr: Optional[NodeReference] = None
        client_id: Optional[UUID] = None
        user_ptr: Optional[NodeReference] = None
        user_id: Optional[UUID] = None
        computer_ptr: Optional[NodeReference] = None
        computer_id: Optional[UUID] = None

    # accessories
    owned: list[Union["User", "Bench", "Organization"]] = p_system(52)
    memberships: list[Union["Bench", "Organization"]] = p_system(53)

    def __content_str__(self):
        content_parts = []
        if self.is_authenticated:
            content_parts.append("is_authenticated")
        else:
            content_parts.append("is_anonymous")
        if self.is_staff:
            content_parts.append("is_staff")
        if self.client:
            content_parts.append(f"client={self.client!r}")
        elif self.user:
            content_parts.append(f"user={self.user!r}")
        return ", ".join(content_parts)

    @property
    def is_anonymous(self) -> bool:
        return not self.is_authenticated and not self.is_staff

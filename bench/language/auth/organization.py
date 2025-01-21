from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    NAME_CONSTRAINT,
    EnumType,
    LocalNodeList,
    Node,
    NodeType,
    Region,
    StructType,
    enum_,
    node_,
    p_node_children,
    p_regular,
    p_system,
)
from bench.pb2 import OrganizationData
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Bench, Handle, Icon, Text

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.ORGANIZATION_STATUS)
class OrganizationStatus(IdEnum):
    # NOTE UserStatus/OrganizationStatus ids for same statuses should match
    REGISTERED = 4  # created org
    ACTIVATED = 10  # has main bench


@node_(NodeType.ORGANIZATION, roots=())
class Organization(Node[OrganizationData]):
    """
    A Bench organization with Users as members.
    Until activation only its creator has access.
    """

    main_handle: Optional["Handle"] = p_system(
        31, require=False, array=False, references=NodeType.HANDLE, fk=True
    )  # not actually optional but Handle.parent = Organization
    handles: LocalNodeList["Handle"] = p_node_children(NodeType.HANDLE)
    slug: Optional[str] = p_system(32, unique=True)  # must match main handle
    name: str = p_regular(33, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(34, default=None, struct=StructType.TEXT)
    icon: Optional["Icon"] = p_regular(35, default=None, struct=StructType.ICON)
    main_bench: Optional["Bench"] = p_system(
        36, array=False, require=False, references=NodeType.BENCH, fk=True
    )
    region: "Region" = p_system(37, require=True)
    status: OrganizationStatus = p_system(38)

    # flags
    # ...

    @property
    def bench(self) -> "Bench":
        assert self.main_bench is not None, f"{self!r} is not activated"
        return self.main_bench

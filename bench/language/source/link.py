from datetime import timedelta
from typing import TYPE_CHECKING, Optional, Union, assert_never
from uuid import UUID

from bench.language.core import (
    NAME_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    IsComputable,
    IsModal,
    IsTemplatable,
    NodeType,
    PackageNode,
    RunStatus,
    RunType,
    StructType,
    TypeBase,
    enum_,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
)
from bench.pb2 import LinkData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Color,
        Flow,
        NodeReference,
        RunOptions,
        Text,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PORT_SIDE)
class PortSide(BuiltinEnum):
    INCOMING = 1
    OUTGOING = 2


@enum_(EnumType.LINK_TYPE)
class LinkType(BuiltinEnum):
    DECIDE = 10, "Decide", "Determine when and how to call", "far fa-shuffle"
    REQUIRE = 20, "Require", "Determine how to call", "fas fa-arrow-right-long"
    # MESSAGE? WAIT? STREAM?


@enum_(EnumType.LINK_TRIGGER)
class LinkTrigger(BuiltinEnum):
    ON_COMPLETED = 1, "On completed", "If the action succeeds", "fas fa-check"
    ON_FAILED = 2, "On failed", "If the action fails", "fas fa-xmark"
    ON_TERMINATED = 3, "On terminated", "Always, success or failure", "fas fa-check-double"


SIGN_BY_LINK_TYPE: dict[LinkType, str] = {
    LinkType.DECIDE: "-*>",
    LinkType.REQUIRE: "-=>",
}
LINK_TYPES_BY_SIGN: dict[str, LinkType] = {v: k for k, v in SIGN_BY_LINK_TYPE.items()}


@node_(NodeType.LINK, has_subtypes=True)
class Link(IsTemplatable, IsModal, IsComputable, PackageNode[LinkData]):
    """
    A Link between Actions in a Flow (source = outgoing, target = incoming).
    """

    parent: Union["Flow", "Action", None] = p_node_parent(4, NodeType.FLOW, NodeType.ACTION)

    # meta
    type: LinkType = p_internal(30)
    name: str | None = p_regular(32, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    text: Optional["Text"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.TEXT
    )
    source: "Action" = p_regular(35, require=True, references=NodeType.ACTION, ckless=True)
    target: "Action" = p_regular(36, require=True, references=NodeType.ACTION, ckless=True)
    if TYPE_CHECKING:
        source_ptr: Optional[NodeReference] = None
        source_id: Optional[UUID] = None
        target_ptr: Optional[NodeReference] = None
        target_id: Optional[UUID] = None
    options: Optional["RunOptions"] = p_regular(
        39, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )

    # trigger
    trigger: LinkTrigger = p_regular(40, default=LinkTrigger.ON_COMPLETED)

    # modulation
    delay: Optional[timedelta] = p_regular(50, default=None)

    # flags
    is_manual: bool = p_regular(
        60, default=False, description="Whether to link this automatically."
    )
    # is_automap? (dynamically generate inputs?)
    # is_streaming: bool = p_regular(80, default=False)

    # flow
    color: Optional["Color"] = p_regular(
        80, default=None, require=False, array=False, struct=StructType.COLOR
    )

    def __content_str__(self) -> str:
        sign = SIGN_BY_LINK_TYPE.get(self.type, "???")
        source = self.source
        target = self.target
        return f"{source.absolute_path if source else '???'} {sign} {target.absolute_path if target else '???'}"

    @property
    def run_type(self) -> RunType:
        return RunType.LINK

    @property
    def flow(self) -> "Flow | None":
        """Gets the containing ancestor Flow (if any)"""
        from bench.language import Flow

        parent = self.parent
        while parent is not None:
            if isinstance(parent, Flow):
                return parent
        return None

    def to_type_maybe(self) -> "TypeBase | None":
        return None

    @property
    def resource_type(self) -> "TypeBase | None":
        return None  # Links don't have resources (?)

    @property
    def input_type(self) -> "TypeBase | None":
        return None  # Links don't have inputs (?)

    @property
    def output_type(self) -> "TypeBase | None":
        return None  # Links don't have outputs (?)

    def is_triggered_by(self, status: RunStatus):
        """Whether this Link is triggered by the given status."""
        if self.trigger == LinkTrigger.ON_COMPLETED:
            return status == RunStatus.COMPLETED
        elif self.trigger == LinkTrigger.ON_FAILED:
            return status == RunStatus.FAILED
        elif self.trigger == LinkTrigger.ON_TERMINATED:
            return status.is_terminal
        else:
            assert_never(self.trigger)

    @staticmethod
    def new(type: LinkType, name: str, **kwargs) -> "Link":
        return Link(type=type, name=name, **kwargs)

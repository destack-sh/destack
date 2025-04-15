from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    NAME_CONSTRAINT,
    BuiltinEnum,
    EnumType,
    IsComputable,
    IsModal,
    IsRunnable,
    IsTemplatable,
    IsType,
    NodeType,
    PackageNode,
    RunType,
    StructType,
    enum_,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
)
from bench.pb2 import TransitionData
from bench.utils.fractional import INTEGER_ZERO

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Claim,
        Color,
        Flow,
        NodeReference,
        Text,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PORT_SIDE)
class PortSide(BuiltinEnum):
    INCOMING = 1
    OUTGOING = 2


@enum_(EnumType.TRANSITION_TYPE)
class TransitionType(BuiltinEnum):
    MANUAL = 10, "Manual", "Manually triggered", "fas fa-link"
    DECIDE = 20, "Decide", "Determine when and how to call", "far fa-shuffle"
    REQUIRE = 30, "Require", "Determine how to call", "fas fa-arrow-right-long"
    # MESSAGE? WAIT? STREAM?


SIGN_BY_LINK_TYPE: dict[TransitionType, str] = {
    TransitionType.MANUAL: "-!>",
    TransitionType.DECIDE: "-*>",
    TransitionType.REQUIRE: "-=>",
}
LINK_TYPES_BY_SIGN: dict[str, TransitionType] = {v: k for k, v in SIGN_BY_LINK_TYPE.items()}


@node_(NodeType.TRANSITION)
class Transition(
    IsTemplatable,
    IsModal,
    IsComputable,
    IsRunnable,
    PackageNode[TransitionData],
):
    """
    A Transition between nodes in a Flow (source = outgoing, target = incoming).
    NOTE :Architecture: maybe add IsTransitionable trait?
    """

    parent: Union["Flow", None] = p_node_parent(4, NodeType.FLOW)

    # meta
    type: TransitionType = p_internal(30)
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

    # modulation
    color: Optional["Color"] = p_regular(
        60, default=None, require=False, array=False, struct=StructType.COLOR
    )
    # is_automap? (dynamically generate inputs?)
    # is_streaming: bool = p_regular(80, default=False)

    def __content_str__(self) -> str:
        sign = SIGN_BY_LINK_TYPE.get(self.type, "???")
        source = self.source
        target = self.target
        return f"{source.absolute_path if source else '???'} {sign} {target.absolute_path if target else '???'}"

    @property
    def run_type(self) -> RunType:
        return RunType.TRANSITION

    @property
    def flow(self) -> "Flow | None":
        """Gets the containing ancestor Flow (if any)"""
        from bench.language import Flow

        parent = self.parent
        while parent is not None:
            if isinstance(parent, Flow):
                return parent
        return None

    @property
    def claims(self) -> tuple["Claim", ...]:
        return ()

    def to_type_maybe(self) -> "IsType | None":
        return None

    @property
    def resource_type(self) -> "IsType | None":
        return None  # Links don't have resources (?)

    @property
    def input_type(self) -> "IsType | None":
        return None  # Links don't have inputs (?)

    @property
    def output_type(self) -> "IsType | None":
        return None  # Links don't have outputs (?)

    @staticmethod
    def new(type: TransitionType, name: str, **kwargs) -> "Transition":
        return Transition(type=type, name=name, **kwargs)

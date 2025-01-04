from datetime import timedelta
from typing import TYPE_CHECKING, Optional, Union
from uuid import UUID

from bench.language.core import (
    NAME_CONSTRAINT,
    EnumType,
    FieldType,
    NodeType,
    RunType,
    SourceNode,
    StructType,
    enum_,
    node_,
    node_subtype_,
    p_internal,
    p_node_parent,
    p_regular,
)
from bench.proto.wire import PipeData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import IdEnum

from .field import TypeBase, TypeConstraint

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Block,
        Color,
        Expression,
        NodeReference,
        RunOptions,
        Text,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PORT_SIDE)
class PortSide(IdEnum):
    INCOMING = 1
    OUTGOING = 2


FIELD_ZONES_BY_SIDE: dict[PortSide, tuple[FieldType, ...]] = {
    PortSide.INCOMING: (FieldType.VARIABLE, FieldType.INPUT),
    PortSide.OUTGOING: (FieldType.OUTPUT,),
}


@enum_(EnumType.PIPE_TYPE)
class PipeType(IdEnum):
    FORWARD = 1, "Always go forward"
    FORWARD_AND_BACK = 2, "Always go forward, then back"
    SELECT = 10, "Maybe go forward"
    SELECT_AND_BACK = 11, "Maybe go forward, then back"


SIGN_BY_PIPE_TYPE: dict[PipeType, str] = {
    PipeType.FORWARD: "->",
    PipeType.FORWARD_AND_BACK: "<->",
    PipeType.SELECT: "-?>",
    PipeType.SELECT_AND_BACK: "<-?>",
}
PIPE_TYPES_BY_SIGN: dict[str, PipeType] = {v: k for k, v in SIGN_BY_PIPE_TYPE.items()}


@node_(NodeType.PIPE)
class Pipe(SourceNode[PipeData]):
    """
    A connection between two Steps in a Flow (source = outgoing, target = incoming).
    Pipes are stored in the containing Flow or containing Step.
    """

    parent: Union["Block", "Action", None] = p_node_parent(4, NodeType.BLOCK, NodeType.ACTION)

    # connection
    type: PipeType = p_internal(30)
    name: str = p_regular(32, constraint=NAME_CONSTRAINT)
    order_key: str = p_internal(33, default=INTEGER_ZERO)
    text: Optional["Text"] = p_regular(
        34, default=None, require=False, array=False, struct=StructType.TEXT
    )
    source: "Action" = p_regular(35, require=True, references=NodeType.ACTION)
    target: "Action" = p_regular(36, require=True, references=NodeType.ACTION)
    if TYPE_CHECKING:
        source_ptr: Optional[NodeReference] = None
        source_id: Optional[UUID] = None
        source_ck: Optional[str] = None
        target_ptr: Optional[NodeReference] = None
        target_id: Optional[UUID] = None
        target_ck: Optional[str] = None

    # associated_fields, ...?

    # run
    run_options: Optional["RunOptions"] = p_regular(
        50, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )

    # filter
    condition: Optional["Expression"] = p_regular(
        60, default=None, require=False, array=False, struct=StructType.EXPRESSION
    )
    constraint: Optional["TypeConstraint"] = p_regular(
        62, default=None, array=False, struct=StructType.TYPE_CONSTRAINT
    )

    # modulation
    delay: Optional[timedelta] = p_regular(80, default=None)

    # view
    color: Optional["Color"] = p_regular(
        91, default=None, require=False, array=False, struct=StructType.COLOR
    )
    is_hidden: bool = p_regular(92, default=False)
    is_name_shown: bool = p_regular(93, default=False)

    def __content_str__(self) -> str:
        sign = SIGN_BY_PIPE_TYPE.get(self.type, "???")
        source = self.source
        target = self.target
        return f"{source.absolute_path if source else '???'} {sign} {target.absolute_path if target else '???'}"

    @property
    def run_type(self) -> RunType:
        return RunType.PIPE

    @property
    def block(self) -> "Block | None":
        """Gets the containing ancestor Block (if any)"""
        from bench.language.source.block import Block

        parent = self.parent
        while parent is not None:
            if isinstance(parent, Block):
                return parent
            parent = parent.parent
        return None

    @property
    def variable_type(self) -> "TypeBase | None":
        return None  # Pipes don't have variables (?)

    @property
    def input_type(self) -> "TypeBase | None":
        return None  # Pipes don't have inputs (?)

    @property
    def output_type(self) -> "TypeBase | None":
        return None  # Pipes don't have outputs (?)

    @staticmethod
    def new(type: PipeType, name: str, **kwargs) -> "Pipe":
        return Pipe(type=type, name=name, **kwargs)


@node_subtype_(PipeType.SELECT)
class SelectPipe(Pipe):
    text: Optional["Text"] = p_regular(
        100, default=None, require=False, array=False, struct=StructType.TEXT
    )

from datetime import timedelta
from typing import TYPE_CHECKING, Optional, Union, assert_never
from uuid import UUID

from bench.language.core import (
    NAME_CONSTRAINT,
    EnumType,
    NodeType,
    RunType,
    SourceNode,
    StructType,
    TypeBase,
    enum_,
    node_,
    p_internal,
    p_node_parent,
    p_regular,
    subnode_,
)
from bench.language.core.const import RunStatus
from bench.pb2 import PipeData
from bench.utils.fractional import INTEGER_ZERO
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import (
        Action,
        Block,
        Color,
        NodeReference,
        RunOptions,
        Text,
    )

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.PORT_SIDE)
class PortSide(IdEnum):
    INCOMING = 1
    OUTGOING = 2


@enum_(EnumType.PIPE_TYPE)
class PipeType(IdEnum):
    CALL = 1, "Always call"
    # FAIL?
    SELECT = 10, "Call only if selected"
    # MESSAGE?
    # WAIT?
    # STREAM?


@enum_(EnumType.PIPE_TRIGGER)
class PipeTrigger(IdEnum):
    ON_COMPLETED = 1, "If the action succeeds"
    ON_FAILED = 2, "If the action fails"
    ON_TERMINATED = 3, "Always, success or failure"


SIGN_BY_PIPE_TYPE: dict[PipeType, str] = {
    PipeType.CALL: "->",
    PipeType.SELECT: "-?>",
}
PIPE_TYPES_BY_SIGN: dict[str, PipeType] = {v: k for k, v in SIGN_BY_PIPE_TYPE.items()}


@node_(NodeType.PIPE, has_subtypes=True)
class Pipe(SourceNode[PipeData]):
    """
    A connection between two Steps in a Flow (source = outgoing, target = incoming).
    Pipes are stored in the containing Flow or containing Step.
    """

    parent: Union["Block", "Action", None] = p_node_parent(4, NodeType.BLOCK, NodeType.ACTION)

    # meta
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
    run_options: Optional["RunOptions"] = p_regular(
        39, default=None, require=False, array=False, struct=StructType.RUN_OPTIONS
    )

    # trigger
    trigger: PipeTrigger = p_regular(40, default=PipeTrigger.ON_COMPLETED)

    # modulation
    delay: Optional[timedelta] = p_regular(50, default=None)

    # flags
    # is_automap? (dynamically generate inputs?)
    # is_streaming: bool = p_regular(80, default=False)

    # flow
    color: Optional["Color"] = p_regular(
        80, default=None, require=False, array=False, struct=StructType.COLOR
    )

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

    def is_triggered_by(self, status: RunStatus):
        """Whether this Pipe is triggered by the given status."""
        if self.trigger == PipeTrigger.ON_COMPLETED:
            return status == RunStatus.COMPLETED
        elif self.trigger == PipeTrigger.ON_FAILED:
            return status == RunStatus.FAILED
        elif self.trigger == PipeTrigger.ON_TERMINATED:
            return status.is_terminal
        else:
            assert_never(self.trigger)

    @staticmethod
    def new(type: PipeType, name: str, **kwargs) -> "Pipe":
        return Pipe(type=type, name=name, **kwargs)


@subnode_(PipeType.SELECT)
class SelectPipe(Pipe):
    text: Optional["Text"] = p_regular(
        100, default=None, require=False, array=False, struct=StructType.TEXT
    )

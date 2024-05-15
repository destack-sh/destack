from typing import TYPE_CHECKING, Any, Collection, Optional

import structlog

from bench.language.const import EnumType, NodeType, StructType, enum_
from bench.language.node import Node, node
from bench.language.property import (
    Property,
    p_internal,
    p_node_parent,
    p_system,
    p_value_packed,
    p_value_runtime,
)
from bench.language.step import Step
from bench.language.text import Text
from bench.language.validation import ValidationHandler
from bench.language.value import HasValues
from bench.proto.wire import NodeReferenceData
from bench.utils.func import IdEnum
from bench.utils.uuidt import UUIDT

if TYPE_CHECKING:
    from bench.language import Block, Package, Request, Run, Session

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@enum_(EnumType.LOG_KIND)
class LogKind(IdEnum):
    MESSAGE = 1
    ACCESS = 2


@enum_(EnumType.LOG_LEVEL)
class LogLevel(IdEnum):
    TRACE = 1
    DEBUG = 2
    INFO = 3
    WARNING = 4
    ERROR = 5
    CRITICAL = 6


@node(NodeType.LOG, stored=True, local=True, no_ck=True, id_factory=UUIDT)
class Log(Node, HasValues):
    """
    A Log (entry) is a timestamped event of something happening:
     an unstructured message, an 'event', a Request / an Access (read, edit, use), ...
    """

    parent: "Package" = p_node_parent(4, NodeType.PACKAGE)

    # content
    kind: LogKind = p_system(30)
    level: LogLevel = p_system(31)
    logger: Optional[str] = p_system(32, default=None)
    event: Optional[str] = p_system(33, default=None)
    title: Optional[str] = p_internal(34, default=None)
    text: Optional[Text] = p_internal(
        35, default=None, require=False, array=False, struct=StructType.TEXT
    )
    value_packed: Any | None = p_value_packed(37)
    # secret_value_packed?
    value: Any = p_value_runtime(37)
    request: Optional["Request"] = p_system(
        39, require=False, array=False, struct=StructType.REQUEST
    )

    # context
    session: Optional["Session"] = p_system(
        40, require=False, array=False, references=NodeType.SESSION
    )
    run: Optional["Run"] = p_system(41, require=False, array=False, references=NodeType.RUN)
    block: Optional["Block"] = p_system(42, require=False, array=False, references=NodeType.BLOCK)
    step: Optional["Step"] = p_system(43, require=False, array=False, references=NodeType.STEP)
    if TYPE_CHECKING:
        session_ptr: Optional[NodeReferenceData] = None
        run_ptr: Optional[NodeReferenceData] = None
        block_ptr: Optional[NodeReferenceData] = None
        step_ptr: Optional[NodeReferenceData] = None

    def __content_str__(self):
        return f"[{self.kind.bench_name}:{self.level.bench_name}] '{self.event or self.title or self.text or '<empty>'}' ({self.created_at})"

    def _validate_component(
        self, properties: Collection[Property], invalid: ValidationHandler
    ) -> None:
        if self.step_ptr is not None and self.block_ptr is None:
            invalid(self, "step without block", (Run.step, Run.block))

from typing import TYPE_CHECKING, Optional, Union

import structlog

from bench.language.core import (
    HasRuntimeContext,
    HasTimeIdentity,
    HasTrace,
    LogType,
    NodeType,
    PackageNode,
    Severity,
    StructType,
    Text,
    p_node_parent,
    p_regular,
    p_system,
    timed_node_,
)
from bench.pb2 import LogData

if TYPE_CHECKING:
    from bench.language import Bench, Run

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)


@timed_node_(NodeType.LOG)
class Log(HasTimeIdentity, PackageNode[LogData], HasRuntimeContext, HasTrace):
    """
    A Log of something happening in a Bench.
    """

    # meta
    parent: Union["Bench", "Run", None] = p_node_parent(4, NodeType.BENCH, NodeType.RUN)
    type: LogType = p_system(30)
    severity: Severity = p_system(31)
    title: Optional[str] = p_regular(32, default=None)
    text: Optional["Text"] = p_regular(33, default=None, array=False, struct=StructType.TEXT)
    # context
    # ...HasRuntimeContext[80-99]

    def __content_str__(self):
        return f"[{self.type.bench_name}:{self.severity.bench_name}] ({self.created_at})"

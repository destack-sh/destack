import asyncio
import hashlib
import sys
import traceback
from copy import deepcopy
from datetime import datetime
from typing import TYPE_CHECKING, Any, Optional, Union
from uuid import UUID

import msgpack
from asgiref.sync import async_to_sync, sync_to_async

from bench.language.const import (
    TERMINAL_RUN_STATUSES,
    BenchError,
    NodeType,
    RunErrorKind,
    RunStatus,
    StructType,
    TriggerType,
)
from bench.language.node import (
    NS,
    Node,
    ScopeNode,
    Struct,
    node,
    node_component,
    p_ancestor,
    p_child,
    p_internal,
    p_parent,
    struct,
)
from bench.language.value import HasValue
from bench.sql.core import PrimitiveType
from bench.utils.dt import utcnow_with_tz

if TYPE_CHECKING:
    from bench.language import Block, Server, Session


@node_component
class HasRun(Node):
    """A block block"""

    @property
    def _is_async(self) -> Optional[bool]:  # set in supporting components e.g. HasCode
        """Whether this block is async."""
        return None

    @property
    def cache(self):
        from bench.language.cache import Cache

        return Cache(self.package, subkey=self.ck.hex)

    @property
    def current_run(self):
        return self.package.session.current_run

    def _call_inner(self, *args, **kwargs):
        assert (
            self.attached and self._status == NS.TRACKED
        ), f"cannot call {self!r} (status={self._status!r})"
        try:
            asyncio.get_running_loop()
            is_outer_async = True
        except RuntimeError:
            is_outer_async = False
        inner_call = self._call_inner_async if self._is_async else self._call_inner_sync

        if is_outer_async and not self._is_async:
            inner_call = sync_to_async(inner_call)
        elif not is_outer_async and self._is_async:
            inner_call = async_to_sync(inner_call)

        return inner_call(*args, **kwargs)

    def _call_inner_sync(self, *args, **kwargs):
        raise NotImplementedError

    async def _call_inner_async(self, *args, **kwargs):
        raise NotImplementedError


def get_run_cache_subkey(inputs_raw: Any, content_id: Optional[str] = None):
    inputs_bytes = msgpack.packb(inputs_raw, use_bin_type=True)
    input_hash = hashlib.sha256(inputs_bytes).hexdigest()
    if content_id:
        return f"run.{content_id}.{input_hash}"
    else:
        return f"run.{input_hash}"


@node(NodeType.RUN, index_in_os=True, local=True)
class Run(ScopeNode, HasValue):
    """
    A 'run' of a block (in a session).
    """

    # NOTE we don't 'track' runs in sessions yet
    #  (because we don't edit them outside of the source session,
    #   and because it's unclear how run/session edits should interact with 'regular' package edits)

    parent: Union["Session", "Run"] = p_parent(4, NodeType.SESSION, NodeType.RUN)
    session: "Session" = p_ancestor(
        30, NodeType.SESSION, require=True, store=True, wire=True, index_in_pg=True
    )
    root: Optional["Run"] = p_ancestor(
        31,
        NodeType.RUN,
        require=False,
        nearest=False,
        include_self=False,
        store=True,
        wire=True,
        index_in_pg=True,
    )
    server: Optional["Server"] = p_internal(
        32, index_in_pg=True, require=False, array=False, references=NodeType.SERVER
    )
    node: Optional["Block"] = p_internal(
        34, references=NodeType.BLOCK, require=False, array=False, index_in_pg=True
    )
    node_path: Optional[str] = p_internal(35, default=None)
    scheduled_at: Optional[datetime] = p_internal(36, default=None)
    started_at: Optional[datetime] = p_internal(37, default=None)
    terminated_at: Optional[datetime] = p_internal(38, default=None)
    trigger_type: Optional[TriggerType] = p_internal(39, default=None)
    trigger_id: Optional[UUID] = p_internal(40, default=None)
    status: RunStatus = p_internal(42, index_in_pg=True)
    # NOTE ideally we should :GeneralizeHasValue for inputs/outputs as well (not needed yet, see note above)
    inputs_packed: Optional[dict[str, Any]] = p_internal(
        43, default=None, primitive_type=PrimitiveType.JSON
    )
    outputs_packed: Optional[dict[str, Any]] = p_internal(
        44, default=None, primitive_type=PrimitiveType.JSON
    )
    value_packed: Any | None = p_internal(
        45,
        default=None,
        copy=deepcopy,
        primitive_type=PrimitiveType.JSON,
        ignore_conflicts=True,
    )
    error: Optional["RunError"] = p_internal(46, default=None, primitive_type=PrimitiveType.JSON)
    runs: list["Run"] = p_child(NodeType.RUN)

    def __content_str__(self):
        value_keys_str = ", ".join(self.value.keys()) if self.value else ""
        return f"{self.node} ({self.status}, value={value_keys_str or '<none>'}, {self.id})"

    def _mark_dead_if_active(self):
        if self.active:
            self.terminated_at = utcnow_with_tz()
            self.status = RunStatus.ABORTED if self.started_at else RunStatus.CANCELLED

    @property
    def active(self) -> bool:
        return self.status not in TERMINAL_RUN_STATUSES

    @property
    def duration(self) -> float:
        if self.terminated_at is None:
            return 0
        return (self.terminated_at - self.started_at).total_seconds()

    def walk_descendants(self):
        yield self
        for child in self.runs:
            yield from child.walk_descendants()


_IGNORED_PACKAGE_PREFIXES = ("bench.runtime", "bench.bench", "asgiref", "concurrent")
_IGNORED_PACKAGE_PATHS = tuple(package.replace(".", "/") for package in _IGNORED_PACKAGE_PREFIXES)


@struct(StructType.RUN_CODE_FRAME)
class RunCodeFrame(Struct):
    node: Node = p_internal(30, array=False, require=True, references=NodeType.BLOCK)
    lineno: int = p_internal(31)
    name: str = p_internal(32)
    locals: Optional[dict[str, Any]] = p_internal(
        33, default=None, primitive_type=PrimitiveType.JSON
    )
    line: str = p_internal(34)

    @staticmethod
    def clean(
        stack: list["RunCodeFrame"], from_block: "Block", session: "Session"
    ) -> list["RunCodeFrame"]:
        from bench.language.block import Block
        from bench.language.code_ import HasCode

        code_by_method: dict[str, HasCode] = {
            node._transform.method_name: node
            for node in list(session.package._nodes)
            if isinstance(node, Block) and getattr(node, "_transform", None)
        }
        if getattr(from_block, "_transform", None):
            # from block may not be in package (e.g. if detached when running anonymous code)
            code_by_method[from_block._transform.method_name] = from_block

        found_start = False
        cleaned_stack = []
        for frame in stack:
            if any(prefix in frame.node for prefix in _IGNORED_PACKAGE_PATHS):
                continue  # skip support code
            if not found_start:
                # impute bench source info into instantiated code callables
                code = code_by_method.get(frame.name)
                if code is not None:
                    if code == from_block:
                        found_start = True
                    elif not found_start:
                        continue  # ignore
                    frame.node = from_block
                    frame.name = from_block.name or "<unnamed>"
                    frame.lineno = frame.lineno - code._transform.start_offset
                    frame.line = code.code.splitlines()[frame.lineno - 1]
                    frame.locals = frame.locals or {}
                    for ident, var in code._block_references.items():
                        if ident not in frame.locals and var.id in session.package._tree:
                            frame.locals[ident] = repr(session.package._tree[var.id])
            if found_start:
                # trim file path for python packages
                python_version = f"{sys.version_info.major}.{sys.version_info.minor}"
                if python_version in frame.node:
                    frame.node = frame.node.split(python_version)[-1][1:]  # skip slash
                cleaned_stack.append(frame)
        return [f for f in cleaned_stack if f.line]


@struct(StructType.RUN_ERROR)
class RunError(Struct, BenchError):
    kind: RunErrorKind = p_internal(30)
    type: str = p_internal(31)
    message: Optional[str] = p_internal(32, default=None)
    node: Optional["Node"] = p_internal(33, require=False, array=False, references=NodeType.BLOCK)
    traceback: list[RunCodeFrame] = p_internal(
        34, default_factory=list, struct=StructType.RUN_CODE_FRAME
    )

    @staticmethod
    def from_exception(e: BaseException, block: Optional["Block"]) -> "RunError":
        if isinstance(e, RunError):
            return e
        stack = RunCodeFrame.from_stack(traceback.extract_tb(e.__traceback__))
        stack = RunCodeFrame.clean(stack, block, block.session)
        if isinstance(e, SyntaxError):  # ignore (..., line x) because it's not useful
            err_str = e.msg
        else:
            err_str = str(e)
        return RunError(
            kind=RunErrorKind.RUNTIME,
            type=type(e).__name__,
            message=err_str,
            block=block,
            traceback=stack,
        )


@node(NodeType.PAUSE, local=True)
class Pause(Node):
    """A resumable interruption in the execution (Run) of a block."""

    parent: "Run" = p_parent(4, NodeType.RUN)
    session: "Session" = p_ancestor(30, NodeType.SESSION, require=True, store=True)
    # (placeholder)

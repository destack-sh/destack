from typing import TYPE_CHECKING, Generator, override

import structlog
from opentelemetry import trace

from bench.language import (
    Action,
    Agent,
    FieldType,
    File,
    FileType,
    Message,
    Node,
    NodeType,
    Page,
    Plan,
    Run,
    Span,
    Thread,
)
from bench.runtime.core import ThreadHandle
from bench.runtime.model import (
    AudioPiece,
    BreakPiece,
    CodePiece,
    CompoundPiece,
    ImagePiece,
    Piece,
    Prompt,
    Tokenizer,
    piece_,
    raise_if_none,
)

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@piece_()
class NodePiece[N: Node](CompoundPiece):
    """Render a Node directly."""

    node: N = raise_if_none()

    @override
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        rendered_node = prompt.renderer.render_statement(self.node, append=False, format=True)
        yield CodePiece(code=rendered_node)


@piece_(NodeType.THREAD)
class ThreadPiece(NodePiece[Thread]):
    """Render a Thread with its messages."""

    thread: ThreadHandle = raise_if_none()
    max_messages: int | None = None

    @override
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        yield NodePiece(node=self.thread.thread)
        yield BreakPiece()
        messages = list(self.thread.messages)
        messages.sort(key=lambda m: m.created_at)
        if self.max_messages is not None:
            messages = messages[-self.max_messages :]
        for i, message in enumerate(messages):
            yield MessagePiece(node=message, priority=i)


@piece_(NodeType.MESSAGE)
class MessagePiece(NodePiece[Message]):
    @override
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        rendered_node = prompt.renderer.render_statement(self.node, append=False, format=True)
        if created_by_ptr := self.node.created_by_ptr:
            # NOTE :Incomplete: load relevant Users (in ThreadHandle)?
            if created_by := self.node.created_by:
                created_by_alias = prompt.renderer.aliasing.get_or_add(created_by)
            else:
                created_by_alias = prompt.renderer.aliasing.get_or_add(created_by_ptr)
            if created_by_ptr.id == prompt.subject.id:
                created_by_alias += " (YOU)"
        else:
            created_by_alias = "<system>"
        ago = prompt.now - self.node.created_at
        rendered_node = (
            f"# from {created_by_alias} {round(ago.total_seconds())}s ago\n{rendered_node}"
        )
        yield CodePiece(code=rendered_node)
        for node in self.node.nodes:
            if isinstance(node, File):
                if node.type == FileType.IMAGE:
                    yield ImagePiece(file=node)
                elif node.type == FileType.AUDIO:
                    yield AudioPiece(file=node)


@piece_(NodeType.PAGE)
class PagePiece(NodePiece[Page]):
    # nocheckin: show & edit Pages
    pass


@piece_(NodeType.PLAN)
class PlanPiece(NodePiece[Plan]):
    pass


@piece_(NodeType.ACTION)
class ActionPiece(NodePiece[Action]):
    @override
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        rendered_node = prompt.renderer.render_statement(self.node, append=False, format=True)
        alias = prompt.renderer.aliasing.get_or_add(self.node)
        path = self.node.absolute_path
        inputs_examples_str = tuple(
            f"{f.name}=..." for f in self.node.fields if f.type == FieldType.INPUT
        )
        call_args_str = ", ".join([alias, *inputs_examples_str])
        rendered_node = f"# {path}: call like CALL({call_args_str})\n{rendered_node}"
        yield CodePiece(code=rendered_node)


@piece_(NodeType.AGENT)
class AgentPiece(NodePiece[Agent]):
    @override
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        rendered_node = prompt.renderer.render_statement(self.node, append=False, format=True)
        if prompt.subject.id == self.node.id:
            self_alias = prompt.renderer.aliasing.get_or_add(self.node)
            rendered_node = f"# THIS IS WHO YOU ARE: {self_alias}\n{rendered_node}"
        yield CodePiece(code=rendered_node)


@piece_(NodeType.RUN)
class RunPiece(NodePiece[Run]):
    is_last_action: bool = False

    @override
    def compile(
        self, prompt: "Prompt", tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        alias = prompt.renderer.aliasing.get_or_add(self.node)
        ago = prompt.now - self.node.created_at
        runnable = self.node.runnable
        assert runnable is not None, f"no runnable for {self.node!r}"
        runnable_alias = prompt.renderer.aliasing.get_or_add(runnable)
        run_parts: list[str] = []
        if self.is_last_action:
            run_parts.append(f"# THIS IS THE LAST ACTION YOU JUST CALLED: {alias}")
        run_parts.append(f"# Run of `{runnable_alias}` from {round(ago.total_seconds())}s ago")  # noqa: FURB113
        run_parts.append(f"# Status: {self.node.status.name}")
        if (duration := self.node.duration) is not None:
            run_parts.append(f"# Duration: {round(duration.total_seconds())}s")
        if (inputs := self.node.inputs) is not None:
            run_parts.append(f"# Inputs: {inputs!r}")
        if (outputs := self.node.outputs) is not None:
            run_parts.append(f"# Outputs: {outputs!r}")
        if (error := self.node.error) is not None:
            run_parts.append(f"# Error: {error!r}")
        run_parts.append(f"{alias} = Run(action={runnable_alias}, ...)")
        yield CodePiece(code="\n".join(run_parts))


@piece_()
class AttemptPiece(NodePiece[Span]):
    pass

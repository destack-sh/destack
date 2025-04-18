from typing import TYPE_CHECKING, Generator, Sequence, Union, override

import structlog
from opentelemetry import trace

from bench.language import (
    Action,
    Agent,
    Database,
    FieldType,
    File,
    Message,
    Node,
    NodeReference,
    NodeType,
    Page,
    Plan,
    Run,
    Span,
    Task,
    Thread,
)
from bench.runtime.core import ThreadHandle
from bench.runtime.model import (
    PIECE_BY_NODE_TYPE,
    BreakPiece,
    CodePiece,
    CompoundPiece,
    FilePiece,
    Piece,
    Prompt,
    SeparatorPiece,
    TextPiece,
    Tokenizer,
    get_file_piece,
    piece_,
    raise_if_none,
)

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def _get_node_piece(node: Node) -> Union[FilePiece, "NodePiece"]:
    """Get the appropriate NodePiece for a Node."""
    if isinstance(node, File):
        return get_file_piece(node)
    elif (piece_cls := PIECE_BY_NODE_TYPE.get(node.metatype)) is not None:
        assert issubclass(piece_cls, NodePiece)
        return piece_cls(node=node)
    else:
        return NodePiece(node=node)


@piece_()
class NodePiece[N: Node](CompoundPiece):
    """Render a Node directly."""

    node: N = raise_if_none()
    prepend_path: bool = True

    def prefetch(self, prompt: "Prompt") -> Sequence[Node | NodeReference]:
        """Prepare a list of Nodes to load before compilation."""
        return ()

    @override
    def compile(self, prompt: "Prompt", tokenizer: Tokenizer) -> Generator[Piece, None, None]:
        alias = prompt.renderer.aliasing.get_or_add(self.node)
        rendered_node = prompt.renderer.render_statement(self.node, append=False, format=True)
        if self.prepend_path:
            rendered_node = f"# [@{alias}] = {self.node.absolute_path}\n{rendered_node}"
        yield CodePiece(code=rendered_node)


@piece_(NodeType.THREAD)
class ThreadPiece(NodePiece[Thread]):
    """Render a Thread with its messages."""

    thread: ThreadHandle = raise_if_none()
    max_messages: int | None = None

    @override
    def compile(self, prompt: "Prompt", tokenizer: Tokenizer) -> Generator[Piece, None, None]:
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
    def compile(self, prompt: "Prompt", tokenizer: Tokenizer) -> Generator[Piece, None, None]:
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
                yield get_file_piece(node)


@piece_(NodeType.PAGE)
class PagePiece(NodePiece[Page]):
    @override
    def prefetch(self, prompt: "Prompt") -> Sequence[Node | NodeReference]:
        missing_PAGE_NODEs: list[NodeReference] = []
        for block in self.node.blocks:
            if block.type.is_node and (node_ptr := block.node_ptr) is not None:
                missing_PAGE_NODEs.append(node_ptr)
        return missing_PAGE_NODEs

    @override
    def compile(self, prompt: "Prompt", tokenizer: Tokenizer) -> Generator[Piece, None, None]:
        yield SeparatorPiece()
        # header (page)
        alias = prompt.renderer.aliasing.get_or_add(self.node)
        rendered_page = prompt.renderer.render_statement(self.node, append=False, format=True)
        rendered_page = f"""\
# [@{alias}] = {self.node.absolute_path}
# Text Blocks are markdown with their alias prepended like `[@Block1] <line>`.
# You SHOULD use `ADD_PAGE_TEXT` and `REPLACE_PAGE_TEXT` to edit continuous text sections.
# You MAY reference Blocks directly by their alias (like to `Block7.line = ...`). 
{rendered_page}
"""
        yield CodePiece(code=rendered_page)
        yield SeparatorPiece()

        # blocks
        text_block_parts: list[str] = []

        def _flush_text_block_parts() -> CodePiece:
            rendered_blocks = "\n".join(text_block_parts)
            text_block_parts.clear()
            return CodePiece(code=rendered_blocks)

        for block in self.node.blocks:
            block_alias = prompt.renderer.aliasing.get_or_add(block)
            if block.type.is_text:
                # text block
                line_str = block.line.to_markdown() if block.line is not None else ""
                text_block_parts.append(f"[@{block_alias}] {line_str}")
            else:
                # node block
                if text_block_parts:
                    yield _flush_text_block_parts()
                if (node := block.node) is not None:
                    yield _get_node_piece(node)
                elif (node_ptr := block.node_ptr) is not None:
                    node_alias = prompt.renderer.aliasing.get_or_add(node_ptr)
                    yield TextPiece(
                        text=f"[@{block_alias}] <UNLOADED {node_ptr.node_type.name} NODE: {node_alias}>"
                    )
        if text_block_parts:
            yield _flush_text_block_parts()

        # footer (page)
        yield SeparatorPiece()


@piece_(NodeType.DATABASE)
class DatabasePiece(NodePiece[Database]):
    pass


@piece_(NodeType.ACTION)
class ActionPiece(NodePiece[Action]):
    @override
    def compile(self, prompt: "Prompt", tokenizer: Tokenizer) -> Generator[Piece, None, None]:
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
    def compile(self, prompt: "Prompt", tokenizer: Tokenizer) -> Generator[Piece, None, None]:
        rendered_node = prompt.renderer.render_statement(self.node, append=False, format=True)
        if prompt.subject.id == self.node.id:
            self_alias = prompt.renderer.aliasing.get_or_add(self.node)
            rendered_node = f"# THIS IS WHO YOU ARE: {self_alias}\n{rendered_node}"
        yield CodePiece(code=rendered_node)


@piece_(NodeType.PLAN)
class PlanPiece(NodePiece[Plan]):
    @override
    def compile(self, prompt: "Prompt", tokenizer: Tokenizer) -> Generator[Piece, None, None]:
        yield SeparatorPiece()
        # header (plan)
        rendered_node = prompt.renderer.render_statement(self.node, append=False, format=True)
        rendered_node = f"# {self.node.absolute_path}\n{rendered_node}"
        yield CodePiece(code=rendered_node)
        yield SeparatorPiece()
        # tasks
        for task in self.node.tasks:
            yield TaskPiece(node=task)
        yield SeparatorPiece()


@piece_(NodeType.TASK)
class TaskPiece(NodePiece[Task]):
    pass


@piece_(NodeType.RUN)
class RunPiece(NodePiece[Run]):
    is_last_action: bool = False

    @override
    def compile(self, prompt: "Prompt", tokenizer: Tokenizer) -> Generator[Piece, None, None]:
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
            rendered_inputs = prompt.renderer.render_custom_object(inputs)
            run_parts.append(f"# Inputs: {rendered_inputs}")
        if (outputs := self.node.outputs) is not None:
            rendered_outputs = prompt.renderer.render_custom_object(outputs)
            run_parts.append(f"# Outputs: {rendered_outputs}")
        if (error := self.node.error) is not None:
            rendered_error = prompt.renderer.render_builtin_object(error)
            run_parts.append(f"# Error: {rendered_error}")
        run_parts.append(f"{alias} = Run(action={runnable_alias}, ...)")
        yield CodePiece(code="\n".join(run_parts))


@piece_()
class AttemptPiece(NodePiece[Span]):
    prepend_path: bool = False

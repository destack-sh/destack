from typing import TYPE_CHECKING, Union, override

import anthropic
import structlog
from anthropic import NOT_GIVEN
from anthropic import types as anthropic_types
from opentelemetry import trace

from bench.language import Code, Session, download_file_batch
from bench.runtime.core import NotSupportedError
from bench.utils.utils import get_from_env

from .chat import ChatModelRunner
from .code import StreamingCodeRunner
from .piece import (
    AudioPiece,
    BreakPiece,
    CodePiece,
    ImagePiece,
    LeafPiece,
    PieceRole,
    SeparatorPiece,
    TextPiece,
)
from .prompt import LOG_COMPLETIONS, LOG_PROMPTS, Prompt, compile_prompt, log_completion, log_prompt
from .token import TiktokenTokenizer, Tokenizer

if TYPE_CHECKING:
    pass


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

anthropic_client = anthropic.AsyncClient(
    api_key=get_from_env("ANTHROPIC_API_KEY", description="Anthropic API key")
)

BREAK = "\n"
SEPARATOR = "#" * 32  # = exactly 1 token


async def build_anthropic_messages(
    prompt: Prompt, tokenizer: Tokenizer, max_tokens: int, session: Session
) -> tuple[str, list[anthropic_types.MessageParam], list["LeafPiece"]]:
    """
    Prepare the messages for an Anthropic chat completion.
    """

    pieces, _ = compile_prompt(prompt=prompt, tokenizer=tokenizer, max_tokens=max_tokens)

    # download media
    files_to_download = [
        piece.file
        for piece in pieces
        if isinstance(piece, (ImagePiece, AudioPiece))
        if piece.file._cached_content is None
    ]
    if files_to_download:
        await download_file_batch(files_to_download, include_content=True, session=session)

    # render
    current_content: list[
        Union[anthropic_types.TextBlockParam, anthropic_types.ImageBlockParam]
    ] = []
    current_text_pieces: list[str] = []
    current_role: PieceRole | None = None
    messages: list[anthropic_types.MessageParam] = []

    def _flush_text() -> None:
        if current_text_pieces:
            current_content.append({"type": "text", "text": "\n".join(current_text_pieces)})
            current_text_pieces.clear()

    def _flush_content() -> None:
        if current_role and current_content:
            messages.append({"role": current_role, "content": tuple(current_content)})  # type: ignore
            current_content.clear()

    for piece in pieces:
        # flush on role change
        piece_role = piece.role
        if current_role is not None and piece_role != current_role:
            _flush_text()
            _flush_content()
        current_role = piece_role

        if isinstance(piece, BreakPiece):
            current_text_pieces.append(BREAK)
        elif isinstance(piece, SeparatorPiece):
            current_text_pieces.append(SEPARATOR)
        elif isinstance(piece, TextPiece):
            text = "\n".join([f"# {line}" for line in piece.text.splitlines()])
            current_text_pieces.append(text)
        elif isinstance(piece, CodePiece):
            current_text_pieces.append(piece.code)
        elif isinstance(piece, ImagePiece):
            _flush_text()
            mime_type = piece.file.mime_type
            if mime_type not in ("image/jpeg", "image/png", "image/gif", "image/webp"):
                raise NotSupportedError(f"unsupported image mime type {mime_type!r}")
            current_content.append(
                {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": mime_type,
                        "data": piece.file.read_content_b64(),
                    },
                }
            )
        else:
            raise NotSupportedError(f"unexpected piece {piece!r}")

    # final flush
    _flush_text()
    _flush_content()

    return prompt.system_prompt, messages, pieces


class AnthropicChatModelRunner(ChatModelRunner):
    """Run any Anthropic chat model."""

    @override
    async def run(self) -> None:
        from bench.runtime import MACROS, AgentRunner

        # build
        max_tokens = 20_000
        system_prompt, messages, pieces = await build_anthropic_messages(
            prompt=self.prompt,
            tokenizer=TiktokenTokenizer(),
            max_tokens=max_tokens,
            session=self.session,
        )
        if LOG_PROMPTS:
            log_prompt(self.prompt, pieces)

        # generate & execute simultaneously
        agent_runner = self.closest_runner_like(AgentRunner)
        code_runner = StreamingCodeRunner(
            runner=agent_runner, macros=MACROS, aliasing=self.prompt.aliasing
        )
        completion = await anthropic_client.messages.create(
            system=system_prompt,
            max_tokens=8192,
            model=self.model_id,
            messages=messages,
            temperature=NOT_GIVEN,
            stream=True,
        )
        async for chunk in completion:
            if chunk.type == "content_block_delta" and chunk.delta.type == "text":
                chunk_content = chunk.delta.text
                code_runner.add_and_execute(chunk_content)
        code_runner.complete_and_execute()
        if LOG_COMPLETIONS:
            log_completion(code_runner.code)
        self.code = Code.from_string(code_runner.code or "pass", language="python")

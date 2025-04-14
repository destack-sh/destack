from typing import TYPE_CHECKING, Union, override

import anthropic
import structlog
from anthropic import NOT_GIVEN
from anthropic import types as anthropic_types
from opentelemetry import trace

from bench.language import Code, download_file_batch
from bench.runtime.core import NotSupportedError
from bench.utils.utils import get_from_env

from .chat import ChatModelRunner
from .code import StreamingCodeRunner
from .piece import (
    AudioPiece,
    BreakPiece,
    CodePiece,
    ImagePiece,
    SeparatorPiece,
    TextPiece,
)
from .prompt import LOG_PROMPTS, compile_prompt, log_completion, log_prompt
from .token import TiktokenTokenizer

if TYPE_CHECKING:
    pass


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


anthropic_client = anthropic.AsyncClient(
    api_key=get_from_env("ANTHROPIC_API_KEY", description="Anthropic API key")
)
ANTHROPIC_DEFAULT_MODEL = "claude-3-7-sonnet-20250219"


class AnthropicChatModelRunner(ChatModelRunner):
    """Run any Anthropic chat model."""

    SEPARATOR = "#" * 32  # = exactly 1 token

    @override
    async def run(self) -> None:
        from bench.runtime import MACROS, AgentRunner

        model_id = self.model_id or ANTHROPIC_DEFAULT_MODEL
        tokenizer = TiktokenTokenizer()
        max_tokens = 20_000
        pieces, _ = compile_prompt(prompt=self.prompt, tokenizer=tokenizer, max_tokens=max_tokens)
        if LOG_PROMPTS:
            log_prompt(self.prompt, pieces)

        # download media
        files_to_download = [
            piece.file
            for piece in pieces
            if isinstance(piece, (ImagePiece, AudioPiece))
            if piece.file._cached_content is None
        ]
        if files_to_download:
            await download_file_batch(files_to_download, include_content=True, session=self.session)

        # compile
        content_pieces: list[
            Union[anthropic_types.TextBlockParam, anthropic_types.ImageBlockParam]
        ] = []
        text_pieces: list[str] = []

        def _flush_text() -> None:
            if text_pieces:
                content_pieces.append({"type": "text", "text": "\n".join(text_pieces)})
                text_pieces.clear()

        for piece in pieces:
            if isinstance(piece, BreakPiece):
                text_pieces.append("\n\n")
            elif isinstance(piece, SeparatorPiece):
                text_pieces.append(self.SEPARATOR)
            elif isinstance(piece, TextPiece):
                text = "\n".join([f"# {line}" for line in piece.text.splitlines()])
                text_pieces.append(text)
            elif isinstance(piece, CodePiece):
                text_pieces.append(piece.code)
            elif isinstance(piece, ImagePiece):
                _flush_text()
                mime_type = piece.file.mime_type
                if mime_type not in ("image/jpeg", "image/png", "image/gif", "image/webp"):
                    raise NotSupportedError(f"unsupported image mime type {mime_type!r}")
                content_pieces.append(
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": mime_type,
                            "data": piece.file.content_b64,
                        },
                    }
                )
            else:
                raise NotSupportedError(f"unexpected piece {piece!r}")

        _flush_text()

        # generate & execute simultaneously
        agent_runner = self.closest_runner_like(AgentRunner)
        code_runner = StreamingCodeRunner(
            runner=agent_runner, macros=MACROS, aliasing=self.prompt.aliasing
        )
        messages: list[anthropic_types.MessageParam] = [{"role": "user", "content": content_pieces}]
        completion = await anthropic_client.messages.create(
            system=[
                {
                    "type": "text",
                    "text": self.prompt.system_prompt,
                }
            ],
            max_tokens=8192,
            model=model_id,
            messages=messages,
            temperature=NOT_GIVEN,
            stream=True,
        )
        async for chunk in completion:
            if chunk.type == "content_block_delta" and chunk.delta.type == "text":
                chunk_content = chunk.delta.text
                code_runner.add(chunk_content)
        code_runner.complete()
        if LOG_PROMPTS:
            log_completion(code_runner.code)
        self.code = Code.from_string(code_runner.code or "pass", language="python")

from typing import TYPE_CHECKING, Mapping, Union, override

import anthropic
import structlog
from anthropic import NOT_GIVEN
from anthropic import types as anthropic_types
from opentelemetry import trace

from bench.language import Code, ModelType, RunOptions, download_file_batch
from bench.runtime.core import NotSupportedError
from bench.runtime.model.token import TiktokenTokenizer
from bench.utils.utils import get_from_env

from .chat import ChatModelRunner, strip_code_completion
from .piece import (
    AudioPiece,
    BreakPiece,
    CodePiece,
    ImagePiece,
    SeparatorPiece,
    TextPiece,
)
from .prompt import Prompt, compile_prompt

if TYPE_CHECKING:
    pass


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


anthropic_client = anthropic.AsyncClient(
    api_key=get_from_env("ANTHROPIC_API_KEY", description="Anthropic API key")
)
ANTHROPIC_MODEL_BY_TYPE: Mapping[ModelType, str] = {
    ModelType.ANTHROPIC_CLAUDE_3_5_SONNET: "claude-3-5-sonnet-20241022",
    ModelType.ANTHROPIC_CLAUDE_3_7_SONNET: "claude-3-7-sonnet-20250219",
}
ANTHROPIC_DEFAULT_MODEL = ModelType.ANTHROPIC_CLAUDE_3_5_SONNET


class AnthropicChatModelRunner(ChatModelRunner):
    """Compile a Prompt into Anthropic chat messages."""

    SEPARATOR = "#" * 32  # = exactly 1 token

    @override
    async def generate(self, prompt: Prompt, options: RunOptions) -> Code:
        model_id = ANTHROPIC_MODEL_BY_TYPE.get(self.model_type)
        if model_id is None:
            raise NotSupportedError(f"unsupported model type {self.model_type!r}")

        tokenizer = TiktokenTokenizer()
        max_tokens = 20_000
        pieces, _ = compile_prompt(prompt=prompt, tokenizer=tokenizer, max_tokens=max_tokens)

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

        # generate
        messages: list[anthropic_types.MessageParam] = [{"role": "user", "content": content_pieces}]
        temperature = options.text_options.temperature if options.text_options else None
        completion = await anthropic_client.messages.create(
            system=[
                {
                    "type": "text",
                    "text": prompt.system_prompt,
                    "cache_control": {"type": "ephemeral"},
                }
            ],
            max_tokens=8192,
            model=model_id,
            messages=messages,
            temperature=temperature or NOT_GIVEN,
        )
        completion_text = getattr(completion.content[0], "text", None)
        if isinstance(completion_text, str):
            completion_text = strip_code_completion(completion_text)
        return Code.from_string(completion_text or "pass")

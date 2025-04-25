from typing import override

import openai
import structlog
from openai.types import chat as openai_chat_types
from opentelemetry import trace

from bench.language import Code, Session, download_file_batch
from bench.runtime.core import IncapableError
from bench.utils.func import hash_stable_hex
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

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

openai_client = openai.AsyncClient(
    api_key=get_from_env("OPENAI_API_KEY", description="OpenAI API key")
)

BREAK = "\n"
SEPARATOR = "#" * 32  # = exactly 1 token


async def build_openai_chat_messages(
    prompt: Prompt, tokenizer: Tokenizer, max_tokens: int, session: Session
) -> tuple[list[openai_chat_types.ChatCompletionMessageParam], list[LeafPiece]]:
    """
    Prepare the messages for an OpenAI chat completion.
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
    current_content: list[openai_chat_types.ChatCompletionContentPartParam] = []
    current_text_pieces: list[str] = []
    current_role: PieceRole | None = None
    messages: list[openai_chat_types.ChatCompletionMessageParam] = [
        {"role": "developer", "content": prompt.system_prompt},
    ]

    def _flush_text() -> None:
        if current_text_pieces:
            current_content.append({"type": "text", "text": "\n".join(current_text_pieces)})
            current_text_pieces.clear()

    def _flush_content() -> None:
        if current_role and current_content:
            messages.append({"role": current_role, "content": tuple(current_content)})  # type: ignore
            current_content.clear()

    # render
    for piece in pieces:
        # flush on role change
        if current_role is not None and piece.role != current_role:
            _flush_text()
            _flush_content()
        current_role = piece.role

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
            if piece.file.url is not None:
                current_content.append({"type": "image_url", "image_url": {"url": piece.file.url}})
            else:
                current_content.append(
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": f"data:{piece.file.mime_type};base64,{piece.file.read_content_b64()}"
                        },
                    }
                )
        else:
            raise IncapableError(f"unexpected piece {piece!r}")

    # final flush
    _flush_text()
    _flush_content()

    return messages, pieces


class OpenAIChatModelRunner(ChatModelRunner):
    """Run any OpenAI chat model."""

    @override
    async def run(self) -> None:
        from bench.runtime import MACROS, AgentRunner

        # build
        max_tokens = 16_384
        messages, pieces = await build_openai_chat_messages(
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
        completion = await openai_client.chat.completions.create(
            messages=messages,
            model=self.model_id,
            max_tokens=max_tokens,
            user=hash_stable_hex(self.runtime.bench.id.int),
            stream=True,
        )
        async for chunk in completion:
            if chunk.choices and chunk.choices[0].delta.content:
                chunk_content = chunk.choices[0].delta.content
                code_runner.add_and_execute(chunk_content)
        if LOG_COMPLETIONS:
            log_completion(code_runner.code)
        code_runner.complete_and_execute()
        self.code = Code.from_string(code_runner.code or "pass", language="python")

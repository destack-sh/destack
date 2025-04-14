from typing import TYPE_CHECKING, override

import google.generativeai as genai
import structlog
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
from .prompt import LOG_PROMPTS, Prompt, compile_prompt, log_completion, log_prompt
from .token import TiktokenTokenizer, Tokenizer

if TYPE_CHECKING:
    pass


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

genai.configure(api_key=get_from_env("GEMINI_API_KEY", description="Gemini API key"))

BREAK = "\n"
SEPARATOR = "#" * 32  # = exactly 1 token


async def build_google_chat_messages(
    prompt: Prompt, tokenizer: Tokenizer, max_tokens: int, session: Session
) -> tuple[str, list[genai.types.ContentDict], list["LeafPiece"]]:
    """
    Prepare the messages for a Google Gemini chat completion.
    """
    pieces, _ = compile_prompt(prompt=prompt, tokenizer=tokenizer, max_tokens=max_tokens)
    if LOG_PROMPTS:
        log_prompt(prompt, pieces)

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
    current_content: list[genai.types.BlobDict | str] = []
    current_text_parts: list[str] = []
    current_role: PieceRole | None = None
    messages: list[genai.types.ContentDict] = []

    def _flush_text() -> None:
        if current_text_parts:
            current_content.append("\n".join(current_text_parts))
            current_text_parts.clear()

    def _flush_content() -> None:
        if current_role and current_content:
            messages.append({"role": current_role, "parts": tuple(current_content)})  # type: ignore
            current_content.clear()

    for piece in pieces:
        # flush on role change
        piece_role = piece.role
        if current_role is not None and piece_role != current_role:
            _flush_text()
            _flush_content()
        current_role = piece_role

        if isinstance(piece, BreakPiece):
            current_text_parts.append(BREAK)
        elif isinstance(piece, SeparatorPiece):
            current_text_parts.append(SEPARATOR)
        elif isinstance(piece, TextPiece):
            text = "\n".join([f"# {line}" for line in piece.text.splitlines()])
            current_text_parts.append(text)
        elif isinstance(piece, CodePiece):
            current_text_parts.append(piece.code)
        elif isinstance(piece, ImagePiece):
            _flush_text()
            mime_type = piece.file.mime_type
            if not mime_type:
                raise NotSupportedError(f"file {piece.file!r} not supported yet")
            current_content.append({"mime_type": mime_type, "data": piece.file.content})
        else:
            raise RuntimeError(f"unexpected piece {piece!r}")

    # final flush
    _flush_text()
    _flush_content()

    return prompt.system_prompt, messages, pieces


class GoogleChatModelRunner(ChatModelRunner):
    """Run any Google chat model."""

    @override
    async def run(self) -> None:
        from bench.runtime import MACROS, AgentRunner

        # build
        max_tokens = 20_000
        system_prompt, messages, pieces = await build_google_chat_messages(
            prompt=self.prompt,
            tokenizer=TiktokenTokenizer(),
            max_tokens=max_tokens,
            session=self.session,
        )
        if LOG_PROMPTS:
            log_prompt(self.prompt, pieces)

        # generate & execute simultaneously
        # NOTE :Performance: maybe re-use genai.GenerativeModel instance?
        #  (but we may need different system prompts for different runs)
        agent_runner = self.closest_runner_like(AgentRunner)
        code_runner = StreamingCodeRunner(
            runner=agent_runner, macros=MACROS, aliasing=self.prompt.aliasing
        )
        model = genai.GenerativeModel(self.model_id, system_instruction=system_prompt)
        completion = await model.generate_content_async(
            messages,
            generation_config=genai.GenerationConfig(temperature=0.1),
            stream=True,
        )
        async for chunk in completion:
            if chunk.text:
                code_runner.add(chunk.text)
        code_runner.complete()
        if LOG_PROMPTS:
            log_completion(code_runner.code)
        self.code = Code.from_string(code_runner.code or "pass", language="python")

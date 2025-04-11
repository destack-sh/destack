from typing import Mapping, override

import openai
import structlog
from openai.types import chat as openai_chat_types
from opentelemetry import trace

from bench.language import Code, ModelType, download_file_batch
from bench.runtime.core import IncapableError, NotSupportedError
from bench.utils.func import hash_stable_hex
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

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

openai_client = openai.AsyncClient(
    api_key=get_from_env("OPENAI_API_KEY", description="OpenAI API key")
)
OPENAI_MODEL_BY_TYPE: Mapping[ModelType, str] = {
    ModelType.OPENAI_GPT4_0: "gpt-4o-2024-11-20",
    ModelType.OPENAI_GPT4_O_MINI: "gpt-4o-mini-2024-07-18",
    ModelType.OPENAI_GPT4_5: "gpt-4.5-preview-2025-02-27",
    ModelType.OPENAI_O1_MINI: "o1-mini-2024-09-12",
    ModelType.OPENAI_O3_MINI: "o3-mini-2025-01-31",
    ModelType.OPENAI_O1: "o1-2024-12-17",
}
OPENAI_DEFAULT_MODEL = ModelType.OPENAI_GPT4_0


class OpenAIChatModelRunner(ChatModelRunner):
    """Compile a Prompt into OpenAI chat messages."""

    BREAK = "\n"
    SEPARATOR = "#" * 32  # = exactly 1 token

    @override
    async def run(self) -> None:
        from bench.runtime import MACROS, AgentRunner

        agent = self.agent
        assert agent is not None, f"{self!r} has no Agent"
        model_id = OPENAI_MODEL_BY_TYPE.get(self.model_type)
        if model_id is None:
            raise NotSupportedError(f"unsupported model type {self.model_type!r}")

        tokenizer = TiktokenTokenizer()
        max_tokens = 16_384
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

        # render
        content: list[openai_chat_types.ChatCompletionContentPartParam] = []
        text_pieces: list[str] = []

        def _flush_text() -> None:
            if text_pieces:
                content.append({"type": "text", "text": "\n".join(text_pieces)})
                text_pieces.clear()

        for piece in pieces:
            if isinstance(piece, BreakPiece):
                text_pieces.append(self.BREAK)
            elif isinstance(piece, SeparatorPiece):
                text_pieces.append(self.SEPARATOR)
            elif isinstance(piece, TextPiece):
                text = "\n".join([f"# {line}" for line in piece.text.splitlines()])
                text_pieces.append(text)
            elif isinstance(piece, CodePiece):
                text_pieces.append(piece.code)
            elif isinstance(piece, ImagePiece):
                _flush_text()
                if piece.file.external_url is not None:
                    content.append(
                        {"type": "image_url", "image_url": {"url": piece.file.external_url}}
                    )
                else:
                    content.append(
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": f"data:image/jpeg;base64,{piece.file.content_b64}"
                            },
                        }
                    )
            else:
                raise IncapableError(f"unexpected piece {piece!r}")

        _flush_text()

        # generate & execute simultaneously
        agent_runner = self.closest_runner_like(AgentRunner)
        code_runner = StreamingCodeRunner(
            runner=agent_runner, macros=MACROS, aliasing=self.prompt.aliasing
        )
        messages: list[openai_chat_types.ChatCompletionMessageParam] = [
            {"role": "developer", "content": self.prompt.system_prompt},
            {"role": "user", "content": content},
        ]
        completion = await openai_client.chat.completions.create(
            messages=messages,
            model=model_id,
            max_tokens=max_tokens,
            user=hash_stable_hex(self.runtime.bench.id.int),
            stream=True,
        )
        async for chunk in completion:
            if chunk.choices and chunk.choices[0].delta.content:
                chunk_content = chunk.choices[0].delta.content
                code_runner.add(chunk_content)
        if LOG_PROMPTS:
            log_completion(code_runner.code)
        code_runner.complete()
        self.code = Code.from_string(code_runner.code or "pass", language="python")

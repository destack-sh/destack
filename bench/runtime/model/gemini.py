from typing import TYPE_CHECKING, Mapping, cast, override

import google.generativeai as genai
import structlog
from opentelemetry import trace

from bench.language import Code, ModelType, Runnable, SpanType, download_file_batch
from bench.runtime.code import CodeFunctionRunner
from bench.runtime.core import ATTEMPT_ONCE, NotSupportedError, Runner
from bench.runtime.model.token import TiktokenTokenizer
from bench.utils.utils import get_from_env

from .chat import ChatModelRunner
from .piece import (
    AudioPiece,
    BreakPiece,
    CodePiece,
    ImagePiece,
    SeparatorPiece,
    TextPiece,
)
from .prompt import LOG_PROMPTS, compile_prompt, log_prompt

if TYPE_CHECKING:
    pass


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

genai.configure(api_key=get_from_env("GEMINI_API_KEY", description="Gemini API key"))

GEMINI_MODEL_BY_TYPE: Mapping[ModelType, str] = {
    ModelType.GOOGLE_GEMINI_2_0_FLASH: "gemini-2.0-flash-exp",
    ModelType.GOOGLE_GEMINI_2_0_FLASH_THINKING: "gemini-2.0-flash-thinking-exp-01-21",
}
GEMINI_DEFAULT_MODEL = ModelType.GOOGLE_GEMINI_2_0_FLASH


class GeminiChatModelRunner(ChatModelRunner):
    """Compile a Prompt into Gemini chat messages."""

    SEPARATOR = "#" * 32  # = exactly 1 token

    @override
    async def run(self) -> None:
        model_id = GEMINI_MODEL_BY_TYPE.get(self.model_type)
        if model_id is None:
            raise NotSupportedError(f"unsupported model type {self.model_type!r}")

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
        content_parts: list[genai.types.BlobDict | str] = []
        text_parts: list[str] = []

        def _flush_text() -> None:
            if text_parts:
                content_parts.append("\n".join(text_parts))
                text_parts.clear()

        for piece in pieces:
            if isinstance(piece, BreakPiece):
                text_parts.append("\n\n")
            elif isinstance(piece, SeparatorPiece):
                text_parts.append(self.SEPARATOR)
            elif isinstance(piece, TextPiece):
                text = "\n".join([f"# {line}" for line in piece.text.splitlines()])
                text_parts.append(text)
            elif isinstance(piece, CodePiece):
                text_parts.append(piece.code)
            elif isinstance(piece, ImagePiece):
                _flush_text()
                mime_type = piece.file.mime_type
                if not mime_type:
                    raise NotSupportedError(f"file {piece.file!r} not supported yet")
                content_parts.append({"mime_type": mime_type, "data": piece.file.content})
            else:
                raise RuntimeError(f"unexpected piece {piece!r}")

        _flush_text()

        # generate
        # NOTE :Performance: maybe re-use genai.GenerativeModel instance?
        #  (but we may need different system prompts for different runs)
        temperature = self.options.text_options.temperature if self.options.text_options else 0.1
        model = genai.GenerativeModel(model_id, system_instruction=self.prompt.system_prompt)
        completion = await model.generate_content_async(
            {"role": "user", "parts": content_parts},
            generation_config=genai.GenerationConfig(temperature=temperature),
        )
        code_str = completion.parts[0].text
        code_str = self.clean_code(code_str)

        # run
        code = Code.from_string(code_str or "pass", language="python")
        code_runner = CodeFunctionRunner(
            runtime=self.runtime,
            node=self.node,
            code=code,
            aliasing=self.prompt.aliasing,
            options=ATTEMPT_ONCE,
            inputs=self.inputs,
            outputs=self.output_type,
            parent=cast(Runner[Runnable], self),
            run=SpanType.MODEL_PARSE,
        )
        await self.runtime.run_runner(code_runner)

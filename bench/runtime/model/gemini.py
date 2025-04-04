from typing import TYPE_CHECKING, Mapping, Sequence, override

import google.generativeai as genai

from bench.language import Code, FileType, ModelType, RunOptions, download_file_batch
from bench.runtime.core import NotSupportedError
from bench.utils.utils import get_from_env

from .chat import ChatModelRunner, strip_code_completion
from .prompt import (
    Prompt,
    PromptBreak,
    PromptCode,
    PromptComponent,
    PromptFile,
    PromptSeparator,
    PromptText,
)

if TYPE_CHECKING:
    pass


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
    async def generate(self, prompt: Prompt, options: RunOptions) -> Code:
        model_id = GEMINI_MODEL_BY_TYPE.get(self.model_type)
        if model_id is None:
            raise NotSupportedError(f"unsupported model type {self.model_type!r}")

        # download media
        files_to_download = [
            part.file
            for part in parts
            if isinstance(part, PromptFile)
            if part.file._cached_content is None
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

        for part in parts:
            if isinstance(part, PromptBreak):
                text_parts.append("\n\n")
            elif isinstance(part, PromptSeparator):
                text_parts.append(self.SEPARATOR)
                if part.title:
                    text_parts.append(f"# {part.title}")
                    if part.text:
                        text_parts.append(f"# {part.text}")
                    text_parts.append(self.SEPARATOR)
            elif isinstance(part, PromptText):
                # prepend every text line
                text = "\n".join([f"# {line}" for line in part.text.splitlines()])
                text = f"# {part.title}\n{text}" if part.title else text
                text_parts.append(text)
            elif isinstance(part, PromptCode):
                text = f"# {part.title}\n{part.code}" if part.title else part.code
                text_parts.append(text)
            elif isinstance(part, PromptFile):
                _flush_text()
                if part.file.type == FileType.IMAGE:
                    mime_type = part.file.mime_type
                    if not mime_type:
                        raise NotSupportedError(f"file {part.file!r} not supported yet")
                    content_parts.append({"mime_type": mime_type, "data": part.file.content})
                else:
                    raise NotSupportedError(f"file {part.file!r} not supported yet")
            else:
                raise RuntimeError(f"unexpected part {part!r}")

        _flush_text()

        # generate
        # NOTE :Performance: maybe re-use genai.GenerativeModel instance?
        #  (but we may need different system prompts for different runs)
        temperature = options.text_options.temperature if options.text_options else 0.1
        model = genai.GenerativeModel(model_id, system_instruction=prompt.system_prompt)
        completion = await model.generate_content_async(
            {"role": "user", "parts": content_parts},
            generation_config=genai.GenerationConfig(temperature=temperature),
        )
        completion_text = completion.parts[0].text
        if isinstance(completion_text, str):
            completion_text = strip_code_completion(completion_text)
        return Code.from_string(completion_text or "pass")

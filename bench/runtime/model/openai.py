from typing import Mapping, Sequence, override

import openai
from openai.types import chat as openai_chat_types

from bench.language import Code, ModelType, RunOptions
from bench.language.compute.file import FileType, download_file_batch
from bench.runtime.core import NotSupportedError
from bench.utils.utils import get_from_env

from .chat import ChatModelRunner, strip_code_completion
from .instruct import get_system_prompt
from .prompt import (
    Prompt,
    PromptBreak,
    PromptCode,
    PromptElement,
    PromptFile,
    PromptSeparator,
    PromptText,
)

openai_client = openai.AsyncClient(
    api_key=get_from_env("OPENAI_API_KEY", description="OpenAI API key")
)
OPENAI_MODEL_BY_TYPE: Mapping[ModelType, str] = {
    ModelType.OPENAI_GPT4_0: "gpt-4o-2024-01-29",
    ModelType.OPENAI_GPT4_O_MINI: "gpt-4o-mini-2024-07-18",
    ModelType.OPENAI_O1_MINI: "o1-mini-2024-09-12",
    ModelType.OPENAI_O3_MINI: "o3-mini-2025-01-31",
    ModelType.OPENAI_O1: "o1-2024-12-17",
}
OPENAI_DEFAULT_MODEL = ModelType.OPENAI_GPT4_0


class OpenaiChatModelRunner(ChatModelRunner):
    """Compile a Prompt into OpenAI chat messages."""

    SEPARATOR = "#" * 32  # = exactly 1 token

    @override
    async def generate(
        self,
        prompt: Prompt,
        parts: Sequence[PromptElement],
        user_id: str,
        options: RunOptions,
    ) -> Code:
        model_id = OPENAI_MODEL_BY_TYPE.get(self.model_type)
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
        content: list[openai_chat_types.ChatCompletionContentPartParam] = []
        text_parts: list[str] = []

        def _flush_text() -> None:
            if text_parts:
                content.append({"type": "text", "text": "\n".join(text_parts)})
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
                    if part.file.external_url is not None:
                        content.append(
                            {"type": "image_url", "image_url": {"url": part.file.external_url}}
                        )
                    else:
                        content.append(
                            {
                                "type": "image_url",
                                "image_url": {
                                    "url": f"data:image/jpeg;base64,{part.file.content_b64}"
                                },
                            }
                        )
                else:
                    raise NotSupportedError(f"file {part.file!r} not supported yet")
            else:
                raise RuntimeError(f"unexpected part {part!r}")

        _flush_text()

        # generate
        messages: list[openai_chat_types.ChatCompletionMessageParam] = [
            {"role": "developer", "content": get_system_prompt(prompt)},
            {"role": "user", "content": content},
        ]
        temperature = options.text_options.temperature if options.text_options else 0.1
        completion = await openai_client.chat.completions.create(
            messages=messages,
            model=model_id,
            temperature=temperature,
            user=user_id,
        )
        completion_text = completion.choices[0].message.content
        if completion_text:
            completion_text = strip_code_completion(completion_text)
        return Code.from_string(completion_text or "pass")

from typing import TYPE_CHECKING, Mapping, Sequence, Union, override

import anthropic

from bench.language import Code, FileType, ModelType, RunOptions, download_file_batch
from bench.runtime.core import NotSupportedError
from bench.runtime.model.instruct import SYSTEM_PROMPT
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


from anthropic import NOT_GIVEN
from anthropic import types as anthropic_types

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
        content_parts: list[
            Union[anthropic_types.TextBlockParam, anthropic_types.ImageBlockParam]
        ] = []
        text_parts: list[str] = []

        def _flush_text() -> None:
            if text_parts:
                content_parts.append({"type": "text", "text": "\n".join(text_parts)})
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
                    if mime_type not in ("image/jpeg", "image/png", "image/gif", "image/webp"):
                        raise NotSupportedError(f"unsupported image mime type {mime_type!r}")
                    content_parts.append(
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": mime_type,
                                "data": part.file.content_b64,
                            },
                        }
                    )
                else:
                    raise NotSupportedError(f"file {part.file!r} not supported yet")
            else:
                raise RuntimeError(f"unexpected part {part!r}")

        _flush_text()

        # generate
        messages: list[anthropic_types.MessageParam] = [{"role": "user", "content": content_parts}]
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

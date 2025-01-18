from typing import TYPE_CHECKING, Mapping, Sequence, override

import anthropic

from bench.language import Code, ModelType, RunOptions
from bench.runtime.core import NotSupportedError
from bench.utils.utils import get_from_env

from .chat import ChatModelRunner, get_system_prompt, strip_code_completion
from .prompt import Prompt, PromptBreak, PromptElement, PromptFile, PromptText

if TYPE_CHECKING:
    pass


from anthropic import NOT_GIVEN
from anthropic import types as anthropic_types

anthropic_client = anthropic.AsyncClient(
    api_key=get_from_env("ANTHROPIC_API_KEY", description="Anthropic API key")
)
ANTHROPIC_MODEL_BY_TYPE: Mapping[ModelType, str] = {
    ModelType.ANTHROPIC_CLAUDE_3_5_SONNET: "claude-3-5-sonnet-20241022"
}
ANTHROPIC_DEFAULT_MODEL = ModelType.ANTHROPIC_CLAUDE_3_5_SONNET


class AnthropicChatModelRunner(ChatModelRunner[anthropic_types.MessageParam]):
    """Compile a Prompt into Anthropic chat messages."""

    SEPARATOR = "#" * 32  # = exactly 1 token

    @override
    async def assemble(self, parts: Sequence[PromptElement]) -> list[anthropic_types.MessageParam]:
        content: list[anthropic_types.MessageParam] = []
        for part in parts:
            if isinstance(part, PromptBreak):
                content.append({"role": "user", "content": self.SEPARATOR})
                if part.title:
                    content.append({"role": "user", "content": f"{part.title}"})
                    if part.text:
                        content.append({"role": "user", "content": part.text})
                    content.append({"role": "user", "content": self.SEPARATOR})
            elif isinstance(part, PromptText):
                text = part.text.to_string() if not isinstance(part.text, str) else part.text
                if part.title:
                    text = f"{part.title}\n{text}"
                content.append({"role": "user", "content": text})
            elif isinstance(part, PromptFile):
                raise NotSupportedError(f"file {part.file!r} not supported yet")
            else:
                raise RuntimeError(f"unexpected part {part!r}")
        return content

    @override
    async def generate(
        self,
        prompt: Prompt,
        model: ModelType,
        rendered_prompt: Sequence[anthropic_types.MessageParam],
        user_id: str,
        options: RunOptions,
    ) -> Code:
        model_id = ANTHROPIC_MODEL_BY_TYPE.get(model)
        if model_id is None:
            raise NotSupportedError(f"unsupported model type {model!r}")
        messages: list[anthropic_types.MessageParam] = [*rendered_prompt]
        temperature = options.text_options.temperature if options.text_options else None
        completion = await anthropic_client.messages.create(
            system=get_system_prompt(prompt),
            max_tokens=8192,
            model=model_id,
            messages=messages,
            temperature=temperature or NOT_GIVEN,
        )
        completion_text = getattr(completion.content[0], "text", None)
        if isinstance(completion_text, str):
            completion_text = strip_code_completion(completion_text)
        return Code.from_string(completion_text or "pass")

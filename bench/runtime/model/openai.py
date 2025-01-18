from typing import Mapping, Sequence, override

import openai
from openai.types import chat as openai_chat_types

from bench.language import Code, ModelType, RunOptions
from bench.runtime.core import NotSupportedError
from bench.utils.utils import get_from_env

from .chat import ChatModelRunner, get_system_prompt, strip_code_completion
from .prompt import Prompt, PromptBreak, PromptElement, PromptFile, PromptText

openai_client = openai.AsyncClient(
    api_key=get_from_env("OPENAI_API_KEY", description="OpenAI API key")
)
OPENAI_MODEL_BY_TYPE: Mapping[ModelType, str] = {
    ModelType.OPENAI_GPT4_0: "gpt-4o-2024-11-20",
    ModelType.OPENAI_GPT4_O_MINI: "gpt-4o-mini-2024-07-18",
    ModelType.OPENAI_O1_MINI: "o1-mini-2024-09-12",
    ModelType.OPENAI_O1: "o1-2024-12-17",
}
OPENAI_DEFAULT_MODEL = ModelType.OPENAI_GPT4_0


class OpenaiChatModelRunner(ChatModelRunner[openai_chat_types.ChatCompletionMessageParam]):
    """Compile a Prompt into OpenAI chat messages."""

    SEPARATOR = "#" * 32  # = exactly 1 token

    @override
    async def assemble(
        self, parts: Sequence[PromptElement]
    ) -> Sequence[openai_chat_types.ChatCompletionMessageParam]:
        content: list[openai_chat_types.ChatCompletionContentPartParam] = []
        for part in parts:
            if isinstance(part, PromptBreak):
                content.append({"type": "text", "text": self.SEPARATOR})
                if part.title:
                    content.append({"type": "text", "text": f"{part.title}"})
                    if part.text:
                        content.append({"type": "text", "text": part.text})
                    content.append({"type": "text", "text": self.SEPARATOR})
            elif isinstance(part, PromptText):
                text = part.text.to_string() if not isinstance(part.text, str) else part.text
                if part.title:
                    text = f"{part.title}\n{text}"
                content.append({"type": "text", "text": text})
            elif isinstance(part, PromptFile):
                raise NotSupportedError(f"file {part.file!r} not supported yet")
            else:
                raise RuntimeError(f"unexpected part {part!r}")
        return [{"role": "user", "content": content}]

    @override
    async def generate(
        self,
        prompt: Prompt,
        model: ModelType,
        rendered_prompt: Sequence[openai_chat_types.ChatCompletionMessageParam],
        user_id: str,
        options: RunOptions,
    ) -> Code:
        model_id = OPENAI_MODEL_BY_TYPE.get(model)
        if model_id is None:
            raise NotSupportedError(f"unsupported model type {model!r}")
        messages: list[openai_chat_types.ChatCompletionMessageParam] = [
            {"role": "developer", "content": get_system_prompt(prompt)},
            *rendered_prompt,
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

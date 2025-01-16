from typing import TYPE_CHECKING, Mapping, Sequence, override

import anthropic

from bench.language import ModelType, RunOptions
from bench.runtime.core import NotSupportedError
from bench.utils.utils import get_from_env

from .chat import ChatModelRunner
from .prompt import Prompt, PromptElement

if TYPE_CHECKING:
    pass


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

    @override
    async def assemble(self, parts: Sequence[PromptElement]) -> list[anthropic_types.MessageParam]:
        raise NotImplementedError

    @override
    async def generate(
        self,
        prompt: Prompt,
        model: ModelType,
        rendered_prompt: Sequence[anthropic_types.MessageParam],
        user_id: str,
        options: RunOptions,
    ) -> str:
        assert model in ANTHROPIC_MODEL_BY_TYPE, f"unsupported model type {model!r}"
        raise NotSupportedError("Anthropic is not yet supported")

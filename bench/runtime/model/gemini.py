from typing import TYPE_CHECKING, Any, Mapping, Sequence, override

from bench.language import Code, ModelType, RunOptions
from bench.runtime.core import NotSupportedError

from .chat import ChatModelRunner
from .prompt import Prompt, PromptElement

if TYPE_CHECKING:
    pass


# gemini_client = genai.AsyncClient(
#     api_key=get_from_env("GEMINI_API_KEY", description="Gemini API key")
# )
GEMINI_MODEL_BY_TYPE: Mapping[ModelType, str] = {}
GEMINI_DEFAULT_MODEL = ModelType.GOOGLE_GEMINI_2_0_FLASH

MessagePart = Any  # ???


class GeminiChatModelRunner(ChatModelRunner):
    """Compile a Prompt into Gemini chat messages."""

    @override
    async def generate(
        self,
        prompt: Prompt,
        parts: Sequence[PromptElement],
        model: ModelType,
        user_id: str,
        options: RunOptions,
    ) -> Code:
        assert model in GEMINI_MODEL_BY_TYPE, f"unsupported model type {model!r}"
        raise NotSupportedError("Gemini is not yet supported")

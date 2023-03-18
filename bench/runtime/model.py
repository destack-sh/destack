import typing
import uuid
from dataclasses import dataclass, field
from typing import Any, Callable, Optional
from uuid import UUID

import openai
import PIL.Image
import pydub
import structlog

from bench.language.type import XBlock, XSource
from bench.runtime.type import ModelInstance

logger = structlog.get_logger(__name__)


@dataclass(repr=False)
class InferenceContext:
    model: ModelInstance
    n: int
    user_opaque_id: str
    streaming_callback: Optional[Callable[[XBlock], None]]
    id: UUID = field(default_factory=uuid.uuid4)

    @property
    def streaming(self) -> bool:
        return self.streaming_callback is not None


InferenceEndpoint = typing.Callable[[InferenceContext, ...], typing.Awaitable[Any]]


@dataclass
class OpenAITextModel:
    complete_chat: Optional["OpenAIChatCompletion"]
    complete_text: Optional["OpenAITextCompletion"]
    embed_text: Optional["OpenAITextEmbedding"]


@dataclass
class OpenAIChatCompletionSettings:
    temperature: float
    max_tokens: int
    top_p: Optional[float]
    stop: Optional[list[str]]
    presence_penalty: Optional[float]
    frequency_penalty: Optional[float]
    logit_bias: Optional[dict[str, float]]


@dataclass
class OpenAIChatCompletion:
    ctx: InferenceContext

    async def __call__(
        self,
        input: list[XBlock[str, None]],
        output: XBlock[None, OpenAIChatCompletionSettings],
    ) -> str:
        role_map = {
            XSource.System: "system",
            XSource.Developer: "developer",
            XSource.User: "user",
            XSource.Model: "assistant",
        }
        messages = [{"role": role_map[x.source], "content": x.value} for x in input]
        response = await openai.Completion.create(
            model=self.ctx.model.external_name,
            messages=messages,
            temperature=output.settings.temperature,
            max_tokens=output.settings.max_tokens,
            top_p=output.settings.top_p,
            stop=output.settings.stop,
            presence_penalty=output.settings.presence_penalty,
            frequency_penalty=output.settings.frequency_penalty,
            logit_bias=output.settings.logit_bias,
            user=self.ctx.user_opaque_id,
        )
        text = response["choices"][0]["message"]["content"]
        return text


@dataclass
class OpenAITextCompletionSettings:
    temperature: float
    max_tokens: int
    top_p: Optional[float]
    top_k: Optional[int]
    stop: Optional[list[str]]
    logit_bias: Optional[dict[str, float]]


@dataclass
class OpenAITextCompletion:
    ctx: InferenceContext

    async def __call__(
        self,
        input: list[XBlock[str, None]],
        output: XBlock[None, OpenAITextCompletionSettings],
    ) -> str:
        role_map = {
            XSource.System: "system",
            XSource.Developer: "developer",
            XSource.User: "user",
            XSource.Model: "assistant",
        }
        messages = [{"role": role_map[x.source], "content": x.value} for x in input]
        prompt = "\n".join(f"{x['role']}: {x['content']}" for x in messages)
        response = await openai.Completion.create(
            model=self.ctx.model.external_name,
            prompt=prompt,
            max_tokens=output.settings.max_tokens,
            temperature=output.settings.temperature,
            top_p=output.settings.top_p,
            top_k=output.settings.top_k,
            stop=output.settings.stop,
            logit_bias=output.settings.logit_bias,
            user=self.ctx.user_opaque_id,
        )
        text = response["choices"][0]["text"]
        return text


@dataclass
class OpenAITextEmbedding:
    ctx: InferenceContext

    async def __call__(self, input: XBlock[str, None]) -> list[float]:
        rep = await openai.Embedding.acreate(input.value, model=self.ctx.model.external_name)
        return rep["data"][0]["embedding"]


@dataclass
class OpenAIAudioModel:
    transcribe: "OpenAIAudioTranscription"


@dataclass
class OpenAIAudioTranscriptionSettings:
    prompt: Optional[str]


@dataclass
class OpenAIAudioTranscription:
    ctx: InferenceContext

    async def __call__(
        self,
        input: XBlock[pydub.AudioSegment, None],
        output: XBlock[None, OpenAIAudioTranscriptionSettings],
    ) -> str:
        raise NotImplementedError


@dataclass
class AnthropicTextModel:
    complete_text: "AnthropicTextCompletion"


@dataclass
class AnthropicTextCompletionSettings:
    temperature: float
    max_tokens: int
    top_p: Optional[float]
    top_k: Optional[int]
    stop: Optional[list[str]]
    logit_bias: Optional[dict[str, float]]


@dataclass
class AnthropicTextCompletion:
    ctx: InferenceContext

    async def __call__(
        self,
        input: list[XBlock[str, None]],
        output: XBlock[None, AnthropicTextCompletionSettings],
    ) -> str:
        raise NotImplementedError


@dataclass
class StabilityAIImageGenerationSettings:
    seed: int
    steps: int
    width: int
    height: int
    cfg_scale: float


@dataclass
class StabilityAIImageGenerationInput:
    prompt: str
    init_image: Optional[PIL.Image.Image]


class StabilityAIImageGeneration:
    async def __call__(
        self,
        ctx: InferenceContext,
        input: XBlock[StabilityAIImageGenerationInput, None],
        output: XBlock[None, StabilityAIImageGenerationSettings],
    ) -> PIL.Image:
        raise NotImplementedError

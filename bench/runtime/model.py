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
from bench.runtime.type import ModelInference, ModelInstance, TextGenerationSettings

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
class OpenAIChatCompletionSettings(TextGenerationSettings):
    temperature: float
    max_tokens: int
    top_p: Optional[float]
    stop: Optional[list[str]]
    presence_penalty: Optional[float]
    frequency_penalty: Optional[float]
    logit_bias: Optional[dict[str, float]]


@dataclass
class OpenAIChatCompletion(ModelInference):
    ctx: InferenceContext

    async def generate_text(
        self,
        input: list[XBlock[str]],
        settings: OpenAIChatCompletionSettings,
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
            temperature=settings.temperature,
            max_tokens=settings.max_tokens,
            top_p=settings.top_p,
            stop=settings.stop,
            presence_penalty=settings.presence_penalty,
            frequency_penalty=settings.frequency_penalty,
            logit_bias=settings.logit_bias,
            user=self.ctx.user_opaque_id,
        )
        text = response["choices"][0]["message"]["content"]
        return text


@dataclass
class OpenAITextCompletionSettings(TextGenerationSettings):
    temperature: float
    max_tokens: int
    top_p: Optional[float]
    top_k: Optional[int]
    stop: Optional[list[str]]
    logit_bias: Optional[dict[str, float]]


@dataclass
class OpenAITextCompletion(ModelInference):
    ctx: InferenceContext

    async def generate_text(
        self,
        input: list[XBlock[str]],
        settings: OpenAITextCompletionSettings,
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
            max_tokens=settings.max_tokens,
            temperature=settings.temperature,
            top_p=settings.top_p,
            top_k=settings.top_k,
            stop=settings.stop,
            logit_bias=settings.logit_bias,
            user=self.ctx.user_opaque_id,
        )
        text = response["choices"][0]["text"]
        return text


@dataclass
class OpenAITextEmbedding(ModelInference):
    ctx: InferenceContext

    async def embed_text(self, input: XBlock[str]) -> list[float]:
        rep = await openai.Embedding.acreate(input.value, model=self.ctx.model.external_name)
        return rep["data"][0]["embedding"]


@dataclass
class OpenAIAudioTranscription(ModelInference):
    ctx: InferenceContext

    async def transcribe_audio(
        self,
        input: XBlock[pydub.AudioSegment | str],
    ) -> str:
        raise IncapableError


@dataclass
class AnthropicTextCompletionSettings(TextGenerationSettings):
    temperature: float
    max_tokens: int
    top_p: Optional[float]
    top_k: Optional[int]
    stop: Optional[list[str]]
    logit_bias: Optional[dict[str, float]]


@dataclass
class AnthropicTextCompletion(ModelInference):
    ctx: InferenceContext

    async def generate_text(
        self,
        input: list[XBlock[str]],
        settings: AnthropicTextCompletionSettings,
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


@dataclass(repr=False)
class StabilityAIImageGeneration(ModelInference):
    ctx: InferenceContext

    async def generate_image(
        self,
        input: XBlock[StabilityAIImageGenerationInput],
        settings: StabilityAIImageGenerationSettings,
    ) -> PIL.Image:
        raise NotImplementedError

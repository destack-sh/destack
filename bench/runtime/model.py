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
from bench.runtime.type import (
    IncapableError,
    ModelInference,
    ModelInstance,
    TextGenerationSettings,
    Modality,
)

logger = structlog.get_logger(__name__)


@dataclass(repr=False)
class InferenceContext:
    model: ModelInstance
    modality: Modality
    n: int
    user_opaque_id: str
    streaming_callback: Optional[Callable[[XBlock], None]]
    id: UUID = field(default_factory=uuid.uuid4)

    @property
    def streaming(self) -> bool:
        return self.streaming_callback is not None


InferenceEndpoint = typing.Callable[[InferenceContext, ...], typing.Awaitable[Any]]


@dataclass
class OpenAIChatCompletion(ModelInference):
    ctx: InferenceContext

    async def generate_text(
        self,
        input: list[XBlock[str]],
        settings: TextGenerationSettings,
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
            logit_bias=settings.logit_bias,
            user=self.ctx.user_opaque_id,
        )
        text = response["choices"][0]["message"]["content"]
        return text


@dataclass
class OpenAITextCompletion(ModelInference):
    ctx: InferenceContext

    async def generate_text(
        self,
        input: list[XBlock[str]],
        settings: TextGenerationSettings,
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
            stop=settings.stop,
            logit_bias=settings.logit_bias,
            user=self.ctx.user_opaque_id,
        )
        text = response["choices"][0]["text"]
        return text


@dataclass
class OpenAITextEmbedding(ModelInference):
    ctx: InferenceContext

    async def embed(self, input: list[XBlock[str]], settings: None) -> list[float]:
        prompt = "\n".join(x.value for x in input)
        rep = await openai.Embedding.acreate(prompt, model=self.ctx.model.external_name)
        return rep["data"][0]["embedding"]


@dataclass
class OpenAIAudioTranscription(ModelInference):
    ctx: InferenceContext

    async def generate_text(
        self,
        input: list[XBlock[pydub.AudioSegment | str]],
        settings: None,
    ) -> str:
        raise IncapableError


@dataclass
class AnthropicTextCompletion(ModelInference):
    ctx: InferenceContext

    async def generate_text(
        self,
        input: list[XBlock[str]],
        settings: TextGenerationSettings,
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

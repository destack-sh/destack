import typing
from dataclasses import dataclass
from typing import Any, Callable, Optional

import openai
import PIL.Image
import pydub
import structlog

from bench.language.type import XBlock
from bench.runtime.type import ModelInstance, TextGeneration

logger = structlog.get_logger(__name__)


@dataclass(repr=False)
class InferenceContext:
    model: ModelInstance
    streaming_callback: Optional[Callable[[XBlock], None]]

    @property
    def streaming(self) -> bool:
        return self.streaming_callback is not None


InferenceEndpoint = typing.Callable[[Any], typing.Coroutine[Any]]


@dataclass
class OpenAITextCompletionSettings:
    temperature: float
    max_tokens: int
    stop: Optional[list[str]]


class OpenAIChatCompletion:
    async def __call__(
        self,
        input: list[XBlock[str]],
        output: XBlock[str, OpenAITextCompletionSettings],
        ctx: InferenceContext,
    ) -> TextGeneration:
        raise NotImplementedError


class OpenAITextCompletion:
    async def __call__(
        self,
        input: list[XBlock[str]],
        output: XBlock[str, OpenAITextCompletionSettings],
        ctx: InferenceContext,
    ) -> TextGeneration:
        raise NotImplementedError


@dataclass
class OpenAIAudioTranscriptionSettings:
    prompt: Optional[str]


class OpenAIAudioTranscription:
    async def __call__(
        self,
        input: XBlock[pydub.AudioSegment, OpenAIAudioTranscriptionSettings],
        ctx: InferenceContext,
    ) -> str:
        raise NotImplementedError


class OpenAITextEmbedding:
    async def __call__(self, input: XBlock[str], ctx: InferenceContext) -> list[float]:
        rep = await openai.Embedding.acreate(input.value, model=ctx.model.external_name)
        return rep["data"][0]["embedding"]


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
        input: XBlock[StabilityAIImageGenerationInput, StabilityAIImageGenerationSettings],
        ctx: InferenceContext,
    ) -> PIL.Image:
        raise NotImplementedError

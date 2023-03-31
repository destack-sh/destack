import os
import typing
from dataclasses import dataclass
from typing import Any, Callable, Optional

import anthropic
import openai
import PIL.Image
import pydub
import structlog

from bench.language.type import Model, XBlock, XSource
from bench.runtime.type import (
    ImageGenerationSettings,
    IncapableError,
    Modality,
    ModelInference,
    TextGenerationSettings,
)

logger = structlog.get_logger(__name__)


@dataclass(repr=False)
class InferenceContext:
    model: Model
    user_opaque_id: Optional[str] = None
    streaming_callback: Optional[Callable[[XBlock], None]] = None

    @property
    def streaming(self) -> bool:
        return self.streaming_callback is not None


InferenceEndpoint = typing.Callable[[..., Any], typing.Awaitable[Any]]

endpoints: dict[(str, Modality), InferenceEndpoint] = {}


def endpoint(models: list[str], *modalities: Modality):
    """Register an inference endpoint for a model and set of modalities."""

    def decorator(fn: InferenceEndpoint) -> InferenceEndpoint:
        for model in models:
            for modality in modalities:
                key = model, modality
                if key in endpoints:
                    raise ValueError(f"duplicate endpoint for {key}: {fn} != {endpoints[key]}")
                endpoints[key] = fn
        return dataclass(fn)

    return decorator


def get_endpoint(model: Model, modality: Modality) -> InferenceEndpoint:
    key = model.fqn, modality
    if key not in endpoints:
        raise IncapableError(f"no endpoint for {key}")
    return endpoints[key]


def get_endpoints(model: Model) -> list[tuple[Modality, InferenceEndpoint]]:
    for (fqn, modality), endpoint in endpoints.items():
        if fqn == model.fqn:
            yield modality, endpoint


@endpoint(["openai.std.text.gpt4", "openai.std.text.gpt-3-5-turbo"], Modality.GenerateText)
class OpenAIChatCompletion(ModelInference):
    ctx: InferenceContext

    async def generate_text(
        self,
        input: list[XBlock[str]],
        settings: TextGenerationSettings,
    ) -> str:
        role_map = {
            XSource.System: "system",
            XSource.Developer: "system",
            XSource.User: "user",
            XSource.Model: "assistant",
        }
        messages = [{"role": role_map[x.source], "content": x.value} for x in input]
        response = await openai.ChatCompletion.acreate(
            model=self.ctx.model.external_name,
            messages=messages,
            temperature=settings.temperature,
            max_tokens=settings.max_tokens,
            top_p=settings.top_p,
            stop=settings.stop or None,
            logit_bias=settings.logit_bias,
            user=self.ctx.user_opaque_id,
        )
        text = response["choices"][0]["message"]["content"]
        return text


@endpoint(
    ["openai.std.text.text-davinci-003", "openai.std.text.text-ada-001"], Modality.GenerateText
)
class OpenAITextCompletion(ModelInference):
    ctx: InferenceContext

    async def generate_text(
        self,
        input: list[XBlock[str]],
        settings: TextGenerationSettings,
    ) -> str:
        role_map = {
            XSource.System: "System",
            XSource.Developer: "Developer",
            XSource.User: "User",
            XSource.Model: "Assistant",
        }
        messages = [{"role": role_map[x.source], "content": x.value} for x in input]
        messages.append({"role": role_map[XSource.Model], "content": ""})  # empty assistant prompt
        prompt = "\n\n".join(f"{x['role']}: {x['content']}" for x in messages)
        response = await openai.Completion.acreate(
            model=self.ctx.model.external_name,
            prompt=prompt,
            max_tokens=settings.max_tokens,
            temperature=settings.temperature,
            top_p=settings.top_p,
            stop=settings.stop or None,
            logit_bias=settings.logit_bias,
            user=self.ctx.user_opaque_id,
        )
        text = response["choices"][0]["text"]
        return text


@endpoint(["openai.std.text.text-ada-001"], Modality.Embed)
class OpenAITextEmbedding(ModelInference):
    ctx: InferenceContext

    async def embed(self, input: list[XBlock[str]], settings: None) -> list[float]:
        prompt = "\n\n".join(x.value for x in input)
        rep = await openai.Embedding.acreate(prompt, model=self.ctx.model.external_name)
        return rep["data"][0]["embedding"]


@endpoint(["openai.std.audio.whisper"], Modality.GenerateText)
class OpenAIAudioTranscription(ModelInference):
    ctx: InferenceContext

    async def generate_text(
        self,
        input: list[XBlock[pydub.AudioSegment | str]],
        settings: None,
    ) -> str:
        raise IncapableError()


@endpoint(["anthropic.std.text.claude", "anthropic.std.text.claude-instant"], Modality.GenerateText)
class AnthropicTextCompletion(ModelInference):
    ctx: InferenceContext

    def __init__(self):
        self.client = anthropic.Client(os.environ["ANTHROPIC_API_KEY"])

    async def generate_text(
        self,
        input: list[XBlock[str]],
        settings: TextGenerationSettings,
    ) -> str:
        # see https://console.anthropic.com/docs/api
        role_map = {
            XSource.System: "System",
            XSource.Developer: "Developer",
            XSource.User: "Human",
            XSource.Model: "Assistant",
        }
        messages = [{"role": role_map[x.source], "content": x.value} for x in input]
        messages.append({"role": role_map[XSource.Model], "content": ""})  # empty assistant prompt
        prompt = "\n\n".join(f"{x['role']}: {x['content']}" for x in messages)
        rep = await self.client.acompletion(
            prompt=prompt,
            model=self.ctx.model.external_name,
            stop_sequences=[anthropic.HUMAN_PROMPT, *(settings.stop or [])],
            temperature=settings.temperature,
            max_tokens_to_sample=settings.max_tokens,
            top_p=settings.top_p,
        )
        return rep["completion"]


@endpoint(["stabilityai.std.image.stable-diffusion"], Modality.GenerateImage)
class StabilityAIImageGeneration(ModelInference):
    ctx: InferenceContext

    async def generate_image(
        self,
        input: list[XBlock[str | PIL.Image.Image]],
        settings: ImageGenerationSettings,
    ) -> PIL.Image:
        raise NotImplementedError

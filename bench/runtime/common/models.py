import os
from dataclasses import dataclass

import anthropic
import openai
import PIL.Image
import structlog

from bench.language import Model
from bench.language.build import XSource, XBlock
from bench.language.inference import (
    Modality,
    InferenceEndpoint,
    ModelInference,
    TextGenerationSettings,
    ImageGenerationSettings,
)

logger = structlog.get_logger(__name__)

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


def get_inference_endpoint_cls(model: Model | str, modality: Modality) -> InferenceEndpoint:
    if not isinstance(model, str):
        model = model.fqn
    return endpoints[model, modality]


def get_model_key_from_env(model: Model | str) -> str:
    # maps model fqn to PROVIDER_API_KEY
    if not isinstance(model, str):
        model = model.fqn
    provider = model.split(".")[0]
    return os.environ[f"{provider.upper()}_API_KEY"]


def get_inference_endpoint(
    model: Model | str, modality: Modality, external_name: str, key: str
) -> InferenceEndpoint:
    if not isinstance(model, str):
        model = model.fqn
    return get_inference_endpoint_cls(model, modality)(external_name=external_name, key=key)


def get_inference_endpoints_cls(model: Model | str) -> list[tuple[Modality, InferenceEndpoint]]:
    if not isinstance(model, str):
        model = model.fqn
    for (fqn, modality), endpoint in endpoints.items():
        if fqn == model:
            yield modality, endpoint


@endpoint(["openai.std.text.gpt4", "openai.std.text.gpt3"], Modality.GenerateText)
class OpenAIChatCompletion(ModelInference):
    role_map = {
        XSource.System: "system",
        XSource.Developer: "assistant",
        XSource.User: "user",
        XSource.Model: "assistant",
    }

    async def generate_text(
        self,
        input: list[XBlock[str]],
        settings: TextGenerationSettings,
    ) -> str:
        messages = [{"role": self.role_map[x.source], "content": x.value} for x in input]
        response = await openai.ChatCompletion.acreate(
            model=self.external_name,
            messages=messages,
            temperature=settings.temperature,
            max_tokens=settings.max_tokens,
            top_p=settings.top_p,
            stop=settings.stop or None,
            logit_bias=settings.logit_bias,
            api_key=self.key,
        )
        text = response["choices"][0]["message"]["content"]
        return text


# no endpoints currently
class OpenAITextCompletion(ModelInference):
    role_map = {
        XSource.System: "System",
        XSource.Developer: "Developer",
        XSource.User: "User",
        XSource.Model: "Assistant",
    }

    async def generate_text(
        self,
        input: list[XBlock[str]],
        settings: TextGenerationSettings,
    ) -> str:
        messages = [{"role": self.role_map[x.source], "content": x.value} for x in input]
        prompt = "\n\n".join(f"{x['role']}: {x['content']}" for x in messages) + "\n\nAssistant:"
        response = await openai.Completion.acreate(
            model=self.external_name,
            prompt=prompt,
            max_tokens=settings.max_tokens,
            temperature=settings.temperature,
            top_p=settings.top_p,
            stop=settings.stop or None,
            logit_bias=settings.logit_bias,
            api_key=self.key,
        )
        text = response["choices"][0]["text"]
        return text


@endpoint(["openai.std.text.text-ada-001"], Modality.Embed)
class OpenAITextEmbedding(ModelInference):
    async def embed(self, input: list[XBlock[str]], settings: None) -> list[float]:
        prompt = "\n\n".join(x.value for x in input)
        rep = await openai.Embedding.acreate(prompt, model=self.external_name, api_key=self.key)
        return rep["data"][0]["embedding"]


@endpoint(["openai.std.audio.whisper"], Modality.GenerateText)
class OpenAIAudioTranscription(ModelInference):
    pass


@endpoint(["anthropic.std.text.claude", "anthropic.std.text.claude-instant"], Modality.GenerateText)
class AnthropicTextCompletion(ModelInference):
    role_map = {
        XSource.System: "Human",
        XSource.Developer: "Human",
        XSource.User: "Human",
        XSource.Model: "Assistant",
    }

    def __post_init__(self):
        self.client = anthropic.Client(self.key)

        # monkey patch Anthropic's validation (which is broken)
        from anthropic import api

        api._validate_prompt_length = lambda *args, **kwargs: None

    async def generate_text(
        self,
        input: list[XBlock[str]],
        settings: TextGenerationSettings,
    ) -> str:
        # see https://console.anthropic.com/docs/api
        messages = [{"role": self.role_map[x.source], "content": x.value} for x in input]
        prompt = (
            "\n\n"
            + "\n\n".join(f"{x['role']}: {x['content']}" for x in messages)
            + "\n\nAssistant:"
        )
        rep = await self.client.acompletion(
            prompt=prompt,
            model=self.external_name,
            stop_sequences=[anthropic.HUMAN_PROMPT, *(settings.stop or [])],
            temperature=settings.temperature,
            max_tokens_to_sample=settings.max_tokens,
            top_p=settings.top_p,
        )
        return rep["completion"]


@endpoint(["stabilityai.std.image.stable-diffusion"], Modality.GenerateImage)
class StabilityAIImageGeneration(ModelInference):
    model: Model

    async def generate_image(
        self,
        input: list[XBlock[str | PIL.Image.Image]],
        settings: ImageGenerationSettings,
    ) -> PIL.Image:
        raise NotImplementedError

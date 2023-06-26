import asyncio
import enum
import hashlib
import json
import typing
from dataclasses import asdict, dataclass
from datetime import datetime
from json import JSONDecodeError
from logging import Logger
from typing import Any, Optional

import anthropic
import openai
import pytz
import structlog

from bench.bench.core import Scope, Symbol, node
from bench.bench.remote import RemoteObject
from bench.utils.cache import redis
from bench.utils.func import describe_type
from bench.utils.utils import get_from_env, required_field

if typing.TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)

INFERENCE_CACHE_EXPIRY = get_from_env("INFERENCE_CACHE_EXPIRY", 60 * 60 * 24 * 30, type_cast=int)


@node
class Model(Symbol):
    external_name: str = required_field()
    _is_async: bool = True
    _remote: bool = False
    _endpoint: Optional["ModelEndpoint"] = None

    def _clear(self) -> None:
        self._endpoint = None

    def _interp(self, scope: Scope) -> None:
        pass

    def __str__(self):
        return f"{self.external_name}"

    async def __call__(self, timeout: int = None, cache: bool = True, **inputs):
        if self._endpoint is None and not self._remote:
            self._endpoint = get_model_endpoint(self.model.fqn)

        cache_key = get_inference_cache_key(self.model.fqn, inputs)
        log = logger.bind(
            model=self.model.fqn,
            inputs=describe_type(inputs),
            cache_key=cache_key,
            cache_inferences=self.cache_inferences,
        )

        # try to read from cache if enabled
        if self.cache_inferences and cache is not False:
            # TODO @Performance: use leases to cooperatively inference endpoints
            cached_inference = await redis.get(cache_key)
            if cached_inference is not None:
                try:
                    inference = Inference.from_json_str(cached_inference)
                    log.debug("inference.cache.hit", output=describe_type(inference.output))
                    self.tracer.inference_cached(self.model, inputs, inference)
                    return inference.output
                except (ValueError, TypeError, JSONDecodeError):
                    log.warning("inference.cache.error", excinfo=True)
                    # ignore and continue, will be overwritten

        # request remote inference if needed
        if self._remote:
            from bench.msg.core import NMessage, request
            from bench.msg.messages import (
                NMessageType,
                RepRunInferencePayload,
                ReqRunInferencePayload,
            )

            timeout = timeout if timeout is not None else self.timeout
            rep: NMessage[RepRunInferencePayload] = await request(
                NMessageType.REQUEST_RUN_INFERENCE,
                ReqRunInferencePayload(model_fqn=self.model.fqn, inputs=inputs, timeout=timeout),
                RepRunInferencePayload,
                timeout=timeout + 1,  # for network
            )
            if rep.p.output is None:
                raise RuntimeError("remote inference failed")
            return rep.p.output

        # otherwise run inference through endpoint
        try:
            self.tracer.inference_enter(self.model, inputs)
            timeout = timeout if timeout is not None else self.timeout
            result = await asyncio.wait_for(
                asyncio.shield(run_inference(self.endpoint, inputs, cache_key, log)),
                timeout,
            )
            self.tracer.inference_exit(self.model, inputs, result)
            return result
        except Exception as exception:
            self.tracer.inference_exception(self.model, inputs, exception)
            log.debug("inference.exception", exc_info=True)
            raise

    def to_sync(self):
        raise NotImplementedError


class IncapableError(NotImplementedError):
    pass


@dataclass
class ModelEndpoint:
    model: str
    key: str | None

    async def __call__(self, **kwargs) -> Any:
        raise NotImplementedError


model_endpoints: dict[str, type[ModelEndpoint]] = {}


def get_model_endpoint(fqn: str) -> ModelEndpoint:
    if fqn not in model_endpoints:
        raise RuntimeError(f"no model endpoint registered for {fqn}")
    return model_endpoints[fqn](fqn)


def model(fqns: list[str]):
    """Registers a model endpoint."""

    def decorator(endpoint: type[ModelEndpoint]) -> type[ModelEndpoint]:
        for fqn in fqns:
            if fqn in model_endpoints:
                raise RuntimeError(f"model endpoint already exists {fqn}: {model_endpoints[fqn]}")
            model_endpoints[fqn] = endpoint
        return endpoint

    return decorator


@dataclass(slots=True)
class Inference:
    """A model inference."""

    generated_at: datetime
    duration: float
    inputs: Any
    output: Any

    def to_json_str(self) -> str:
        inference_json = {
            "generated_at": self.generated_at.isoformat(),
            "duration": self.duration,
            "inputs": self.inputs,
            "output": self.output,
        }
        return json.dumps(inference_json)

    @classmethod
    def from_json_str(cls, json_str: str):
        data = json.loads(json_str)
        return cls(
            generated_at=datetime.fromisoformat(data["generated_at"]),
            duration=data["duration"],
            inputs=data["inputs"],
            output=data["output"],
        )


InferenceEndpoint = typing.Callable[[..., Any], typing.Awaitable[Any]]


async def run_inference(
    endpoint: InferenceEndpoint,
    inputs: Any,
    cache_key: str,
    log: Logger = logger,
    write_to_cache: bool = True,
) -> Any:
    """
    Runs inference on the given endpoint without timeout.
    This should be asyncio.shield-ed to ensure we write the result to cache.
    """
    started_at = datetime.utcnow().replace(tzinfo=pytz.utc)
    log.debug("inference.enter")
    output = await endpoint(**inputs)
    now = datetime.utcnow().replace(tzinfo=pytz.utc)
    duration = (now - started_at).total_seconds()
    if write_to_cache:
        # result is assumed to be JSON serializable, will obviously error here if not
        inference = Inference(generated_at=now, duration=duration, inputs=inputs, output=output)
        await redis.set(cache_key, inference.to_json_str(), ex=INFERENCE_CACHE_EXPIRY)
    log.debug("inference.exit", ret=describe_type(output), write_to_cache=write_to_cache)
    return output


def get_inference_cache_key(model_fqn: str, inputs: Any):
    input_hash = hashlib.sha256(json.dumps(inputs).encode("utf-8")).hexdigest()
    cache_key = f"inference.{model_fqn}.{input_hash}"
    return cache_key


class OpenAIChatRole(enum.StrEnum):
    system = "system"
    developer = "developer"
    assistant = "assistant"
    user = "user"
    function = "function"


@dataclass
class OpenAIChatMessage:
    role: OpenAIChatRole
    content: str


@dataclass
class OpenAIChatCompletionSettings:
    temperature: float = 1.0
    max_tokens: int = None
    top_p: float = 1.0
    stop: Optional[str] = None
    logit_bias: Optional[dict[str, float]] = None
    frequence_penalty: float = 0.0
    presence_penalty: float = 0.0
    function_call: Optional[str] = None
    user: Optional[str] = None


@dataclass
class OpenAIFunction:
    name: str
    description: str
    parameters: "OpenAIFunctionParameter"


@dataclass
class OpenAIFunctionParameter:
    type: str
    description: str
    properties: dict[str, "OpenAIFunctionParameter"] | None = None
    enum: list[str] | None = None
    required: list[str] | None = None


@dataclass
class OpenAIFunctionCall:
    name: str
    parameters: dict[str, Any]


@dataclass
class OpenAITokenUsage:
    prompt_tokens: int
    completion_tokens: Optional[int]
    total_tokens: int


@dataclass
class OpenAIChatCompletion:
    text: Optional[str]
    function_call: Optional[OpenAIFunctionCall]
    usage: OpenAITokenUsage


@model(["openai.std.text.gpt4", "openai.std.text.gpt3"])
class OpenAIChatCompletionEndpoint(ModelEndpoint):
    async def __call__(
        self,
        messages: list[OpenAIChatMessage],
        functions: dict[str, OpenAIFunction],
        settings: OpenAIChatCompletionSettings,
    ) -> OpenAIChatCompletion:
        response = await openai.ChatCompletion.acreate(
            model=self.model,
            messages=[asdict(m) for m in messages],
            temperature=settings.temperature,
            max_tokens=settings.max_tokens,
            top_p=settings.top_p,
            stop=settings.stop or None,
            logit_bias=settings.logit_bias,
            api_key=self.key,
        )
        response_message = response["choices"][0]["message"]
        text = response_message.get("text")
        function_call = response_message.get("function_call")
        return OpenAIChatCompletion(
            text=text,
            function_call=function_call,
            usage=OpenAITokenUsage(
                prompt_tokens=response["usage"]["prompt_tokens"],
                completion_tokens=response["usage"].get("completion_tokens"),
                total_tokens=response["usage"]["total_tokens"],
            ),
        )


@dataclass
class OpenAITextEmbedding:
    embedding: Optional[list[float]]
    embeddings: Optional[list[list[float]]]
    usage: OpenAITokenUsage


@model(["openai.std.text.ada"])
class OpenAITextEmbeddingEndpoint(ModelEndpoint):
    async def __call__(self, text: str | list[str]) -> OpenAITextEmbedding:
        rep = await openai.Embedding.acreate(text, model=self.model, api_key=self.key)
        if isinstance(text, str):
            embedding = rep["data"][0]["embedding"]
            embeddings = None
        else:
            embedding = None
            embeddings = [d["embedding"] for d in rep["data"]]
        return OpenAITextEmbedding(
            embedding=embedding,
            embeddings=embeddings,
            usage=OpenAITokenUsage(
                prompt_tokens=rep["usage"]["prompt_tokens"],
                completion_tokens=rep["usage"].get("completion_tokens"),
                total_tokens=rep["usage"]["total_tokens"],
            ),
        )


class OpenAIAudioTranscription(ModelEndpoint):
    async def __call__(self, audio: RemoteObject) -> str:
        raise NotImplementedError


@dataclass
class AnthropicTextCompletionSettings:
    temperature: float = 1.0
    top_p: Optional[float] = None
    top_k: Optional[int] = None
    max_tokens_to_sample: int = 64
    stop_sequences: list[str] | None = None


@dataclass
class AnthropicTextCompletion:
    stop_reason: str


@model(
    [
        "anthropic.std.text.claude-1",
        "anthropic.std.text.claude-1-100k",
        "anthropic.std.text.clause-instant-1",
        "anthropic.std.text.clause-instant-1-100k",
    ]
)
class AnthropicTextCompletionEndpoint(ModelEndpoint):
    def __post_init__(self):
        self.client = anthropic.Client(self.key)

        # monkey patch Anthropic's validation (which is broken)
        from anthropic import api

        api._validate_prompt_length = lambda *args, **kwargs: None

    async def __call__(
        self, prompt: str, settings: AnthropicTextCompletionSettings
    ) -> AnthropicTextCompletion:
        # see https://console.anthropic.com/docs/api
        rep = await self.client.acompletion(
            prompt=prompt,
            model=self.model,
            stop_sequences=[anthropic.HUMAN_PROMPT, *(settings.stop or [])],
            temperature=settings.temperature,
            max_tokens_to_sample=settings.max_tokens_to_sample,
            top_p=settings.top_p,
        )
        return rep["completion"]

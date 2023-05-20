import asyncio
import enum
import hashlib
import json
import typing
from dataclasses import asdict, dataclass, field
from datetime import datetime
from json import JSONDecodeError
from typing import Any, Optional

import pytz
import structlog

from bench.language import Model, XBlock
from bench.utils.cache import redis
from bench.utils.func import describe_type
from bench.utils.utils import get_from_env

if typing.TYPE_CHECKING:
    from bench.runtime.tracing import Tracer

logger = structlog.get_logger(__name__)


#
# Model inference
#

# Ideally, endpoint settings should be 1) extensible and 2) types in the std lib.
# For now, we just use internal dataclasses. :TypeSafeSettings


class IncapableError(NotImplementedError):
    pass


class Modality(enum.StrEnum):
    """Core modality capabilities of a model."""

    GenerateText = "generate_text"  # any -> text
    GenerateImage = "generate_image"  # any -> image
    GenerateAudio = "generate_audio"  # any -> audio
    Embed = "embed"  # any -> embedding
    Struct = "struct"  # any -> struct(ture prediction)


@dataclass
class TextGenerationSettings:
    temperature: float
    max_tokens: int
    top_p: Optional[float]
    stop: Optional[list[str]] = field(default_factory=list)
    logit_bias: Optional[dict[str, float]] = field(default_factory=dict)


@dataclass
class ImageGenerationSettings:
    seed: int
    steps: int
    width: int
    height: int
    cfg_scale: float


@dataclass
class AudioGenerationSettings:
    pass


@dataclass
class EmbeddingSettings:
    pass


@dataclass
class StructSettings:
    pass


SETTINGS_CLS_BY_MODALITY = {
    Modality.GenerateText: TextGenerationSettings,
    Modality.GenerateImage: ImageGenerationSettings,
    Modality.GenerateAudio: AudioGenerationSettings,
    Modality.Embed: EmbeddingSettings,
}


class ModelInference:
    """Generic model with an endpoint for each core modality."""

    def incapable_error(self, method):
        modality = Modality(method.__name__)
        return IncapableError(f"{self} is incapable of modality {modality}")

    async def generate_text(self, input: list[XBlock], settings: TextGenerationSettings) -> str:
        raise self.incapable_error(self.generate_text)

    async def generate_image(self, input: list[XBlock], settings: ImageGenerationSettings) -> bytes:
        raise self.incapable_error(self.generate_image)

    async def generate_audio(self, input: list[XBlock], settings: AudioGenerationSettings) -> bytes:
        raise self.incapable_error(self.generate_audio)

    async def embed(self, input: list[XBlock], settings: EmbeddingSettings) -> list[float]:
        raise self.incapable_error(self.embed)

    async def struct(self, input: list[XBlock], settings: StructSettings) -> Any:
        raise self.incapable_error(self.struct)


BASE_SETTINGS_BY_MODALITY = {
    Modality.GenerateText: TextGenerationSettings,
    Modality.GenerateImage: ImageGenerationSettings,
    Modality.GenerateAudio: AudioGenerationSettings,
    Modality.Embed: EmbeddingSettings,
    Modality.Struct: StructSettings,
}

INFERENCE_CACHE_EXPIRY = get_from_env(
    "INFERENCE_CACHE_EXPIRY", 60 * 60 * 24 * 30, type_cast=int
)  # 1 month


@dataclass(slots=True)
class Inference:
    """The cached inference struct"""

    generated_at: datetime
    duration: float
    result: Any

    def to_json_str(self) -> str:
        inference_json = {
            "generated_at": self.generated_at.isoformat(),
            "duration": self.duration,
            "result": self.result,
        }
        return json.dumps(inference_json)

    @classmethod
    def from_json_str(cls, json_str: str):
        data = json.loads(json_str)
        return cls(
            generated_at=datetime.fromisoformat(data["generated_at"]),
            duration=data["duration"],
            result=data["result"],
        )


InferenceEndpoint = typing.Callable[[..., Any], typing.Awaitable[Any]]


class InferenceProxy:
    """A worker-side proxy for tracing (and caching) a specific inference endpoint."""

    def __init__(
        self,
        model: Model,
        modality: Modality,
        endpoint: InferenceEndpoint,
        tracer: "Tracer",
        cache_inferences: bool,
        timeout: int,
        retries: int,
    ):
        if retries < 0:
            raise ValueError("retries must be >= 0")
        self.model = model
        self.modality = modality
        self.endpoint = endpoint
        self.tracer = tracer
        self.cache_inferences = cache_inferences
        self.timeout = timeout
        self.retries = retries

    # insecure hash is fine here, it's just for caching
    # noinspection InsecureHash
    async def __call__(self, blocks: list[XBlock], settings: Any) -> Any:
        # make hash key
        block_strings = [f"{b.kind}{b.source}{b.value}{b.path}" for b in blocks]
        blocks_hash = hashlib.sha256("".join(block_strings).encode("utf-8")).hexdigest()
        settings_hash = hashlib.sha256(
            json.dumps(asdict(settings), sort_keys=True).encode("utf-8")
        ).hexdigest()
        cache_key = f"inference.{self.model.fqn}.{self.modality}:{settings_hash}:{blocks_hash}"

        log = logger.bind(
            model=self.model.fqn,
            modality=self.modality,
            blocks=len(blocks),
            cache_key=cache_key,
            cache_inferences=self.cache_inferences,
        )

        if self.cache_inferences:
            # TODO @Performance: use leases to cooperatively inference endpoints
            cached_inference = await redis.get(cache_key)
            if cached_inference is not None:
                try:
                    inference = Inference.from_json_str(cached_inference)
                    log.debug("inference.cache.hit", ret=describe_type(inference.result))
                    self.tracer.inference_cached(self.model, blocks, settings, inference)
                    return inference.result
                except (ValueError, TypeError, JSONDecodeError):
                    log.warning("inference.cache.error", excinfo=True)
                    # ignore and continue, will be overwritten

        remaining_attempts = self.retries + 1
        while remaining_attempts > 0:
            remaining_attempts -= 1
            try:
                generated_at = datetime.utcnow().replace(tzinfo=pytz.utc)
                self.tracer.inference_enter(self.model, blocks, settings)
                log.debug("inference.enter")
                result = await asyncio.wait_for(self.endpoint(blocks, settings), self.timeout)
                self.tracer.inference_exit(self.model, blocks, settings, result)
                log.debug("inference.exit", ret=describe_type(result))
                if self.cache_inferences:
                    now = datetime.utcnow().replace(tzinfo=pytz.utc)
                    inference = Inference(
                        generated_at=generated_at,
                        duration=(now - generated_at).total_seconds(),
                        # ret is assumed to be JSON-serializable
                        # (may not be true when we get to images, but this will error obviously enough)
                        result=result,
                    )
                    await redis.set(cache_key, inference.to_json_str(), ex=INFERENCE_CACHE_EXPIRY)
                return result
            except TimeoutError as exception:
                self.tracer.inference_exception(self.model, blocks, settings, exception)
                log.debug("inference.exception", excinfo=True)
                if remaining_attempts <= 0:
                    raise
            except Exception as exception:
                self.tracer.inference_exception(self.model, blocks, settings, exception)
                log.debug("inference.exception", excinfo=True)
                raise

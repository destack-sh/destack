import asyncio
import hashlib
import json
import os
import typing
from dataclasses import dataclass
from datetime import datetime
from json import JSONDecodeError
from logging import Logger
from typing import Any

import pytz
import structlog

from bench.bench.core import Scope, Symbol, node
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
    _key: str = None

    def _clear(self) -> None:
        self._endpoint = None
        self._remote = False

    def _interp(self, scope: Scope) -> None:
        # model is remote if we don't have the key in scope or environment
        provider = self.fqn.split(".")[0]
        self._key = os.environ.get(f"{provider.upper()}_API_KEY")
        self._remote = self._key is None

    def __str__(self):
        return f"{self.external_name}"

    async def __call__(self, timeout: int = None, cache: bool = True, **inputs):
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
                asyncio.shield(run_inference(self._endpoint, inputs, cache_key, log)),
                timeout,
            )
            self.tracer.inference_exit(self.model, inputs, result)
            return result
        except Exception as exception:
            self.tracer.inference_exception(self.model, inputs, exception)
            log.debug("inference.exception", exc_info=True)
            raise

    async def _endpoint(self, **kwargs) -> Any:
        raise NotImplementedError

    def to_sync(self):
        raise NotImplementedError


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

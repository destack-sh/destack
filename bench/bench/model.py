import asyncio
import functools
import os
import typing
from dataclasses import dataclass
from datetime import datetime
from json import JSONDecodeError
from logging import Logger
from typing import Any, Self

import msgpack
import pytz
import structlog

from bench.bench.const import StatementType
from bench.bench.core import Scope, Statement, node
from bench.bench.type import HasType, TypeTag, check_type, instantiate_py_value, strip_py_value
from bench.bench.utils import get_execution_cache_key
from bench.utils.cache import redis
from bench.utils.func import describe_type
from bench.utils.utils import DotDict, get_from_env

if typing.TYPE_CHECKING:
    from bench.bench.task import Task, TaskCompiler

logger = structlog.get_logger(__name__)

INFERENCE_CACHE_EXPIRY = get_from_env("INFERENCE_CACHE_EXPIRY", 60 * 60 * 24 * 30, type_cast=int)
ALLOW_KEY_FROM_ENV = get_from_env("MODEL_API_KEY_FROM_ENV", True, type_cast=bool)


@node
class Model(HasType, Statement):
    external_name: typing.Optional[str] = None
    description: typing.Optional[str] = None
    tag: TypeTag = TypeTag.FUNCTION
    type: StatementType = StatementType.MODEL
    _is_async: bool = True
    _remote: bool = False
    _endpoint_impl: typing.Optional[typing.Callable] = None
    _compiler_impl: typing.Optional[typing.Callable] = None
    _key: typing.Optional[str] = None

    def _clear(self) -> None:
        HasType._clear(self)
        self._key = None
        self._remote = True
        self._endpoint_impl = None
        self._compiler_impl = None

    def _interp(self, scope: Scope) -> None:
        HasType._interp(self, scope)
        # model is remote if we don't have the key in scope or environment
        provider = self.path.split(".")[0]
        if ALLOW_KEY_FROM_ENV:
            self._key = os.environ.get(f"{provider.upper()}_API_KEY")
        self._remote = self._key is None

    async def __call__(self, timeout: int = None, cache: bool = True, **inputs):
        inputs_raw = strip_py_value(inputs, self, is_output=False, ignore_outer_map=True)
        cache_key = get_execution_cache_key(self.path, inputs_raw)
        log = logger.bind(model=self, inputs=describe_type(inputs), cache_key=cache_key)
        log.debug("inference.enter.pre")

        # try to read from cache if enabled
        if cache is not False and self.session.cache_inferences:
            # TODO @Performance: use leases to cooperatively inference endpoints
            cached_inference = await redis.get(cache_key)
            if cached_inference is not None:
                try:
                    inference = Inference.from_json_bytes(cached_inference)
                    outputs = instantiate_py_value(
                        inference.outputs, self, ignore_outer_map=True, is_output=True
                    )
                    log.debug("inference.cache.hit", output=describe_type(outputs))
                    self.session.tracer.run_cached(
                        self, inputs, inference, inference.generated_at, inference.duration
                    )
                    check_type(outputs, self, is_output=True)
                    return DotDict(outputs)
                except (ValueError, TypeError, JSONDecodeError) as e:
                    log.warning("inference.cache.error", e=e, excinfo=e)
                    # ignore and continue, will be overwritten

        if self._remote:
            # request remotely proxied inference if needed (key not available locally)
            from bench.msg.core import NMessage, request
            from bench.msg.messages import (
                NMessageType,
                RepRunInferencePayload,
                ReqRunInferencePayload,
            )

            self.session.tracer.run_enter(self, inputs)
            timeout = timeout if timeout is not None else self.session.inference_timeout
            try:
                req = ReqRunInferencePayload(
                    model_path=self.path,
                    inputs=inputs_raw,
                    timeout=timeout,
                )
                rep: NMessage[RepRunInferencePayload] = await request(
                    NMessageType.REQUEST_RUN_INFERENCE,
                    req,
                    RepRunInferencePayload,
                    timeout=timeout + 2,
                )
                if rep.p.outputs is None:
                    raise RuntimeError(f"remote {self} failed")
                outputs = instantiate_py_value(rep.p.outputs, self, is_output=True)
                self.session.tracer.run_exit(self, outputs)
                return DotDict(outputs)
            except (ValueError, RuntimeError, TypeError) as e:
                self.session.tracer.run_exception(self, e)
                raise
        else:
            # otherwise run inference through endpoint :LibImplementation
            try:
                self.session.tracer.run_enter(self, inputs)
                timeout = timeout if timeout is not None else self.session.inference_timeout
                outputs = await asyncio.wait_for(
                    asyncio.shield(self._inference(inputs, cache_key, log)), timeout
                )
                self.session.tracer.run_exit(self, outputs)
                return outputs
            except Exception as e:
                self.session.tracer.run_exception(self, e)
                raise

    async def _inference(
        self,
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
        output = await self._endpoint_resolved(**inputs)
        now = datetime.utcnow().replace(tzinfo=pytz.utc)
        duration = (now - started_at).total_seconds()
        if write_to_cache:
            # result is assumed to be JSON serializable, will obviously error here if not
            inference = Inference(
                generated_at=now,
                duration=duration,
                inputs=strip_py_value(inputs, self, is_output=False, ignore_outer_map=True),
                outputs=strip_py_value(output, self, is_output=True, ignore_outer_map=True),
            )
            await redis.set(cache_key, inference.to_json_bytes(), ex=INFERENCE_CACHE_EXPIRY)
        log.debug(
            "inference.exit",
            ret=describe_type(output),
            write_to_cache_key=cache_key if write_to_cache else None,
        )
        return output

    @property
    def _endpoint_resolved(self):
        if self._endpoint_impl is None:
            from bench.bench import libs

            # get actual model implementation from libs  :LibImplementation
            # (this is a stop gap until we fully support model statements, then it's just like Code)
            actual_model_impl = libs.lookup_model_impl(self.path)
            if actual_model_impl is None:
                raise ValueError(f"cannot find {self} in libs")
            self._endpoint_impl = functools.partial(actual_model_impl, self=self)
        return self._endpoint_impl

    async def _endpoint(self, **kwargs) -> Any:
        raise NotImplementedError  # fake stub for :LibImplementation of model

    @property
    def _compiler_resolved(self):
        if self._compiler_impl is None:
            from bench.bench import libs

            # get actual model implementation from libs  :LibCompiler (see above)
            actual_model_compiler = libs.lookup_model_compiler(self.path)
            if actual_model_compiler is None:
                raise ValueError(f"cannot find {self} in libs")
            self._compiler_impl = actual_model_compiler
        return self._compiler_impl

    def _compiler(self, task: "Task", inputs: dict, is_batched: bool) -> "TaskCompiler":
        raise NotImplementedError  # stub for :LibCompiler of model compiler

    def compile(self, task: "Task", inputs: dict, is_batched: bool) -> "TaskCompiler":
        return self._compiler_resolved(self, task, inputs, is_batched)

    def to_async(self) -> "Self":
        return self

    def to_sync(self) -> "Self":
        if self._is_async:
            return ModelProxy.to_sync(self)
        return self


class ModelProxy:  # :SyncProxy
    """A simple proxy for Model to enable to_sync/to_async while keeping the original Model object."""

    def __init__(self, model: Model, is_async: bool):
        self._model = model
        self._is_async = is_async
        self._task_callable_sync = None

    def __call__(self, *args, **kwargs):
        if self._is_async:
            return self._model(*args, **kwargs)
        else:
            if self._task_callable_sync is None:
                self._task_callable_sync = self._model.session.async_to_sync(self._model.__call__)
            return self._task_callable_sync(*args, **kwargs)

    def __getattr__(self, item):
        return getattr(self._model, item)

    def to_async(self):
        return self._model

    @classmethod
    def to_sync(cls, model: Model):
        return cls(model, is_async=False)


@dataclass(slots=True)
class Inference:
    """A model inference - inputs/outputs are raw."""

    generated_at: datetime
    duration: float
    inputs: Any
    outputs: Any

    def to_json_bytes(self) -> bytes:
        inference_json = {
            "generated_at": self.generated_at.isoformat(),
            "duration": self.duration,
            "inputs": self.inputs,
            "output": self.outputs,
        }
        return msgpack.packb(inference_json, use_bin_type=True)

    @classmethod
    def from_json_bytes(cls, json_str: str):
        data = msgpack.unpackb(json_str, raw=False)
        return cls(
            generated_at=datetime.fromisoformat(data["generated_at"]),
            duration=data["duration"],
            inputs=data["inputs"],
            outputs=data["output"],
        )

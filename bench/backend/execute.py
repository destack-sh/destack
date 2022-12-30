from __future__ import annotations

import abc
import hashlib
import json
import os
import textwrap
import typing
import uuid
from asyncio import iscoroutinefunction
from dataclasses import replace
from functools import partial
from random import Random
from typing import Any, Union
from uuid import UUID

import structlog

from bench.backend.builtins import CODE_BUILTINS
from bench.backend.openai import OpenAIProvider
from bench.backend.provider import Completion, ModelHandle, ModelProvider
from bench.backend.tracing import Tracer
from bench.backend.types import (
    AsyncCodeCallable,
    CodeInstance,
    DatasetInstance,
    ModelInstance,
    ProviderKey,
    SyncCodeCallable,
)
from bench.language.types import Code, Dataset, LiteralValue, Model, ModelInferenceSettings, Value
from bench.settings import DEBUG, TEST
from bench.utils.record import RecordBatch

# TODO @Security: don't pass internal secrets to workers via environment variables
PROVIDERS: dict[ProviderKey, ModelProvider] = {}
if "OPENAI_API_KEY" in os.environ:
    PROVIDERS[ProviderKey.OPENAI] = OpenAIProvider(api_key=os.environ["OPENAI_API_KEY"])

STATIC_BUILTINS = {**CODE_BUILTINS}
DEFAULT_IMPORTS: dict = {Model: ModelHandle, Dataset: RecordBatch}
CAN_EXEC = DEBUG or TEST


logger = structlog.stdlib.get_logger()


class ExecutionError(Exception):
    pass


ModelInference = typing.NamedTuple("ModelInference", [("id", UUID), ("output", dict)])


class ModelInferenceCache(abc.ABC):
    async def get(self, model: Model, input: dict, settings: dict) -> ModelInference | None:
        raise NotImplementedError

    async def set(self, model: Model, input: dict, settings: dict, output: dict) -> None:
        raise NotImplementedError


class ModelInferenceCacheDict(ModelInferenceCache):
    def __init__(self):
        self._cache = {}

    # insecure hashing is fine since it's only used for caching
    # noinspection InsecureHash
    def key(self, model: Model, input: dict, settings: dict) -> str:
        settings_str = json.dumps(settings, sort_keys=True)
        settings_hash = hashlib.md5(settings_str.encode("utf-8")).hexdigest()
        input_str = json.dumps(input, sort_keys=True)
        input_hash = hashlib.md5(input_str.encode("utf-8")).hexdigest()
        return f"{model.definition.id}.{settings_hash}.{settings_hash}"

    async def get(self, model: Model, input: dict, settings: dict) -> ModelInference | None:
        key = self.key(model, input, settings)
        return self._cache.get(key)

    async def set(self, model: Model, input: dict, settings: dict, output: dict) -> None:
        key = self.key(model, input, settings)
        self._cache[key] = ModelInference(id=uuid.uuid4(), output=output)


class ModelProxy(ModelHandle):
    """A worker-side proxy for model tracing and caching."""

    def __init__(
        self, model: ModelInstance, tracer: Tracer, cache: typing.Optional[ModelInferenceCache]
    ):
        self.model = model
        self.tracer = tracer
        self.cache = cache

    @property
    def settings(self) -> ModelInferenceSettings:
        return self.model.handle.settings

    def configure(self, **settings: dict[str, Any]) -> ModelHandle:
        new_handle = self.model.handle.configure(**settings)
        new_model = replace(self.model, handle=new_handle)
        return ModelProxy(new_model, self.tracer, self.cache)

    async def complete(
        self, prompt: str, settings: typing.Optional[dict[str, Any]] = None
    ) -> Union[Completion, list[Completion]]:
        inference_id = uuid.uuid4()
        # set up parameters and cache keys
        if settings is not None:
            settings_merged = {**self.settings.as_dict(omit_empty=True), **settings}
        else:
            settings_merged = self.settings.as_dict(omit_empty=True)
        log = logger.bind(
            model=self.model,
            handle=self.model.handle,
            operation="complete",
            inference_id=inference_id,
        )

        self.tracer.model_complete_enter(self.model, prompt)
        log.debug("model.complete.enter", prompt=len(prompt))

        # try to get from cache if enabled
        cached_result = None
        if self.cache:
            cached_result = await self.cache.get(
                model=self.model, input={"prompt": prompt}, settings=settings_merged
            )

        # cache miss or cache disabled
        if cached_result is None:
            log.debug("model.complete.cache.miss")
            completion = await self.model.handle.complete(prompt)

            # write to cache
            if self.cache:
                await self.cache.set(
                    model=self.model,
                    input={"prompt": prompt},
                    settings=settings_merged,
                    output=completion,
                )
            log.debug("model.complete.cache.put")
        else:
            completion = cached_result
            log.debug("model.complete.cache.hit")

        completion_length = (
            len(completion["text"])
            if isinstance(completion, dict)
            else (len(c["text"]) for c in completion)
        )
        logger.debug("model.complete.exit", completion=completion_length)
        self.tracer.model_complete_exit(self.model, prompt, completion, inference_id)
        return completion

    async def embed(self, text: str, settings: typing.Optional[dict[str, Any]] = None) -> bytes:
        raise NotImplementedError


class SyncCodeProxy:
    """A worker-side proxy for code tracing."""

    def __init__(self, code: CodeInstance, tracer: Tracer):
        self.code = code
        self.tracer = tracer

    def __call__(self, *args, **kwargs):
        log = logger.bind(code=self.code, args=len(args), kwargs=_summarize_args(kwargs))
        self.tracer.code_enter(self.code, args, kwargs)
        log.debug("code.call.enter")
        try:
            result = self.code.code_callable(*args, **kwargs)
            log.debug("code.call.exit", result=_summarize_args(result))
            self.tracer.code_exit(self.code, args, kwargs, result)
            return result
        except Exception as exception:
            log.debug("code.call.exception", exception=exception)
            self.tracer.code_exception(self.code, args, kwargs, exception)
            raise


class AsyncCodeProxy:
    """A worker-side proxy for code tracing."""

    def __init__(self, code: CodeInstance, tracer: Tracer):
        self.code = code
        self.tracer = tracer

    async def __call__(self, *args, **kwargs):
        log = logger.bind(code=self.code, args=len(args), kwargs=_summarize_args(kwargs))
        self.tracer.code_enter(self.code, args, kwargs)
        log.debug("code.call.enter")
        try:
            result = await self.code.code_callable(*args, **kwargs)
            log.debug("code.call.exit", result=_summarize_args(result))
            self.tracer.code_exit(self.code, args, kwargs, result)
            return result
        except Exception as exception:
            log.debug("code.call.exception", exception=exception)
            self.tracer.code_exception(self.code, args, kwargs, exception)
            raise


class Proxy:
    """A worker-side proxy for wrapping symbol access."""

    def __init__(self, tracer: Tracer, cache: ModelInferenceCache):
        self.tracer = tracer
        self.cache = cache

    def proxy_dataset(self, dataset: DatasetInstance) -> DatasetInstance:
        return dataset  # not proxied

    def proxy_model(self, model: ModelInstance) -> ModelInstance:
        model_proxy = ModelProxy(model, self.tracer, cache=self.cache)
        return replace(model, handle=model_proxy)

    def proxy_code(self, code: CodeInstance) -> CodeInstance:
        code_proxy_cls = (
            AsyncCodeProxy if iscoroutinefunction(code.code_callable) else SyncCodeProxy
        )
        code_proxy = code_proxy_cls(code, self.tracer)
        return replace(code, code_callable=code_proxy)


def unwrap_dataset(dataset: DatasetInstance) -> RecordBatch:
    return dataset.records


def unwrap_model(model: ModelInstance) -> ModelHandle:
    return model.handle


def unwrap_code(code: CodeInstance) -> SyncCodeCallable | AsyncCodeCallable:
    return code.code_callable


def unwrap(
    value: Value | DatasetInstance | ModelInstance | CodeInstance,
) -> Value | RecordBatch | ModelHandle | SyncCodeCallable | AsyncCodeCallable:
    if isinstance(value, DatasetInstance):
        return unwrap_dataset(value)
    elif isinstance(value, ModelInstance):
        return unwrap_model(value)
    elif isinstance(value, CodeInstance):
        return unwrap_code(value)
    else:
        return value


def unwrap_arguments(self, arguments: dict[str, Any]) -> dict[str, Any]:
    return {name: self.unwrap(value) for name, value in arguments.items()}


def get_dynamic_builtins(code: Code) -> dict:
    return {
        "random": Random(code.definition.id.hex.encode()),
    }


def instantiate_dataset(dataset: Dataset, proxy: Proxy) -> DatasetInstance:
    dataset = DatasetInstance(**dataset.__dict__)
    return proxy.proxy_dataset(dataset)


def instantiate_model(model: Model, proxy: Proxy) -> ModelInstance:
    provider = PROVIDERS.get(ProviderKey(model.provider))
    if provider is None:
        raise ValueError(f"unknown provider {model.provider}")
    user_identifier = model.definition.id.hex
    handle = provider.access(model, model.settings, for_user=user_identifier)
    return proxy.proxy_model(ModelInstance(**model.__dict__, handle=handle))


def instantiate_code(code: Code, proxy: Proxy) -> CodeInstance:
    """
    Resolves a code symbol and all its arguments to an async callable.
    """
    if code.builtin_id:
        # builtins are already defined and are just curried using the arguments
        builtin = STATIC_BUILTINS.get(code.builtin_id)
        if builtin is None:
            raise ValueError(f"unknown builtin in {code}: {code.builtin_id}")
        code_callable = partial(builtin)
    else:
        dynamic_builtins = get_dynamic_builtins(code)
        # create python function from code
        func_name = f"_anon_{code.definition.id.hex}"
        async_str = "async " if code.is_async else ""
        indented_code = textwrap.indent(code.code, "    ")
        code_str = f"{async_str}def {func_name}():\n{indented_code}"
        code_callable = _execute_code(code_str, {**STATIC_BUILTINS, **dynamic_builtins})[func_name]
    code_instance = CodeInstance(**code.__dict__, code_callable=code_callable)
    return proxy.proxy_code(code_instance)


def execute(
    code: Code, arguments: dict[str, LiteralValue] | None = None, tracer: Tracer | None = None
) -> LiteralValue:
    tracer = tracer or Tracer()
    proxy = Proxy(tracer=tracer, cache=ModelInferenceCacheDict())
    code_instance = instantiate_code(code, proxy)
    arguments = arguments or {}
    try:
        output = unwrap_code(code_instance)(**arguments)
    except Exception as e:
        raise ExecutionError(f"error executing {code} with {_summarize_args(arguments)}: {e}", e)
    return output


async def execute_async(
    code: Code, arguments: dict[str, LiteralValue] | None = None, tracer: Tracer | None = None
) -> LiteralValue:
    tracer = tracer or Tracer()
    proxy = Proxy(tracer=tracer, cache=ModelInferenceCacheDict())
    code_instance = instantiate_code(code, proxy)
    arguments = arguments or {}
    try:
        output = await unwrap_code(code_instance)(**arguments)
    except Exception as e:
        raise ExecutionError(f"error executing {code} with {_summarize_args(arguments)}: {e}", e)
    return output


def _summarize_args(arguments: Any) -> str:
    """
    Summarize the names (if available) and types of arguments.
    """
    if isinstance(arguments, dict):
        return ", ".join(f"{name}={type(value).__name__}" for name, value in arguments.items())
    elif isinstance(arguments, (list, tuple, set)):
        return ", ".join(type(value).__name__ for value in arguments)
    else:
        return type(arguments).__name__


def _execute_code(code: str, globals: dict[str, Any]) -> dict:
    # remember the globals we started with, do not modify originals
    globals_local = {**DEFAULT_IMPORTS, **STATIC_BUILTINS, **globals}
    globals_local_keys_initial = {*globals_local.keys()}
    _do_execute(code, globals_local)
    new_globals = {
        k: v
        for k, v in globals_local.items()
        if k not in globals_local_keys_initial and k not in ("__builtins__", "__annotations__")
    }
    return new_globals


def _do_execute(code: str, globals: dict):
    if not CAN_EXEC:
        raise RuntimeError("exec outside sandbox is not allowed")

    try:
        exec(code, globals)
    except Exception as e:
        raise ExecutionError(f"error running code with globals {globals}: {e}", e) from e

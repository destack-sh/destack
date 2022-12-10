from __future__ import annotations

import hashlib
import json
import os
import time
import typing
from dataclasses import replace
from functools import partial
from random import Random
from typing import Any, Union

import structlog

from bench.backend.builtins import code_builtins
from bench.backend.openai import OpenAIProvider
from bench.backend.provider import Completion, ModelHandle, ModelProvider
from bench.backend.resolver import Resolver
from bench.backend.tracing import (
    ExecutionTrace,
    ExecutionTracer,
    ExecutionTracker,
    MultiTracer,
    Trace,
    Tracer,
    ValidationTracer,
)
from bench.backend.types import (
    CodeCallable,
    LoadedCode,
    LoadedDataset,
    LoadedModel,
    LoadedSymbol,
    ResolvedCode,
    ResolvedDataset,
    ResolvedModel,
    ResolvedSymbol,
    Value,
)
from bench.models import Dataset, Model, Symbol, SymbolContent
from bench.models.code import Code
from bench.models.model import ModelInference, ModelInferenceSettings, ModelOperation, ProviderKey
from bench.settings import DEBUG, TEST
from bench.utils.record import RecordBatch

logger = structlog.stdlib.get_logger()


class SandboxError(Exception):
    def __init__(self, message: str, exception: typing.Optional[Exception] = None):
        super().__init__(message)
        self.exception = exception


def _arguments_summary(arguments: Any) -> str:
    """
    Summarize the names (if available) and types of arguments.
    """
    if isinstance(arguments, dict):
        return ", ".join(f"{name}={type(value).__name__}" for name, value in arguments.items())
    elif isinstance(arguments, (list, tuple, set)):
        return ", ".join(type(value).__name__ for value in arguments)
    else:
        return type(arguments).__name__


class ModelProxy(ModelHandle):
    """
    A worker-side model proxy for wrapping model access with tracers and caches.
    """

    def __init__(self, model: LoadedModel, tracer: Tracer, use_cache: bool):
        self.model = model
        self.tracer = tracer
        self.use_cache = use_cache

    @property
    def settings(self) -> ModelInferenceSettings:
        return self.model.handle.settings

    def configure(self, **settings: dict[str, Any]) -> ModelHandle:
        new_handle = self.model.handle.configure(**settings)
        new_model = replace(self.model, handle=new_handle)
        return ModelProxy(new_model, self.tracer, self.use_cache)

    # insecure hashing is fine here since it's just for caching
    # noinspection InsecureHash
    async def complete(
        self, prompt: str, settings: typing.Optional[dict[str, Any]] = None
    ) -> Union[Completion, list[Completion]]:
        # set up parameters and cache keys
        if settings is not None:
            settings_merged = {**self.settings.as_dict(omit_empty=True), **settings}
        else:
            settings_merged = self.settings.as_dict(omit_empty=True)
        settings_as_str = json.dumps(settings_merged, sort_keys=True)
        settings_hash = hashlib.md5(settings_as_str.encode()).hexdigest()
        input_hash = hashlib.md5(prompt.encode()).hexdigest()
        log = logger.bind(
            model=self.model,
            handle=self.model.handle,
            operation=ModelOperation.COMPLETE,
            settings_hash=settings_hash,
            input_hash=input_hash,
        )

        self.tracer.model_complete_enter(self.model, prompt)
        log.debug("model.complete.enter", prompt=len(prompt))

        # try to get from cache if enabled
        cached_result = None
        if self.use_cache:
            inference = await ModelInference.objects.filter(
                model_id=self.model.content_id,
                operation=ModelOperation.COMPLETE,
                settings_hash=settings_hash,
                input_hash=input_hash,
            ).afirst()
            if inference is not None:
                cached_result = inference.output

        # cache miss or cache disabled
        if cached_result is None:
            log.debug("model.complete.cache.miss")
            start_time = time.time()
            completion = await self.model.handle.complete(prompt)
            duration_ms = (time.time() - start_time) * 1000

            # write to cache
            inference = await ModelInference.objects.acreate(
                model_id=self.model.content_id,
                operation=ModelOperation.COMPLETE,
                settings_hash=settings_hash,
                input_hash=input_hash,
                input=prompt,
                output=completion,
                duration_ms=duration_ms,
            )
            log.debug(
                "model.complete.cache.put", duration_ms=duration_ms, inference_id=inference.id
            )
        else:
            completion = cached_result
            log.debug("model.complete.cache.hit")

        completion_length = (
            len(completion["text"])
            if isinstance(completion, dict)
            else (len(c["text"]) for c in completion)
        )
        logger.debug("model.complete.exit", completion=completion_length)
        self.tracer.model_complete_exit(self.model, prompt, completion, inference.id)
        return completion

    async def embed(self, text: str, settings: typing.Optional[dict[str, Any]] = None) -> bytes:
        raise NotImplementedError


class CodeProxy:
    """
    A worker-side code proxy for wrapping code access with tracers and caches.
    """

    def __init__(self, code: LoadedCode, tracer: Tracer):
        self.code = code
        self.tracer = tracer

    async def __call__(self, *args, **kwargs):
        log = logger.bind(
            code=self.code,
            callable=self.code.callable_name,
            args=len(args),
            kwargs=_arguments_summary(kwargs),
        )
        self.tracer.code_enter(self.code, args, kwargs)
        log.debug("code.call.enter")
        try:
            result = await self.code.code_callable(*args, **kwargs)
            log.debug("code.call.exit", result=_arguments_summary(result))
            self.tracer.code_exit(self.code, args, kwargs, result)
            return result
        except Exception as exception:
            log.debug("code.call.exception", exception=exception)
            self.tracer.code_exception(self.code, args, kwargs, exception)
            raise


class Proxy:
    """
    A proxy that wraps direct access to loaded symbol content.
    """

    def __init__(self, tracer: Tracer):
        self.tracer = tracer

    def proxy_dataset(self, dataset: LoadedDataset) -> LoadedDataset:
        return dataset  # not proxied

    def proxy_model(self, model: LoadedModel) -> LoadedModel:
        model_proxy = ModelProxy(
            model, self.tracer, use_cache=True
        )  # should make this configurable
        return replace(model, handle=model_proxy)

    def proxy_code(self, code: LoadedCode) -> LoadedCode:
        code_proxy = CodeProxy(code, self.tracer)
        return replace(code, code_callable=code_proxy)

    def unwrap_dataset(self, dataset: LoadedDataset) -> RecordBatch:
        return dataset.records

    def unwrap_model(self, model: LoadedModel) -> ModelHandle:
        return model.handle

    def unwrap_code(self, code: LoadedCode) -> CodeCallable:
        return code.code_callable

    def unwrap(
        self, value: Value | LoadedDataset | LoadedModel | LoadedCode
    ) -> Value | RecordBatch | ModelHandle | CodeCallable:
        if isinstance(value, LoadedDataset):
            return self.unwrap_dataset(value)
        elif isinstance(value, LoadedModel):
            return self.unwrap_model(value)
        elif isinstance(value, LoadedCode):
            return self.unwrap_code(value)
        else:
            return value


class Executor:
    def __init__(self, resolver: Resolver):
        # TODO @Security: don't pass internal secrets to workers via environment variables
        self.providers: dict[ProviderKey, ModelProvider] = {
            ProviderKey.OPENAI: OpenAIProvider(api_key=os.environ["OPENAI_API_KEY"]),
        }
        self.can_exec = DEBUG or TEST  # or sandboxed
        self.static_builtins = {**code_builtins}
        self.default_imports: dict = {Model: ModelHandle, Dataset: RecordBatch}
        self.resolver = resolver

    async def _load_arguments(
        self, arguments: dict[str, Value | ResolvedSymbol], proxy: Proxy
    ) -> dict[str, Value | ResolvedSymbol]:
        loaded_arguments = {}
        for name, argument in arguments.items():
            if isinstance(argument, ResolvedDataset):
                loaded_arguments[name] = await self._load_dataset(argument, proxy)
            elif isinstance(argument, ResolvedModel):
                loaded_arguments[name] = await self._load_model(argument, proxy)
            elif isinstance(argument, ResolvedCode):
                loaded_arguments[name] = await self._load_code(argument, proxy)
            else:
                loaded_arguments[name] = argument
        return loaded_arguments

    def _unwrap_arguments(
        self, arguments: dict[str, Value | LoadedSymbol], proxy: Proxy
    ) -> dict[str, Value | ResolvedSymbol]:
        unwrapped_arguments = {}
        for name, argument in arguments.items():
            unwrapped_arguments[name] = proxy.unwrap(argument)
        return unwrapped_arguments

    async def _load_dataset(self, dataset: ResolvedDataset, proxy: Proxy) -> LoadedDataset:
        return LoadedDataset(**dataset.__dict__)

    async def _load_model(self, model: ResolvedModel, proxy: Proxy) -> LoadedModel:
        provider = self.providers.get(ProviderKey(model.provider))
        if provider is None:
            raise ValueError(f"unknown provider {model.provider}")
        # TODO @Compliance: set actual user identifier for model access (e.g. for OpenAI)
        user_identifier = model.content_id.hex
        handle = await provider.access(
            model, model.settings or model.default_settings, for_user=user_identifier
        )
        return proxy.proxy_model(LoadedModel(**model.__dict__, handle=handle))

    async def _load_code(self, code: ResolvedCode, proxy: Proxy) -> LoadedCode:
        """
        Resolves a code symbol and all its arguments to an async callable.
        """
        loaded_arguments = await self._load_arguments(code.arguments, proxy)
        unwrapped_arguments = self._unwrap_arguments(loaded_arguments, proxy)

        if code.builtin_id:
            # builtins are already defined and are just curried using the arguments
            builtin = self.static_builtins.get(code.builtin_id)
            if builtin is None:
                raise ValueError(f"unknown builtin in {code}: {code.builtin_id}")
            code_callable = partial(builtin, **unwrapped_arguments)
        else:
            dynamic_builtins = self._get_dynamic_builtins(code)

            # Code can be an anonymous function (just lines of code) or define an actual function.
            # Note that we don't actually run the inner function code here, we only initialise.
            if code.code_function_name is not None:
                # Run code to get function symbol.
                symbols = await self.run_text(
                    code.code_text, {**dynamic_builtins, **unwrapped_arguments}
                )
                if code.code_function_name not in symbols:
                    raise ValueError(
                        f"{code} function {code.code_function_name} not defined in code"
                    )
                code_callable = symbols[code.code_function_name]
            else:
                # TODO @Performance @Cleanup: just compile anonymous functions into named functions?
                #  Currently re exec() the code every time it's called.
                async def _run_anonymous(**kwargs):
                    await self._do_exec(
                        code.code_text, {**dynamic_builtins, **loaded_arguments, **kwargs}
                    )

                _run_anonymous.__name__ = f"_anon_{code.content_id.hex}"
                code_callable = _run_anonymous
        loaded_code = LoadedCode(
            **code.__dict__,
            code_callable=code_callable,
            loaded_arguments=loaded_arguments,
        )
        return proxy.proxy_code(loaded_code)

    def _get_dynamic_builtins(self, code: ResolvedCode) -> dict:
        return {
            "random": Random(code.content_id.hex.encode()),
        }

    async def _do_exec(self, code: str, globals: dict):
        if not self.can_exec:
            raise RuntimeError("exec outside sandbox is not allowed")

        try:
            exec(code, globals)
        except Exception as e:
            raise SandboxError(f"error running code with globals {globals}: {e}", e) from e

    def _make_tracer(self, execution_tracker: ExecutionTracker, traces: list[Trace]) -> Tracer:
        # TODO @Cleanup: passing empty traces to populate is a bit messy
        tracers = []
        for trace in traces:
            if isinstance(trace, ExecutionTrace):
                tracers.append(ExecutionTracer(execution_tracker, trace))
            else:
                raise ValueError(f"unknown trace type {trace}")
        tracers.append(ValidationTracer())
        return MultiTracer(tracers)

    async def resolve_and_run(
        self,
        code: Code,
        arguments: dict[str, Value | Symbol | SymbolContent],
        traces: list[Trace] | None = None,
    ):
        resolved_arguments = {
            name: await self.resolver.resolve_argument(argument)
            for name, argument in arguments.items()
        }
        resolved_code = await self.resolver.resolve_code(code)
        return await self.run(resolved_code, resolved_arguments, traces)

    async def run(
        self,
        code: ResolvedCode,
        arguments: dict[str, Value | ResolvedSymbol],
        traces: list[Trace] | None = None,
    ) -> dict[str, Any] | list[dict[str, Any]] | None:
        execution_tracker = ExecutionTracker()
        tracer = self._make_tracer(execution_tracker, traces or [])
        proxy = Proxy(tracer=tracer)
        loaded_arguments = await self._load_arguments(arguments, proxy)
        unwrapped_arguments = self._unwrap_arguments(loaded_arguments, proxy)

        try:
            loaded_code = await self._load_code(code, proxy)
        except Exception as e:
            raise ValueError(
                f"error resolving code {code} with arguments {_arguments_summary(unwrapped_arguments)}: {e}"
            ) from e

        try:
            output = await proxy.unwrap_code(loaded_code)(**unwrapped_arguments)
        except SandboxError:
            # re-raise sandbox errors
            raise
        except Exception as e:
            raise SandboxError(
                f"error running {code} with arguments {_arguments_summary(unwrapped_arguments)}: {e}",
                e,
            ) from e
        finally:
            # TODO @Architecture: execution tracker should likely be long running
            await execution_tracker.process_until_empty()
        return output

    async def run_text(self, code_text: str, globals: dict[str, Any]) -> dict:
        # remember the globals we started with, do not modify originals
        globals_local = {**self.default_imports, **self.static_builtins, **globals}
        globals_local_keys_initial = {*globals_local.keys()}
        await self._do_exec(code_text, globals_local)
        new_globals = {
            k: v
            for k, v in globals_local.items()
            if k not in globals_local_keys_initial and k not in ("__builtins__", "__annotations__")
        }
        return new_globals

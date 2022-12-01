from __future__ import annotations

import hashlib
import json
import os
import time
import typing
import uuid
from functools import partial
from random import Random
from typing import Any, Union

import structlog
from django.db.models import QuerySet

from bench.backend.base import Completion, ModelHandle, ModelProvider
from bench.backend.openai import OpenAIProvider
from bench.executor.builtins import instruction_builtins
from bench.models import Dataset, Model, SymbolContent, SymbolDefinition, SymbolType
from bench.models.dataset import DatasetView
from bench.models.instruction import (
    Instruction,
    InstructionArgument,
    InstructionParameter,
    InstructionParameterType,
)
from bench.models.model import ModelInference, ModelInferenceSettings, ModelOperation, ProviderKey
from bench.settings import DEBUG, TEST
from bench.utils.record import RecordBatch, RecordList

logger = structlog.stdlib.get_logger()


class SandboxError(Exception):
    def __init__(self, message: str, exception: Exception):
        super().__init__(message)
        self.exception = exception


class Debugger:
    """
    A basic debugger that can be attached to an executor.
    """

    pass


class Frame:
    """
    A single frame in the execution stack.
    """

    def __init__(self, instruction: Instruction, arguments: dict[str, Any]):
        self.instruction = instruction
        self.arguments = arguments


class Trace:
    """
    A trace of the execution stack of frames.
    """

    def __init__(self, frame: Frame):
        self.frames: list[Frame] = []
        self._frame = frame

    def push(self, frame: Frame):
        self.frames.append(self._frame)
        self._frame = frame

    def pop(self):
        self._frame = self.frames.pop()

    @property
    def current(self):
        return self._frame


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


InstructionCallable = typing.Callable[..., typing.Coroutine]


class ModelProxy(ModelHandle):
    def __init__(self, handle: ModelHandle, model: Model, use_cache: bool):
        self.handle = handle
        self.model = model
        self.use_cache = use_cache

    @property
    def settings(self) -> ModelInferenceSettings:
        return self.handle.settings

    def configure(self, **settings: dict[str, Any]) -> ModelHandle:
        return ModelProxy(self.handle.configure(**settings), self.model, self.use_cache)

    # insecure hashing is fine here since it's just for caching
    # noinspection InsecureHash
    async def complete(
        self, prompt: str, settings: typing.Optional[dict[str, Any]] = None
    ) -> Union[Completion, list[Completion]]:
        logger.info(
            "model.complete.enter", model=self.model, handle=self.handle, prompt=len(prompt)
        )
        if settings is not None:
            settings_merged = {**self.settings.as_dict(omit_empty=True), **settings}
        else:
            settings_merged = self.settings.as_dict(omit_empty=True)
        settings_as_str = json.dumps(settings_merged, sort_keys=True)
        settings_hash = hashlib.md5(settings_as_str.encode()).hexdigest()
        input_hash = hashlib.md5(prompt.encode()).hexdigest()

        log = logger.bind(
            model=self.model,
            operation=ModelOperation.COMPLETE,
            settings_hash=settings_hash,
            input_hash=input_hash,
        )

        # try to get from cache if enabled
        cached_result = None
        if self.use_cache:
            cached_inference = await ModelInference.objects.filter(
                model=self.model,
                operation=ModelOperation.COMPLETE,
                settings_hash=settings_hash,
                input_hash=input_hash,
            ).afirst()
            if cached_inference is not None:
                cached_result = cached_inference.output

        # cache miss or cache disabled
        if cached_result is None:
            log.debug("model.complete.cache.miss")
            start_time = time.time()
            completion = await self.handle.complete(prompt)
            duration_ms = (time.time() - start_time) * 1000

            # write to cache
            inference = await ModelInference.objects.acreate(
                model=self.model,
                operation=ModelOperation.COMPLETE,
                settings_hash=settings_hash,
                input_hash=input_hash,
                input=prompt,
                output=completion,
                duration_ms=duration_ms,
            )
            log.debug("model.complete.cache.put", duration_ms=duration_ms, inference=inference)
        else:
            completion = cached_result
            log.debug("model.complete.cache.hit")

        completion_length = (
            len(completion["text"])
            if isinstance(completion, dict)
            else (len(c["text"]) for c in completion)
        )
        logger.info("model.complete.exit", completion=completion_length)
        return completion

    async def embed(self, text: str, settings: typing.Optional[dict[str, Any]] = None) -> bytes:
        raise NotImplementedError


class InstructionProxy:
    def __init__(self, callable: InstructionCallable, instruction: Instruction):
        self.callable = callable
        self.instruction = instruction

    async def __call__(self, *args, **kwargs):
        # get callable name (if partial get underlying func name)
        if isinstance(self.callable, partial):
            callable_name = self.callable.func.__name__
        else:
            callable_name = self.callable.__name__
        log = logger.bind(
            instruction=self.instruction,
            callable=callable_name,
            args=len(args),
            kwargs=_arguments_summary(kwargs),
        )
        log.info("instruction.call.enter")
        result = await self.callable(*args, **kwargs)
        logger.info("instruction.call.exit", result=_arguments_summary(result))
        return result


class Executor:
    def __init__(self):
        self.executor_id = uuid.uuid4().hex
        self.can_exec = DEBUG or TEST  # or sandboxed
        self.providers: dict[ProviderKey, ModelProvider] = {
            ProviderKey.OPENAI: OpenAIProvider(api_key=os.environ["OPENAI_API_KEY"]),
        }
        self.static_builtins = {**instruction_builtins}
        self.default_imports: dict = {Model: ModelHandle, Dataset: RecordBatch}
        self.use_model_cache = True

    def _get_instruction_code(self, instruction: Instruction) -> str:
        """
        Gets the literal code defining this instruction.
        """

        if instruction.builtin_id is not None:
            raise ValueError("cannot get code for builtin instruction")
        elif instruction.code is not None:
            instruction_code = instruction.code
        else:
            raise ValueError(f"instruction {instruction} has no code or builtin id")
        return instruction_code

    def _get_dynamic_builtins(self, instruction) -> dict:
        return {
            "random": Random(instruction.id.hex.encode()),
        }

    async def _do_exec(self, code: str, globals: dict):
        if not self.can_exec:
            raise RuntimeError("exec outside sandbox is not allowed")

        try:
            exec(code, globals)
        except Exception as e:
            raise SandboxError(f"error running code with globals {globals}: {e}", e) from e

    async def _resolve_model(
        self, model: Model, settings: typing.Optional[ModelInferenceSettings]
    ) -> ModelHandle:
        provider = self.providers.get(ProviderKey(model.provider))
        if provider is None:
            raise ValueError(f"unknown provider {model.provider}")
        # TODO @Compliance: set actual user identifier for model access (e.g. for OpenAI)
        user_identifier = model.id.hex
        return await provider.access(
            model, settings or model.default_settings, for_user=user_identifier
        )

    async def _resolve_dataset(
        self, dataset: Dataset, view: typing.Optional[DatasetView]
    ) -> RecordBatch:
        if view is not None:
            raise NotImplementedError("dataset views are not implemented yet")
        # TODO @Performance: do not load all records when resolving dataset arguments
        #  All functions are executed async, but dataset access is neater if it's synchronous.
        #  So we pre-load everything and wrap it in a synchronous wrapper.
        records = []
        async for record in dataset:
            records.append(record)
        return RecordList(records)

    async def _resolve_instruction(
        self, instruction: Instruction
    ) -> tuple[dict[str, Any], dict[str, Any], InstructionCallable]:
        """
        Resolves an instruction definition and all its arguments to an async callable.
        """

        parameters = await self._get_instruction_parameters(instruction)
        arguments = await self._resolve_instruction_arguments(instruction)
        # check arguments types (ignoring missing parameters for now since they could be bound later)
        # TODO @Cleanup: not sure if it's okay to be lenient on missing parameters during instruction resolution
        #  Doesn't this also depend on whether the instruction is anonymous, named or builtin?
        self._check_arguments(instruction, parameters, arguments, check_required=False)

        if instruction.builtin_id:
            # builtins are already defined and are just curried using the arguments
            builtin = self.static_builtins.get(instruction.builtin_id)
            if builtin is None:
                raise ValueError(f"unknown builtin in {instruction}: {instruction.builtin_id}")
            return parameters, arguments, partial(builtin, **arguments)
        else:
            code = self._get_instruction_code(instruction)
            dynamic_builtins = self._get_dynamic_builtins(instruction)

            # A code instruction can be a linear piece of code to call every time or define a function to call.
            # Note that we don't actually run the function code here, we just resolve and initialise.
            if not instruction.anonymous:
                # Run code to get function definition.
                output = await self.run_get_definitions(code, {**dynamic_builtins, **arguments})
                if instruction.code_function_name not in output:
                    raise ValueError(
                        f"{instruction} function {instruction.code_function_name} not defined in code"
                    )
                return parameters, arguments, output[instruction.code_function_name]
            else:
                # TODO @Performance @Cleanup: should we just compile anonymous functions into named functions?
                #  Otherwise, we-exec the code every time it's called.
                async def _run_anonymous(**kwargs):
                    await self._do_exec(code, {**dynamic_builtins, **arguments, **kwargs})

                _run_anonymous.__name__ = f"_anon_{instruction.id.hex}"
                return parameters, arguments, _run_anonymous

    async def _proxy_model(self, model_handle: ModelHandle, model: Model) -> ModelProxy:
        return ModelProxy(handle=model_handle, model=model, use_cache=self.use_model_cache)

    async def _proxy_instruction(
        self, callable: InstructionCallable, instruction: Instruction
    ) -> InstructionProxy:
        return InstructionProxy(callable=callable, instruction=instruction)

    async def _get_instruction_parameters(
        self, instruction: Instruction
    ) -> dict[str, InstructionParameter]:
        parameters = {}
        async for parameter in instruction.parameters.all():
            parameters[parameter.name] = parameter
        return parameters

    async def _resolve_instruction_arguments(self, instruction: Instruction) -> dict[str, Any]:
        bound_arguments: QuerySet[InstructionArgument] = instruction.arguments.all().select_related(
            "reference",
            "reference__model",
            "reference__model__default_settings",
            "reference__dataset",
            "reference__instruction",
        )
        bound_arguments_resolved: dict[str, Any] = {}
        async for argument in bound_arguments:
            if argument.type == InstructionParameterType.JSON:
                bound_arguments_resolved[argument.name] = argument.value
                continue
            if argument.reference is None:
                raise ValueError(f"argument {argument} has no reference definition")
            # resolve symbol reference
            bound_arguments_resolved[argument.name] = await self._resolve_instruction_argument(
                argument.reference
            )
        return bound_arguments_resolved

    async def _resolve_instruction_argument(self, value: Any | SymbolDefinition | SymbolContent):
        if isinstance(value, SymbolContent):
            value = value.definition
        if isinstance(value, SymbolDefinition):
            if value.type == SymbolType.MODEL:
                model = value.model_
                model_handle = await self._resolve_model(model, settings=None)
                model_proxy = await self._proxy_model(model_handle, model)
                return model_proxy
            elif value.type == SymbolType.DATASET:
                dataset = value.dataset_
                dataset_handle = await self._resolve_dataset(dataset, view=None)
                return dataset_handle
            elif value.type == SymbolType.INSTRUCTION:
                instruction = value.instruction_
                _, _, callable = await self._resolve_instruction(instruction)
                callable_proxy = await self._proxy_instruction(callable, instruction)
                return callable_proxy
            else:
                raise ValueError(f"unexpected argument type: {value}")
        else:
            return value

    def _check_arguments(
        self,
        instruction: Instruction,
        parameters: dict[str, InstructionParameter],
        arguments: dict[str, Any],
        check_required: bool,
    ) -> None:
        """
        Checks that all required arguments are present and valid for the instruction, raising an error if not.
        """
        for parameter in parameters.values():
            # check that all required arguments are present
            if check_required and parameter.name not in arguments:
                raise ValueError(f"required parameter {parameter.name} not bound for {instruction}")
            if not check_required and parameter.name not in arguments:
                continue
            # check that all arguments are of the correct type
            # TODO @Robustness: check that the argument has the correct schema
            if parameter.type == InstructionParameterType.INSTRUCTION:
                if not callable(arguments[parameter.name]):
                    raise ValueError(
                        f"argument {parameter.name} for {instruction} is not a callable"
                    )
            elif parameter.type == InstructionParameterType.MODEL:
                if not isinstance(arguments[parameter.name], ModelHandle):
                    raise ValueError(
                        f"argument {parameter.name} for {instruction} is not a model handler"
                    )
            elif parameter.type == InstructionParameterType.DATASET:
                if not isinstance(arguments[parameter.name], RecordBatch):
                    raise ValueError(
                        f"argument {parameter.name} for {instruction} is not a dataset"
                    )
            elif parameter.type == InstructionParameterType.JSON:
                # check that the argument is a JSON object or primitive
                if not isinstance(
                    arguments[parameter.name], (dict, list, str, int, float, bool, type(None))
                ):
                    raise ValueError(
                        f"argument {parameter.name} for {instruction} is not a JSON object or primitive"
                    )
            else:
                raise RuntimeError(f"unknown parameter type for {instruction}: {parameter.type}")

        # ignore extraneous arguments

    async def run(
        self, instruction: Instruction, arguments: dict[str, Any | SymbolDefinition | SymbolContent]
    ) -> dict[str, Any] | list[dict[str, Any]] | None:
        if not isinstance(arguments, dict):
            raise ValueError(f"instruction arguments must be a dict: {arguments}")

        # resolve arguments
        arguments = {k: await self._resolve_instruction_argument(v) for k, v in arguments.items()}

        try:
            parameters, bound_arguments, inner_func = await self._resolve_instruction(instruction)
            func_proxy: InstructionProxy = await self._proxy_instruction(inner_func, instruction)
        except Exception as e:
            raise ValueError(
                f"error resolving instruction {instruction} with arguments {_arguments_summary(arguments)}: {e}"
            ) from e

        # check free arguments are bound as they should
        # it feels a bit hacky to check the free arguments like this, but whatever for now.
        missing_parameters = {k: v for k, v in parameters.items() if k not in bound_arguments}
        self._check_arguments(instruction, missing_parameters, arguments, check_required=True)

        try:
            return await func_proxy(**arguments)
        except SandboxError:
            # re-raise sandbox errors
            raise
        except Exception as e:
            raise SandboxError(
                f"error running {instruction} with arguments {_arguments_summary(arguments)}: {e}",
                e,
            ) from e

    async def run_get_definitions(self, code: str, globals: dict[str, Any]) -> dict:
        # remember the globals we started with, do not modify originals
        globals_local = {**self.default_imports, **self.static_builtins, **globals}
        globals_local_keys_initial = {*globals_local.keys()}
        await self._do_exec(code, globals_local)
        new_globals = {
            k: v
            for k, v in globals_local.items()
            if k not in globals_local_keys_initial and k not in ("__builtins__", "__annotations__")
        }
        return new_globals

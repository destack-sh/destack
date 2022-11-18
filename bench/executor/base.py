from __future__ import annotations

import os
import typing
import uuid
from typing import Any, Union

import structlog
from django.db.models import QuerySet

from bench.backend.base import ModelHandle, ModelProvider
from bench.backend.openai import OpenAIProvider
from bench.models import Dataset, Model
from bench.models.dataset import DatasetView
from bench.models.instruction import (
    Instruction,
    InstructionArgument,
    InstructionParameter,
    InstructionParameterType,
)
from bench.models.model import ModelInferenceSettings, ProviderKey
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

    def __init__(
        self, instruction: Instruction, arguments: dict[str, Any], parent: typing.Optional[Frame]
    ):
        self.instruction = instruction
        self.arguments = arguments
        self.parent = parent


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
    def __init__(self, handle: ModelHandle, model: Model):
        self.handle = handle
        self.model = model

    async def complete(
        self, prompt: str
    ) -> Union[tuple[str, list[float]], list[tuple[str, list[float]]]]:
        logger.info(
            "model.complete.enter", model=self.model, handle=self.handle, prompt=len(prompt)
        )
        result = await self.handle.complete(prompt)
        logger.info("model.complete.exit", model=self.model, handle=self.handle, result=len(result))
        return result

    async def embed(self, text: str) -> bytes:
        raise NotImplementedError


class InstructionProxy:
    def __init__(self, callable: InstructionCallable, instruction: Instruction):
        self.callable = callable
        self.instruction = instruction

    async def __call__(self, *args, **kwargs):
        logger.info(
            "instruction.call.enter",
            instruction=self.instruction,
            callable=self.callable,
            args=len(args),
            kwargs=_arguments_summary(kwargs),
        )
        result = await self.callable(*args, **kwargs)
        logger.info(
            "instruction.call.exit",
            instruction=self.instruction,
            callable=self.callable,
            result=_arguments_summary(result),
        )
        return result


class Executor:
    def __init__(self):
        self.executor_id = uuid.uuid4().hex
        self.builtin_instructions = {}
        self.can_exec = DEBUG or TEST  # or sandboxed
        self.providers: dict[ProviderKey, ModelProvider] = {
            ProviderKey.OPENAI: OpenAIProvider(api_key=os.environ["OPENAI_API_KEY"]),
        }
        self.default_imports = {Model: ModelHandle, Dataset: RecordBatch}

    def _get_instruction_code(self, instruction: Instruction) -> str:
        """
        Gets the literal code defining this instruction.
        """

        if instruction.code_id is not None:
            if instruction.code_id not in self.builtin_instructions:
                raise ValueError(f"unknown builtin instruction code id: {instruction}")

            instruction_code = self.builtin_instructions[instruction.code_id]
        else:
            instruction_code = instruction.code
        return instruction_code

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
        provider = self.providers.get(model.provider)
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
        self,
        instruction: Instruction,
    ) -> InstructionCallable:
        """
        Resolves an instruction and all its arguments to an async callable.
        """

        parameters = await self._get_instruction_parameters(instruction)
        arguments = await self._resolve_instruction_arguments(instruction)
        self._check_arguments(instruction, parameters, arguments)

        code = self._get_instruction_code(instruction)
        # An instruction can be a linear piece of code to call every time or define a function to call.
        # Note that we don't actually run the code here, we just resolve and initialise.
        if not instruction.anonymous:
            output = await self.run_get_definitions(code, {**arguments})
            if instruction.name not in output:
                raise ValueError(f"function {instruction.name} not defined in code")
            return output[instruction.name]
        else:

            async def _run_anonymous(**kwargs):
                await self._do_exec(code, {**arguments, **kwargs})

            return _run_anonymous

    async def _proxy_model(self, model_handle: ModelHandle, model: Model) -> ModelProxy:
        return ModelProxy(handle=model_handle, model=model)

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
            "model", "model__default_settings", "dataset", "instruction"
        )
        bound_arguments_resolved = {}
        async for argument in bound_arguments:
            if argument.model:
                model_handle = await self._resolve_model(argument.model, argument.model_settings)
                model_proxy = await self._proxy_model(model_handle, argument.model)
                bound_arguments_resolved[argument.name] = model_proxy
            elif argument.dataset:
                bound_arguments_resolved[argument.name] = await self._resolve_dataset(
                    argument.dataset, argument.dataset_view
                )
            elif argument.instruction:
                callable = await self._resolve_instruction(argument.instruction)
                callable_proxy = await self._proxy_instruction(callable, argument.instruction)
                bound_arguments_resolved[argument.name] = callable_proxy
            else:
                bound_arguments_resolved[argument.name] = argument.value
        return bound_arguments_resolved

    def _check_arguments(
        self,
        instruction: Instruction,
        parameters: dict[str, InstructionParameter],
        arguments: dict[str, Any],
    ) -> None:
        """
        Checks that all required arguments are present and valid for the instruction, raising an error if not.
        """

        for parameter in parameters.values():
            # check that all required arguments are present
            if parameter.name not in arguments:
                raise ValueError(f"required parameter {parameter.name} not bound for {instruction}")
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
                raise RuntimeError(f"unknown parameter type: {parameter.type}")

        # ignore extraneous arguments

    async def run(
        self, instruction: Instruction, arguments: dict[str, Any]
    ) -> typing.Optional[dict[str, Any]]:
        # TODO @Feature: trace model/instruction executions (with context, recursively)
        try:
            callable = await self._resolve_instruction(instruction)
        except Exception as e:
            raise ValueError(
                f"error resolving instruction {instruction} with arguments {_arguments_summary(arguments)}: {e}"
            ) from e

        try:
            return await callable(**arguments)
        except SandboxError:
            # re-raise sandbox errors
            raise
        except Exception as e:
            raise SandboxError(
                f"error running {instruction} with arguments {_arguments_summary(arguments)}: {e}",
                e,
            ) from e

    async def run_get_definitions(self, code: str, globals: dict[str, Any]) -> dict[str, Any]:
        # remember the globals we started with, do not modify originals
        globals_copy = {**self.default_imports, **globals}
        globals_copy_keys = {*globals_copy.keys()}
        await self._do_exec(code, globals_copy)
        new_globals = {
            k: v
            for k, v in globals_copy.items()
            if k not in globals_copy_keys and k not in ("__builtins__", "__annotations__")
        }
        return new_globals

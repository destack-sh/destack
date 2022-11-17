from __future__ import annotations

import typing
import uuid
from typing import Any, Iterator, Union

from django.db.models import QuerySet

from bench.model.base import ModelHandler
from bench.models import Dataset, Model
from bench.models.instruction import (
    Instruction,
    InstructionArgument,
    InstructionParameter,
    InstructionParameterType,
)
from bench.settings import DEBUG, TEST
from bench.utils.record import Record, RecordBatch, RecordList


class DatasetWrapper(RecordBatch):
    """
    Wraps a Dataset as a simple array-like record batch.
    """

    def __init__(self, dataset: Dataset):
        self._dataset = dataset

    @typing.overload
    def __getitem__(self, index: int) -> Record:
        ...

    @typing.overload
    def __getitem__(self, index: slice) -> RecordBatch:
        ...

    @typing.overload
    def __getitem__(self, index: str) -> list:
        ...

    def __getitem__(self, index: Union[int, slice, str]) -> Union[Record, RecordBatch, list]:
        if isinstance(index, int):
            return self._dataset.get(index).data
        elif isinstance(index, slice):
            return self._dataset.get_slice(index.start, index.stop)
        elif isinstance(index, str):
            return self._dataset.get_field(index)
        else:
            raise TypeError(f"Invalid index type: {type(index)}")

    def __iter__(self) -> Iterator[Record]:
        yield from self._dataset

    def __len__(self) -> int:
        return len(self._dataset)


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

    pass


def _arguments_summary(arguments: dict[str, Any]) -> str:
    """
    Summarize the names and types of arguments.
    """
    return ", ".join(f"{name}={type(value).__name__}" for name, value in arguments.items())


class Executor:
    def __init__(self):
        self.executor_id = uuid.uuid4().hex
        self.builtin_instructions = {}
        self.can_exec = DEBUG or TEST  # or sandboxed
        self.default_imports = {Model: ModelHandler, Dataset: RecordBatch}

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

    async def _resolve_model(self, model: Model) -> ModelHandler:
        if model.handler_id == "openai":
            from bench.model.openai import OpenAIHandler

            return OpenAIHandler(**model.handler_arguments)
        else:
            raise ValueError(f"unknown model handler: {model.handler_id}")

    async def _resolve_dataset(self, dataset: Dataset) -> RecordBatch:
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
    ) -> typing.Callable[..., typing.Coroutine]:
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

    async def _get_instruction_parameters(
        self, instruction: Instruction
    ) -> dict[str, InstructionParameter]:
        parameters = {}
        async for parameter in instruction.parameters.all():
            parameters[parameter.name] = parameter
        return parameters

    async def _resolve_instruction_arguments(self, instruction: Instruction) -> dict[str, Any]:
        bound_arguments: QuerySet[InstructionArgument] = instruction.arguments.all().select_related(
            "model", "dataset", "instruction"
        )
        bound_arguments_resolved = {}
        async for argument in bound_arguments:
            if argument.model:
                bound_arguments_resolved[argument.name] = await self._resolve_model(argument.model)
            elif argument.dataset:
                bound_arguments_resolved[argument.name] = await self._resolve_dataset(
                    argument.dataset
                )
            elif argument.instruction:
                bound_arguments_resolved[argument.name] = await self._resolve_instruction(
                    argument.instruction
                )
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
                if not isinstance(arguments[parameter.name], ModelHandler):
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

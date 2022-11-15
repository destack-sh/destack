from __future__ import annotations

import typing
import uuid
from typing import Any, Iterator, Union
from unittest import mock

from django.db.models import QuerySet

from bench.model.base import ModelHandler
from bench.models import Dataset, Model
from bench.models.instruction import Instruction, InstructionArgument
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


class Executor:
    def __init__(self):
        self.executor_id = uuid.uuid4().hex
        self.builtin_instructions = {}
        self.can_exec = DEBUG or TEST

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
            raise SandboxError(f"error executing code: {e}", e) from e

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

        arguments = await self._resolve_instruction_arguments(instruction)

        code = self._get_instruction_code(instruction)
        # An instruction can be a linear piece of code to call every time or define a function to call.
        # Note that we don't actually run the code here, we just resolve and initialise.
        if not instruction.anonymous:
            output = await self.run_get_definitions(code, arguments)
            if instruction.name not in output:
                raise ValueError(f"function {instruction.name} not defined in code")
            return output[instruction.name]
        else:

            async def _run_anonymous(**kwargs):
                await self._do_exec(code, {**arguments, **kwargs})

            return _run_anonymous

    async def _resolve_instruction_arguments(self, instruction: Instruction) -> dict[str, Any]:
        bound_arguments: QuerySet[InstructionArgument] = instruction.arguments.all()
        bound_arguments_resolved = {}
        async for argument in bound_arguments:
            if argument.model:
                bound_arguments_resolved[argument.name] = self._resolve_model(argument.model)
            elif argument.dataset:
                bound_arguments_resolved[argument.name] = self._resolve_dataset(argument.dataset)
            elif argument.instruction:
                bound_arguments_resolved[argument.name] = await self._resolve_instruction(
                    argument.instruction
                )
            else:
                bound_arguments_resolved[argument.name] = argument.value
        return bound_arguments_resolved

    async def run(
        self, instruction: Instruction, arguments: dict[str, Any]
    ) -> typing.Optional[dict[str, Any]]:
        # TODO @Feature: trace model/instruction executions (with context, recursively)
        callable = await self._resolve_instruction(instruction)
        return await callable(**arguments)

    async def run_get_definitions(self, code: str, globals: dict[str, Any]) -> dict[str, Any]:
        available_globals = {
            "benv": mock.MagicMock(),  # don't need actual bench execution env values here
            **globals,
        }
        # remember the globals we started with
        available_globals_keys = {*available_globals.keys()}
        await self._do_exec(code, available_globals)
        new_globals = {
            k: v
            for k, v in available_globals.items()
            if k not in available_globals_keys and k != "__builtins__"
        }
        return new_globals

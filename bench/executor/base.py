from __future__ import annotations

import uuid
from typing import Any
from unittest import mock

from bench.models.instruction import Instruction
from bench.settings import DEBUG, TEST


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
            raise ValueError("exec outside sandbox is not allowed")

        exec(code, globals)

    async def run(self, instruction: Instruction, arguments: dict[str, Any]) -> dict[str, Any]:
        code = self._get_instruction_code(instruction)
        # TODO @Feature: load and impute arguments from instruction arguments
        # TODO @Feature: load nested instructions

        output = await self.run_get_definitions(code, arguments)
        if instruction.name in output and callable(output[instruction.name]):
            run_function = output[instruction.name]
            output = await run_function(**arguments)
        return output

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

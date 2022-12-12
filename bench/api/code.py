from typing import Optional

import strawberry
from asgiref.sync import async_to_sync
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api import types
from bench.backend.executor import Executor
from bench.backend.resolver import Resolver
from bench.backend.tracing import ExecutionTrace


@strawberry.input
class RunCodeValueArgumentInput:
    name: str
    value: str


@strawberry.input
class RunCodeInput:
    code_id: GlobalID
    arguments: list[RunCodeValueArgumentInput]


@strawberry.type
class RunCodeOutput:
    name: str
    value: str


@strawberry.type
class RunCodePayload:
    code: types.Code
    execution: types.Execution
    outputs: Optional[list[RunCodeOutput]]


@strawberry.type
class CodeMutation:
    @strawberry.mutation
    def run(self, input: RunCodeInput) -> RunCodePayload:
        code = models.Code.objects.get(id=input.code_id.node_id)
        # assumes only value arguments
        arguments = {arg.name: arg.value for arg in input.arguments}
        executor = Executor(Resolver())
        execution_trace = ExecutionTrace(frames=[])
        output = async_to_sync(executor.resolve_and_run)(code, arguments, [execution_trace])

        if execution_trace.root is None:
            raise RuntimeError(f"empty execution trace for {code}")

        if isinstance(output, dict):
            outputs = [RunCodeOutput(name=name, value=value) for name, value in output.items()]
        else:
            outputs = [RunCodeOutput(name="output", value=output)]
        execution = models.Execution.objects.get(id=execution_trace.root.id)
        return RunCodePayload(code=code, execution=execution, outputs=outputs)

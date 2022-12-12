from __future__ import annotations

from typing import TYPE_CHECKING, Annotated, Optional

import strawberry
from asgiref.sync import async_to_sync
from strawberry import auto, lazy
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.misc import SchemaElement
from bench.api.symbol import Symbol, SymbolContent
from bench.backend.executor import Executor
from bench.backend.resolver import Resolver
from bench.backend.tracing import ExecutionTrace

if TYPE_CHECKING:
    from bench.api.model import Model
    from bench.api.task import Task


@gql.django.type(models.Code)
class Code(SymbolContent):
    symbol: Symbol
    input_schema: SchemaElement
    output_schema: SchemaElement
    task: Optional[Annotated["Task", lazy(".task")]]
    builtin_id: auto
    code: auto


@gql.django.type(models.Execution)
class Execution(gql.Node):
    created_at: auto
    updated_at: auto
    started_at: auto
    terminated_at: auto
    duration_millis: auto
    status: auto
    inputs: auto
    outputs: auto
    error: auto
    parent: Optional[Execution]
    children: list[Execution]
    code: Code
    model: Optional[Annotated["Model", lazy(".misc")]]


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
    code: Code
    execution: Execution
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

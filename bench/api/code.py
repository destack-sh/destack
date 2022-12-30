from typing import TYPE_CHECKING, Annotated, Optional

import strawberry
from asgiref.sync import async_to_sync
from strawberry import auto, lazy
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.symbol import Statement, SymbolContent
from bench.backend.execute import Executor
from bench.backend.tracing import ExecutionTrace

if TYPE_CHECKING:
    from bench.api.model import Model


@gql.django.type(models.Code)
class Code(SymbolContent):
    definition: Statement
    builtin_id: auto
    code: auto
    length: auto


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
    parent: Optional["Execution"]
    children: list["Execution"]
    code: Code
    model: Optional[Annotated["Model", lazy(".model")]]


@gql.input
class CodeUpdateContentCode:
    statement_id: GlobalID
    builtin_id: Optional[str] = None
    code: Optional[str] = None


@gql.type
class CodeMutation:
    @gql.mutation
    def update_code_content(self, input: CodeUpdateContentCode) -> Statement:
        statement: models.Statement = models.Statement.objects.get(id=input.statement_id.node_id)
        statement.code_.builtin_id = input.builtin_id
        statement.code_.code = input.code
        statement.code_.save()
        return statement


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
class CodeRunMutation:
    @strawberry.mutation
    def run(self, input: RunCodeInput) -> RunCodePayload:
        code = models.Code.objects.get(id=input.code_id.node_id)
        # assumes only value arguments
        arguments = {arg.name: arg.value for arg in input.arguments}
        executor = Executor()
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

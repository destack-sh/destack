from __future__ import annotations

from typing import TYPE_CHECKING, Annotated, Optional

import strawberry
from asgiref.sync import async_to_sync
from strawberry import auto, lazy
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.backend.executor import Executor
from bench.backend.resolver import Resolver
from bench.compiler import Compiler, get_stdlib_model

if TYPE_CHECKING:
    from bench.api.code import Code
    from bench.api.model import Model
    from bench.api.symbol import Symbol
    from bench.api.task import Task


@gql.django.type(models.Compilation)
class Compilation(gql.Node):
    created_at: auto
    updated_at: auto
    task: Annotated["Task", lazy(".task")]
    name: auto
    backends: list[Annotated["Model", lazy(".model")]]
    target_task: Optional[Annotated["Task", lazy(".task")]]
    target_code: Optional[Annotated["Code", lazy(".code")]]
    mappings: list[SourceMapping]


@gql.django.type(models.SourceMapping)
class SourceMapping(gql.Node):
    compilation: Compilation
    source: Annotated["Symbol", lazy(".symbol")]
    source_path: auto
    target: Annotated["Symbol", lazy(".symbol")]
    target_path: auto


@strawberry.input
class CompileInput:
    compilation_id: GlobalID


@strawberry.type
class CompilePayload:
    compilation: Compilation


@strawberry.input
class AddCompilationInput:
    task_symbol_id: GlobalID
    name: str
    backends: list[str]


@strawberry.type
class AddCompilationPayload:
    compilation: Compilation


class CompilationMutation:
    @strawberry.mutation
    def add_compilation_target(self, input: AddCompilationInput) -> AddCompilationPayload:
        task = models.Symbol.objects.get(id=input.task_symbol_id.node_id).task_
        backends = [get_stdlib_model(backend) for backend in input.backends]
        compilation = task.add_compilation(input.name, backends)
        return AddCompilationPayload(compilation=compilation)

    @strawberry.mutation
    def compile(self, input: CompileInput) -> CompilePayload:
        compilation = (
            models.Compilation.objects.all()
            .select_related("project_version", "task", "target_task", "target_code")
            .get(id=input.compilation_id.node_id)
        )
        executor = Executor(Resolver())
        compiler = Compiler(executor)
        async_to_sync(compiler.compile)(compilation)
        return CompilePayload(compilation=compilation)

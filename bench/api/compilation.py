from typing import TYPE_CHECKING, Annotated

import strawberry
from asgiref.sync import async_to_sync
from strawberry import auto, lazy
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID

import bench.models.compilation
from bench import models
from bench.backend.compiler import Compiler, get_stdlib_model
from bench.backend.executor import Executor
from bench.backend.mapper import Mapper

if TYPE_CHECKING:
    from bench.api.symbol import Statement


@gql.django.type(bench.models.compilation.Compilation)
class Compilation(gql.Node):
    mappings: list["SourceMapping"]


@gql.django.type(bench.models.compilation.SourceMapping)
class SourceMapping(gql.Node):
    source: Annotated["Statement", lazy(".symbol")]
    source_path: auto
    source_revision: auto
    target: Annotated["Statement", lazy(".symbol")]
    target_path: auto
    target_revision: auto


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


@strawberry.type
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
            bench.models.compilation.Compilation.objects.all()
            .select_related("project_version", "task", "target_task", "target_code")
            .get(id=input.compilation_id.node_id)
        )
        executor = Executor(Mapper())
        compiler = Compiler(executor)
        async_to_sync(compiler.compile)(compilation)
        return CompilePayload(compilation=compilation)

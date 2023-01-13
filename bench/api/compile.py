import strawberry
from asgiref.sync import async_to_sync
from strawberry_django_plus.relay import GlobalID

from bench import models
from bench.api.symbol import Compilation


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
        async_to_sync(compiler.compile)(compilation)
        return CompilePayload(compilation=compilation)

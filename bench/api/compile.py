import strawberry
from asgiref.sync import async_to_sync
from strawberry_django_plus.relay import GlobalID

from bench.api.symbol import Statement


@strawberry.input
class CompileInput:
    compilation_id: GlobalID


@strawberry.type
class CompilePayload:
    compilation: Statement


@strawberry.input
class AddCompilationInput:
    task_symbol_id: GlobalID
    name: str
    backends: list[str]


@strawberry.type
class CompilationMutation:
    @strawberry.mutation
    def compile(self, input: CompileInput) -> CompilePayload:
        compilation = (
            bench.models.compilation.Compilation.objects.all()
            .select_related("project_version", "task", "target_task", "target_code")
            .get(id=input.compilation_id.node_id)
        )
        async_to_sync(compiler.compile)(compilation)
        return CompilePayload(compilation=compilation)

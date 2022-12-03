from asgiref.sync import async_to_sync
from django.core.management import BaseCommand
from django.core.management.base import CommandParser

from bench.compiler import Compiler, get_stdlib_model
from bench.executor import Executor
from bench.models import Project, SymbolType


class Command(BaseCommand):
    help = "Compiles a task into optimized code"

    def add_arguments(self, parser: CommandParser):
        # project name as organization/project
        parser.add_argument("organization_project", type=str)
        # task file path (must exist and end in .py)
        parser.add_argument("task_name", type=str)
        # backend names
        parser.add_argument("--backends", type=str, nargs="+", required=True)

    def handle(self, organization_project: str, task_name: str, *args, **options):
        project = Project.objects.get_by_slug(*organization_project.split("/"))
        if project is None:
            raise ValueError(f"project not found: {organization_project}")

        project_v = project.head_
        task_def = project_v.symbol_definition(task_name, SymbolType.TASK)
        # get backend models as owner/model from its backends library
        backends = []
        for backend in options["backends"]:
            backends.append(get_stdlib_model(backend))
        if not backends:
            raise ValueError("no backends provided")

        executor = Executor()
        compiler = Compiler(executor)
        compilation, _ = task_def.task.compilations.get_or_create(
            project_version=project_v, name="default"
        )
        compilation.backends.set(backends)
        task_def.task.compilations.set([compilation])
        async_to_sync(compiler.compile)(compilation)

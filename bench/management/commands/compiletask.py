from asgiref.sync import async_to_sync
from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.compiler import Compiler, get_backend_model
from bench.executor import Executor
from bench.models import Compilation, Organization, Project, ProjectFileType


class Command(BaseCommand):
    help = "Compiles a task into optimized instructions"

    def add_arguments(self, parser: CommandParser):
        # project name as organization/project
        parser.add_argument("organization_project", type=str)
        # task file path (must exist and end in .py)
        parser.add_argument("task_name", type=str)
        # backend names
        parser.add_argument("--backends", type=str, nargs="+", required=True)

    @transaction.atomic
    def handle(self, organization_project: str, task_name: str, *args, **options):
        organization = Organization.objects.get(slug=organization_project.split("/")[0])
        project = Project.objects.filter(
            slug=organization_project.split("/")[1], organization=organization
        ).first()
        if project is None:
            raise ValueError(f"project not found: {organization_project}")

        project_version = project.head
        if project_version is None:
            raise ValueError(f"project has no head: {project}")
        task = project_version.files.get(type=ProjectFileType.TASK, name=task_name).task
        if task is None:
            raise ValueError(f"project version has no task {task_name}: {project_version}")

        # get backend models as owner/model from its backends library
        backends = []
        for backend in options["backends"]:
            backends.append(get_backend_model(backend))
        if not backends:
            raise ValueError("no backends provided")

        executor = Executor()
        compiler = Compiler(executor)
        compilation = Compilation.objects.create(
            task=task, source_instruction=task.template_implementation
        )
        compilation.backends.set(backends)
        async_to_sync(compiler.compile)(compilation)

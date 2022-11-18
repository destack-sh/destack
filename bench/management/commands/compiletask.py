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
        parser.add_argument("project", type=str)
        # task file path (must exist and end in .py)
        parser.add_argument("task", type=str)
        # backend names
        parser.add_argument("--backends", type=str, nargs="+", required=True)

    @transaction.atomic
    def handle(self, project: str, task: str, *args, **options):
        organization, project = project.split("/")
        organization = Organization.objects.get(slug=organization)
        project = Project.objects.filter(slug=project, organization=organization).first()
        project_version = project.head
        task = project_version.files.get(type=ProjectFileType.TASK, name=task).task

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
        async_to_sync(compiler.compile)(compilation)

from asgiref.sync import async_to_sync
from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.compiler import Compiler, get_backend_model
from bench.executor import Executor
from bench.models import Compilation, Organization, Project, ProjectFileType


class Command(BaseCommand):
    help = "Sets up dev environment with sample data"

    def add_arguments(self, parser: CommandParser):
        # task file path (must exist and end in .py)
        parser.add_argument("task", type=str)
        # organization name
        parser.add_argument("--organization", type=str, required=True)
        # project name
        parser.add_argument("--project", type=str, required=True)
        # backend names
        parser.add_argument("--backends", type=str, nargs="+", required=True)

    @transaction.atomic
    def handle(self, *args, **options):
        organization = Organization.objects.get(slug=options["organization"])
        project = Project.objects.filter(slug=options["project"], organization=organization).first()
        project_version = project.head
        task = project_version.files.get(type=ProjectFileType.TASK, name=options["task"]).task

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

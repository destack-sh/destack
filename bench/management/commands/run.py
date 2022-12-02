from __future__ import annotations

from asgiref.sync import async_to_sync
from django.core.management import BaseCommand
from django.core.management.base import CommandParser

from bench.executor import Executor
from bench.models import Compilation, Project, SymbolType
from bench.models.project import ProjectType


class Command(BaseCommand):
    help = "Runs a project's program with the given arguments"

    def add_arguments(self, parser: CommandParser) -> None:
        # project as organization/project[:compilation]
        parser.add_argument("organization_project", type=str)
        # add input string as only variable
        parser.add_argument("input", type=str)

    def handle(self, organization_project: str, input: str, **kwargs):
        if ":" in organization_project:
            organization_project, compilation_name = organization_project.split(":")
        else:
            compilation_name = None

        project = Project.objects.get_by_slug(*organization_project.split("/"))
        if project.type != ProjectType.EXECUTABLE:
            raise ValueError(f"project must be executable: {project}")
        project_v = project.head_
        main_program = project_v.main_program
        if main_program.type != SymbolType.TASK:
            raise ValueError(f"main program must be a task: {main_program}")
        if main_program.task.compilations.count() == 0:
            raise ValueError(f"main program must be compiled: {main_program}")
        if main_program.task.compilations.count() > 1 and not compilation_name:
            raise ValueError(f"no compilation name given and there are multiple: {main_program}")

        if not compilation_name:
            compilation: Compilation = main_program.task.compilations.get()
        else:
            compilation = main_program.task.compilations.get(name=compilation_name)
        main_code = compilation.output_code

        executor = Executor()
        output = async_to_sync(executor.run)(main_code, {"input": input})
        print(output)

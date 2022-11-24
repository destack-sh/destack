from asgiref.sync import async_to_sync
from django.core.management import BaseCommand
from django.core.management.base import CommandParser

from bench.executor import Executor
from bench.models import Organization, Project
from bench.models.project import ProjectType


class Command(BaseCommand):
    help = "Runs a project's program with the given arguments"

    def add_arguments(self, parser: CommandParser) -> None:
        # project as organization/project
        parser.add_argument("organization_project", type=str)
        # add input string as only variable
        parser.add_argument("input", type=str)

    def handle(self, organization_project: str, input: str, *args, **kwargs):
        organization = Organization.objects.get(slug=organization_project.split("/")[0])
        project: Project = Project.objects.get(
            slug=organization_project.split("/")[1], organization=organization
        )
        if project.type != ProjectType.EXECUTABLE:
            raise ValueError(f"project must be executable: {project}")
        project_version = project.head
        if project_version is None:
            raise ValueError(f"project has no head: {project}")
        program = project_version.program
        if program is None:
            raise ValueError(f"project version has no program: {project_version}")

        # TODO @Feature: set current compiled project builds automatically
        #  (and allow for multiple builds?, ask if there is more than one)
        # just use the latest version of the program for now
        compiled_program = program.implementations.order_by("-created_at").first()

        executor = Executor()
        output = async_to_sync(executor.run)(compiled_program, {"input": input})
        print(output)

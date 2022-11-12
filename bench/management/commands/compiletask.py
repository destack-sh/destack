from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.models import Organization, Project


class Command(BaseCommand):
    help = "Sets up dev environment with sample data"

    def add_arguments(self, parser: CommandParser):
        # task file path (must exist and end in .py)
        parser.add_argument("task", type=str)
        # organization name
        parser.add_argument("--organization", type=str, required=True)
        # project name
        parser.add_argument("--project", type=str, required=True)

    @transaction.atomic
    def handle(self, *args, **options):
        organization = Organization.objects.get(slug=options["organization"])
        project = Project.objects.filter(slug=options["project"], organization=organization).first()
        project_version = project.head
        task = project_version.tasks.get(name=options["task"])
        print(task)

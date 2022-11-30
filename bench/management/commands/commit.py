from __future__ import annotations

import time

from django.core.management import BaseCommand
from django.core.management.base import CommandParser

from bench.models import Project


class Command(BaseCommand):
    help = "Commits a project"

    def add_arguments(self, parser: CommandParser):
        # project name as organization/project
        parser.add_argument("organization_project", type=str)
        # old version name (optional)
        parser.add_argument("name", type=str, nargs="?")

    def handle(self, organization_project: str, name: str | None, *args, **options):
        project = Project.objects.get_by_slug(*organization_project.split("/"))
        if project is None:
            raise ValueError(f"project not found: {organization_project}")
        start_time = time.time()
        parent = project.head_
        project.create_version(parent=parent, commit_name=name)
        self.stdout.write(f"committed {parent} (took {time.time() - start_time:.2f}s)")

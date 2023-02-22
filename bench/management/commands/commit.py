from __future__ import annotations

import time

from django.core.management import BaseCommand
from django.core.management.base import CommandParser

from bench.models import Project


class Command(BaseCommand):
    help = "Commits a project"

    def add_arguments(self, parser: CommandParser):
        # project name as owner/project
        parser.add_argument("owner_project", type=str)
        # old version name (optional)
        parser.add_argument("name", type=str, nargs="?")

    def handle(self, owner_project: str, name: str | None, *args, **options):
        project = Project.objects.get_by_slug(*owner_project.split("/"))
        if project is None:
            raise ValueError(f"project not found: {owner_project}")
        start_time = time.time()
        parent = project.head_
        project.create_version(parent=parent, commit_name=name)
        self.stdout.write(f"committed {parent} (took {time.time() - start_time:.2f}s)")

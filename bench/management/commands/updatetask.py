from dataclasses import dataclass
from typing import Optional

from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.models import Organization, Project


@dataclass
class TaskFileSegment:
    header: str
    lines: list[str]


class Command(BaseCommand):
    help = "Loads a task from a file into a project"

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
        if project is None:
            project = Project.objects.create(
                organization=organization, name=options["project"], slug=options["project"]
            )

        # read task file lines
        with open(options["task"], "r") as f:
            lines = f.readlines()

        # parse all bench segments from lines (look like this # @bench ... # @/bench)
        segments: list[TaskFileSegment] = []
        segment: Optional[TaskFileSegment] = None

        for line in lines:
            if line.startswith("# @bench"):
                segment = TaskFileSegment(header=line[8:].strip(), lines=[])
            elif line.startswith("# @/bench"):
                segments.append(segment)
                segment = None
            elif segment is not None:
                segment.lines.append(line.strip())

        print(segments)

        new_version = project.create_version()
        new_version.reset()

        # possible instructions (in header are):
        # ignore: ignore this segment (used for imports)
        # task: define a task
        # flow: define a flow

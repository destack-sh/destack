from dataclasses import dataclass
from typing import Optional

from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction


@dataclass
class TaskFileSegment:
    header: str
    lines: list[str]


class Command(BaseCommand):
    help = "Sets up dev environment with sample data"

    def add_arguments(self, parser: CommandParser):
        # task file path (must exist and end in .py)
        parser.add_argument("task", type=str)

    @transaction.atomic
    def handle(self, *args, **options):
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
                segment.lines.append(line)

        print(segments)

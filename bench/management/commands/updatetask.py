from dataclasses import dataclass
from typing import Optional, re
from unittest import mock

from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.models import Dataset, Instruction, Organization, Project, Task
from bench.models.project import ProjectFileType


@dataclass
class TaskFileSegment:
    header: str
    lines: list[str]
    source_index: int

    @property
    def full_code(self):
        return "\n".join(self.lines)


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
            project = Project.objects.create_project(
                organization=organization, name=options["project"], slug=options["project"]
            )

        # read task file lines
        with open(options["task"], "r") as f:
            lines = f.readlines()

        # parse all bench segments from lines (look like this # @bench ... # @/bench)
        segments: list[TaskFileSegment] = []
        segment: Optional[TaskFileSegment] = None

        for i, line in enumerate(lines):
            if "@bench" in line:
                if segment is not None:
                    # close previous segment
                    segments.append(segment)

                segment = TaskFileSegment(header=line[8:].strip(), lines=[], source_index=i)
            elif "@/bench" in line:
                segments.append(segment)
                segment = None
            elif segment is not None:
                segment.lines.append(line)
        if segment is None:
            raise ValueError("task file must contain at least one @bench segment")

        # close last segment
        segments.append(segment)

        new_version = project.create_version()
        new_version.reset()

        def exec_get_definitions(code: str):
            """
            Execute the given code and return new definitions.
            This is UNSAFE and should only be run on trusted code or in a sandboxed environment.
            """
            available_globals = {
                "benv": mock.MagicMock(),  # don't need actual bench execution env values here
            }
            # remember the globals we started with
            available_globals_keys = {*available_globals.keys()}
            exec(code, available_globals)
            new_globals = {
                k: v
                for k, v in available_globals.items()
                if k not in available_globals_keys and k != "__builtins__"
            }
            return new_globals

        # convert segments to a single task definition tree
        tasks = {}
        instructions = {}
        datasets = {}
        for segment in segments:
            if segment.header.startswith("ignore"):
                continue

            # The possible definitions (in header) are:
            #  ignore: ignore this segment (used for imports)
            #  task [args]: <name> -> define a task
            #  instruction [args]: <name> -> define a instruction
            #  dataset [args]: <name> -> define a dataset
            definitions = exec_get_definitions(segment.full_code)
            args, file_name = segment.header.split(":", 1)
            args = args.split(" ")
            file_name = file_name.strip()
            if args[0] == "task":
                task_definition = definitions[file_name]
                tasks[file_name] = Task.objects.create(
                    name=file_name,
                    description=(task_definition["description"]),
                    schema=(task_definition["schema"]),
                )
            elif args[0] == "instruct":
                # parse header "instruction [args]: <name>"
                instruction_definition = definitions[file_name]
                # instruction definition must be a function
                if not callable(instruction_definition):
                    raise ValueError(
                        f"instruction definition: {instruction_definition} must be a function"
                    )
                instruction = Instruction.objects.create(
                    name=file_name, type=args[1], code=segment.full_code
                )
                # assign instruction as implementation to task
                if instruction.name in tasks:
                    instruction.task = tasks[instruction.name]
                    instruction.save()
                instructions[file_name] = instruction

                # parse instruction parameters from code
                # they are defined as type only definitions like:
                # name: Model
                # name: Dataset
                # name: Callable
                # name: <type>
                parameters = re.findall(r"^(\w+): (\w+)$", segment.full_code)

                if instruction.type == "expect":
                    tasks[args[2]].expectations.add(instruction)
            elif args[0] == "dataset":
                dataset_records = definitions[file_name]
                # schema is just keys and types of values of the first element
                schema = {k: type(v).__name__ for k, v in dataset_records[0].items()}
                dataset = Dataset.objects.create(name=file_name, schema=schema, type=args[1])
                dataset.extend(dataset_records)
                datasets[file_name] = dataset

                if dataset.type == "examples":
                    tasks[args[2]].examples.add(dataset)
                elif dataset.type == "explanations":
                    tasks[args[2]].explanations.add(dataset)
            else:
                raise ValueError(f"Unknown segment header: {segment.header}")

        # create files for tasks, instructions and datasets
        for task in tasks.values():
            new_version.files.create(name=task.name, type=ProjectFileType.TASK, task=task)
        for instruction in instructions.values():
            new_version.files.create(
                name=instruction.name, type=ProjectFileType.INSTRUCTION, instruction=instruction
            )
        for dataset in datasets.values():
            new_version.files.create(
                name=dataset.name, type=ProjectFileType.DATASET, dataset=dataset
            )

        # advance head to new version
        project.head = new_version
        project.save()

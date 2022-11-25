import typing
from dataclasses import dataclass
from typing import Optional

from asgiref.sync import async_to_sync
from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.compiler import get_backend_model
from bench.executor import Executor
from bench.executor.builtins import instruction_builtins
from bench.models import Dataset, Instruction, Organization, Project, Task
from bench.models.instruction import InstructionParameterType, InstructionScope


@dataclass
class TaskFileSegment:
    header: str
    lines: list[str]
    source_index: int

    @property
    def full_code(self):
        # code already contains newlines
        return "".join(self.lines)


class Command(BaseCommand):
    help = "Loads a task definition from a file into a project"

    def add_arguments(self, parser: CommandParser):
        # project as organization/project
        parser.add_argument("organization_project", type=str)
        # task file path (must exist and end in .py)
        parser.add_argument("task", type=str)
        # the task to make the new main program
        parser.add_argument("--main", type=str, required=False)

    @transaction.atomic
    def handle(self, organization_project: str, main: Optional[str] = None, **options):
        organization_slug, project_slug = organization_project.split("/")
        organization = Organization.objects.get(slug=organization_slug)
        project = Project.objects.filter(slug=project_slug, organization=organization).first()
        if project is None:
            project = Project.objects.create_project(
                organization=organization, name=project_slug, slug=project_slug
            )

        # read task file lines
        with open(options["task"], "r") as f:
            lines = f.readlines()

        segments = self.parse_task_file_segments(lines)

        new_version = project.create_version()
        new_version.reset()
        executor = Executor()

        # convert segments to a single task definition tree
        tasks: dict[str, Task] = {}
        instructions: dict[str, Instruction] = {}
        datasets: dict[str, Dataset] = {}
        for segment in segments:
            if segment.header.startswith("ignore"):
                continue

            definitions = async_to_sync(executor.run_get_definitions)(segment.full_code, {})

            def _get_definition(name: str, required: bool = True):
                if name not in definitions:
                    if not required:
                        return None
                    else:
                        raise ValueError(
                            f"definition '{name}' not found in segment '{segment.header}':\n{segment.full_code}"
                        )
                return definitions[name]

            def _get_relevant_code(definition: typing.Callable):
                # this only works for functions
                relevant_lines = []

                in_def = False
                # find first line index where def definition.__name__ is
                # then get all lines following definition until indent is gone
                for line in segment.lines:
                    if "def " + definition.__name__ in line:
                        in_def = True
                    if not in_def:
                        continue
                    # exit when we're out
                    if "def " not in line and not line.startswith("   ") and not line.strip() == "":
                        break
                    relevant_lines.append(line)

                # code already contains newlines
                return "".join(relevant_lines)

            # The current headers are task, instruct, dataset
            # in the form <header> *args: <name>,[<name>,...]
            args_str, names_str = segment.header.split(":", 1)
            args = args_str.split(" ")
            names = names_str.strip().split(",")
            del names_str  # prevent accidental use
            if args[0] == "task":
                name = names[0].strip()
                schema = _get_definition("schema")
                task = Task.objects.create(
                    name=name,
                    schema=schema,
                )
                tasks[name] = task
                self.stdout.write(f"Created task {task}")
            elif args[0] == "instruct":
                # parse header "instruction [args]: <name>[,<name>...]"
                for name in names:
                    definition = _get_definition(name)
                    # create instructions/datasets for instruct
                    if callable(definition):
                        # get only relevant code since multiple names may be defined
                        relevant_code = _get_relevant_code(definition)
                        # create instruction
                        instruction = Instruction.objects.create(
                            name=name,
                            scope=InstructionScope.FUNCTION,
                            code=relevant_code,
                        )
                        instructions[name] = instruction

                        # parse instruction parameters from code
                        # if multiple instructions are defined in this instruct just re-use all parameters
                        # (this isn't great, but we'll have modules soon)
                        self._attach_instruction_parameters(
                            segment, instruction, datasets, instructions
                        )
                        self.stdout.write(f"Created instruction {instruction}")
                    elif isinstance(definition, list):
                        # create dataset
                        dataset_records = _get_definition(name)
                        dataset = Dataset.objects.from_list(name, dataset_records)
                        datasets[name] = dataset
                        self.stdout.write(f"Created dataset {dataset}")

                if args[1] == "expect":
                    # attach to args[2] task as expectations
                    description = _get_definition("expectation")
                    expectation = tasks[args[2]].expectations.create(
                        index=tasks[args[2]].expectations.count(), description=description
                    )
                    example_datasets = [datasets[n] for n in names if n in datasets]
                    expectation.examples_datasets.set(example_datasets)
                    expect_instructions = [instructions[n] for n in names if n in instructions]
                    expectation.instructions.set(expect_instructions)
                    self.stdout.write(f"Created expectation {expectation}")
                elif args[1] == "task":
                    if len(names) > 1:
                        raise ValueError(f"only one task implementation can be provided: {names}")
                    # add as task implementation
                    task = tasks[args[2]]
                    if task.template_implementation is not None:
                        raise ValueError(f"task {task} already has a template implementation")
                    implementation = instructions[names[0]]
                    task.template_implementation = implementation
                    self.stdout.write(f"Set implementation for {task} to {implementation}")

            elif args[0] == "dataset":
                if len(names) > 1:
                    raise ValueError("only one dataset name allowed per segment")
                name = names[0]
                del names  # prevent accidental re-use

                dataset_records = _get_definition(name)
                dataset = Dataset.objects.from_list(name, dataset_records)
                datasets[name] = dataset
                self.stdout.write(f"Created dataset {dataset}")
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

        # set task as new main program
        if main:
            new_version.program = tasks[main]
            new_version.save()
            self.stdout.write(f"Set {new_version.program} as main program in {new_version}")

        # advance head to new version
        project.head = new_version
        project.save()
        self.stdout.write(f"Updated head in {project}")

    def _attach_instruction_parameters(
        self,
        segment: TaskFileSegment,
        instruction: Instruction,
        datasets: dict[str, Dataset],
        instructions: dict[str, Instruction],
    ):
        # parameters are defined as type only definition lines like:
        # name: Model
        # name: Dataset
        # name: Callable
        # name: <type>
        # Parameters are automatically bound unless # @parameter is appended.
        for line in segment.lines:
            # assume all parameters are declared up front
            if ":" not in line or "=" in line or "(" in line:
                break
            if "#" in line:
                comment = line[line.find("#") :]
                line = line[: line.find("#")]
            else:
                comment = ""
            param_name, param_type = line.split(":", 1)
            param_name = param_name.strip()
            param_type = param_type.strip()

            if param_name in instruction_builtins:
                # ignore builtin instructions
                continue

            param_schema = None
            if param_type == "Dataset":
                param_type = InstructionParameterType.DATASET
            elif param_type == "Model":
                param_type = InstructionParameterType.MODEL
            elif "Callable" in param_type:
                param_type = InstructionParameterType.INSTRUCTION
            else:
                # just use python type as schema for now
                param_schema = param_type
                param_type = InstructionParameterType.JSON
            instruction.add_parameter(
                name=param_name,
                type=param_type,
                schema=param_schema,
            )

            if "@parameter" in comment:
                continue

            # otherwise resolve argument of same name
            if param_type == InstructionParameterType.DATASET:
                instruction.arguments.create(
                    name=param_name,
                    type=InstructionParameterType.DATASET,
                    dataset=datasets[param_name],
                )
            elif param_type == InstructionParameterType.MODEL:
                if "@backend" in comment:
                    # parse model backend from # @backend <backend>
                    backend = comment[comment.find("@backend") + 9 :].strip()
                    model = get_backend_model(backend)
                else:
                    raise NotImplementedError(
                        f"generic model argument resolution not implemented yet: {line}"
                    )
                instruction.bind_argument(param_name, model)
            elif param_type == InstructionParameterType.INSTRUCTION:
                instruction = instructions[param_name]
                instruction.bind_argument(param_name, instruction)
            elif param_type == InstructionParameterType.JSON:
                raise NotImplementedError(f"json argument resolution not implemented yet: {line}")
            else:
                raise ValueError(f"unknown parameter type: {param_type}")

    def parse_task_file_segments(self, lines: list[str]) -> list[TaskFileSegment]:
        # parse all bench segments from lines (look like this # @bench ... # @/bench)
        segments: list[TaskFileSegment] = []
        segment: Optional[TaskFileSegment] = None
        for i, line in enumerate(lines):
            if "@bench" in line:
                if segment is not None:
                    # close previous segment
                    segments.append(segment)
                # start new segment
                segment = TaskFileSegment(header=line[8:].strip(), lines=[], source_index=i)
            elif segment is not None:
                segment.lines.append(line)
        if segment is None:
            raise ValueError("task file must contain at least one @bench segment")
        # close last segment
        segments.append(segment)
        return segments

from dataclasses import dataclass
from typing import Optional

from asgiref.sync import async_to_sync
from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.compiler import get_backend_model
from bench.executor import Executor
from bench.models import Dataset, Instruction, Organization, Project, Task
from bench.models.instruction import InstructionParameterType
from bench.models.project import ProjectFileType
from bench.models.task import ExpectationType


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
        # project as organization/project
        parser.add_argument("project", type=str)
        # task file path (must exist and end in .py)
        parser.add_argument("task", type=str)
        # the task to make the new main program
        parser.add_argument("--main", type=str, required=False)

    @transaction.atomic
    def handle(self, project: str, main: str = None, *args, **options):
        organization, project = project.split("/")
        organization = Organization.objects.get(slug=organization)
        project = Project.objects.filter(slug=project, organization=organization).first()
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
        executor = Executor()

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
            definitions = async_to_sync(executor.run_get_definitions)(segment.full_code, {})

            def _get_definition(name: str):
                if name not in definitions:
                    raise ValueError(
                        f"definition '{name}' not found in segment '{segment.header}':\n{segment.full_code}"
                    )
                return definitions[name]

            args, file_name = segment.header.split(":", 1)
            args = args.split(" ")
            file_name = file_name.strip()
            if args[0] == "task":
                task_definition = _get_definition(file_name)
                tasks[file_name] = Task.objects.create(
                    name=file_name,
                    schema=(task_definition["schema"]),
                )
            elif args[0] == "instruct":
                # parse header "instruction [args]: <name>"
                instruction_definition = _get_definition(file_name)
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
                # Parameters are automatically bound unless # @parameter is appended.
                for line in segment.lines:
                    # assume all parameters are declared up front
                    if ":" not in line:
                        break
                    if "#" in line:
                        comment = line[line.find("#") :]
                        line = line[: line.find("#")]
                    else:
                        comment = ""
                    param_name, param_type = line.split(":", 1)
                    param_name = param_name.strip()
                    param_type = param_type.strip()

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
                    instruction.parameters.create(
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
                                "generic model argument resolution not implemented yet"
                            )
                        instruction.arguments.create(
                            name=param_name,
                            type=InstructionParameterType.MODEL,
                            model=model,
                        )
                    elif param_type == InstructionParameterType.INSTRUCTION:
                        instruction.arguments.create(
                            name=param_name,
                            type=InstructionParameterType.INSTRUCTION,
                            instruction=instructions[param_name],
                        )
                    elif param_type == InstructionParameterType.JSON:
                        raise NotImplementedError("json argument resolution not implemented yet")
                    else:
                        raise ValueError(f"unknown parameter type: {param_type}")

                if instruction.type == "expect":
                    # sloppily determine expectation type based on function name
                    if "invar" in instruction.name:
                        expect_type = ExpectationType.INVARIANCE
                    elif "var" in instruction.name:
                        expect_type = ExpectationType.VARIANCE
                    elif "verif" in instruction.name:
                        expect_type = ExpectationType.VERIFICATION
                    else:
                        raise ValueError(f"unable to guess expectation type: {instruction.name}")

                    tasks[args[2]].expectations.create(type=expect_type, instruction=instruction)
            elif args[0] == "dataset":
                dataset_records = _get_definition(file_name)
                # schema is just keys and types of values of the first element
                schema = {k: type(v).__name__ for k, v in dataset_records[0].items()}
                dataset = Dataset.objects.create(name=file_name, schema=schema, type=args[1])
                dataset.extend(dataset_records)
                datasets[file_name] = dataset

                if dataset.type == "example" and len(args) > 2:
                    tasks[args[2]].examples.create(dataset=dataset)
                elif dataset.type == "explain" and len(args) > 2:
                    tasks[args[2]].explanations.create(dataset=dataset)
                elif len(args) > 2:
                    raise ValueError(f"unknown dataset type: {dataset.type}")
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

        # advance head to new version
        project.head = new_version
        project.save()

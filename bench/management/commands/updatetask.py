from dataclasses import dataclass
from functools import cached_property
from typing import Optional

from asgiref.sync import async_to_sync
from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.compiler import get_backend_model
from bench.executor import Executor
from bench.executor.builtins import instruction_builtins
from bench.models import (
    Dataset,
    Expectation,
    Instruction,
    Organization,
    Project,
    ProjectVersion,
    SymbolContent,
    SymbolType,
    Task,
)
from bench.models.instruction import InstructionParameterType


@dataclass
class FileSegment:
    virtual_path: str
    header: str  # in the format <symbol_type> [key=value]*: <name>
    lines: list[str]
    source_index: int

    @cached_property
    def symbol_name(self) -> str:
        return self.header.split(":")[1].strip()

    @cached_property
    def symbol_type(self) -> SymbolType:
        symbol_type = self.header.split(":")[0].split(" ")[0]
        try:
            return SymbolType(symbol_type)
        except ValueError:
            raise ValueError(f"unknown symbol type {symbol_type}")

    @cached_property
    def symbol_args(self) -> dict[str, str]:
        args = self.header.split(":")[0].split(" ")[1:]
        return {arg.split("=")[0]: arg.split("=")[1] for arg in args}

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

        project_v = project.create_version()
        project_v.reset()
        executor = Executor()

        # convert segments to a single task definition tree
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

            segment_parsers: dict[str, callable] = {
                SymbolType.TASK: self.parse_task,
                SymbolType.EXPECTATION: self.parse_expect,
                SymbolType.INSTRUCTION: self.parse_instruct,
                SymbolType.DATASET: self.parse_data,
            }
            segment_parser = segment_parsers.get(segment.symbol_type)
            if segment_parser is None:
                raise ValueError(f"unexpected segment type: {segment.symbol_type}")
            symbol_content: SymbolContent = segment_parser(project_v, segment, _get_definition)
            file = project_v.create_file_from_path(segment.virtual_path, exists_ok=True)
            symbol_def = project_v.define_symbol(segment.symbol_name, symbol_content, file)
            self.stdout.write(f"Define {symbol_def} in {symbol_def.file}")

        # set task as new main program
        if main:
            project_v.program = tasks[main]
            project_v.save()
            self.stdout.write(f"Set {project_v.program} as main program in {project_v}")

        # advance head to new version
        project.head = project_v
        project.save()
        self.stdout.write(f"Updated head in {project}")

    def parse_task_file_segments(self, lines: list[str]) -> list[FileSegment]:
        # parse all bench segments from lines (look like this # @bench ... # @/bench)
        segments: list[FileSegment] = []
        segment: Optional[FileSegment] = None
        virtual_path = "main"  # default to main
        for i, line in enumerate(lines):
            if "@path" in line or "@symbol" in line:
                if segment is not None:
                    # close previous segment
                    segments.append(segment)
                    print(f"segment {segment.header} with {len(segment.lines)} lines")
                    segment = None
            if "@path" in line:
                # parse path like # @path <path>
                virtual_path = line[line.find("@path") + 5 :].strip()
                print(f"path = {virtual_path}")
            if "@symbol" in line:
                # start new segment
                segment = FileSegment(
                    virtual_path=virtual_path, header=line[10:].strip(), lines=[], source_index=i
                )
            elif segment is not None:
                segment.lines.append(line)
        if segment is None:
            raise ValueError("task file must contain at least one @bench segment")
        # close last segment
        segments.append(segment)
        print(f"segment {segment.header} with {len(segment.lines)} lines")
        return segments

    def parse_task(
        self, project_v: ProjectVersion, segment: FileSegment, lookup_def: callable
    ) -> SymbolContent:
        schema = lookup_def("schema")
        task = Task(schema=schema)

        if "parent" in segment.symbol_args:
            parent_name = segment.symbol_args["parent"]
            parent = project_v.get_symbol(parent_name, SymbolType.TASK)
            if parent is None:
                raise ValueError(f"parent task '{parent_name}' not found in {project_v}")
            task.parent = parent

        return task

    def parse_instruct(
        self, project_v: ProjectVersion, segment: FileSegment, lookup_def: callable
    ) -> SymbolContent:
        function = lookup_def(segment.symbol_name)
        if not callable(function):
            raise ValueError(f"instruct symbol '{segment.symbol_name}' is not callable")
        instruction = Instruction(code=segment.full_code)

        if "task" in segment.symbol_args:
            task_name = segment.symbol_args["task"]
            task_symbol = project_v.get_symbol(task_name, SymbolType.TASK)
            if task_symbol is None:
                raise ValueError(f"task '{task_name}' not found in {project_v}")
            instruction.task = task_symbol
            task: Task = project_v.resolve(task_symbol)
            task.template_implementation = instruction

        return instruction

    def parse_data(self, project_v: ProjectVersion, segment: FileSegment, lookup_def: callable):
        dataset_records = lookup_def(segment.symbol_name)
        if not isinstance(dataset_records, list):
            raise ValueError(f"data symbol '{segment.symbol_name}' is not a list")
        dataset = Dataset.objects.from_list(dataset_records)
        return dataset

    def parse_expect(self, project_v: ProjectVersion, segment: FileSegment, lookup_def: callable):
        description = lookup_def("expectation")
        if not isinstance(description, str):
            raise ValueError(f"expectation description '{segment.symbol_name}' is not a str")
        statements_names = lookup_def("statements")
        if not isinstance(statements_names, list):
            raise ValueError(f"expectation statements '{segment.symbol_name}' is not a list")

        expectation = Expectation(description=description)
        for statement_path in statements_names:
            # statement path is <name>.<type>
            statement_name, statement_type_name = statement_path.split(".")
            statement_type = SymbolType(statement_type_name)

            statement_def = project_v.get_symbol_definition(statement_name, statement_type)
            if statement_def is None:
                raise ValueError(
                    f"expectation statement symbol '{statement_path}' not found in {project_v}"
                )
            if statement_def.type in (
                SymbolType.INSTRUCTION,
                SymbolType.DATASET,
                SymbolType.DATASET_VIEW,
            ):
                expectation.statements.add(statement_def.symbol)
            else:
                raise ValueError(
                    f"expectation statement symbol '{statement_path}' is not a valid statement type: {statement_def}"
                )
        return expectation

    def bind_instruction_parameters(
        self,
        segment: FileSegment,
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

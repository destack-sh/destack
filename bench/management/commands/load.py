import typing
from dataclasses import dataclass
from functools import cached_property
from typing import Optional

import structlog
from asgiref.sync import async_to_sync
from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

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
    SymbolDefinition,
    SymbolType,
    Task,
)
from bench.models.instruction import InstructionParameterType

logger = structlog.get_logger(__name__)


@dataclass
class LibraryImport:
    library: str


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
        # symbol file path (must exist and end in .py)
        parser.add_argument("path", type=str)
        # the task to make the new main program
        parser.add_argument("--main", type=str, required=False)

    @transaction.atomic
    def handle(
        self, organization_project: str, path: str, main: Optional[str] = None, *args, **options
    ):
        organization_slug, project_slug = organization_project.split("/")
        organization = Organization.objects.get(slug=organization_slug)
        project = Project.objects.filter(slug=project_slug, organization=organization).first()
        if project is None:
            project = Project.objects.create_project(
                organization=organization, name=project_slug, slug=project_slug
            )

        project_v = project.create_version()
        project_v.reset()

        load_symbols(project_v, path)

        # set task as new main program
        # (not sure if we'll have a single "main" going forward)
        if main:
            project_v.main_program = project_v.symbol_definition(main, SymbolType.TASK)
            project_v.save()
            logger.info(f"Set {project_v.main_program} as main program in {project_v}")

        # advance head to new version
        project.head = project_v
        project.save()
        logger.info(f"Updated head in {project}")


def load_symbols(project_v: ProjectVersion, path: str):
    """Loads symbol definitions from a file into a project version"""

    # read task file lines
    with open(path, "r") as f:
        lines = f.readlines()
    imports, segments = parse_task_file_segments(lines)
    for library_import in imports:
        library_org, library_slug = library_import.library.split("/")
        library = Project.objects.get_by_slug(library_org, library_slug)
        if library is None:
            raise ValueError(f"library {library_import.library} not found")
        library_v = library.head  # just use head
        project_v.libraries.add(library_v)
        logger.info(f"Import library {library_v}")
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

        segment_parsers: dict[str, typing.Callable] = {
            SymbolType.TASK: parse_task,
            SymbolType.EXPECTATION: parse_expect,
            SymbolType.INSTRUCTION: parse_instruct,
            SymbolType.DATASET: parse_data,
        }
        segment_parser = segment_parsers.get(segment.symbol_type)
        if segment_parser is None:
            raise ValueError(f"unexpected segment type: {segment.symbol_type}")

        # create symbol, content and corresponding definition
        # create symbol first so segment parser can use 'self' during definition
        result = segment_parser(project_v=project_v, segment=segment, lookup_def=_get_definition)
        if isinstance(result, tuple):
            symbol_content, on_defined = result
        else:
            symbol_content = result
            on_defined = None

        file = project_v.create_file_from_path(segment.virtual_path, exists_ok=True)
        symbol_def = project_v.define_symbol(segment.symbol_name, content=symbol_content, file=file)

        if on_defined:
            on_defined(symbol_def)

        logger.info(f"Define {symbol_def} in {symbol_def.file}")


def parse_task_file_segments(lines: list[str]) -> tuple[list[LibraryImport], list[FileSegment]]:
    segments: list[FileSegment] = []
    imports: list[LibraryImport] = []

    # segments are controlled via @ switches in comments
    switches = {"@path", "@symbol", "@ignore", "@import"}

    segment: Optional[FileSegment] = None
    virtual_path = "main"  # default to main
    for i, line in enumerate(lines):
        # close previous segment if we're starting a new one
        if segment is not None and any(switch in line for switch in switches):
            segments.append(segment)
            print(f"segment {segment.header} with {len(segment.lines)} lines")
            segment = None
        if "@import" in line:
            # library import
            library = line.split("@import")[1].strip()
            imports.append(LibraryImport(library=library))
            print(f"import {library}")
        elif "@path" in line:
            # change path
            # parse path like # @path <path>
            virtual_path = line[line.find("@path") + 5 :].strip()
            print(f"path = {virtual_path}")
        elif "@symbol" in line:
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

    return imports, segments


def parse_task(
    project_v: ProjectVersion,
    segment: FileSegment,
    lookup_def: typing.Callable,
) -> tuple[SymbolContent, typing.Callable[[SymbolDefinition], None]]:
    schema = lookup_def("schema")
    task = Task.objects.create(schema=schema)

    def on_defined(symbol_def: SymbolDefinition):
        if "parent" in segment.symbol_args:
            parent_name = segment.symbol_args["parent"]
            parent = project_v.symbol_definition(parent_name, SymbolType.TASK)
            parent.children.add(symbol_def)

    return task, on_defined


def parse_instruct(
    project_v: ProjectVersion,
    segment: FileSegment,
    lookup_def: typing.Callable,
):
    function = lookup_def(segment.symbol_name)
    if not callable(function):
        raise ValueError(f"instruct symbol '{segment.symbol_name}' is not typing.Callable")
    instruction = Instruction.objects.create(
        code=segment.full_code, code_function_name=function.__name__
    )

    if "task" in segment.symbol_args:
        task_name = segment.symbol_args["task"]
        task = project_v.symbol_definition(task_name, SymbolType.TASK).task_
        instruction.task = task
        task.template_implementation = instruction

    bind_instruction_parameters(segment, project_v, instruction)

    return instruction


def parse_data(
    project_v: ProjectVersion,
    segment: FileSegment,
    lookup_def: typing.Callable,
):
    dataset_records = lookup_def(segment.symbol_name)
    if not isinstance(dataset_records, list):
        raise ValueError(f"data symbol '{segment.symbol_name}' is not a list")
    dataset = Dataset.objects.from_list(dataset_records)
    return dataset


def parse_expect(
    project_v: ProjectVersion,
    segment: FileSegment,
    lookup_def: typing.Callable,
):
    description = lookup_def("expectation")
    if not isinstance(description, str):
        raise ValueError(f"expectation description '{segment.symbol_name}' is not a str")
    statements_names = lookup_def("statements")
    if not isinstance(statements_names, list):
        raise ValueError(f"expectation statements '{segment.symbol_name}' is not a list")

    expectation = Expectation.objects.create(description=description)

    for statement_path in statements_names:
        # statement path is <name>.<type>
        statement_name, statement_type_name = statement_path.split(".")
        statement_type = SymbolType(statement_type_name)

        statement_def = project_v.symbol_definition(statement_name, statement_type)
        if statement_def.type in (
            SymbolType.INSTRUCTION,
            SymbolType.DATASET,
            SymbolType.DATASET_VIEW,
        ):
            expectation.statements.add(statement_def)
        else:
            raise ValueError(
                f"expectation statement symbol '{statement_path}' is not a valid statement type: {statement_def}"
            )
    if "task" in segment.symbol_args:
        task_name = segment.symbol_args["task"]
        task = project_v.symbol_definition(task_name, SymbolType.TASK).task_
        task.expectations.add(expectation)

    return expectation


def bind_instruction_parameters(
    segment: FileSegment,
    project_v: ProjectVersion,
    instruction: Instruction,
):
    # parameters are defined as type only definition lines like:
    # name: Task|Instruction|Model|Dataset|DatasetView
    # name: <type>
    # Parameters are bound to their name or an @alias unless @param is appended (in comment).
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
            continue  # ignore builtins

        param_schema = None
        if param_type == "Dataset":
            param_type = InstructionParameterType.DATASET
        elif param_type == "Model":
            param_type = InstructionParameterType.MODEL
        elif param_type == "Instruction":
            param_type = InstructionParameterType.INSTRUCTION
        else:
            # just use python type as schema for now
            param_schema = param_type
            param_type = InstructionParameterType.JSON
        instruction.add_parameter(name=param_name, type=param_type, schema=param_schema)

        if "@param" in comment:
            # free parameter, don't try to bind argument value
            continue

        # use alias if set
        if "@alias" in comment:
            symbol_ref_name = comment[comment.find("@alias") + 6 :].strip()
        else:
            symbol_ref_name = param_name

        if param_type == InstructionParameterType.JSON:
            raise NotImplementedError(f"json argument resolution not supported: {line}")
        symbol_def = project_v.symbol_definition(symbol_ref_name)
        instruction.bind_argument(param_name, symbol_def)

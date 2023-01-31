from __future__ import annotations

import enum
from collections import OrderedDict
from contextlib import contextmanager
from dataclasses import dataclass, field
from itertools import chain
from typing import Optional
from uuid import uuid4

import structlog

from bench.language.reconstruct import render_type_node
from bench.language.type import (
    Code,
    CodeContent,
    Compilation,
    Dataset,
    DatasetContent,
    File,
    InterpSymbol,
    LiteralValue,
    Model,
    Module,
    SourceMapping,
    Statement,
    StatementType,
    SymbolType,
    Task,
    TypeNode,
    TypeTag,
)

logger = structlog.get_logger(__name__)


#
# Compilation
#
# On a high level, compilation is a meta-program that takes a compilation and produces optimal
# executable code (and any other symbols) for a task given the compilation constraints.
#
# More formally: compile is a function of task to executable code given a compilation.
# Implementing compile equals some/many forms of optimization problem, we'll see.
#


class CompileErrorType(enum.Enum):
    INTERNAL = 0, "Internal error"
    RUN = 1, "Error running user code"

    def __new__(cls, value, description):
        obj = object.__new__(cls)
        obj._value_ = value
        obj.description = description
        return obj


class CompileError(ValueError):
    def __init__(
        self,
        _t: CompileErrorType,
        symbol: Optional[InterpSymbol],
        cause: Optional[Exception] = None,
    ):
        self.type = _t
        self.symbol = symbol
        self.cause = cause
        super().__init__(self.type.description)


@dataclass
class CompilationState:
    compilation: Compilation
    task: Task
    model: Model
    target_symbols: list[InterpSymbol] = field(default_factory=list)
    # TODO @Incomplete: track source mappings during compilation
    source_mappings: list[SourceMapping] = field(default_factory=list)


class DataBuilder:
    """Build a dataset."""

    def __init__(self, name: str, type_node: TypeNode):
        self.name = name
        self.type_node = type_node
        self.records: list[LiteralValue] = []

    def append(self, record: LiteralValue):
        self.records.append(record)

    def extend(self, records: list[LiteralValue]):
        self.records.extend(records)


class PromptBuilder:
    """Build a BPL dynamic prompt."""

    def __init__(self, name: str | None = None, type_node: TypeNode | None = None):
        self.name = name
        self.type_node = type_node
        self.bpl_lines = []
        self.current_indent = 0

    def prepend(self, line: str):
        indent = " " * 4 * self.current_indent
        self.bpl_lines.insert(0, indent + line)

    def append(self, line: str):
        indent = " " * 4 * self.current_indent
        self.bpl_lines.append(indent + line)

    def extend(self, lines: list[str]):
        for line in lines:
            self.append(line)

    def emit(self, line: str):
        # escape quotes and newlines
        line = line.replace("\n", "\\n")
        line = line.replace('"', '\\"')
        self.append(f'"{line}"')

    def emit_many(self, lines: list[str]):
        for line in lines:
            self.emit(line)

    @contextmanager
    def block(self, line: str | None = None, indent: int = 1):
        if line:
            self.append(line)
        self.current_indent += indent
        yield
        self.current_indent -= indent

    def blank(self):
        self.append("")

    def comment(self, line: str):
        self.bpl_lines.append(f"# {line}")

    def pragma(self, **kwargs):
        self.prepend(f"pragma({', '.join(f'{k}={v}' for k, v in kwargs.items())})")

    def to_code_content(self) -> str:
        return "\n".join(self.bpl_lines)


async def compile(compilation: Compilation) -> tuple[list[InterpSymbol], list[SourceMapping]]:
    logger.debug("compile.start", compilation=compilation)
    if len(compilation.tasks) != 1 or len(compilation.models) != 1:
        raise CompileError(CompileErrorType.INTERNAL, compilation)

    state = CompilationState(
        compilation=compilation, task=(compilation.tasks[0]), model=(compilation.models[0])
    )
    await _compile_task(state, state.task)
    logger.debug("compile.end", compilation=compilation, state=state)
    return state.target_symbols, state.source_mappings


async def _compile_task(state: CompilationState, task: Task) -> None:
    task_t = task.type_node
    target_code = PromptBuilder(name=task.name, type_node=task_t)
    target_code.comment("Task metadata")
    target_code.emit(f'task "{task.name}"\n')
    target_code.emit(task.description + "\n")
    target_code.blank()

    # task type explanation
    target_code.comment("Task type instruction")
    target_code.emit(f'type of task"{task.name}\n')
    input_type_str = render_type_node(task_t.input)
    output_type_str = render_type_node(task_t.output)
    target_code.emit(f"{input_type_str}\n")
    target_code.emit(f"{output_type_str}\n")
    target_code.blank()
    for node in chain(task_t.input.children, task_t.output.children):
        # explain task subtypes (only struct and enum for now, not recursive yet)
        if node.type == TypeTag.STRUCT:
            target_code.emit(f"type of {node.source_reference}:\n")
            target_code.emit_many(render_type_node(node, ignore_reference=True).splitlines())
            target_code.blank()
        elif node.type == TypeTag.ENUM:
            target_code.emit(f"enum {node.source_reference} as (key: info):\n")
            for member in node.members:
                target_code.emit(f'{member.value}: "{member.description or member.name}"')
            target_code.emit("value must be one of the above keys")
            target_code.blank()

    # task examples
    examples_type = TypeNode(
        name=task.name + "_unravelled",
        type=TypeTag.STRUCT,
        children=[*task_t.input.children, task_t.output],
    )
    examples_data = DataBuilder(name=task.name + " examples", type_node=examples_type)

    # task example instruction
    # TODO @Incomplete: generate examples for task
    target_code.comment("Task example instruction")
    target_code.emit(f'examples for task "{task.name}":')
    target_code.emit(task.description + "\n")
    with target_code.block(f"for example in context['{examples_data.name}']:"):
        for node in examples_type.children:
            # weird _aliasing because BPL
            # 1) doesn't get variable scoping and
            # 2) can't understand [..] inside f-strings
            target_code.append(f"_{node.name} = example['{node.name}']")
            target_code.emit(f'"{node.name}": {{_{node.name}}}\n')
    target_code.blank()

    # task inference
    target_code.comment("Task inference")
    # task input fields
    for node in task_t.input.children:
        target_code.emit(f'"{node.name}": "{{{node.name}}}"\n')
    # task final output fields (to be generated by model)
    # (currently only works if the output is a single valued field (no array or struct))
    field_type_str = render_type_node(task_t.output, ignore_name=True, ignore_description=True)
    target_code.emit(f'"{task_t.output.name}": [{task_t.output.name}: {field_type_str}]')

    # target pragma
    # TODO @Incomplete: generate task target pragma properly
    target_code.pragma(temperature=0.5, max_tokens=1024, model=f'context["{state.model.name}"]')

    # TODO @Cleanup: create target symbols nicely with mappings (in state?)
    state.target_symbols.append(
        Dataset(
            id=uuid4(),
            name=examples_data.name,
            type_node=examples_data.type_node,
            records=examples_data.records,
            abstract=False,
            source=None,
            symbol_type=SymbolType.DATASET,
            modifier=None,
            context=OrderedDict(),
            description=None,
            language="jsonl",
        )
    )
    state.target_symbols.append(
        Code(
            id=uuid4(),
            name=target_code.name,
            type_node=target_code.type_node,
            language="bpl",
            code=target_code.to_code_content(),
            abstract=False,
            builtin_id=None,
            symbol_type=SymbolType.CODE,
            modifier=None,
            context=OrderedDict(),
            description=None,
            source=None,
        )
    )


def down(symbols: list[InterpSymbol], file: File | None = None) -> File:
    """Map high-level interpreted symbols back to lower level statements."""

    if file is None:
        file = File(module=Module(name="<generated>"), path="<generated>")

    # render symbols themselves
    for symbol in symbols:
        if isinstance(symbol, Dataset):
            content = down_dataset_content(symbol)
        elif isinstance(symbol, Code):
            content = down_code_content(symbol)
        else:
            raise RuntimeError(f"unexpected symbol {symbol}")
        statement = Statement(
            id=symbol.id,
            type=StatementType.DEFINITION,
            symbol_type=symbol.symbol_type,
            modifier=symbol.modifier,
            name=symbol.name,
            content=content,
            file=file,
            parent=None,
            index=len(file.statements),
        )
        file.statements.append(statement)

    # render relations
    pass  # (not needed yet)

    return file


def down_dataset_content(dataset: Dataset) -> DatasetContent:
    return DatasetContent(
        description=dataset.description,
        language=dataset.language,
        type_node=dataset.type_node,
        records=dataset.records,
    )


def down_code_content(code: Code) -> CodeContent:
    return CodeContent(
        description=code.description,
        language=code.language,
        type_node=code.type_node,
        code=code.code,
        builtin_id=code.builtin_id,
    )

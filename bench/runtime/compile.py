from __future__ import annotations

import enum
from collections import OrderedDict
from contextlib import contextmanager
from dataclasses import dataclass, field
from typing import Optional
from uuid import uuid4

import structlog

from bench.language.type import (
    Compilation,
    Dataset,
    InterpSymbol,
    LiteralValue,
    Model,
    SourceMapping,
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

    def __init__(self, name: str, type_node: TypeNode):
        self.name = name
        self.type_node = type_node
        self.bpl_lines = []
        self.current_indent = 0

    def append(self, line: str):
        indent = " " * 4 * self.current_indent
        self.bpl_lines.append(indent + line)

    def extend(self, lines: list[str]):
        for line in lines:
            self.append(line)

    def emit(self, line: str):
        self.bpl_lines.append(f'"{line}"')

    @contextmanager
    def block(self, line: str | None = None, indent: int = 1):
        if line:
            self.append(line)
        self.current_indent += indent
        yield
        self.current_indent -= indent

    def comment(self, line: str):
        self.bpl_lines.append(f"# {line}")


async def compile(compilation: Compilation) -> tuple[list[InterpSymbol], list[SourceMapping]]:
    if len(compilation.tasks) != 1 or len(compilation.models) != 1:
        raise CompileError(CompileErrorType.INTERNAL, compilation)

    state = CompilationState(
        compilation=compilation, task=(compilation.tasks[0]), model=(compilation.models[0])
    )
    await _compile_task(state, state.task)
    return state.target_symbols, state.source_mappings


async def _compile_task(state: CompilationState, task: Task) -> None:
    examples_type = TypeNode(
        name=task.name + "_unravelled",
        type=TypeTag.STRUCT,
        children=[*task.type_node.input.children, task.type_node.output],
    )
    examples_data = DataBuilder(name=task.name + "_examples", type_node=examples_type)

    target_code = PromptBuilder(name=task.name, type_node=task.type_node)
    target_code.emit(task.description)

    with target_code.block(f"for example in {examples_data.name}:"):
        for field in examples_type.children:
            target_code.emit(f"{field.name} = example['{field.name}']")
            target_code.emit(f'"{field.name}": {field.name}')

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

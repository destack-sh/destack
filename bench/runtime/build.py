from __future__ import annotations

import ast
import asyncio
import enum
import uuid
from dataclasses import dataclass, field
from typing import Optional, Union

import structlog

from bench.language import ModuleIndex
from bench.language.type import (
    Build,
    Code,
    CodeContent,
    Dataset,
    DatasetContent,
    Expectation,
    File,
    InterpSymbol,
    LiteralValue,
    Model,
    Module,
    Record,
    Statement,
    StatementType,
    Task,
    Type,
    TypeNode,
)
from bench.language.typer import check_type
from bench.runtime.reactivity import RawMapping, TrackedNodeType, TrackedTree, track_interp_symbol
from bench.utils.fractional import generate_n_keys_between

logger = structlog.get_logger(__name__)

#
# Build
#
# On a high level, build is a meta-program that takes a build and produces optimal
# executable symbols executable code (and any other symbols) given constraints (e.g. expectations).
#
# More formally: build is a function of task to executable code given a build.
# Implementing build entails interesting optimization problems, we'll see...
#


class BuildErrorType(enum.Enum):
    INTERNAL = 0, "Internal error"
    RUN = 1, "Error running user code"

    def __new__(cls, value, description):
        obj = object.__new__(cls)
        obj._value_ = value
        obj.description = description
        return obj


class BuildError(ValueError):
    def __init__(
        self,
        _t: BuildErrorType,
        symbol: Optional[InterpSymbol],
        cause: Optional[Exception] = None,
    ):
        self.type = _t
        self.symbol = symbol
        self.cause = cause
        super().__init__(self.type.description)


@dataclass(repr=False)
class BuildState:
    build: Build
    candidates: list[BuildCandidate] = field(default_factory=list)


@dataclass(repr=False)
class BuildCandidate:
    state: BuildState
    build: Build
    root_task: Task
    models: list[Model]
    candidate_id: uuid = field(default_factory=uuid.uuid4)
    target_symbols: list[InterpSymbol] = field(default_factory=list)
    dependencies: TrackedTree = field(default_factory=TrackedTree)
    source_mappings: list[RawMapping] = field(default_factory=list)
    # weak references are references to symbols outside the build that are not "strong" references
    # for e.g. string references in code that don't have a foreign key
    # later/soon we'll want this strongly linked inside the symbol content probably
    # :WeakReferences
    weak_references: list[InterpSymbol] = field(default_factory=list)

    def __post_init__(self):
        self.track_dependency(self.build)

    def use_weak_ref(self, symbol: InterpSymbol):
        self.track_dependency(symbol)
        existing_symbol = next((s for s in self.weak_references if s.name == symbol.name), None)
        if existing_symbol is not None and existing_symbol.name == symbol.name:
            # This fragile since it means we can't use the same name for different symbols
            #  without aliasing/scoping them, which would require hacking any "weak" output (like BPL code).
            #  I hope we'll fix :WeakReferences before this becomes a problem.
            if existing_symbol.id != symbol.id:
                raise ValueError(f"weakly referenced symbol already exists: {symbol.name}")
            else:
                return
        self.weak_references.append(symbol)

    def create_data(self, builder: DataBuilder) -> Dataset:
        order_keys = generate_n_keys_between(None, None, len(builder.records))
        records = [Record(order_key=ok, data=d) for ok, d in zip(order_keys, builder.records)]
        dataset = Dataset(
            name=builder.name,
            type_node=builder.type_node,
            type=builder.type_node.to_type(),
            records=records,
            description=None,
            language="jsonl",
        )
        # type check records
        for record in dataset.records:
            check_type(record, dataset.type)
        self.target_symbols.append(dataset)
        return dataset

    def create_code(self, builder: XBuilder) -> Code:
        code = Code(
            name=builder.name,
            type_node=builder.type_node,
            type=builder.type_node.to_type(),
            language="x",
            code=builder.to_code_content(),
            description=None,
        )
        # try to parse code just to make sure it's syntactically valid
        ast.parse(code.code)
        self.target_symbols.append(code)
        return code

    def track_dependency(self, source: InterpSymbol):
        """
        Tracks a source symbol and all its context/references (recursively).
        We could do this as part of the build, but it feels simpler to do it in one place
         to ensure we really have tracked all dependencies.
        """
        if source.source is None:
            raise ValueError(f"source symbol must have a source: {source}")

        track_interp_symbol(self.dependencies, source)

    def map_source(self, source: InterpSymbol, target: InterpSymbol):
        """Map the source symbol to the generated target symbol."""
        self.track_dependency(source)
        self.source_mappings.append(
            RawMapping(type=TrackedNodeType.STATEMENT, source_id=source.id, target_id=target.id)
        )

    def merge(self, other: BuildCandidate) -> None:
        self.target_symbols.extend(other.target_symbols)
        self.dependencies.merge(other.dependencies)
        self.source_mappings.extend(other.source_mappings)
        self.weak_references.extend(other.weak_references)

    def to_result(self) -> BuildResult:
        # Convert dependencies into source mappings without a target
        combined_mappings = [*self.source_mappings]
        for dependency in self.dependencies:
            combined_mappings.append(
                RawMapping(type=dependency.type, source_id=dependency.id, target_id=None)
            )
        return BuildResult(
            build=self.build,
            target_symbols=self.target_symbols,
            source_mappings=combined_mappings,
            weak_references=self.weak_references,
        )


@dataclass
class BuildResult:
    build: Build
    target_symbols: list[InterpSymbol]
    source_mappings: list[RawMapping]
    weak_references: list[InterpSymbol]

    @staticmethod
    def empty(build: Build) -> BuildResult:
        return BuildResult(build=build, target_symbols=[], source_mappings=[], weak_references=[])

    def to_file(self, module: Module | None = None) -> File:
        if module:
            module = Module(name="<build>")
        file = File(path=self.build.id.hex[:8], generated=True, module=module)
        return generate(self.target_symbols, self.weak_references, file)


class BuildContext:
    pass


class DataBuilder:
    """Build a dataset."""

    def __init__(self, name: str, type_node: TypeNode):
        self.name = name
        self.type_node = type_node
        self.records: list[LiteralValue] = []

    def append(self, record: LiteralValue):
        check_type(record, self.type_node)
        self.records.append(record)

    def extend(self, records: list[LiteralValue], ignore_type_errors: bool):
        # ignore_type_errors is a stopgap since records should already be checked here
        for record in records:
            try:
                self.append(record)
            except TypeError as e:
                if not ignore_type_errors:
                    raise e


class XBuilder:
    """Build a structured X prompt."""

    def __init__(self, name: str, type: TypeNode):
        self.name = name
        self.type = type


async def make_build(build: Build) -> BuildResult:
    logger.debug("build.start", build=build)
    if len(build.models) != 1:
        raise BuildError(BuildErrorType.INTERNAL, build)

    state = BuildState(build=build)
    # parallelize by task (don't have metrics yet so can't parallelize by model)
    candidates = [
        BuildCandidate(state=state, build=build, root_task=task, models=build.models)
        for task in build.tasks
    ]
    if not candidates:
        return BuildResult.empty(build)

    builds = [_build_task(candidate, candidate.root_task) for candidate in candidates]
    await asyncio.gather(*builds)
    logger.debug("build.end", build=build, candidates=candidates)
    # merge candidates (should really merge results)
    first_candidate = candidates[0]
    for candidate in candidates[1:]:
        first_candidate.merge(candidate)
    return first_candidate.to_result()


Expect = Union[Task, Code, Dataset, Expectation]


def _gather_expectations(symbol: Type | Expectation | Task) -> list[Expect]:
    expects = []
    if isinstance(symbol, Expectation):
        expects.append(symbol)
    if isinstance(symbol, (Type, Task, Expectation)):
        for child in symbol.expectations:
            if not isinstance(child, Expectation):
                expects.append(child)
            expects.extend(_gather_expectations(child))
    return expects


async def _build_task(state: BuildCandidate, task: Task) -> None:
    target_code_type = task.type_node.deepcopy(keep_id=False, keep_reference=True)
    target_code = XBuilder(name=task.name, type=target_code_type)

    # ensure weak references are available for parsing
    for node in task.type.output.walk():
        if isinstance(node.reference, Type):
            state.use_weak_ref(node.reference)
    for model in state.models:
        state.use_weak_ref(model)

    code = state.create_code(target_code)
    state.map_source(task.definition, code)


def generate(
    symbols: list[InterpSymbol], weak_references: list[InterpSymbol], file: File | None = None
) -> File:
    """Map high-level interpreted symbols back to lower level statements."""

    if file is None:
        file = File(module=Module(name="<generated>"), path="<generated>")

    order_keys = generate_n_keys_between(None, None, len(symbols) + len(weak_references))

    # render weak references :WeakReferences
    for order_key, symbol in zip(order_keys, weak_references):
        if symbol.source is None:
            raise RuntimeError(f"weak reference {symbol} has no source")
        statement = Statement(
            type=StatementType.IMPORT,
            symbol_type=symbol.symbol_type,
            modifier=symbol.modifier,
            name=symbol.name,
            # point directly to underling definition, won't work with :Variables
            reference=symbol.source.underlying_definition,
            content=None,
            file=file,
            parent=None,
            order_key=order_key,
            generated=True,
        )
        file.statements.append(statement)

    # render symbols themselves
    for order_key, symbol in zip(order_keys, symbols):
        if isinstance(symbol, Dataset):
            content = generate_dataset_content(symbol)
        elif isinstance(symbol, Code):
            content = generate_code_content(symbol)
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
            order_key=order_key,
            generated=True,
        )
        file.statements.append(statement)

    return file


def generate_dataset_content(dataset: Dataset) -> DatasetContent:
    return DatasetContent(
        description=dataset.description,
        language=dataset.language,
        type_node=dataset.type_node,
        records=dataset.records,
    )


def generate_code_content(code: Code) -> CodeContent:
    return CodeContent(
        description=code.description,
        language=code.language,
        type_node=code.type_node,
        code=code.code,
    )


def get_builds_for(symbol: Task, module_idx: ModuleIndex) -> list[Build]:
    """Get all builds for a given task."""
    builds = []
    for build in module_idx.symbols_of_type(Build):
        if not build.is_definition:
            continue
        if any(t.definition.id == symbol.id for t in build.tasks):
            builds.append(build)
    return builds

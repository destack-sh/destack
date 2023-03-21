from __future__ import annotations

import asyncio
import enum
import uuid
from dataclasses import dataclass, field
from itertools import chain
from typing import Any, Optional, Union
from uuid import UUID

import structlog

from bench.language import ModuleIndex
from bench.language.type import (
    Build,
    Code,
    Dataset,
    Expectation,
    File,
    InterpSymbol,
    LiteralValue,
    Model,
    Module,
    Record,
    Task,
    Type,
    TypeNode,
    XBlock,
)
from bench.language.typer import check_type
from bench.runtime.evaluate import Evaluation, aggregate_evaluations, evaluate_task
from bench.runtime.generate import generate
from bench.runtime.reactivity import RawMapping, TrackedNodeType, TrackedTree, track_interp_symbol
from bench.utils.fractional import generate_n_keys_between

Expect = Union[Task, Code, Dataset, Expectation]

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
class BuildContext:
    build: Build
    max_candidates: int = 10  # TODO @Build: pick candidates limit settings more carefully
    candidates: list[BuildCandidate] = field(default_factory=list)
    best_candidate: Optional[BuildCandidate] = field(default=None)

    @property
    def exhausted(self) -> bool:
        """Whether we can make any more candidates"""
        return len(self.candidates) >= self.max_candidates


@dataclass(repr=False)
class BuildState:
    build: Build
    target_symbols: list[InterpSymbol] = field(default_factory=list)
    dependencies: TrackedTree = field(default_factory=TrackedTree)
    source_mappings: list[RawMapping] = field(default_factory=list)
    # weak references are references to symbols outside the build that are not "strong" references
    # for e.g. string references in code that don't have a foreign key
    # later/soon we'll want this strongly linked inside the symbol content probably
    # :WeakReferences
    weak_references: list[InterpSymbol] = field(default_factory=list)

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


@dataclass(repr=False)
class InstructionSource:
    target_symbol: InterpSymbol

    async def __call__(self):
        raise NotImplementedError


@dataclass(repr=False)
class InstructionRender:
    def __call__(self) -> list[XBlock]:
        raise NotImplementedError


@dataclass(repr=False)
class InstructionPlan:
    task: Task
    model: Model
    base_settings: Optional[dict[str, Any]]
    sources: list[InstructionSource] = field(default_factory=dict)
    renders: list[InstructionRender] = field(default_factory=list)


@dataclass(repr=False)
class BuildPlan:
    models: list[Model]
    finetunes: list[Any] = field(default_factory=list)  # not used yet
    task_plans: list[InstructionPlan] = field(default_factory=list)


@dataclass(repr=False)
class BuildCandidate:
    ctx: BuildContext
    root_tasks: list[Task]
    plan: BuildPlan
    state: BuildState = field(default_factory=BuildState)
    evaluation: Optional[Evaluation] = field(default=None)
    id: UUID = field(default_factory=uuid.uuid4)

    def __post_init__(self):
        self.state.track_dependency(self.ctx.build)

    @property
    def build(self) -> Build:
        return self.ctx.build

    @property
    def models(self) -> list[Model]:
        return self.plan.models


@dataclass
class BuildResult:
    build: Build
    target_symbols: list[InterpSymbol]
    source_mappings: list[RawMapping]
    weak_references: list[InterpSymbol]
    evaluation: Evaluation | None = None

    @staticmethod
    def empty(build: Build) -> BuildResult:
        return BuildResult(build=build, target_symbols=[], source_mappings=[], weak_references=[])

    def to_file(self, module: Module | None = None) -> File:
        if module:
            module = Module(name="<build>")
        file = File(path=self.build.id.hex[:8], generated=True, module=module)
        return generate(self.target_symbols, self.weak_references, file)


async def build(build: Build) -> BuildResult:
    log = logger.bind(build=build)
    log.info("build.start")
    if len(build.models) != 1:
        raise BuildError(BuildErrorType.INTERNAL, build)

    ctx = BuildContext(build=build)
    if not build.tasks:
        log.info("build.abort", reason="no tasks")
        return BuildResult.empty(build)

    plans = await generate_plans(ctx)
    while not ctx.exhausted and len(plans) > 0:
        log.info("build.step", best_candidate=ctx.best_candidate)
        candidates = [BuildCandidate(ctx=ctx, root_tasks=build.tasks, plan=plan) for plan in plans]
        ctx.candidates.extend(candidates)
        build_tasks = [do_build(candidate) for candidate in candidates]
        await asyncio.gather(*build_tasks)
        # evaluate and rank candidates
        build_results = [candidate.state.to_result() for candidate in candidates]
        for candidate, build_result in zip(candidates, build_results):
            tasks = [t for t in build_result.target_symbols if isinstance(t, Task)]
            evaluations = await asyncio.gather(*[evaluate_task(t) for t in tasks])
            candidate.evaluation = aggregate_evaluations(evaluations)

        # update the best candidate (if changed)
        raise NotImplementedError

    log.info("build.complete", best_candidate=ctx.best_candidate)
    return ctx.best_candidate.state.to_result()


async def generate_plans(ctx: BuildContext) -> list[BuildPlan]:
    # TODO @Broken: don't assume all models are equally capable
    for model in ctx.build.models:
        raise NotImplementedError


async def do_build(candidate: BuildCandidate) -> None:
    """Populates candidate state according to the build plan."""

    # gather instruction sources in parallel
    all_sources = list(chain(*[plan.sources for plan in candidate.plan.task_plans]))
    await asyncio.gather(*[source() for source in all_sources])

    for task_plan in candidate.plan.task_plans:
        xbuilder = XBuilder(task_plan.task.name, task_plan.task.type)


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

    def append(self, xblock: XBlock):
        pass


@dataclass(repr=False)
class InstructionDatasetSampleSource(InstructionSource):
    dataset: Dataset
    count: int
    seed: int


@dataclass(repr=False)
class InstructionCodeSampleSource(InstructionSource):
    code: Code
    count: int
    seed: int


@dataclass(repr=False)
class InstructionGenerateSource(InstructionSource):
    type: Type
    count: int
    seed: int


@dataclass(repr=False)
class InstructionRenderTask(InstructionRender):
    task: Task


@dataclass(repr=False)
class InstructionRenderFewshot(InstructionRender):
    source: InstructionSource


@dataclass(repr=False)
class InstructionRenderType(InstructionRender):
    type: Type
    include_sample: bool = False


@dataclass(repr=False)
class InstructionRenderOutput(InstructionRender):
    output_type: Type


def get_builds_for(symbol: Task, module_idx: ModuleIndex) -> list[Build]:
    """Get all builds for a given task."""
    builds = []
    for build in module_idx.symbols_of_type(Build):
        if not build.is_definition:
            continue
        if any(t.definition.id == symbol.id for t in build.tasks):
            builds.append(build)
    return builds

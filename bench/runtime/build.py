from __future__ import annotations

import asyncio
import enum
import json
import uuid
from dataclasses import dataclass, field, replace
from itertools import chain
from typing import Any, Optional
from uuid import UUID

import structlog

from bench.language import ModuleIndex
from bench.language.type import (
    Build,
    Code,
    Dataset,
    File,
    InterpSymbol,
    Model,
    Module,
    Task,
    Type,
    TypeNode,
    TypeTag,
    XBlock,
    XSource,
)
from bench.language.typer import fabricate_value
from bench.runtime.evaluate import (
    EvaluationMetric,
    EvaluationResult,
    aggregate_metrics,
    compare_evaluations,
    evaluate_task,
)
from bench.runtime.instruct import SampleSource
from bench.runtime.map import map_to_file
from bench.runtime.model import TextGenerationSettings
from bench.runtime.reactivity import RawMapping, TrackedNodeType, TrackedTree, track_interp_symbol
from bench.runtime.run import instantiate
from bench.runtime.type import BuildCandidateStatus, EvaluationKind, EvaluationScope, Modality
from bench.runtime.x import DynamicXBlock, XBuilder, xinput, xoutput, xsettings, xstatic
from bench.utils.random import get_random_veggie_name

logger = structlog.get_logger(__name__)


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
    max_candidates: int = 2  # TODO @Build: pick candidates limit settings more carefully
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

    def add_target(self, symbol: InterpSymbol, source: InterpSymbol | None = None):
        self.target_symbols.append(symbol)
        if source:
            self.map_source(source, symbol)

    def to_result(self) -> BuildResult:
        # Convert dependencies into source mappings without a target
        combined_mappings = [*self.source_mappings]
        for dependency in self.dependencies:
            combined_mappings.append(
                RawMapping(type=dependency.type, source_id=dependency.id, target_id=None)
            )
        # the mappings here are raw mappings (without revision info), if that errors come back
        # and figure out a way to get revmaps here for the updated build
        updated_build = replace(self.build, source_mappings=combined_mappings)

        # TODO @Cleanup: insert weak references into symbol context more orderly :WeakReferences
        # add model weak references as context to all the targets
        # this is usually done in interp, but we want it available immediately.. obviously hacky
        for target in self.target_symbols:
            for ref in self.weak_references:
                # task is also a weak ref, would create loop here because
                # instantiate is not smart enough to handle loops yet
                if isinstance(ref, Model):
                    target.context[ref.name] = ref

        return BuildResult(
            build=updated_build,
            target_symbols=self.target_symbols,
            source_mappings=combined_mappings,
            weak_references=self.weak_references,
        )


@dataclass(repr=False)
class XEmit:
    """Generate X blocks for models with dynamic code to manage dynamic values."""

    async def __call__(self) -> XBlock | DynamicXBlock | list[XBlock | DynamicXBlock]:
        raise NotImplementedError

    @property
    def sources(self) -> list[InterpSymbol]:
        return []


def xemit(func):
    # just forward to dataclass(repr=False, slots=True)
    return dataclass(repr=False, slots=True)(func)


@dataclass(repr=False)
class TaskPlan:
    task: Task
    model: Model
    modality: Modality
    base_settings: Optional[dict[str, Any]] = None
    sources: list[SampleSource] = field(default_factory=dict)
    emits: list[XEmit] = field(default_factory=list)

    def __str__(self):
        return f"task={self.task}, model={self.model}, modality={self.modality}, sources={len(self.sources)}, targets={len(self.emits)}"

    def __repr__(self):
        return f"<TaskPlan {self}>"

    def source(self, source: SampleSource):
        self.sources.append(source)

    def emit(self, *target: XEmit | list[XEmit]):
        self.emits.extend(target)


@dataclass(repr=False)
class BuildPlan:
    models: list[Model]
    finetunes: list[Any] = field(default_factory=list)  # not used yet
    task_plans: list[TaskPlan] = field(default_factory=list)

    def __str__(self):
        return f"models={self.models}, finetunes={self.finetunes}, task_plans={self.task_plans}"

    def __repr__(self):
        return f"<BuildPlan {self}>"


@dataclass(repr=False)
class BuildCandidate:
    ctx: BuildContext
    status: BuildCandidateStatus
    root_tasks: list[Task]
    plan: BuildPlan
    state: BuildState
    name: str = field(default_factory=get_random_veggie_name)
    evaluation: Optional[EvaluationResult] = field(default=None)
    id: UUID = field(default_factory=uuid.uuid4)

    def __post_init__(self):
        self.state.track_dependency(self.ctx.build)

    def __str__(self):
        return f"{self.ctx.build} {self.name} ({self.id})"

    def __repr__(self):
        return f"<BuildCandidate {self}>"

    @property
    def build(self) -> Build:
        return self.ctx.build

    @property
    def models(self) -> list[Model]:
        return self.plan.models


@dataclass(repr=False)
class BuildResult:
    build: Build
    target_symbols: list[InterpSymbol]
    source_mappings: list[RawMapping]
    weak_references: list[InterpSymbol]
    evaluation: EvaluationResult | None = None

    def __str__(self):
        return f"{self.build} -> symbols={len(self.target_symbols)} ({self.evaluation or '<not yet evaluated>'})"

    def __repr__(self):
        return f"<BuildResult {self}>"

    @staticmethod
    def empty(build: Build) -> BuildResult:
        return BuildResult(build=build, target_symbols=[], source_mappings=[], weak_references=[])

    def get_target(self, symbol: InterpSymbol) -> Optional[InterpSymbol]:
        target_id = self.build.get_target(symbol.definition.id)
        # doesn't seem worth making a dict for this yet
        return next((s for s in self.target_symbols if s.id == target_id), None)

    def to_file(self, module: Module | None = None) -> File:
        if module:
            module = Module(name="<build>")
        file = File(path=f"build/{self.build.name}", generated=True, module=module)
        return map_to_file(self.target_symbols, self.weak_references, file)


class BuildTracker:
    def step(self, ctx: BuildContext):
        pass

    def candidates_planned(self, candidates: list[BuildCandidate]):
        pass

    def candidates_built(self, candidates: list[BuildCandidate]):
        pass

    def candidates_evaluated(self, candidates: list[BuildCandidate]):
        pass

    def completed(self, candidate: BuildCandidate, result: BuildResult):
        pass


async def build(build: Build, tracker: BuildTracker = None) -> BuildResult:
    tracker = tracker or BuildTracker()
    log = logger.bind(build=build)
    log.info("build.start")
    if len(build.models) != 1:
        raise BuildError(BuildErrorType.INTERNAL, build)

    ctx = BuildContext(build=build)
    if not build.tasks:
        log.info("build.abort", reason="no tasks")
        return BuildResult.empty(build)

    # TODO @Feature: make build/instruction metric weights configurable
    metric_weights = {
        EvaluationMetric.Performance: 1,
        EvaluationMetric.TypeValidity: 1,
        EvaluationMetric.InstructionSatisfaction: 1,
    }
    plans = await generate_plans(ctx)
    while not ctx.exhausted and len(plans) > 0:
        log.debug("build.step", best_candidate=ctx.best_candidate)
        tracker.step(ctx)
        # build all candidates
        candidates = [
            BuildCandidate(
                status=BuildCandidateStatus.Building,
                ctx=ctx,
                root_tasks=build.tasks,
                plan=plan,
                state=BuildState(build=build),
            )
            for plan in plans
        ]
        tracker.candidates_planned(candidates)
        ctx.candidates.extend(candidates)
        ctx.best_candidate = candidates[0]  # doesn't matter
        build_tasks = [do_build_candidate(candidate) for candidate in candidates]
        await asyncio.gather(*build_tasks)
        tracker.candidates_built(candidates)

        # evaluate, rank and update best
        build_results = [candidate.state.to_result() for candidate in candidates]
        evaluations = await asyncio.gather(
            *[evaluate_candidate(c, r) for c, r in zip(candidates, build_results)]
        )
        for evaluation, candidate in zip(evaluations, candidates):
            candidate.evaluation = evaluation
            improvement = compare_evaluations(
                candidate.evaluation, ctx.best_candidate.evaluation, metric_weights
            )
            if improvement > 0.0:
                log.debug(
                    "build.new_best_candidate",
                    candidate=candidate,
                    evaluation=candidate.evaluation,
                    improvement=improvement,
                )
                ctx.best_candidate = candidate
        tracker.candidates_evaluated(candidates)

    log.info("build.complete", best_candidate=ctx.best_candidate)
    best_result = ctx.best_candidate.state.to_result()
    tracker.completed(ctx.best_candidate, best_result)
    return best_result


async def evaluate_candidate(candidate: BuildCandidate, result: BuildResult) -> EvaluationResult:
    """Evaluates the generated task implementations of a build candidate."""
    task_instances = [
        instantiate(task, build=result.build, buildmap=result.get_target)
        for task in candidate.root_tasks
    ]
    eval_model = candidate.models[0]  # not sure which model to use here?
    evaluation_tasks = (
        evaluate_task(
            task=task,
            eval_model=eval_model,
            build=result.build,
            build_candidate=candidate,
            n_samples=5,
        )
        for task in task_instances
    )
    tasks_evaluations = await asyncio.gather(*evaluation_tasks)
    return EvaluationResult(
        kind=EvaluationKind.EVALUATION,
        scope=EvaluationScope.BUILD,
        self_metrics=None,
        build=result.build,
        aggregated_metrics=aggregate_metrics(tasks_evaluations),
    )


async def generate_plans(ctx: BuildContext) -> list[BuildPlan]:
    plans = []
    # we treat all explicitly given tasks as root tasks, this seems obvious, but unclear if right
    root_tasks = ctx.build.tasks
    # TODO @Broken: don't assume all models are equally capable
    for model in ctx.build.models:
        instruction_plans = []
        for task in root_tasks:
            # TODO @Incomplete: set modality based on task type & model capabilities :TextGenerationOnly
            settings = TextGenerationSettings(temperature=0.5, max_tokens=512, top_p=1.0)
            plan = TaskPlan(task=task, model=model, modality=Modality.GenerateText)
            plan.emit(
                XEmitSystem(),
                XEmitTask(task=task),
                XEmitInput(input_type=task.type.input),
                XEmitSettings(base_settings=settings.__dict__),
                XEmitTypeExplanation(
                    type=task.type.output,
                    type_label="Output",
                    include_descriptions=True,
                    recursive=True,
                ),
                XEmitTypeSample(type=task.type.output, type_label="Output"),
                XEmitOutput(output_type=task.type.output),
            )
            instruction_plans.append(plan)
        plans.append(BuildPlan(models=[model], task_plans=instruction_plans))
    return plans


async def do_build_candidate(candidate: BuildCandidate) -> None:
    """Builds candidate state according to the build plan."""

    # gather instruction sources in parallel
    all_sources = list(chain(*[plan.sources for plan in candidate.plan.task_plans]))
    await asyncio.gather(*[source() for source in all_sources])
    for source in all_sources:
        candidate.state.add_target(source.target_symbol)

    # render instructions
    for task_plan in candidate.plan.task_plans:
        implementation = await do_build_task_plan(task_plan)
        candidate.state.add_target(implementation, source=task_plan.task.definition)

    # add weak refs for emits referencing external symbols (temporary until :WeakReferences is addressed)
    for model in candidate.plan.models:
        candidate.state.use_weak_ref(model)


async def do_build_task_plan(task_plan: TaskPlan) -> Code:
    """Builds a task implementation from a task instruction plan."""
    xbuilder = XBuilder(
        name=task_plan.task.name,
        type=task_plan.task.type,
        model=task_plan.model,
        modality=task_plan.modality,
    )
    emissions = await asyncio.gather(*[emit() for emit in task_plan.emits])
    for emit in emissions:
        if isinstance(emit, list):
            xbuilder.extend(emit)
        else:
            xbuilder.append(emit)
    return xbuilder.to_symbol()


@dataclass(repr=False)
class XEmitSystem(XEmit):
    """Emits the system message about general expectations."""

    message: str = (
        "You are a helpful, attentive and precise agent that follows instructions as intended.\n"
        "The data types and schemas must be followed exactly (e.g. output only JSON when asked).\n"
    )

    async def __call__(self) -> XBlock:
        return xstatic(self.message, XSource.System)


@xemit
class XEmitTask(XEmit):
    """Emits the task exactly as written"""

    task: Task

    async def __call__(self) -> XBlock:
        return xstatic(f"Task {self.task.name}: {self.task.description}", XSource.Developer)

    @property
    def sources(self) -> list[InterpSymbol]:
        return [self.task]


@xemit
class XEmitFewshot(XEmit):
    """Emits fewshot examples in a specific format"""

    task: Task
    source: Dataset
    task_label: Optional[str] = None

    async def __call__(self) -> XBlock:
        return xstatic(
            f"Some examples of {self.task_label or self.task.name}:\n"
            "\n".join(json.dumps(record.data) for record in self.source.records)
        )

    @property
    def sources(self) -> list[InterpSymbol]:
        return [self.task, self.source]


@xemit
class XEmitTypeExplanation(XEmit):
    """Emits the type exactly as written"""

    type: Type
    type_label: Optional[str]
    include_descriptions: bool
    recursive: bool

    async def __call__(self) -> XBlock:
        unexplained_types = [(self.type_label or self.type.name, self.type)]

        def _render_description(d: str):
            return " # " + d if d and self.include_descriptions else ""

        def _render_simple_type(t: TypeNode):
            if t.is_union_with_null:
                return _render_simple_type(t.children[0]) + "?"
            return t.source_reference.name if t.source_reference else t.tag.value

        el_strs = []
        while unexplained_types:
            label, type = unexplained_types.pop()
            if type.tag == TypeTag.ENUM:
                el_str = f"\n{label} enum:{_render_description(type.description)}\n"
                for choice in type.members:
                    el_str += f"- {choice.name}{_render_description(choice.description)}\n"
            elif type.tag == TypeTag.STRUCT:
                el_str = f"\n{label} struct:{_render_description(type.description)}\n"
                for f in type.children:
                    el_str += f"- {f.name}: {_render_simple_type(f)}{_render_description(f.description)}\n"
            else:
                el_str = f"{label}: {_render_simple_type(type)} {_render_description(type.description)}\n"
            el_strs.append(el_str)

            if self.recursive:
                for f in type.children:
                    if isinstance(f.reference, TypeNode):
                        unexplained_types.append((f.reference.name, f.reference))

        el_str = f"Type schemas:\n{''.join(el_strs)}"
        return xstatic(el_str, XSource.Developer)

    @property
    def sources(self) -> list[InterpSymbol]:
        return [self.type]


@xemit
class XEmitTypeSample(XEmit):
    """Emits a fabricated sample of the given type"""

    type: Type
    type_label: Optional[str]

    async def __call__(self) -> list[XBlock]:
        fabricated_sample = fabricate_value(self.type)
        sample_declaration = xstatic(
            f"Example {self.type_label or self.type.name}:",
            XSource.System,
        )
        sample = xstatic(json.dumps(fabricated_sample), XSource.Developer)
        return [sample_declaration, sample]

    @property
    def sources(self) -> list[InterpSymbol]:
        return [self.type]


@xemit
class XEmitInput(XEmit):
    """Emits the code to input the given type"""

    input_type: Type
    path: str = ""

    @staticmethod
    def impute_input(input: XBlock, value: Any):
        import json

        input.value = json.dumps(value)

    async def __call__(self) -> list[XBlock | DynamicXBlock]:
        input_declaration = xstatic("Input:", XSource.System)
        input = xinput(None, path=self.path)
        return [input_declaration, DynamicXBlock(input, self.impute_input)]

    @property
    def sources(self) -> list[InterpSymbol]:
        return [self.input_type]


@xemit
class XEmitOutput(XEmit):
    """Emits the code to request and read generated output of the given type"""

    output_type: Type
    output_label: str = "Output"
    path: str = ""

    @staticmethod
    def parse_output(output: XBlock):
        import json

        # escape the output if needed (handles trivial model confusions)
        value = output.value.strip()
        if not value.startswith("{") and not value.startswith("[") and not value.startswith('"'):
            value = f'"{value}"'

        return json.loads(value)

    async def __call__(self) -> list[XBlock | DynamicXBlock]:
        if self.output_type.is_flat:
            output_request = xstatic(
                f"{self.output_label} (just the value, not an object):", XSource.System
            )
        elif self.output_type.tag == TypeTag.ARRAY:
            output_request = xstatic(
                f"{self.output_label} (JSON array only, start with [, nothing else):",
                XSource.System,
            )
        else:
            output_request = xstatic(
                f"{self.output_label} (JSON object only, start with {{, nothing else):",
                XSource.System,
            )

        output = xoutput(None, path=self.path)
        return [output_request, DynamicXBlock(output, self.parse_output)]

    @property
    def sources(self) -> list[InterpSymbol]:
        return [self.output_type]


@xemit
class XEmitSettings(XEmit):
    base_settings: Optional[dict[str, Any]] = None

    async def __call__(self) -> XBlock:
        # this should be flexible to modalities :TextGenerationOnly
        settings = TextGenerationSettings(**(self.base_settings or {}))
        return xsettings(settings)

    @property
    def sources(self) -> list[InterpSymbol]:
        return []


def get_builds_for(symbol: Task, idx: ModuleIndex) -> list[Build]:
    """Get all builds for a given task."""
    builds = []
    for b in idx.symbols_of_type(Build):
        if not b.is_definition:
            continue
        if any(t.definition.id == symbol.id for t in b.tasks):
            builds.append(b)
    return builds


def get_build_files_for(build: Build, idx: ModuleIndex) -> list[File]:
    build_files: dict[UUID, File] = {}
    for symbol_id in build.targets:
        symbol = idx.get_symbol_by_id(symbol_id)
        if symbol is not None and symbol.source is not None:
            build_files[symbol.source.id] = symbol.source.file
    return list(build_files.values())

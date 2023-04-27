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
    Expectation,
    File,
    InterpSymbol,
    Model,
    Module,
    StatementModifier,
    Task,
    Type,
    TypeNode,
    TypeTag,
    XBlock,
    XSource,
    make_struct_type,
)
from bench.runtime.evaluate import (
    EvaluationMetric,
    EvaluationResult,
    aggregate_metrics,
    compare_evaluations,
    evaluate_task,
)
from bench.runtime.instruct import (
    InstructionOp,
    SampleDatasetRandom,
    SampleSource,
    fabricate_value,
    instruction_tree_from_symbol,
)
from bench.runtime.map import map_to_file
from bench.runtime.model import TextGenerationSettings
from bench.runtime.reactivity import RawMapping, TrackedNodeType, TrackedTree, track_interp_symbol
from bench.runtime.run import instantiate
from bench.runtime.type import (
    BuildCandidateStatus,
    EvaluationKind,
    EvaluationPlan,
    EvaluationScope,
    Modality,
)
from bench.runtime.x import DynamicXBlock, XBuilder, xinput, xoutput, xsettings, xstatic
from bench.utils.fractional import generate_key_between, generate_n_keys_between
from bench.utils.random import get_random_veggie_name

logger = structlog.get_logger(__name__)


class BuildErrorType(enum.Enum):
    INTERNAL = 0, "Internal error"
    RUN = 1, "Error running user code"
    CONFIG = 2, "Invalid configuration"

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
    candidates_ranked: list[BuildCandidate] = field(default_factory=list)

    @property
    def best_candidate(self) -> Optional[BuildCandidate]:
        return self.candidates_ranked[0] if self.candidates_ranked else None

    @property
    def worst_candidate(self) -> Optional[BuildCandidate]:
        return self.candidates_ranked[-1] if self.candidates_ranked else None

    @property
    def worst_candidate_ok(self) -> Optional[str]:
        return self.worst_candidate.order_key if self.worst_candidate else None

    def ok_around(
        self, candidate: BuildCandidate, weights: dict[str, float]
    ) -> tuple[Optional[str], Optional[str]]:
        for i, c in enumerate(self.candidates_ranked):
            if compare_evaluations(c.evaluation, candidate.evaluation, weights) <= 0:
                if i > 0:
                    return self.candidates_ranked[i - 1].order_key, c.order_key
                else:
                    return None, c.order_key
        if self.candidates_ranked:
            return self.candidates_ranked[-1].order_key, None
        else:
            return None, None

    def insert_ranked_candidate(self, candidate: BuildCandidate, weights: dict[str, float]):
        # not super efficient, but #candidates is small
        above_ok, below_ok = self.ok_around(candidate, weights)
        candidate.order_key = generate_key_between(above_ok, below_ok)
        self.candidates_ranked.append(candidate)
        self.candidates_ranked.sort(key=lambda c: c.order_key)

    @property
    def exhausted(self) -> bool:
        """Whether we can make any more candidates"""
        return len(self.candidates) >= self.max_candidates


@dataclass(repr=False)
class BuildState:
    build: Build
    target_symbols: list[InterpSymbol] = field(default_factory=list)
    dependencies: TrackedTree = field(default_factory=TrackedTree)
    generated_mappings: list[RawMapping] = field(default_factory=list)
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
        self.generated_mappings.append(
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
        combined_mappings = [*self.generated_mappings]
        for dependency in self.dependencies:
            combined_mappings.append(
                RawMapping(type=dependency.type, source_id=dependency.id, target_id=None)
            )
        # the mappings here are raw mappings (without revision info), if that errors come back
        # and figure out a way to get revmaps here for the updated build
        updated_build = replace(self.build, generated_mappings=combined_mappings)

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
            generated_mappings=combined_mappings,
            weak_references=self.weak_references,
        )


@dataclass(repr=False)
class XEmit:
    """Generate X blocks for models with dynamic code to manage dynamic values."""

    async def __call__(self) -> XBlock | DynamicXBlock | list[XBlock | DynamicXBlock]:
        raise NotImplementedError


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
    id: int
    models: list[Model]
    # finetunes: list[Finetune] (soon)
    task_plans: list[TaskPlan] = field(default_factory=list)

    def __str__(self):
        return f"models={self.models}, task_plans={self.task_plans}"

    def __repr__(self):
        return f"<BuildPlan {self}>"


@dataclass(repr=False)
class BuildCandidate:
    ctx: BuildContext
    status: BuildCandidateStatus
    root_tasks: list[Task]
    instruct_model: Model
    plan: BuildPlan
    state: BuildState
    order_key: str
    id: UUID
    name: str = field(default_factory=get_random_veggie_name)
    evaluation: Optional[EvaluationResult] = None

    def __post_init__(self):
        self.state.track_dependency(self.ctx.build)

    def __str__(self):
        return f"{self.ctx.build} {self.name} {self.status} ({self.id})"

    def __repr__(self):
        return f"<BuildCandidate {self}>"

    @property
    def build(self) -> Build:
        return self.ctx.build

    @property
    def models(self) -> list[Model]:
        return self.plan.models

    @staticmethod
    def make_id(build: Build, plan: BuildPlan) -> UUID:
        return uuid.uuid5(build.id, f"build_candidate:{plan.id}")


@dataclass(repr=False)
class BuildResult:
    build: Build
    target_symbols: list[InterpSymbol]
    generated_mappings: list[RawMapping]
    weak_references: list[InterpSymbol]
    evaluation: EvaluationResult | None = None

    def __str__(self):
        return f"{self.build} -> symbols={len(self.target_symbols)} ({self.evaluation or '<not yet evaluated>'})"

    def __repr__(self):
        return f"<BuildResult {self}>"

    @staticmethod
    def empty(build: Build) -> BuildResult:
        return BuildResult(
            build=build, target_symbols=[], generated_mappings=[], weak_references=[]
        )

    def get_target(self, symbol: InterpSymbol) -> Optional[InterpSymbol]:
        # :SymbolDefinitionReference
        target_id = self.build.get_target(symbol.definition.id)
        # doesn't seem worth making a dict for this yet
        return next((s for s in self.target_symbols if s.id == target_id), None)

    def to_file(self, module: Module | None = None) -> File:
        if module:
            module = Module(name="<build>")
        file = File(path=f"__build__/{self.build.name}", generated=True, module=module)
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


async def build(
    build: Build,
    instruct_model: Model,
    evals: dict[UUID, EvaluationPlan],
    tracker: BuildTracker = None,
) -> BuildResult:
    tracker = tracker or BuildTracker()
    log = logger.bind(build=build)
    log.info("build.start")

    ctx = BuildContext(build=build)
    if not build.tasks:
        log.warning("build.abort", reason="no tasks")
        return BuildResult.empty(build)

    # TODO @Feature: make build/instruction metric weights configurable
    metric_weights = {
        EvaluationMetric.Performance: 1,
        EvaluationMetric.TypeValidity: 1,
        EvaluationMetric.InstructionSatisfaction: 1,
    }
    plans = await generate_plans(ctx)
    if not plans:
        log.warning("build.abort", reason="no plans")
        return BuildResult.empty(build)

    while not ctx.exhausted and len(plans) > 0:
        log.debug("build.step", best_candidate=ctx.best_candidate)
        tracker.step(ctx)

        # create candidates for plans
        order_keys = generate_n_keys_between(ctx.worst_candidate_ok, None, len(plans))
        candidates = [
            BuildCandidate(
                id=BuildCandidate.make_id(build, plan),
                status=BuildCandidateStatus.Building,
                ctx=ctx,
                root_tasks=build.tasks,
                plan=plan,
                state=BuildState(build=build),
                order_key=ok,
                instruct_model=instruct_model,
            )
            for ok, plan in zip(order_keys, plans)
        ]
        tracker.candidates_planned(candidates)
        ctx.candidates.extend(candidates)

        # build all candidates
        build_tasks = [do_build_candidate(candidate) for candidate in candidates]
        await asyncio.gather(*build_tasks)
        for candidate in candidates:
            candidate.status = BuildCandidateStatus.Evaluating
        tracker.candidates_built(candidates)

        # evaluate, rank and update best
        build_results = [candidate.state.to_result() for candidate in candidates]
        evals = [
            evaluate_candidate(candidate=c, result=r, evals=evals)
            for c, r in zip(candidates, build_results)
        ]
        evaluations = await asyncio.gather(*evals)
        for evaluation, candidate in zip(evaluations, candidates):
            candidate.evaluation = evaluation
            ctx.insert_ranked_candidate(candidate, weights=metric_weights)
        ctx.best_candidate.status = BuildCandidateStatus.CompletedWon
        for candidate in ctx.candidates_ranked[1:]:
            candidate.status = BuildCandidateStatus.CompletedAbandoned
        tracker.candidates_evaluated(ctx.candidates_ranked)  # update all candidates

        # make new build plans
        plans = await generate_plans(ctx)

    if ctx.best_candidate is None:
        raise RuntimeError(f"failed to generate build candidates: {build}")
    log.info("build.complete", best_candidate=ctx.best_candidate)
    best_result = ctx.best_candidate.state.to_result()
    tracker.completed(ctx.best_candidate, best_result)
    return best_result


async def evaluate_candidate(
    candidate: BuildCandidate, result: BuildResult, evals: dict[UUID, EvaluationPlan]
) -> EvaluationResult:
    """Evaluates the generated task implementations of a build candidate."""
    task_instances = [
        # :SymbolDefinitionReference
        instantiate(task.definition, build=result.build, buildmap=result.get_target)
        for task in candidate.root_tasks
    ]
    tasks_evaluations = await asyncio.gather(
        *(
            evaluate_task(
                task=task, eval=evals[task.id], build=candidate.build, build_candidate=candidate
            )
            for task in task_instances
        )
    )
    return EvaluationResult(
        kind=EvaluationKind.EVALUATION,
        scope=EvaluationScope.BUILD,
        self_metrics=None,
        build=result.build,
        build_candidate=candidate,
        aggregated_metrics=aggregate_metrics(tasks_evaluations),
        children=tasks_evaluations,
    )


async def generate_plans(ctx: BuildContext) -> list[BuildPlan]:
    plans = []
    if ctx.candidates:
        return []  # TODO @Feature: multi-step build plans (if budget allows)

    # we treat all explicitly given tasks as root tasks, this seems obvious, but unclear if right
    root_tasks = ctx.build.tasks
    # TODO @Incomplete: don't assume all models are equally capable
    # TODO @Incomplete: set modality based on task type & model capabilities :TextGenerationOnly
    for model in ctx.build.models:
        instruction_plans = []
        # TODO @Broken: consider context length in X prompt planning/building
        for task in root_tasks:
            if task.type.output.tag == TypeTag.NULL:
                # cannot build task without output, should be caught before this
                raise BuildError(BuildErrorType.CONFIG, ctx.build)

            instruction, tree = instruction_tree_from_symbol(task)
            expectations: list[Expectation] = [
                i.node for i in instruction.walk() if i.op == InstructionOp.ExpectationDefinition
            ]
            data_samples: list[Dataset] = [
                i.node for i in instruction.walk() if i.op == InstructionOp.SampleData
            ]
            # clarity output label as task completion if we don't have a structured output
            plan = TaskPlan(task=task, model=model, modality=Modality.GenerateText)

            # map output type
            output_label = "Output"
            output_type = task.type.output
            output_path = ""
            if output_type.is_flat:
                # lift flat output into object (easier to get model to generate)
                lifted = task.type.output.deepcopy(keep_id=False)
                if lifted.description is None:
                    lifted.description = f"Output for task {task.name}"
                lifted.name = (
                    output_type.reference.name.lower()
                    if output_type.reference
                    else "output_" + output_type.tag.name.lower()
                )
                output_type = make_struct_type(lifted, name=output_label)
                output_path = lifted.name

            # this is obviously hacky and suboptimal and will be replaced
            plan.emit(
                XEmitSystem(),
                XEmitTypeExplanation(
                    type=output_type,
                    type_label=output_label,
                    include_descriptions=True,
                    recursive=True,
                ),
            )
            if expectations:
                plan.emit(XEmitExpectations(task_label=task.name, expectations=expectations))
            for dataset in data_samples:
                if len(dataset) > 0:
                    plan.emit(
                        XEmitSamples(
                            source=SampleDatasetRandom(dataset, count=3, seed=0),
                            task_label=task.name,
                            positive=dataset.modifier == StatementModifier.LIKE,
                        )
                    )
            plan.emit(
                XEmitTask(task=task),
                XEmitTypeSample(type=output_type, type_label=output_label),
            )
            if task.type.input.children:
                plan.emit(XEmitInput(type=task.type.input, type_label="Input"))
            plan.emit(
                # TODO @Broken: adjust & tune generation settings
                XEmitSettings(TextGenerationSettings(temperature=0.5, max_tokens=512, top_p=1.0)),
                XEmitOutput(
                    type=output_type, type_label=f"Output for task {task.name}", path=output_path
                ),
            )
            instruction_plans.append(plan)
        plans.append(BuildPlan(id=len(plans), models=[model], task_plans=instruction_plans))
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
        # :SymbolDefinitionReference
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
    """Emits the system message about general expectations for JSON."""

    message: str = (
        "You are a precise and helpful assistant."
        " Perform the given tasks following the instructions to produce outputs."
        " Only output valid JSON (literal, array or object) as per the type schemas."
    )

    async def __call__(self) -> XBlock:
        return xstatic(self.message, XSource.System)


@xemit
class XEmitTask(XEmit):
    """Emits the task exactly as written"""

    task: Task
    task_label: str = None
    include_description: bool = True

    async def __call__(self) -> XBlock:
        text = f"Task {self.task_label or self.task.name}:"
        if self.include_description:
            text += f" {self.task.description}"
        return xstatic(text, XSource.Developer)


@xemit
class XEmitExpectations(XEmit):
    """Emits the expectation exactly as written"""

    task_label: str
    expectations: list[Expectation]

    async def __call__(self) -> XBlock:
        expectation_strs = [
            f" - {expectation.name}: {expectation.description}" for expectation in self.expectations
        ]
        return xstatic(
            f"For task {self.task_label}, consider:\n" + "\n".join(expectation_strs),
            XSource.Developer,
        )


@xemit
class XEmitSamples(XEmit):
    """Emits fewshot examples in a specific format"""

    source: SampleSource
    task_label: str
    positive: bool

    async def __call__(self) -> XBlock:
        dataset = await self.source()
        if len(dataset) == 0:
            raise RuntimeError(f"expected at least one sample for {self.task.name}")
        if self.positive:
            preamble = f"Good examples of {self.task_label}"
        else:
            preamble = f"Bad examples of {self.task_label} (don't do this!)"
        data_str = "\n".join(json.dumps(record.data, sort_keys=True) for record in dataset.records)
        return xstatic(f"{preamble}:\n{data_str}", XSource.Developer)


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
                for child in type.children:
                    el_str += f"- {child.name}: {_render_simple_type(child)}{_render_description(child.description)}\n"
            elif type.tag == TypeTag.ARRAY:
                el_str = f"\n{label} array of {_render_simple_type(type.children[0])}{_render_description(type.description)}\n"
            else:
                el_str = f"{label}: {_render_simple_type(type)} {_render_description(type.description)}\n"
            el_strs.append(el_str)

            if self.recursive:
                for child in type.children or []:
                    if isinstance(child.reference, TypeNode):
                        unexplained_types.append((child.reference.name, child.reference))
                    elif child.tag in (TypeTag.ARRAY, TypeTag.STRUCT, TypeTag.ENUM, TypeTag.UNION):
                        unexplained_types.append((child.name, child))

        el_str = f"Schemas:\n{''.join(el_strs)}".strip()
        return xstatic(el_str, XSource.Developer)


@xemit
class XEmitTypeSample(XEmit):
    """Emits a single sample of the given type (default to fabricated)"""

    type: Type
    type_label: Optional[str]
    value: Any = None

    async def __call__(self) -> list[XBlock]:
        fabricated_sample = self.value or fabricate_value(self.type)
        sample_declaration = xstatic(
            f"Example {self.type_label or self.type.name}:",
            XSource.System,
        )
        sample = xstatic(json.dumps(fabricated_sample, sort_keys=True), XSource.Developer)
        return [sample_declaration, sample]


@xemit
class XEmitInput(XEmit):
    """Emits the code to input the given type"""

    type: Type
    type_label: str = "Input"
    path: str = ""

    @staticmethod
    def impute_input(input: XBlock, value: Any):
        import json

        input.value = json.dumps(value, sort_keys=True)

    async def __call__(self) -> list[XBlock | DynamicXBlock]:
        input_declaration = xstatic(f"{self.type_label}:", XSource.System)
        input = xinput(None, path=self.path)
        return [input_declaration, DynamicXBlock(input, self.impute_input)]


@xemit
class XEmitOutput(XEmit):
    """Emits the code to request and read generated output of the given type"""

    type: Type
    type_label: str = "Output"
    path: str = ""

    # TODO @Incomplete: support proper jsonpath for xblocks
    @staticmethod
    def parse_output(output: XBlock):
        import json
        import re

        # escape the output if needed (handles trivial model confusions)
        value = output.value.strip()
        if not value.startswith("{") and not value.startswith("[") and not value.startswith('"'):
            value = value.replace("\n", "\\n")
            value = f'"{value}"'

        # escape strings with multiline content
        # these aren't technically valid JSON, but they're very useful for models
        def sub_multiline_str(match):
            # replace line breaks with \n escape sequence
            modified_string = match.group(1).replace("\n", "\\n").replace("\r", "")
            return f'"{modified_string}"'

        value = re.compile(r'"(.*?)(?<!\\)"', re.DOTALL).sub(sub_multiline_str, value)

        try:
            ret = json.loads(value)
            if output.path:
                ret = ret[output.path]
            return ret
        except Exception as e:
            raise ValueError(f"invalid X output: {e}") from e

    async def __call__(self) -> list[XBlock | DynamicXBlock]:
        if self.type.is_flat:
            output_request = xstatic(
                f'{self.type_label}\n(JSON literal, nothing else, not an object, start with " or number)',
                XSource.System,
            )
        elif self.type.tag == TypeTag.ARRAY:
            output_request = xstatic(
                f"{self.type_label}\n(JSON array, nothing else, include ',', start with [)",
                XSource.System,
            )
        else:
            output_request = xstatic(
                f"{self.type_label}\n(JSON object, nothing else, start with {{)",
                XSource.System,
            )

        output = xoutput(None, path=self.path)
        return [output_request, DynamicXBlock(output, self.parse_output)]

    @property
    def sources(self) -> list[InterpSymbol]:
        return [self.type]


@xemit
class XEmitSettings(XEmit):
    settings: Any

    async def __call__(self) -> XBlock:
        return xsettings(self.settings)

    @property
    def sources(self) -> list[InterpSymbol]:
        return []


def get_builds_for(symbol: Task, idx: ModuleIndex) -> list[Build]:
    """
    Get all applicable builds for a given task.
    """
    builds = []
    for b in idx.symbols_of_type(Build):
        if not b.is_definition:
            continue
        # :SymbolDefinitionReference
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

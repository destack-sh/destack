import asyncio
import enum
import uuid
from collections import defaultdict
from itertools import chain
from typing import Any, Optional, cast

import structlog
from more_itertools import first

from bench.language import ModuleIndex
from bench.language.type import (
    Build,
    Code,
    Dataset,
    InterpSymbol,
    Model,
    Task,
    Type,
    TypeNode,
    TypeTag,
    XKind,
    flatten_func_type,
    make_func_type,
    make_struct_type,
)
from bench.runtime.instruct import (
    Instruction,
    InstructionOp,
    SampleGenerateWithModel,
    anonymous_dataset,
    instruction_tree_from_module,
    instruction_tree_from_symbol,
)
from bench.runtime.run import instantiate, run
from bench.runtime.tracing import in_memory_traces, tracer_blocker
from bench.runtime.type import (
    EvaluationKind,
    EvaluationMetric,
    EvaluationResult,
    EvaluationScope,
    Modality,
    TaskInstance,
    TextGenerationSettings,
)
from bench.utils.func import dict_minus

logger = structlog.get_logger(__name__)


class MetricType(enum.StrEnum):
    PERCENTAGE = "percentage"
    COUNT = "count"


ALL_METRICS = set(EvaluationMetric)
COUNT_METRICS = {metric for metric in ALL_METRICS if metric.endswith("_count")}
PERCENTAGE_METRICS = ALL_METRICS - COUNT_METRICS

HIGHER_IS_BETTER = {
    EvaluationMetric.Performance,
    EvaluationMetric.Clarity,
    # Speed is measured in duration, so lower is better
    EvaluationMetric.TypeValidity,
    EvaluationMetric.InstructionSatisfaction,
    EvaluationMetric.FeedbackCorrelation,
    EvaluationMetric.InstructionAgreement,
    EvaluationMetric.InstructionOverlap,
}
LOWER_IS_BETTER = ALL_METRICS - HIGHER_IS_BETTER


def aggregate_metrics(
    evaluations: list[EvaluationResult], weights: dict[uuid.UUID | str, float] = None
) -> dict[str, float]:
    weights = weights or defaultdict(lambda: 1.0)

    # sum the counts
    summed_counts = {}
    for metric in COUNT_METRICS & summed_counts.keys():
        summed_counts[metric] = sum(
            evaluation.aggregated_metrics[metric] * weights.get(evaluation.id, weights[metric])
            for evaluation in evaluations
        )

    # average the percentages (?)
    averaged_percentages = defaultdict(float)
    for evaluation in evaluations:
        for metric, value in evaluation.aggregated_metrics.items():
            averaged_percentages[metric] += value * weights.get(evaluation.id, weights[metric])
    for metric in PERCENTAGE_METRICS & averaged_percentages.keys():
        averaged_percentages[metric] /= len(evaluations)
    aggregated_metrics = {**summed_counts, **averaged_percentages}

    # recompute summary metrics
    aggregated_metrics.update(get_summary_metrics(aggregated_metrics))

    return aggregated_metrics


def get_summary_metrics(metrics: dict[str, float]) -> dict[EvaluationMetric, float]:
    summary_metrics = {}

    # clarity
    if (
        EvaluationMetric.InstructionPerplexity in metrics
        and EvaluationMetric.InstructionAgreement in metrics
        and EvaluationMetric.InstructionOverlap in metrics
    ):
        perplexity = metrics[EvaluationMetric.InstructionPerplexity]
        agreement = metrics[EvaluationMetric.InstructionAgreement]
        overlap = metrics[EvaluationMetric.InstructionOverlap]
        clarity = (1 - min(perplexity, 1.0)) * agreement * (1 - overlap)
        summary_metrics[EvaluationMetric.Clarity] = min(clarity, 0.99)

    # difficulty
    if (
        EvaluationMetric.InstructionCount in metrics
        and EvaluationMetric.InstructionComplexity in metrics
    ):
        count = metrics[EvaluationMetric.InstructionCount]
        complexity = metrics[EvaluationMetric.InstructionComplexity]
        difficulty = count * complexity
        summary_metrics[EvaluationMetric.Difficulty] = difficulty

    # performance
    if (
        EvaluationMetric.TypeValidity in metrics
        or EvaluationMetric.InstructionSatisfaction in metrics
    ):
        performance = (metrics.get(EvaluationMetric.TypeValidity, 1)) * (
            metrics.get(EvaluationMetric.InstructionSatisfaction, 1)
        )
        # perfection is unattainable (... and 100 is suspicious)
        summary_metrics[EvaluationMetric.Performance] = min(performance, 0.99)

    # speed
    if EvaluationMetric.AverageRunDuration in metrics:
        speed = metrics[EvaluationMetric.AverageRunDuration]
        summary_metrics[EvaluationMetric.Speed] = speed

    return summary_metrics


def compare_evaluations(
    a: EvaluationResult, b: EvaluationResult, weights: dict[str, float]
) -> float:
    """
    Compares two evaluations using the weights.
    """
    a_score = score_evaluation(a, weights)
    b_score = score_evaluation(b, weights)
    return a_score - b_score


def score_evaluation(a: EvaluationResult, weights: dict[str, float]) -> float:
    """
    Returns a score for the evaluation, based on the weights.
    """
    score = 0
    for metric, weight in weights.items():
        value = a.aggregated_metrics[metric]
        if metric in HIGHER_IS_BETTER:
            score += value * weight
        elif metric in LOWER_IS_BETTER:
            score += (1 - value) * weight
    return score


def group_aggregate_by_system(evaluations: list[EvaluationResult]) -> list[EvaluationResult]:
    evaluations_by_node = defaultdict(list)
    for evaluation in evaluations:
        evaluations_by_node[evaluation.system.id].append(evaluation)

    aggregated: list[EvaluationResult] = []
    for node_id, evaluations in evaluations_by_node.items():
        all_children = {
            child.id: child for evaluation in evaluations for child in evaluation.children
        }
        evaluation = EvaluationResult(
            kind=evaluations[0].kind,
            scope=evaluations[0].scope,
            system=evaluations[0].system,
            build=evaluations[0].build,
            aggregated_metrics=aggregate_metrics(evaluations),
            children=list(all_children.values()),
        )
        aggregated.append(evaluation)
    return aggregated


async def evaluate_task(
    task: TaskInstance,
    eval_model: Model,
    build: Build,
    n_samples: int,
    build_candidate: Any = None,
) -> EvaluationResult:
    """Evaluates a task implementation against the instructions."""
    log = logger.bind(task=task, build=build)
    task_instruction, _ = instruction_tree_from_symbol(task)

    # generate samples to test
    samples = await SampleGenerateWithModel(
        task=task, type=flatten_func_type(task.type), model=eval_model, count=n_samples, seed=1337
    )()
    with in_memory_traces() as traces:
        runs = (
            run(task.implementation, dict_minus(sample.data, {"output"}))
            for sample in samples.records
        )
        results = await asyncio.gather(*runs, return_exceptions=True)
    outputs = anonymous_dataset(task.type.output, n_samples)
    n_successful_runs = 0
    for i, result in enumerate(results):
        if isinstance(result, Exception):
            log.warning("evaluate.run.failed", result=result)
            continue
        outputs.records[i].data = result
        n_successful_runs += 1

    # evaluate samples against the instructions
    evalable_instructions = [
        instruction
        for instruction in task_instruction.walk()
        if instruction.op in (InstructionOp.ExpectationDefinition, InstructionOp.TaskDefinition)
        or (
            instruction.op == InstructionOp.TypeDefinition
            and instruction.node.description is not None
        )
    ]
    evals = (
        evaluate_output(
            input_type=task.type,
            input=dict_minus(input.data, "output"),
            output_type=task.type.output,
            output=output.data,
            instructions=evalable_instructions,
            eval_model=eval_model,
            build=build,
        )
        for input, output in zip(samples.records, outputs.records)
    )
    evals = await asyncio.gather(*evals)
    instruction_evaluations = list(chain(*evals))
    # group instruction evaluations by evaluated node
    instruction_evaluations = group_aggregate_by_system(instruction_evaluations)
    instruction_metrics = aggregate_metrics(instruction_evaluations)

    # technically tokens_count is #characters
    tokens_count = sum(
        [len(xblock) for xblock in task.implementation.xblocks if xblock.kind != XKind.Settings]
    )
    average_run_duration = sum([r.duration_with_cache for r in traces.roots]) / len(traces.roots)
    performance_metrics = {
        # for type validity we assume that unsuccessful run == type error
        EvaluationMetric.TypeValidity: n_successful_runs / n_samples,
        EvaluationMetric.AverageRunDuration: average_run_duration,
        EvaluationMetric.InstructionSatisfaction: instruction_metrics[
            EvaluationMetric.InstructionSatisfaction
        ],
        EvaluationMetric.FeedbackCorrelation: 1.0,
    }

    metrics = {
        # only 1 always for now :TaskGrouping
        EvaluationMetric.InferencesCount: 1,
        EvaluationMetric.TokensCount: tokens_count,
        **performance_metrics,
    }
    metrics.update(get_summary_metrics(metrics))
    # filter out the task self evaluation
    self_evaluation = first(e for e in instruction_evaluations if e.system.id == task.id)
    return EvaluationResult(
        kind=EvaluationKind.EVALUATION,
        scope=EvaluationScope.INSTRUCTION,
        system=task,
        build=build,
        build_candidate=build_candidate,
        self_metrics=self_evaluation.aggregated_metrics,
        aggregated_metrics=metrics,
        children=[e for e in instruction_evaluations if e.system.id != task.id],
    )


async def evaluate_output(
    input_type: Type,
    input: Any,
    output_type: Type,
    output: Any,
    instructions: list[Instruction],
    eval_model: Model,
    build: Optional[Build],
) -> list[EvaluationResult]:
    from bench.runtime.build import (
        TaskPlan,
        XEmitInput,
        XEmitOutput,
        XEmitSettings,
        XEmitTask,
        XEmitTypeExplanation,
        XEmitTypeSample,
        do_build_task_plan,
    )

    instruction_type = make_struct_type(
        Type(name="description", tag=TypeTag.STRING),
        Type(name="id", tag=TypeTag.NUMBER),
        name="Instruction",
    )
    eval_type = make_struct_type(
        Type(name="id", tag=TypeTag.NUMBER),
        Type(
            name="reasoning",
            tag=TypeTag.STRING,
            description="Assess if the corresponding instruction was satisfied exactly as specified.",
        ),
        Type(
            name="satisfied",
            tag=TypeTag.BOOLEAN,
            description="Whether the instruction was followed exactly. If unclear, set to false.",
        ),
        name="Evaluation",
    )
    eval_task_type = make_func_type(
        Type(name="instructions", tag=TypeTag.ARRAY, children=[instruction_type]),
        Type(name="sample", tag=TypeTag.STRUCT, children=[*input_type.children, output_type]),
        output_type=Type(name="_output", tag=TypeTag.ARRAY, children=[eval_type]),
    )
    eval_task = Task(
        name="evaluate output",
        description="Assess whether the generated output followed the instructions."
        " Set satisfied if the corresponding instruction was followed (in format, style, content, etc.)."
        " If the instruction doesn't ask for follow-ups, the output must not contain one.",
        type=eval_task_type,
        type_node=eval_task_type,
    )
    plan = TaskPlan(task=eval_task, model=eval_model, modality=Modality.GenerateText)
    sample_evaluation = {
        "id": 0,
        "reasoning": "output contained unexpected response",
        "satisfied": False,
    }
    plan.emit(
        XEmitTask(task=eval_task),
        # TODO @Build: tune model eval generation settings (and adapt to model context size)
        XEmitTypeExplanation(
            type=eval_task_type.output,
            type_label="Evaluation",
            include_descriptions=True,
            recursive=True,
        ),
        XEmitInput(type=eval_task_type.input),
        XEmitTypeSample(
            type=eval_task_type.output, type_label="Evaluations", value=[sample_evaluation]
        ),
        XEmitSettings(TextGenerationSettings(temperature=0.3, max_tokens=2048, top_p=1.0)),
        XEmitOutput(
            type=eval_task_type.output, type_label="Evaluations (one for each instruction)"
        ),
    )
    implementation = await do_build_task_plan(plan)
    implementation.context[eval_model.name] = eval_model
    implementation_instance = instantiate(implementation)

    sample = {**input, "_output": output}
    simplified_instructions = []
    for i, instruction in enumerate(instructions):
        if not isinstance(instruction.node, (InterpSymbol, TypeNode)):
            raise RuntimeError(f"unexpected instruction node type: {instruction}")
        name = instruction.node.name or ""
        description = instruction.node.description or ""
        if not name and not description:
            raise RuntimeError(f"instruction has no name or description: {instruction}")
        description = f"{name}: {description}"
        if isinstance(instruction.node, TypeNode):
            description = description + f" (on {instruction.node.name})"
        simplified_instructions.append({"description": description, "id": i})

    # TODO @Robustness @UX: should we really block tracers for internal inferences?
    #  The original reason for putting this here was a JS/Apollo-side issue with the
    #  'sample' value (no idea why, but it errored). Then I realized we probably
    #  shouldn't expose this anyway, so that patched the issue.
    with tracer_blocker():
        evals = await run(
            implementation_instance,
            {"instructions": simplified_instructions, "sample": sample},
        )

    results = []
    for instruction, eval in zip(instructions, evals):
        metrics = {EvaluationMetric.InstructionSatisfaction: 1.0 if eval["satisfied"] else 0.0}
        result = EvaluationResult(
            kind=EvaluationKind.EVALUATION,
            scope=EvaluationScope.INSTRUCTION,
            system=instruction.node,
            build=build,
            self_metrics=metrics,
            aggregated_metrics=metrics,
        )
        results.append(result)
    return results


async def lint_instruction(instruction: Instruction) -> dict[str, float]:
    """Lints a single instruction."""

    # TODO @Incomplete: compute model clarity metrics
    # TODO @Incomplete: compute model difficulty metrics?

    instruction_agreement = 1.0
    instruction_perplexity = 0.0
    instruction_overlap = 0.0

    # below are some heuristics for measuring 'clarity' (i.e. non-perplexity/confusion)
    # TODO @Incomplete: some clarity heuristics should just/also be warnings/errors

    HALF_CONFUSION = 0.25
    FULL_CONFUSION = 0.5

    if instruction.op in (
        InstructionOp.TaskDefinition,
        InstructionOp.ExpectationDefinition,
        InstructionOp.TypeDefinition,
        InstructionOp.DataDefinition,
    ):
        # natural definitions should have names and ideally descriptions
        no_name = (instruction.node.name or "").strip() == ""
        no_description = (instruction.node.description or "").strip() == ""
        if no_name:
            instruction_perplexity += FULL_CONFUSION
        # TODO @Broken: 'inline' function type nodes (which cannot have descriptions) are docked for clarity
        if no_description:
            instruction_perplexity += HALF_CONFUSION

    if instruction.op == InstructionOp.TypeDefinition:
        # types should not be 'anything'
        type = cast(Type, instruction.node)
        if type.tag == TypeTag.ANY:
            instruction_perplexity += FULL_CONFUSION

    if instruction.op == InstructionOp.TaskDefinition:
        # task definitions (not steps) should have types
        task = cast(Task, instruction.node)
        if task.type.output.tag == TypeTag.NULL:  # output type at least?
            instruction_perplexity += FULL_CONFUSION

    if instruction.op in (InstructionOp.SampleData, InstructionOp.DataDefinition):
        # sample data / data defs should have at least 2 samples
        data = cast(Dataset, instruction.node)
        if len(data.records) < 1:
            instruction_perplexity += HALF_CONFUSION
        elif len(data.records) < 2:
            instruction_perplexity += FULL_CONFUSION
        # sample data / data defs should have types
        if len(data.type.children or []) == 0:
            instruction_perplexity += FULL_CONFUSION

    if instruction.op in (InstructionOp.SampleCode, InstructionOp.CheckCode):
        # sample / check code should have input & output type
        code = cast(Code, instruction.node)
        if len(code.type.input.children or []) == 0:
            instruction_perplexity += FULL_CONFUSION
        if code.type.output.tag == TypeTag.NULL:
            instruction_perplexity += FULL_CONFUSION

    self_metrics = {
        EvaluationMetric.InstructionCount: 1,
        EvaluationMetric.InstructionComplexity: 1,
        EvaluationMetric.InstructionAgreement: instruction_agreement,
        EvaluationMetric.InstructionOverlap: instruction_overlap,
        EvaluationMetric.InstructionPerplexity: instruction_perplexity,
    }
    self_metrics.update(get_summary_metrics(self_metrics))
    return self_metrics


async def lint(idx: ModuleIndex) -> EvaluationResult:
    """Lints an entire module."""
    tree = instruction_tree_from_module(idx, exclude_generated=True)
    evaluations: dict[uuid.UUID, EvaluationResult] = {}

    # evaluate nodes individually
    instructions = list(tree.walk_postorder())
    instruction_self_metrics = await asyncio.gather(
        *[lint_instruction(node) for node in instructions]
    )
    for self_metrics, instruction in zip(instruction_self_metrics, instructions):
        evaluation = EvaluationResult(
            kind=EvaluationKind.LINT,
            scope=EvaluationScope.INSTRUCTION,
            system=instruction.node,
            build=None,
            self_metrics=self_metrics,
            aggregated_metrics={**self_metrics},
        )
        evaluations[instruction.id] = evaluation

        # aggregate evaluations (incl. self)
        # this will break when we get cycles :InstructionCircles
        child_evaluations = [evaluations[child.id] for child in instruction.children]
        evaluation.aggregated_metrics = aggregate_metrics([evaluation, *child_evaluations])
        evaluation.children = [c for c in child_evaluations if c.system.id != instruction.node.id]

    root_evaluations = [evaluations[root.id] for root in tree.roots]
    root_evaluation = EvaluationResult(
        kind=EvaluationKind.LINT,
        scope=EvaluationScope.MODULE,
        system=None,
        build=None,
        aggregated_metrics=aggregate_metrics(root_evaluations),
        children=root_evaluations,
    )
    return root_evaluation

import asyncio
import enum
import uuid
from collections import defaultdict
from typing import Any

import structlog

from bench.language import ModuleIndex
from bench.language.type import (
    Build,
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
from bench.runtime.tracing import in_memory_traces
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

    # TODO @Broken: recompute summary metrics when aggregating metrics  :SummaryMetrics
    #  (and probably also just compute summary metrics in a single place)

    return aggregated_metrics


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

    # technically this is characters count, not tokens count
    # we'll want proper token counts soon to properly optimize for the available context
    tokens_count = sum(
        [len(xblock) for xblock in task.implementation.xblocks if xblock.kind != XKind.Settings]
    )
    count_metrics = {
        # only 1 always for now :TaskGrouping
        EvaluationMetric.InferencesCount: 1,
        # this only works for strings
        EvaluationMetric.TokensCount: tokens_count,
    }

    # generate samples to test
    inputs = await SampleGenerateWithModel(
        task=task, type=flatten_func_type(task.type), model=eval_model, count=n_samples, seed=1337
    )()
    with in_memory_traces() as traces:
        runs = (
            run(task.implementation, dict_minus(sample.data, {"output"}))
            for sample in inputs.records
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
    ]
    evals = (
        evaluate_output(
            input_type=task.type,
            input=input.data,
            output_type=task.type.output,
            output=output.data,
            instructions=evalable_instructions,
            eval_model=eval_model,
        )
        for input, output in zip(inputs.records, outputs.records)
    )
    instruction_evaluations = await asyncio.gather(*evals)
    instruction_metrics = aggregate_metrics(instruction_evaluations)

    average_run_duration = sum([r.duration for r in traces.roots]) / len(traces.roots)
    performance_metrics = {
        # for type validity we assume that unsuccessful run == type error
        EvaluationMetric.TypeValidity: n_successful_runs / n_samples,
        EvaluationMetric.AverageRunDuration: average_run_duration,
        # TODO @Incomplete: compute instruction satisfaction
        EvaluationMetric.InstructionSatisfaction: instruction_metrics[
            EvaluationMetric.InstructionSatisfaction
        ],
        EvaluationMetric.FeedbackCorrelation: 1.0,
    }

    # TODO @Incomplete: compute proper summary metrics :SummaryMetrics
    summary_metrics = {
        EvaluationMetric.Performance: performance_metrics[EvaluationMetric.TypeValidity],
        EvaluationMetric.Speed: 60 / performance_metrics[EvaluationMetric.AverageRunDuration],
    }
    return EvaluationResult(
        kind=EvaluationKind.EVALUATION,
        scope=EvaluationScope.INSTRUCTION,
        system=task,
        build=build,
        build_candidate=build_candidate,
        self_metrics=None,
        aggregated_metrics={**count_metrics, **performance_metrics, **summary_metrics},
    )


async def evaluate_output(
    input_type: Type,
    input: Any,
    output_type: Type,
    output: Any,
    instructions: list[Instruction],
    eval_model: Model,
) -> list[EvaluationResult]:
    from bench.runtime.build import (
        TaskPlan,
        XEmitInput,
        XEmitOutput,
        XEmitSettings,
        XEmitTask,
        XEmitTypeSample,
        do_build_task_plan,
    )

    sample_type = Type(
        name="sample", tag=TypeTag.STRUCT, children=[*input_type.children, output_type]
    )
    instruction_type = make_struct_type(
        TypeNode(name="description", tag=TypeTag.STRING),
        TypeNode(name="id", tag=TypeTag.NUMBER),
        name="instruction",
    )
    eval_type = make_struct_type(
        TypeNode(name="id", tag=TypeTag.NUMBER),
        TypeNode(name="satisfied", tag=TypeTag.BOOLEAN),
    )
    eval_task_type = make_func_type(
        sample_type,
        TypeNode(name="instructions", tag=TypeTag.ARRAY, children=[instruction_type]),
        output_type=TypeNode(name="output", tag=TypeTag.ARRAY, children=[eval_type]),
    )
    eval_task = Task(
        name="evaluate output",
        description="Check whether the generated output followed the instructions correctly.",
        type=eval_task_type,
        type_node=eval_task_type,
    )
    plan = TaskPlan(task=eval_task, model=eval_model, modality=Modality.GenerateText)
    plan.emit(
        XEmitTask(task=eval_task),
        # TODO @Build: tune model eval generation settings (and adapt to model context size)
        XEmitInput(type=eval_task_type.input),
        XEmitTypeSample(type=eval_task_type.output, type_label="Output"),
        XEmitSettings(TextGenerationSettings(temperature=0.3, max_tokens=2048, top_p=1.0)),
        XEmitOutput(type=eval_task_type.output, type_label="Evaluation output"),
    )
    implementation = await do_build_task_plan(plan)
    implementation.context[eval_model.name] = eval_model
    implementation_instance = instantiate(implementation)

    sample = {**input, "output": output}
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

    evals = await run(
        implementation_instance,
        {"sample": sample, "instructions": simplified_instructions},
    )

    results = []
    for instruction, eval in zip(instructions, evals):
        metrics = {EvaluationMetric.InstructionSatisfaction: 1.0 if eval["satisfied"] else 0.0}
        result = EvaluationResult(
            kind=EvaluationKind.EVALUATION,
            scope=EvaluationScope.INSTRUCTION,
            system=instruction.node,
            self_metrics=metrics,
            aggregated_metrics=metrics,
        )
        results.append(result)
    return results


async def lint_instruction(instruction: Instruction) -> dict[str, float]:
    """Lints a single instruction."""
    # TODO @Incomplete: compute proper lint metrics
    self_metrics = {EvaluationMetric.InstructionCount: 1, EvaluationMetric.Clarity: 1.0}
    self_metrics[EvaluationMetric.Difficulty] = self_metrics[EvaluationMetric.InstructionCount]
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
            system=instruction,
            build=None,
            self_metrics=self_metrics,
            aggregated_metrics={**self_metrics},
        )
        evaluations[instruction.id] = evaluation

        # aggregate evaluations (incl. self)
        # this will break when we get cycles :InstructionCircles
        child_evaluations = [evaluations[child.id] for child in instruction.children]
        evaluations[instruction.id].aggregated_metrics = aggregate_metrics(
            [evaluation, *child_evaluations]
        )

    root_evaluations = [evaluations[root.id] for root in tree.roots]
    root_evaluation = EvaluationResult(
        kind=EvaluationKind.LINT,
        scope=EvaluationScope.MODULE,
        system=None,
        build=None,
        aggregated_metrics=aggregate_metrics(root_evaluations),
    )
    return root_evaluation

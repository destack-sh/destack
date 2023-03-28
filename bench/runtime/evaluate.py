import asyncio
import enum
import uuid
from collections import defaultdict

import structlog

from bench.language import ModuleIndex
from bench.language.type import Build, Model, XKind, flatten_func_type
from bench.runtime.build import BuildCandidate
from bench.runtime.instruct import (
    Instruction,
    SampleSourceGenerator,
    anonymous_dataset,
    instruction_tree_from_module,
)
from bench.runtime.run import run
from bench.runtime.type import (
    EvaluationKind,
    EvaluationMetric,
    EvaluationResult,
    EvaluationScope,
    TaskInstance,
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
            evaluation.self_metrics[metric] * weights.get(evaluation.id, weights[metric])
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
    return aggregated_metrics


def compare_evaluations(
    a: EvaluationResult, b: EvaluationResult, weights: dict[str, float]
) -> float:
    """
    If a is better than b, return a positive number. Otherwise, return a negative number.
    If a and b are equal, return 0.

    Only metrics in `weights` are considered.
    """
    diff = 0
    for metric, weight in weights.items():
        a_value = a.aggregated_metrics[metric]
        b_value = b.aggregated_metrics[metric]
        if metric in HIGHER_IS_BETTER:
            diff += (a_value - b_value) * weight
        elif metric in LOWER_IS_BETTER:
            diff += (b_value - a_value) * weight
    return diff


async def evaluate_task(
    task: TaskInstance,
    eval_model: Model,
    build: Build,
    n_samples: int,
    build_candidate: BuildCandidate = None,
) -> EvaluationResult:
    """Evaluates a task implementation against the instructions."""
    log = logger.bind(task=task, build=build)
    # technically this is characters count, not tokens count
    # we'll want proper token counts soon to properly optimize for the available context

    tokens_count = sum(
        [len(xblock) for xblock in task.implementation.xblocks if xblock.kind != XKind.Settings]
    )
    count_metrics = {
        EvaluationMetric.NodesCount: 1,
        # only 1 always for now :TaskGrouping
        EvaluationMetric.StepsCount: 1,
        EvaluationMetric.InferencesCount: 1,
        # this only works for strings
        EvaluationMetric.TokensCount: tokens_count,
    }

    # generates samples to test
    inputs = await SampleSourceGenerator(
        flatten_func_type(task.type), model=eval_model, count=n_samples, seed=1337
    )()
    runs = (
        run(task.implementation, dict_minus(sample.data, {"output"})) for sample in inputs.records
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

    performance_metrics = {
        EvaluationMetric.TypeValidity: n_successful_runs / n_samples,
        # TODO @Incomplete: compute instruction satisfaction
        EvaluationMetric.InstructionSatisfaction: 1.0,
        EvaluationMetric.FeedbackCorrelation: 1.0,
    }

    # TODO @Broken: compute proper summary metrics
    summary_metrics = {
        EvaluationMetric.Performance: 0.9,
        EvaluationMetric.Difficulty: 0.14,
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


async def lint_instruction(node: Instruction) -> dict[str, float]:
    """Lints a single instruction."""
    # TODO @Incomplete: compute proper lint metrics
    self_metrics = {EvaluationMetric.NodesCount: 1}
    return self_metrics


async def lint(idx: ModuleIndex) -> EvaluationResult:
    """Lints an entire module."""
    tree = instruction_tree_from_module(idx)
    evaluations: dict[uuid.UUID, EvaluationResult] = {}

    # evaluate nodes individually
    all_self_metrics = await asyncio.gather(*[lint_instruction(node) for node in tree.walk()])
    for self_metrics, node in zip(all_self_metrics, tree.walk()):
        evaluation = EvaluationResult(
            kind=EvaluationKind.LINT,
            scope=EvaluationScope.INSTRUCTION,
            system=node,
            build=None,
            self_metrics=self_metrics,
            aggregated_metrics={**self_metrics},
        )
        evaluations[node.id] = evaluation

    # aggregate evaluations bottom-up (post-order)
    for node in tree.walk_postorder():
        child_evaluations = [evaluations[child.id] for child in node.children]
        evaluations[node.id].aggregated_metrics = aggregate_metrics(child_evaluations)

    root_evaluations = [evaluations[root.id] for root in tree.roots]
    root_evaluation = EvaluationResult(
        kind=EvaluationKind.LINT,
        scope=EvaluationScope.MODULE,
        system=None,
        build=None,
        aggregated_metrics=aggregate_metrics(root_evaluations),
    )
    return root_evaluation

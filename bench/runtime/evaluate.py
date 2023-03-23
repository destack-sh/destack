import enum
import uuid
from collections import defaultdict
from dataclasses import dataclass, field
from itertools import chain
from typing import Optional

from bench.language.type import Build, InterpSymbol
from bench.runtime.build import InstructionSourceGenerate
from bench.runtime.run import RunError, run
from bench.runtime.type import TaskInstance


class SummaryMetric(enum.StrEnum):
    Performance = "performance"
    Clarity = "clarity"
    Sophistication = "sophistication"


class BaseMetric(enum.StrEnum):
    # Performance related
    TypeCorrectness = "type_correctness"
    ExpectationSatisfaction = "expectation_satisfaction"
    FeedbackCorrelation = "feedback_correlation"
    # Clarity related
    InstructionPerplexity = "instruction_perplexity"
    InstructionAgreement = "instruction_agreement"
    InstructionOverlap = "instruction_overlap"
    # Complexity related
    NodesCount = "nodes_count"
    StepsCount = "steps_count"
    TokensCount = "tokens_count"


class MetricType(enum.StrEnum):
    PERCENTAGE = "percentage"
    COUNT = "count"


ALL_METRICS = set(chain(SummaryMetric, BaseMetric))
COUNT_METRICS = {metric for metric in ALL_METRICS if metric.endswith("_count")}
PERCENTAGE_METRICS = ALL_METRICS - COUNT_METRICS

HIGHER_IS_BETTER = {
    SummaryMetric.Performance,
    SummaryMetric.Clarity,
    BaseMetric.TypeCorrectness,
    BaseMetric.ExpectationSatisfaction,
    BaseMetric.FeedbackCorrelation,
    BaseMetric.InstructionAgreement,
    BaseMetric.InstructionOverlap,
}
LOWER_IS_BETTER = {
    SummaryMetric.Sophistication,
    BaseMetric.InstructionPerplexity,
    BaseMetric.NodesCount,
    BaseMetric.StepsCount,
    BaseMetric.TokensCount,
}


@dataclass(repr=False, slots=True)
class Evaluation:
    symbol: Optional[InterpSymbol]
    build: Build
    metrics: dict[str, float]
    id: uuid.UUID = field(default_factory=uuid.uuid4)
    children: list["Evaluation"] = field(default_factory=list)

    def __str__(self):
        return ", ".join(f"{k}: {v:0.02f}" for k, v in self.metrics.items())

    def __repr__(self):
        return f"<Evaluation {self.symbol} {self}>"

    @property
    def summary_metrics(self):
        return {metric: self.metrics[metric] for metric in SummaryMetric}

    @property
    def base_metrics(self):
        return {metric: self.metrics[metric] for metric in BaseMetric}


def aggregate_evaluations(
    evaluations: list[Evaluation], weights: dict[uuid.UUID | str, float] = None
) -> Evaluation:
    """Combines multiple evaluations into one."""
    weights = weights or defaultdict(lambda: 1.0)
    # sum the counts
    summed_counts = {}
    for metric in COUNT_METRICS & summed_counts.keys():
        summed_counts[metric] = sum(
            evaluation.metrics[metric] * weights.get(evaluation.id, weights[metric])
            for evaluation in evaluations
        )
    # average the percentages
    averaged_percentages = defaultdict(float)
    for evaluation in evaluations:
        for metric, value in evaluation.metrics.items():
            averaged_percentages[metric] += value * weights.get(evaluation.id, weights[metric])
    for metric in PERCENTAGE_METRICS & averaged_percentages.keys():
        averaged_percentages[metric] /= len(evaluations)

    # check that the build is the same
    build_ids = {evaluation.build.id for evaluation in evaluations}
    if len(build_ids) > 1:
        raise ValueError("cannot aggregate evaluations from different builds")

    return Evaluation(
        symbol=None,
        build=evaluations[0].build,
        metrics={**summed_counts, **averaged_percentages},
        children=evaluations,
    )


def compare_evaluations(a: Evaluation, b: Evaluation, weights: dict[str, float]) -> float:
    """
    If a is better than b, return a positive number. Otherwise, return a negative number.
    If a and b are equal, return 0.

    Only metrics in `weights` are considered.
    """
    diff = 0
    for metric, weight in weights.items():
        a_value = a.metrics[metric]
        b_value = b.metrics[metric]
        if metric in HIGHER_IS_BETTER:
            diff += (a_value - b_value) * weight
        elif metric in LOWER_IS_BETTER:
            diff += (b_value - a_value) * weight
    return diff


async def evaluate_task(task: TaskInstance, build: Build, nsamples: int) -> Evaluation:
    """Evaluates a task implementation against the instructions."""
    implementation = task.implementation

    task_metrics = {
        # only 1 always for now :TaskGrouping
        BaseMetric.NodesCount: 1,
        BaseMetric.StepsCount: 1,
        # this only works for strings
        BaseMetric.TokensCount: sum([len(xblock.value) for xblock in implementation.xblocks]),
    }

    samples = await InstructionSourceGenerate(task.type, count=nsamples, seed=1337)()
    n_successful_runs = 0
    for sample in samples.records:
        try:
            output = await run(implementation, sample.data)
            n_successful_runs += 1
        except RunError as e:
            print(e)
            continue

    return Evaluation(
        symbol=task,
        build=build,
    )

import enum
import uuid
from collections import defaultdict
from dataclasses import dataclass, field
from typing import Optional

from bench.language.type import InterpSymbol
from bench.runtime.type import TaskInstance


class SummaryMetric(enum.StrEnum):
    Performance = "performance"
    Clarity = "clarity"
    Complexity = "complexity"


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
    ExecutionDuration = "execution_duration"
    NodesCount = "nodes_count"
    StepsCount = "steps_count"


@dataclass
class Evaluation:
    symbol: Optional[InterpSymbol]
    aggregate_metrics: dict[SummaryMetric, float]
    base_metrics: dict[BaseMetric, float]
    id: uuid.UUID = field(default_factory=uuid.uuid4)


async def evaluate_task(task: TaskInstance) -> Evaluation:
    # TODO @Incomplete: implement real evaluation
    return Evaluation(
        symbol=task,
        # zero everything
        aggregate_metrics={metric: 0.0 for metric in SummaryMetric},
        base_metrics={metric: 0.0 for metric in BaseMetric},
    )


async def aggregate_evaluations(
    evaluations: list[Evaluation], weights: dict[uuid.UUID, float] = None
) -> Evaluation:
    weights = weights or defaultdict(lambda: 1.0)
    raise NotImplementedError

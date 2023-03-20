import enum
from dataclasses import dataclass

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
    aggregate_metrics: dict[SummaryMetric, float]


async def evaluate(task: TaskInstance) -> Evaluation:
    raise NotImplementedError

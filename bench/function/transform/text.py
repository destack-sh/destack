from typing import Optional

from bench.function.base import Predicate, Record, Transform, functions
from bench.models import Dataset


@functions.register("named_entity_imputer")
class NamedEntityImputer(Transform):
    input_spec = {"text": str, "entities": dict}
    output_spec = {"text": str, "entities": dict}

    def __init__(self, dataset: Dataset):
        self.dataset = dataset

    def __call__(self, record: Record) -> Record:
        return record


@functions.register("text.upper_case", impl=Transform)
def upper_case(text: str) -> str:
    return text.upper()


@functions.register("text.swap_characters", impl=Transform)
def swap_characters(record: Record) -> Record:
    return record


@functions.register("text.check_length", impl=Predicate)
def check_text_length(
    text: str, min_length: int = 0, max_length: Optional[int] = None
) -> bool:
    return min_length <= len(text) <= (max_length or float("inf"))

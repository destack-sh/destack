from typing import Optional

from bench.dataset.base import DatasetHandler
from bench.function.base import Predicate, Record, Transform, functions
from bench.utils.spec import DatasetType, Json


@functions.register("named_entity_imputer")
class NamedEntityImputer(Transform):
    config_spec = {"dataset": DatasetType(record_spec={})}
    input_spec = {"text": str, "entities": Json}
    output_spec = {"text": str, "entities": Json}

    def __init__(self, dataset: DatasetHandler):
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

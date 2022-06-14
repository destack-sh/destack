import re

from bench.dataset.base import DatasetReader
from bench.function.base import SingleRecordTransform, functions
from bench.utils.record import Record


@functions.register("bench.text.upper_case")
class UpperCaseTextTransform(SingleRecordTransform):
    def transform(self, record: Record) -> Record:
        return {"text": record["text"].upper()}


@functions.register("bench.text.swap")
class TemplateTextSwapper(SingleRecordTransform):
    def __init__(self, swaps_dataset: DatasetReader):
        self.swaps_dataset = swaps_dataset
        self.regex_patterns = [re.compile(pattern) for pattern in swaps_dataset["pattern"]]
        self.replacements = swaps_dataset["replacement"]

    def transform(self, record: Record) -> Record:
        source_text: str = record["text"]
        transformed_text = source_text
        for pattern, replacement in zip(self.regex_patterns, self.replacements):
            transformed_text = pattern.sub(replacement, source_text)
            if transformed_text != source_text:
                break
        return {**record, "text": transformed_text}

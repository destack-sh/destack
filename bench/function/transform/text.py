import abc
import re
from typing import Tuple, Union, cast

from bench.dataset.base import DatasetReader
from bench.function.base import SingleRecordTransform, functions
from bench.utils.func import dict_to_ordered
from bench.utils.record import Record
from bench.utils.spec import convert_to_record_spec


class TextTransform(SingleRecordTransform, abc.ABC):
    input_spec = {"*": convert_to_record_spec({"text": str})}
    output_spec = dict_to_ordered({"*": convert_to_record_spec({"text": str})})

    def transform(self, record: Record) -> Record:
        record = cast(dict, record)  # assume record is dict
        transformed_text = self.transform_text(cast(str, record["text"]))
        if isinstance(transformed_text, str):
            return {**record, "text": transformed_text}
        else:
            output_text, output_record = transformed_text
            return {**record, **output_record, "text": transformed_text}

    def transform_text(self, text: str) -> Union[str, Tuple[str, Record]]:
        raise NotImplementedError


@functions.register("bench.text.upper_case")
class UpperCaseTextTransform(TextTransform):
    def transform_text(self, text: str) -> str:
        return text.upper()


@functions.register("bench.text.swap")
class TemplateTextSwapper(TextTransform):
    def __init__(self, swaps_dataset: DatasetReader):
        self.swaps_dataset = swaps_dataset
        swaps_patterns = cast(list[str], swaps_dataset["pattern"])
        self.regex_patterns = [re.compile(pattern) for pattern in swaps_patterns]
        self.replacements = cast(list[str], swaps_dataset["replacement"])

    def transform_text(self, text: str) -> str:
        transformed_text = text
        for pattern, replacement in zip(self.regex_patterns, self.replacements):
            transformed_text = pattern.sub(replacement, text)
            if transformed_text != text:
                break
        return transformed_text

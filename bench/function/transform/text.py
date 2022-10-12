import abc
import re
from typing import Any, Tuple, Union, cast

from bench.dataset.base import DatasetHandler, DatasetReader
from bench.function.base import FunctionMetadata, SingleRecordTransform, functions
from bench.utils.func import dict_to_ordered
from bench.utils.record import Record, RecordBatch, RecordList
from bench.utils.spec import DatasetType, convert_to_config_type_spec, convert_to_record_spec


class TextTransform(SingleRecordTransform, abc.ABC):
    input_spec = {"*": convert_to_record_spec({"text": str})}
    output_spec = dict_to_ordered({"*": convert_to_record_spec({"text": str})})

    def __init__(self, spread_original: bool = False):
        self.spread_original = spread_original

    @staticmethod
    def make_output(record: dict, transformed_text: Union[str, Tuple[str, Record]]) -> Record:
        if isinstance(transformed_text, str):
            return {**record, "text": transformed_text}
        elif isinstance(transformed_text, dict):
            return {**record, **transformed_text}
        else:
            output_text, output_record = transformed_text
            return {**record, **output_record, "text": transformed_text}

    def transform(self, record: Record) -> Union[Record, RecordBatch]:
        record = cast(dict, record)  # assume record is dict
        original_text = cast(str, record["text"])
        transformed_text = self.transform_text(original_text)
        if self.spread_original:
            outputs = [
                TextTransform.make_output(record, original_text),
                TextTransform.make_output(record, transformed_text),
            ]
            return RecordList(outputs)
        else:
            return TextTransform.make_output(record, transformed_text)

    def transform_text(self, text: str) -> Union[str, Tuple[str, Record]]:
        raise NotImplementedError


@functions.register("bench.text.case.upper")
class UpperCaseTextTransform(TextTransform):
    metadata = FunctionMetadata("Upper case", "Transforms text to upper case", tags=["text"])

    def transform_text(self, text: str) -> str:
        return text.upper()


@functions.register("bench.text.case.lower")
class LowerCaseTextTransform(TextTransform):
    metadata = FunctionMetadata("Lower case", "Transforms text to lower case", tags=["text"])

    def transform_text(self, text: str) -> str:
        return text.upper()


@functions.register("bench.text.substitute")
class TemplateTextSwapper(TextTransform):
    metadata = FunctionMetadata("Text substitute", "Substitutes text patterns", tags=["text"])
    config_spec = convert_to_config_type_spec(
        {
            "swaps_dataset": DatasetType(record_spec={"pattern": str, "replacement": str}),
            "spread_original": bool,
        },
    )

    def __init__(self, swaps_dataset: DatasetReader, spread_original: bool = False):
        super().__init__(spread_original=spread_original)
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


@functions.register("bench.text.templatize")
class TemplatizeTextTransform(SingleRecordTransform):
    metadata = FunctionMetadata("Templatize", "Transforms text to template", tags=["text"])

    def __init__(self, template: str, **kwargs):
        self.dataset_variables: dict[str, DatasetHandler] = {
            key.upper(): val for key, val in kwargs.items() if isinstance(val, DatasetHandler)
        }
        self.input_variables: dict[str, Any] = {
            key.upper(): val for key, val in kwargs.items() if not isinstance(val, DatasetHandler)
        }
        self.template = template

    @staticmethod
    def render_template(template: str, parameters: dict[str, Any]) -> str:
        # find variables in template that look like $VARIABLE or $VARIABLE.field
        slots = re.findall(r"\$(\w+)(?:\.(\w+))?", template)
        rendered_text = template
        for slot in slots:
            if len(slot) == 2:
                variable_name, field_name = slot
                variable_value = parameters[variable_name][field_name]
            else:
                variable_value = parameters[slot[0]]

            # TODO @Broken: render all variable values in templatize, not just the first
            if isinstance(variable_value, list):
                variable_value = variable_value[0]

            # replace slot with value in rendered text
            rendered_text = rendered_text.replace(f"${'.'.join(slot)}", variable_value)
        return rendered_text

    def transform(self, record: dict) -> str:
        return self.render_template(
            self.template, {**self.dataset_variables, "INPUT": {**self.other_variables, **record}}
        )

import abc
import re
from typing import Any, Tuple, Union, cast

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

    def __init__(self, swaps_dataset: RecordBatch, spread_original: bool = False):
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


def render_template(template: str, parameters: dict[str, Any]) -> str:
    # find variables in template that look like $VARIABLE or $VARIABLE.field
    slots = re.findall(r"\$(\w+)(?:\.(\w+))?", template)
    rendered_text = template
    for slot in slots:
        if len(slot) == 2:
            var_name, field_name = slot
        else:
            var_name, field_name = slot[0], None

        if var_name not in parameters:
            raise ValueError(f"variable {var_name} not set")
        var_value = parameters[var_name]
        if field_name is not None:
            if isinstance(var_value, dict) and field_name not in var_value:
                raise ValueError(
                    f"variable {var_name} does not have field {field_name}: {var_value}"
                )
            var_value = var_value[field_name]

        if isinstance(var_value, list):
            var_value = var_value[0]

        # replace slot with value in rendered text
        rendered_text = rendered_text.replace(f"${'.'.join(slot)}", var_value)
    return rendered_text


@functions.register("bench.text.templatize")
class TemplatizeTextTransform(SingleRecordTransform):
    metadata = FunctionMetadata("Templatize", "Transforms text with a template", tags=["text"])
    input_spec = {"*": convert_to_record_spec({"text": str})}
    output_spec = dict_to_ordered({"*": convert_to_record_spec({"text": str})})

    def __init__(self, template: str, **kwargs):
        self.dataset_variables: dict[str, RecordBatch] = {
            key.upper(): val for key, val in kwargs.items() if isinstance(val, RecordBatch)
        }
        self.input_variables: dict[str, Any] = {
            key.upper(): val for key, val in kwargs.items() if not isinstance(val, RecordBatch)
        }
        self.template = template

    def transform(self, record: Record) -> Union[Record, RecordBatch]:
        rendered_text = render_template(
            self.template, {**self.dataset_variables, "INPUT": {**self.input_variables, **record}}
        )
        return {"text": rendered_text}


@functions.register("bench.text.fewshot")
class FewshotTextTransform(TemplatizeTextTransform):
    metadata = FunctionMetadata(
        "Templatize few shot", "Transforms text with a few shot template", tags=["text", "fewshot"]
    )

    def __init__(self, template: str, sample_template: str, samples: RecordBatch, **kwargs):
        super().__init__(template, samples=samples, **kwargs)
        self.sample_template = sample_template
        self.samples = samples

    def transform(self, record: Record) -> Union[Record, RecordBatch]:
        variables = {**self.dataset_variables, "INPUT": {**self.input_variables, **record}}

        # render sample templates for each sample in the SAMPLES dataset variable
        sample_texts = []
        for sample in self.samples:
            local_variables = {**variables, "SAMPLES": sample}
            sample_text = render_template(self.sample_template, local_variables)
            sample_texts.append(sample_text)

        prompt_text = render_template(self.template, variables)

        # concat few shot samples and prompt for final text
        concat_text = "".join(sample_texts) + prompt_text
        return {"text": concat_text}

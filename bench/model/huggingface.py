import json
from typing import Optional, Union, cast

import requests
import torch
from transformers import AutoModelForSequenceClassification, AutoTokenizer

from bench.artifact.base import NO_STATIC_KEYS
from bench.model.base import UnbatchedModelHandler, models
from bench.utils.record import Record, RecordBatch
from bench.utils.spec import ModelSpec, convert_to_record_spec


# TODO @Feature: HuggingFaceModel only works for sequence classification task
@models.register("bench.huggingface.sequence_classification")
class HuggingFaceModelForSequenceClassification(UnbatchedModelHandler):
    base_spec = ModelSpec(
        name="bench.huggingface.sequence_classification",
        description="HuggingFace local model for sequence classification",
        input_spec=convert_to_record_spec({"text": str}),
        output_spec=convert_to_record_spec({"text": str}),
    )
    config_static_keys = {"model_name", "version"}

    def __init__(self, model_name: str, version: Optional[str], **kwargs):
        self.model_name = model_name
        self.tokenizer = AutoTokenizer.from_pretrained(model_name, revision=version)
        self.model = AutoModelForSequenceClassification.from_pretrained(
            model_name, revision=version
        )
        super().__init__(**kwargs)

    def predict(self, record: Record) -> Union[Record, RecordBatch]:
        record = cast(dict, record)
        tokenized_text = self.tokenizer(record["text"], return_tensors="pt")
        tokens = self.tokenizer.convert_ids_to_tokens(
            tokenized_text["input_ids"].tolist()[0], skip_special_tokens=True
        )
        output_logits = self.model(**tokenized_text).logits
        categories = torch.softmax(output_logits, dim=1).tolist()[0]
        # TODO @Broken: remap outputs to proper categories according to output spec
        return {**record, "tokens": tokens, "categories": categories}


class HuggingFaceHostedModel(UnbatchedModelHandler):
    config_static_keys = NO_STATIC_KEYS

    def __init__(self, model_name: str, bearer_token: str, **kwargs):
        self.model_name = model_name
        self.bearer_token = bearer_token
        super().__init__(**kwargs)

    def predict(self, record: Record) -> Union[Record, RecordBatch]:
        record = cast(dict, record)
        headers = {"Authorization": f"Bearer {self.bearer_token}"}
        data = json.dumps(record)
        api_url = f"https://api-inference.huggingface.co/models/{self.model_name}"
        response = requests.request("POST", api_url, headers=headers, data=data)
        output = json.loads(response.content.decode("utf-8"))
        return output


@models.register("bench.huggingface.hosted.text_generation")
class HuggingFaceHostedGenerationModel(HuggingFaceHostedModel):
    def predict(self, record: Record) -> Union[Record, RecordBatch]:
        record = cast(dict, record)
        # TODO @Cleanup @Architecture: generalise model/flow node input/output remapping
        if "text" in record:
            record = {**record, "inputs": record["text"]}
        output = super().predict(record)
        return output

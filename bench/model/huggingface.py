import json
from typing import Optional, Union, cast

import requests
import structlog.stdlib
import torch
from transformers import AutoModelForSequenceClassification, AutoTokenizer

from bench.artifact.base import NO_STATIC_KEYS
from bench.model.base import ModelHandlerMetadata, UnbatchedModelHandler, models
from bench.utils.record import Record, RecordBatch
from bench.utils.spec import ModelType, convert_to_record_spec

logger = structlog.stdlib.get_logger()


# TODO @Feature: HuggingFaceModel only works for sequence classification task
@models.register("bench.huggingface.sequence_classification")
class HuggingFaceModelForSequenceClassification(UnbatchedModelHandler):
    metadata = ModelHandlerMetadata(
        name="HuggingFace sequence classification",
        description="HuggingFace local model for sequence classification",
        tags=["huggingface"],
    )
    base_spec = ModelType(
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

    def run(self, record: Record) -> Union[Record, RecordBatch]:
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

    @property
    def api_url(self):
        return f"https://api-inference.huggingface.co/models/{self.model_name}"

    @property
    def headers(self) -> dict:
        return {"Authorization": f"Bearer {self.bearer_token}"}

    def run(self, record: Record) -> Record:
        data = cast(dict, record)
        response = requests.request(
            "POST", self.api_url, headers=self.headers, data=json.dumps(data)
        )
        if response.status_code == 503:
            logger.debug("waiting_for_hosted_model", model=self.model_name)
            response = requests.request(
                "POST",
                self.api_url,
                headers=self.headers,
                data=json.dumps({**data, "options": {"wait_for_model": "true"}}),
            )

        outputs = json.loads(response.content.decode("utf-8"))
        return outputs[0]


@models.register("bench.huggingface.hosted.text_generation")
class HuggingFaceHostedGenerationModel(HuggingFaceHostedModel):
    metadata = ModelHandlerMetadata(
        name="HuggingFace hosted text generation",
        description="HuggingFace hosted model for text generation",
        tags=["huggingface", "hosted"],
    )
    base_spec = ModelType(
        input_spec=convert_to_record_spec({"text": str}),
        output_spec=convert_to_record_spec({"generated_text": str}),
    )

    def run(self, record: Record) -> Union[Record, RecordBatch]:
        record = cast(dict, record)
        # TODO @Cleanup @Architecture: generalise model/flow node input/output remapping
        if "text" in record:
            record = {"inputs": record["text"]}
        output = super().run(record)
        return {**record, **output}

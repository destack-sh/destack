from typing import Union

import torch
from transformers import AutoModelForSequenceClassification, AutoTokenizer

from bench.model.base import BatchModelHandler, models
from bench.utils.record import Record, RecordBatch
from bench.utils.spec import ModelSpec


@models.register("bench.huggingface.sequence_classification")
class HuggingFaceModelForSequenceClassification(BatchModelHandler):
    spec = ModelSpec(input_spec={"text": str}, output_spec={"text": str})

    def __init__(self, model_name: str):
        self.model_name = model_name
        self.tokenizer = AutoTokenizer.from_pretrained(model_name)
        self.model = AutoModelForSequenceClassification.from_pretrained(model_name)

    def predict(self, record: Record) -> Union[Record, RecordBatch]:
        # TODO @Feature: HuggingFaceModel only works for sequence classification

        tokenized_record = self.tokenizer(record["text"], return_tensors="pt")
        tokens = self.tokenizer.convert_ids_to_tokens(
            tokenized_record["input_ids"].tolist()[0], skip_special_tokens=True
        )
        output_logits = self.model(**tokenized_record).logits
        categories = torch.softmax(output_logits, dim=1).tolist()[0]
        # TODO @Broken: remap outputs to proper categories using output spec
        return {"text": record["text"], "tokens": tokens, "categories": categories}

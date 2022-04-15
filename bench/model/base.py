import abc
from typing import Union

import spacy
import torch
from transformers import AutoModelForSequenceClassification, AutoTokenizer

from bench.utils.record import ListRecordBatch, Record, RecordBatch
from bench.utils.spec import ConfigSpec, ModelSpec


class ModelBase(abc.ABC):
    config_spec: ConfigSpec

    def __init__(self, spec: ModelSpec):
        self.spec = spec

    def forward(self, record: Record) -> Union[Record, RecordBatch]:
        raise NotImplementedError

    def forward_batch(self, records: RecordBatch) -> RecordBatch:
        raise NotImplementedError


class HuggingFaceModel(ModelBase):
    def __init__(self, model_name: str, **kwargs):
        super().__init__(**kwargs)
        self.model_name = model_name
        self.tokenizer = AutoTokenizer.from_pretrained(model_name)
        self.model = AutoModelForSequenceClassification.from_pretrained(model_name)

    def forward(self, record: Record) -> Union[Record, RecordBatch]:
        # TODO @Feature: HuggingFaceModel only works for sequence classification

        tokenized_record = self.tokenizer(record["text"], return_tensors="pt")
        tokens = self.tokenizer.convert_ids_to_tokens(
            tokenized_record["input_ids"].tolist()[0], skip_special_tokens=True
        )
        output_logits = self.model(**tokenized_record).logits
        categories = torch.softmax(output_logits, dim=1).tolist()[0]
        # TODO @Broken: remap outputs to proper categories using output spec
        return {"text": record["text"], "tokens": tokens, "categories": categories}


class SpacyModelBase(ModelBase, abc.ABC):
    def __init__(self, nlp: spacy.language.Language, **kwargs):
        super().__init__(**kwargs)
        self.nlp = nlp

    def _map_doc_to_record(self, doc: spacy.language.Doc) -> Record:
        output = {"text": doc.text, "tokens": list(doc)}
        if self.nlp.has_pipe("ner") or self.nlp.has_pipe("entity_ruler"):
            output["entities"] = doc.ents
        if self.nlp.has_pipe("textcat") or self.nlp.has_pipe("textcat_multilabel"):
            output["categories"] = doc.cats
        return output

    def forward(self, record: Record) -> Union[Record, RecordBatch]:
        doc = self.nlp(record["text"])
        output = self._map_doc_to_record(doc)
        return output

    def forward_batch(self, records: RecordBatch) -> RecordBatch:
        docs = list(self.nlp.pipe(records["text"]))
        output_records = [self._map_doc_to_record(doc) for doc in docs]
        return ListRecordBatch(output_records)


class SpacyModelBundled(SpacyModelBase):
    def __init__(self, model_name: str, **kwargs):
        try:
            nlp = spacy.load(model_name)
        except OSError:
            # load failed, try downloading model and then retry
            from spacy.cli import download

            download(model_name)
            nlp = spacy.load(model_name)
        super().__init__(nlp, **kwargs)


class SpacyModelCustom(SpacyModelBase):
    def __init__(self, model_path: str, config_path: str, **kwargs):
        config = spacy.Config().from_disk(config_path)
        lang_cls = spacy.util.get_lang_class(config["nlp"]["lang"])
        nlp = lang_cls.from_config(config)
        nlp.from_disk(model_path)
        super().__init__(nlp, **kwargs)


class OpenAIModel(ModelBase):
    def __init__(self, api_token: str, **kwargs):
        super().__init__(**kwargs)


if __name__ == "__main__":
    spec = ModelSpec(
        input_spec={},
        output_spec={},
    )
    hf_model = HuggingFaceModel(
        model_name="nlptown/bert-base-multilingual-uncased-sentiment",
        spec=spec,
    )
    record = {
        "text": "Veritatis veritatis unde quo ut hic est et."
        "Rerum repellendus ut fuga dolorem vel. Sint eos nostrum porro."
        "Enim ut nihil molestias ea. Unde quia tempora explicabo consequatur ut corrupti."
    }
    print(hf_model.forward(record))

    spacy_model = SpacyModelBundled("en_core_web_sm", spec=spec)
    print(spacy_model.forward(record))

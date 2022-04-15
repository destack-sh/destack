import abc
from typing import Union

import spacy
from transformers import AutoModel, AutoTokenizer

from bench.models.record import ListRecordBatch, Record, RecordBatch
from bench.models.spec import ConfigSpec, ModelSpec


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
        self.model = AutoModel.from_pretrained(model_name)

    def forward(self, record: Record) -> Union[Record, RecordBatch]:
        # TODO @Feature: HuggingFaceModel only works for sequence classification

        tokens = self.tokenizer(record["text"], return_tensors="pt")
        outputs = self.model(**tokens).logits
        # TODO @Broken: remap outputs to proper categories using output spec
        return {"text": record["text"], "tokens": tokens, "categories": outputs}


class SpacyModel(ModelBase):
    def __init__(self, config_path: str, model_path: str, **kwargs):
        super().__init__(**kwargs)
        self.config = spacy.Config().from_disk(config_path)
        lang_cls = spacy.util.get_lang_class(self.config["nlp"]["lang"])
        self.nlp = lang_cls.from_config(self.config)
        self.nlp.from_disk(model_path)

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


class OpenAIModel(ModelBase):
    def __init__(self, api_token: str, **kwargs):
        super().__init__(**kwargs)

import abc
from typing import Union, cast

import spacy

from bench.model.base import ModelHandler, ModelHandlerMetadata, models
from bench.model.utils import TOKENS_SPEC, make_entities_spec, make_scored_labels_spec
from bench.utils.record import Record, RecordBatch, RecordList
from bench.utils.spec import ClassLabelType, ModelType, convert_to_record_spec


class SpacyModelBase(ModelHandler, abc.ABC):
    def __init__(self, nlp: spacy.language.Language, **kwargs):
        self.nlp = nlp
        self.has_categories = self.nlp.has_pipe("textcat") or self.nlp.has_pipe(
            "textcat_multilabel"
        )
        self.has_entities = self.nlp.has_pipe("ner") or self.nlp.has_pipe("entity_ruler")
        self.base_spec = self.infer_spec()
        super().__init__(**kwargs)

    @property
    def runtime_spec(self) -> ModelType:
        return self.infer_spec()

    def infer_spec(self) -> ModelType:
        input_spec = convert_to_record_spec(name="input", description="", spec={"text": str})
        output_type = {"text": str, "tokens": TOKENS_SPEC}
        if self.has_entities:
            labels = self.nlp.get_pipe("ner").labels
            entity_type = ClassLabelType(num_classes=len(labels), names=list(labels))
            output_type["spans"] = make_entities_spec(entity_type)
        if self.has_categories:
            if self.nlp.has_pipe("textcat"):
                pipe = self.nlp.get_pipe("textcat")
            else:
                pipe = self.nlp.get_pipe("textcat_multilabel")
            category_type = ClassLabelType(num_classes=len(pipe.labels), names=list(pipe.labels))
            output_type["classes"] = make_scored_labels_spec(category_type)

        output_spec = convert_to_record_spec(name="output", description="", spec=output_type)
        return ModelType(input_spec=input_spec, output_spec=output_spec)

    def _map_doc_to_record(self, doc: spacy.language.Doc) -> Record:
        tokens: list[dict] = [
            {
                "text": token.text_with_ws,
                "start": token.idx,
                "end": token.idx + len(token.text),
            }
            for token in doc
        ]
        output = {"text": doc.text, "tokens": tokens}
        if self.has_entities:
            entities = []
            for span in doc.ents:
                end = doc[span.end].idx if span.end < len(doc) else len(doc.text)
                entity = {
                    "text": span.text,
                    "start": doc[span.start].idx,
                    "end": end,
                    "label": span.label_,
                }
                entities.append(entity)
            output["spans"] = entities
        if self.has_categories:
            output["categories"] = [
                {"label": label, "score": score} for label, score in doc.cats.items()
            ]
        return output

    def predict(self, record: Record) -> Union[Record, RecordBatch]:
        record = cast(dict, record)  # assume record is dict
        doc = self.nlp(cast(str, record["text"]))
        output = self._map_doc_to_record(doc)
        return output

    def predict_batch(self, records: RecordBatch) -> RecordBatch:
        texts = cast(list[str], records["text"])
        docs = list(self.nlp.pipe(texts))
        output_records = [self._map_doc_to_record(doc) for doc in docs]
        return RecordList(output_records)


@models.register("bench.spacy.bundled")
class SpacyModelBundled(SpacyModelBase):
    metadata = ModelHandlerMetadata(
        name="SpaCy model bundled",
        description="SpaCy model pre-bundled (from SpaCy " + spacy.__version__ + ")",
        tags=["spacy"],
    )
    config_static_keys = {"model_name"}

    def __init__(self, model_name: str, **kwargs):
        try:
            nlp = spacy.load(model_name)
        except OSError:
            # load failed, try downloading model and then retry
            from spacy.cli import download

            download(model_name)
            nlp = spacy.load(model_name)
        super().__init__(nlp, **kwargs)


@models.register("bench.spacy.custom")
class SpacyModelCustom(SpacyModelBase):
    metadata = ModelHandlerMetadata(
        name="SpaCy model custom",
        description="Custom SpaCy model (compatible with SpaCy " + spacy.__version__ + ")",
        tags=["spacy"],
    )
    config_static_keys = {"model_path", "config_path"}

    def __init__(self, model_path: str, config_path: str, **kwargs):
        config = spacy.Config().from_disk(config_path)
        lang_cls = spacy.util.get_lang_class(config["nlp"]["lang"])
        nlp = lang_cls.from_config(config)
        nlp.from_disk(model_path)
        super().__init__(nlp, **kwargs)

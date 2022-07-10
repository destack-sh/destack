import abc
from typing import Union, cast

import spacy

from bench.model.base import ModelHandler, models
from bench.utils.record import Record, RecordBatch, RecordList


class SpacyModelBase(ModelHandler, abc.ABC):
    def __init__(self, nlp: spacy.language.Language, **kwargs):
        super().__init__(**kwargs)
        self.nlp = nlp
        self.has_categories = self.nlp.has_pipe("textcat") or self.nlp.has_pipe(
            "textcat_multilabel"
        )
        self.has_entities = self.nlp.has_pipe("ner") or self.nlp.has_pipe("entity_ruler")

    def _map_doc_to_record(self, doc: spacy.language.Doc) -> Record:
        tokens: list[dict] = [{"text": token.text} for token in doc]
        output = {"text": doc.text, "tokens": tokens}
        if self.has_entities:
            output["entities"] = [
                {
                    "text": span.text,
                    "start": span.start,
                    "end": span.end,
                    "label": span.label_,
                }
                for span in doc.ents
            ]
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
    """
    Spacy model wrapper for models pre-bundled with spacy.
    """

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
    """
    Spacy model wrapper for custom models (model + config).
    """

    config_static_keys = {"model_path", "config_path"}

    def __init__(self, model_path: str, config_path: str, **kwargs):
        config = spacy.Config().from_disk(config_path)
        lang_cls = spacy.util.get_lang_class(config["nlp"]["lang"])
        nlp = lang_cls.from_config(config)
        nlp.from_disk(model_path)
        super().__init__(nlp, **kwargs)

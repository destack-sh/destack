from typing import Iterator

from bench.function.base import Map, ModelRunner, Predicate, Record, Supplier


class BackTranslation(Map):
    def __init__(
        self, text_key: str, forward_model: ModelRunner, backward_model: ModelRunner
    ):
        self.text_key = text_key
        self.forward_model = forward_model
        self.backward_model = backward_model

    def __call__(self, record: Record) -> Record:
        text_record = {"text": record[self.text_key]}
        translated_record = self.forward_model(text_record)
        backtranslated_record = self.backward_model(translated_record)
        return backtranslated_record


class TemplatedSuggester(Supplier):
    def __init__(self, mlm: ModelRunner):
        self.mlm = mlm

    def __call__(self, record: Record) -> Iterator[Record]:
        return super().__call__(record)


class SentenceLengthConstraint(Predicate):
    def __init__(self, sentence_key: str, max_length: int):
        self.sentence_key = sentence_key
        self.max_length = max_length

    def __call__(self, record: Record, reference_record: Record) -> bool:
        return len(record[self.sentence_key]) <= self.max_length


class SpellcheckConstraint(Predicate):
    def __call__(self, record, reference_record):
        pass

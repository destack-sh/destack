import abc
import enum
from typing import Dict, List, Optional, Tuple, Union

import openai

from bench.model.base import BatchModelHandler, models
from bench.utils.record import ListRecordBatch, Record, RecordBatch
from bench.utils.spec import ModelSpec


class OpenAIModel(BatchModelHandler, abc.ABC):
    class Engine(enum.Enum):
        Ada = "text-ada-001"
        Babbage = "text-babbage-001"
        Curie = "text-curie-001"
        Davinci = "text-davinci-002"
        CodexDavinci = "code-davinci-002"
        CodexCushman = "code-cushman-001"

    def __init__(
        self,
        api_key: str,
        engine_id: Engine = Engine.Ada,
        max_tokens: int = 16,
        temperature: float = 1.0,
        top_p: float = 1.0,
        n: int = 1,
        stop: Optional[Union[str, List[str]]] = None,
        **kwargs
    ):
        super().__init__(**kwargs)
        self._api_key = api_key
        self.engine_id = engine_id.value
        self.max_tokens = max_tokens
        self.temperature = temperature
        self.top_p = top_p
        self.n = n
        self.stop = stop

    def _get_params(self):
        return dict(
            engine=self.engine_id,
            max_tokens=self.max_tokens,
            temperature=self.temperature,
            top_p=self.top_p,
            n=self.n,
            stop=self.stop,
            api_key=self._api_key,
        )


@models.register("bench.openai.completion")
class OpenAIModelForCompletion(OpenAIModel):
    spec = ModelSpec(input_spec={"text": str}, output_spec={"text": str})

    def predict(self, record: Record) -> Union[Record, RecordBatch]:
        output = openai.Completion.create(
            prompt=record["text"],
            **self._get_params(),
        )
        output_records = [choice for choice in output["choices"]]
        if self.n == 1:
            return output_records[0]
        else:
            return ListRecordBatch(output_records)


@models.register("bench.openai.classification")
class OpenAIModelForClassification(OpenAIModel):
    spec = ModelSpec(
        input_spec={
            "text": str,
            "examples": List[Tuple[str, str]],
            "labels": List[str],
        },
        output_spec={"text": str, "categories": Dict[str, float]},
    )

    def predict(self, record: Record) -> Union[Record, RecordBatch]:
        output = openai.Classification.create(
            prompt=record["text"],
            examples=record["examples"],
            labels=record["labels"],
            **self._get_params(),
        )
        output_records = [choice for choice in output["choices"]]
        if self.n == 1:
            return output_records[0]
        else:
            return ListRecordBatch(output_records)

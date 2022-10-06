import abc
from typing import Union, cast

import aiohttp

from bench.model.base import AsyncBatchedModelHandler, ModelHandlerMetadata, models
from bench.model.utils import make_scored_labels_spec
from bench.utils.record import Record, RecordBatch, RecordList
from bench.utils.spec import ModelType, convert_to_record_spec


class OpenAIModel(AsyncBatchedModelHandler, abc.ABC):
    config_static_keys: set[str] = set()  # entire config can be changed dynamically

    def __init__(
        self,
        api_key: str,
        model: str,
        max_tokens: int = 128,
        temperature: float = 0.7,
        top_p: float = 1.0,
        n: int = 1,
        stop: list[str] = None,
        **kwargs,
    ):
        super().__init__(**kwargs)
        self._api_key = api_key
        self.model = model
        self.max_tokens = max_tokens
        self.temperature = temperature
        self.top_p = top_p
        self.n = n
        self.stop = stop

    @property
    def headers(self):
        return {"Content-Type": "application/json", "Authorization": f"Bearer {self._api_key}"}

    def _get_params(self):
        return dict(
            model=self.model,
            max_tokens=self.max_tokens,
            temperature=self.temperature,
            top_p=self.top_p,
            n=self.n,
            stop=self.stop,
        )


@models.register("bench.openai.text_generation")
class OpenAIModelForCompletion(OpenAIModel):
    metadata = ModelHandlerMetadata(
        name="OpenAI text generation",
        description="OpenAI hosted model for text generation",
        tags=["openai", "hosted"],
    )
    spec = ModelType(
        input_spec=convert_to_record_spec(name="input", spec={"text": str}),
        output_spec=convert_to_record_spec(name="output", spec={"generated_text": str}),
    )
    base_spec = spec

    async def run_async(self, record: Record) -> Union[Record, RecordBatch]:
        record = cast(dict, record)
        request = {"model": self.model, "prompt": record["text"], **self._get_params()}

        async with aiohttp.ClientSession(headers=self.headers) as session:
            async with session.post(
                "https://api.openai.com/v1/completions", json=request
            ) as response:
                output = await response.json()

        output_records = [choice for choice in output["choices"]]
        if self.n == 1:
            return output_records[0]
        else:
            return RecordList(output_records)


@models.register("bench.openai.text_classification")
class OpenAIModelForClassification(OpenAIModel):
    metadata = ModelHandlerMetadata(
        name="OpenAI classification",
        description="OpenAI hosted model for text classification",
        tags=["openai", "hosted"],
    )
    spec = ModelType(
        input_spec=convert_to_record_spec(
            {
                "text": str,
                "examples": list[list[str]],
                "labels": list[str],
            }
        ),
        output_spec=convert_to_record_spec({"text": str, "classes": make_scored_labels_spec(str)}),
    )
    base_spec = spec

    async def run_async(self, record: Record) -> Union[Record, RecordBatch]:
        record = cast(dict, record)
        request = {
            "model": self.model,
            "prompt": record["text"],
            "examples": record["examples"],
            "labels": record["labels"],
            **self._get_params(),
        }

        async with aiohttp.ClientSession(headers=self.headers) as session:
            async with session.post(
                "https://api.openai.com/v1/classifications", json=request
            ) as response:
                output = await response.json()

        output_records = [choice for choice in output["choices"]]
        if self.n == 1:
            return output_records[0]
        else:
            return RecordList(output_records)

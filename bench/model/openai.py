from functools import cached_property
from typing import Union

import aiohttp

from bench.model.base import ModelHandler
from bench.utils.record import Record, RecordBatch, RecordList


class OpenAIModel(ModelHandler):
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

    @cached_property
    def _params(self):
        return dict(
            model=self.model,
            max_tokens=self.max_tokens,
            temperature=self.temperature,
            top_p=self.top_p,
            n=self.n,
            stop=self.stop,
        )

    async def complete(self, prompt: str) -> tuple[str, list[float]]:
        request = {"model": self.model, "prompt": str, **self._params}

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

    async def classify(
        self, prompt: str, labels: list[str], examples: list[tuple[str, str]]
    ) -> Union[Record, RecordBatch]:
        request = {
            "model": self.model,
            "prompt": prompt,
            "labels": labels,
            "examples": examples,
            **self._params,
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

from functools import cached_property
from typing import Union

import aiohttp

from bench.backend.base import ModelHandle, ModelProvider
from bench.models import Model, Organization
from bench.models.model import ModelInferenceSettings, ModelInferenceSettingsSerializer


class ForefrontProvider(ModelProvider):
    def __init__(self, api_key: str):
        self._api_key = api_key

    @cached_property
    def headers(self):
        return {"Content-Type": "application/json", "Authorization": f"Bearer {self._api_key}"}

    async def access(
        self, model: Model, settings: ModelInferenceSettings, for_user: Organization
    ) -> ModelHandle:
        return ForefrontModel(headers=self.headers, model=model.name, settings=settings)


class ForefrontModel(ModelHandle):
    def __init__(
        self,
        headers: dict,
        model: str,
        settings: ModelInferenceSettings,
    ):
        self.headers = headers
        self.model = model
        self.settings = settings
        self._aiohttp_session = None

    def __str__(self):
        return f"forefront/{self.model}"

    @cached_property
    def settings_json(self):
        return ModelInferenceSettingsSerializer(self.settings).data

    async def complete(
        self, prompt: str
    ) -> Union[tuple[str, list[float]], list[tuple[str, list[float]]]]:
        request = {"model": self.model, "prompt": str, **self.settings_json}
        async with aiohttp.ClientSession(headers=self.headers) as session:
            async with session.post(
                "https://api.openai.com/v1/completions", json=request
            ) as response:
                if response.status != 200:
                    raise RuntimeError(await response.text())
                output = await response.json()

        outputs = [choice for choice in output["choices"]]
        outputs = [(o["text"], o["logprobs"]) for o in outputs]
        if len(outputs) == 1:
            return outputs[0]
        else:
            return outputs

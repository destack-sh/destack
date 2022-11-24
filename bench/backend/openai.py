import hashlib
from functools import cached_property
from typing import Union

import aiohttp

from bench.backend.base import Completion, ModelHandle, ModelProvider
from bench.models import Model, ModelInferenceSettings


class OpenAIProvider(ModelProvider):
    def __init__(self, api_key: str):
        self._api_key = api_key

    @property
    def headers(self):
        return {"Content-Type": "application/json", "Authorization": f"Bearer {self._api_key}"}

    async def access(
        self, model: Model, settings: ModelInferenceSettings, for_user: str
    ) -> ModelHandle:
        user_hashed = hashlib.shake_256(for_user.encode()).hexdigest(len(for_user) * 2)
        return OpenAIModel(
            headers=self.headers, model=model.name, user_hashed=user_hashed, settings=settings
        )


class OpenAIModel(ModelHandle):
    def __init__(
        self,
        headers: dict,
        model: str,
        user_hashed: str,
        settings: ModelInferenceSettings,
    ):
        self.headers = headers
        self.model = model
        self.user_hashed = user_hashed
        self._settings = settings
        self._aiohttp_session = None

    def __str__(self):
        return f"openai/{self.model}"

    @property
    def settings(self):
        return self._settings

    @cached_property
    def settings_json(self):
        return self.settings.as_dict()

    async def complete(self, prompt: str) -> Union[Completion, list[Completion]]:
        request = {
            "model": self.model,
            "prompt": prompt,
            "user": self.user_hashed,
            **self.settings_json,
        }

        async with aiohttp.ClientSession(headers=self.headers) as session:
            async with session.post(
                "https://api.openai.com/v1/completions", json=request
            ) as response:
                if response.status != 200:
                    raise RuntimeError(await response.text())
                output = await response.json()

        outputs = [choice for choice in output["choices"]]
        completions = []
        for output in outputs:
            completion = Completion(
                text=output["text"],
                logits=output.get("logprobs", {}).get("logits", None),
                tokens=output.get("logprobs", {}).get("tokens", None),
            )
            completions.append(completion)
        if len(completions) == 1:
            return completions[0]
        else:
            return completions

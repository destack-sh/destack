import hashlib
from typing import Any, Optional, Union

import aiohttp

from bench.backend.provider import Completion, ModelHandle, ModelProvider
from bench.language.types import Model
from bench.models import ModelInferenceSettings


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
        if model.external_name is None:
            raise ValueError(f"model has no external name {model}")

        return OpenAIModel(
            headers=self.headers,
            model=model.external_name,
            user_hashed=user_hashed,
            settings=settings,
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

    def configure(self, **settings: dict[str, Any]) -> ModelHandle:
        merged_settings = {**self.settings.as_dict(omit_empty=True), **settings}
        return OpenAIModel(
            headers=self.headers,
            model=self.model,
            user_hashed=self.user_hashed,
            settings=ModelInferenceSettings(**merged_settings),
        )

    async def complete(
        self, prompt: str, settings: Optional[dict[str, Any]] = None
    ) -> Union[Completion, list[Completion]]:
        if settings is not None:
            settings_merged = {**self.settings.as_dict(omit_empty=True), **settings}
        else:
            settings_merged = self.settings.as_dict(omit_empty=True)
        request = {
            "model": self.model,
            "prompt": prompt,
            "user": self.user_hashed,
            **settings_merged,
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

from __future__ import annotations

import abc
from typing import Union

from bench.models import Model
from bench.models.model import ModelInferenceSettings


class ModelProvider(abc.ABC):
    """
    A model provider which hosts model compute.
    """

    async def access(
        self, model: Model, settings: ModelInferenceSettings, for_user: str
    ) -> ModelHandle:
        raise NotImplementedError

    async def finetune(self, model: Model) -> Model:
        raise NotImplementedError


class ModelHandle(abc.ABC):
    """
    Base for model implementations that can run a specific model.
    """

    @property
    def settings(self) -> ModelInferenceSettings:
        raise NotImplementedError

    async def complete(
        self, prompt: str
    ) -> Union[tuple[str, list[float]], list[tuple[str, list[float]]]]:
        """
        Generate a completion for the given prompt.
        @return: (completion, logprobs) or [(completion, logprops)] if n > 1
        """
        raise NotImplementedError

    async def embed(self, text: str) -> bytes:
        """
        Embed the given text into a vector.
        @return: vector
        """
        raise NotImplementedError

from __future__ import annotations

import abc
from typing import TypedDict, Union

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


Completion = TypedDict(
    "Completion",
    {"text": str, "logits": Union[None, list[float]], "tokens": Union[None, list[str]]},
)


class ModelHandle(abc.ABC):
    """
    Base for model implementations that can run a specific model.
    """

    @property
    def settings(self) -> ModelInferenceSettings:
        raise NotImplementedError

    async def complete(self, prompt: str) -> Union[Completion, list[Completion]]:
        """
        Generate a completion for the given prompt.
        @return: Completion or list[Completion] if n > 1
        """
        raise NotImplementedError

    async def embed(self, text: str) -> bytes:
        """
        Embed the given text into a vector.
        @return: vector
        """
        raise NotImplementedError

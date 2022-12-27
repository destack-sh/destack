from __future__ import annotations

import abc
from typing import TYPE_CHECKING, Any, Optional, TypedDict, Union

from bench.models.model import ModelInferenceSettings

if TYPE_CHECKING:
    from bench.backend.executor import ModelData
else:
    ResolvedModel = Any


class ModelProvider(abc.ABC):
    """
    A model provider which hosts model compute.
    """

    async def access(
        self, model: ModelData, settings: ModelInferenceSettings, for_user: str
    ) -> ModelHandle:
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

    def configure(self, **settings: dict[str, Any]) -> ModelHandle:
        """
        Configure the model.
        """
        raise NotImplementedError

    async def complete(
        self, prompt: str, settings: Optional[dict[str, Any]] = None
    ) -> Union[Completion, list[Completion]]:
        """
        Generate a completion for the given prompt.
        @return: Completion or list[Completion] if n > 1
        """
        raise NotImplementedError

    async def embed(self, text: str, settings: Optional[dict[str, Any]] = None) -> bytes:
        """
        Embed the given text into a vector.
        @return: vector
        """
        raise NotImplementedError

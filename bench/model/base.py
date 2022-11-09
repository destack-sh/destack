import abc
from dataclasses import dataclass
from typing import Any, Mapping, Type

from bench.utils.record import Record


@dataclass
class ModelHandlerMetadata:
    name: str
    description: str
    tags: list[str]


class ModelHandler(abc.ABC):
    """
    Base for model implementations that can load and run a model from some source.
    """

    # Config schema to configure this handler.
    config_schema: Mapping[str, Any]

    async def complete(self, prompt: str) -> tuple[str, list[float]]:
        raise NotImplementedError

    async def classify(
        self, text: str, labels: list[str], examples: list[tuple[str, str]]
    ) -> Record:
        raise NotImplementedError

    async def embed(self, text: str) -> bytes:
        raise NotImplementedError


SPECIAL_MODEL_CONFIG_KEYS = {"artifact_id", "version", "spec"}


def map_to_model_cls(model_cls: Type[ModelHandler], *args) -> Type[ModelHandler]:
    if hasattr(model_cls, "config_schema"):
        raise NotImplementedError
    else:
        declared_config_schema = None
    # TODO @Robustness: check declared_config_schema against inferred_config_schema
    inferred_config_schema = infer_config_type(model_cls.__init__)  # noqa

    # overwrite config spec with clean config
    config_schema = declared_config_schema or inferred_config_schema
    # filter to exclude ignored keys
    config_schema = {
        key: typ for key, typ in config_schema.items() if key not in SPECIAL_MODEL_CONFIG_KEYS
    }
    model_cls.config_schema = config_schema
    return model_cls

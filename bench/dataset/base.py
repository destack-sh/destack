import abc

# TODO @Cleanup: Dataset"Handler" is not a great name (not descriptive enough)
from typing import Any, Dict, Optional, Set, Type

from bench.utils.spec import ConfigSpec, DatasetSpec


class DatasetHandler(abc.ABC):
    """
    Base for dataset implementations that can load and parse a dataset from some source.
    """

    # Known base dataset spec for all datasets of this handler.
    base_spec: Optional[DatasetSpec] = None
    # Config spec to configure this handler.
    config_spec: ConfigSpec
    # Config values are immutable after init. If not set, defaults to all keys.
    config_static_keys: Set[str]

    def __init__(
        self, spec: Optional[DatasetSpec] = None, version: Optional[str] = None
    ):
        if spec is None and self.base_spec is None:
            raise ValueError(
                "DatasetHandler must define either static `spec` or dynamic `get_spec`"
            )
        elif spec is not None:
            self.spec = spec
        self.version = version

    def update_config(self, **kwargs):
        pass


def get_dataset_cls(handler_id: str) -> Type[DatasetHandler]:
    raise NotImplementedError


def load_dataset(
    handler_id: str,
    storage_uri: Optional[str],
    arguments: Dict[str, Any],
    spec: Optional[DatasetSpec],
) -> DatasetHandler:
    dataset_cls = get_dataset_cls(handler_id)
    dataset = dataset_cls(spec=spec, **arguments)
    return dataset

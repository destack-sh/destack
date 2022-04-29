import abc

# TODO @Cleanup: Dataset"Handler" is not a great name (not descriptive enough)
from typing import Any, Dict, Optional, Set, Type, Union

from bench.utils.record import RecordBatch
from bench.utils.registry import Registry
from bench.utils.spec import ConfigSpec, DatasetSpec, DatasetType


class DatasetHandler(RecordBatch, abc.ABC):
    """
    Base for dataset implementations that can load and parse a dataset from some source.
    """

    # Known base dataset spec for all datasets of this handler.
    base_spec: Union[None, DatasetType, DatasetSpec] = None
    # Config spec to configure this handler.
    config_spec: ConfigSpec
    # Config values are immutable after init. If not set, defaults to all keys.
    config_static_keys: Set[str]

    def __init__(
        self,
        spec: Union[None, DatasetType, DatasetSpec] = None,
        version: Optional[str] = None,
    ):
        if spec is None and self.base_spec is None:
            raise ValueError(
                "DatasetHandler must define static `base_spec` or get `spec` argument"
            )
        elif spec is not None:
            self.spec = spec
        self.version = version

    def update_config(self, **kwargs):
        pass


def map_to_dataset_cls(
    func: Any, impl: Optional[Type[DatasetHandler]]
) -> Type[DatasetHandler]:
    raise NotImplementedError


datasets: Registry[Type[DatasetHandler]] = Registry(
    ("datasets",), mapper=map_to_dataset_cls
)


def get_dataset_cls(handler_id: str) -> Type[DatasetHandler]:
    return datasets[handler_id]


def load_dataset(
    handler_id: str,
    storage_uri: Optional[str],
    arguments: Dict[str, Any],
    spec: Optional[DatasetSpec],
) -> DatasetHandler:
    dataset_cls = get_dataset_cls(handler_id)
    dataset = dataset_cls(spec=spec, **arguments)
    return dataset

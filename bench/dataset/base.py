import abc
from typing import Any, Dict, Iterable, Optional, Type, Union

from fsspec import AbstractFileSystem

from bench.artifact.base import ArtifactHandler
from bench.utils.record import Record, RecordBatch
from bench.utils.registry import Registry
from bench.utils.spec import DatasetSpec, DatasetType, RecordSpec


class DatasetHandler(ArtifactHandler):
    """
    Base for dataset implementations.
    """

    # Known base dataset spec for all datasets of this handler.
    base_spec: Union[None, DatasetType, DatasetSpec] = None

    def __init__(
        self,
        fs: AbstractFileSystem = None,
        path: Optional[str] = None,
        version: Optional[str] = None,
        spec: Union[None, DatasetType, DatasetSpec] = None,
    ):
        super().__init__(fs=fs, path=path, version=version)
        if spec is None:
            if self.base_spec is None:
                raise ValueError("DatasetHandler must define `base_spec` or get `spec` argument")
            else:
                self.spec = self.base_spec
        else:
            self.spec = spec

    def update_config(self, **kwargs):
        pass


class DatasetReader(DatasetHandler, RecordBatch, abc.ABC):
    """
    Base for dataset implementations that can read their data in a common format.
    """

    pass


class DatasetWriter(DatasetHandler, abc.ABC):
    """
    Base for dataset implementations that can modify their data locally.
    """

    # -- Modify schema --
    def set_spec(self, spec: RecordSpec):
        pass

    # -- Modify data ---
    def append(self, record: Record):
        raise NotImplementedError

    def extend(self, records: Iterable[Record]):
        raise NotImplementedError

    def update(self, index: int, record: Record):
        raise NotImplementedError

    def delete(self, index: int):
        raise NotImplementedError


def map_to_dataset_cls(func: Any, impl: Optional[Type[DatasetHandler]]) -> Type[DatasetHandler]:
    if impl is not None:
        raise ValueError("specifying impl type is not supported")

    return func


datasets: Registry[Type[DatasetHandler]] = Registry(("datasets",), mapper=map_to_dataset_cls)
# TODO @Feature: figure out better registration mechanism for registered objects
import bench.dataset.huggingface  # noqa


def get_dataset_cls(handler_id: str) -> Type[DatasetHandler]:
    return datasets[handler_id]


def load_dataset(
    handler_id: str,
    storage_uri: Optional[str],
    version: Optional[str],
    arguments: Dict[str, Any],
    spec: Optional[DatasetSpec],
) -> DatasetHandler:
    dataset_cls = get_dataset_cls(handler_id)
    dataset = dataset_cls(spec=spec, version=version, **arguments)  # noqa
    return dataset

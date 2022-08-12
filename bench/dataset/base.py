import abc
from dataclasses import dataclass
from typing import Any, Iterable, Optional, Tuple, Type

import cachetools
from cachetools import LRUCache
from fsspec import AbstractFileSystem

from bench.artifact.base import ArtifactHandler
from bench.utils.record import Record, RecordBatch
from bench.utils.registry import Registry
from bench.utils.spec import ConfigTypeSpec, DatasetType, RecordSpec, infer_config_type


@dataclass
class DatasetHandlerMetadata:
    name: str
    description: str
    tags: list[str]


class DatasetHandler(ArtifactHandler):
    """
    Base for dataset implementations.
    """

    # Known base dataset spec for all datasets of this handler.
    metadata: DatasetHandlerMetadata
    base_spec: Optional[DatasetType] = None
    spec: DatasetType

    def __init__(
        self,
        fs: AbstractFileSystem = None,
        path: Optional[str] = None,
        version: Optional[str] = None,
        spec: Optional[DatasetType] = None,
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
    def append(self, record: Record) -> int:
        """Appends the given record, returning its index"""
        raise NotImplementedError

    def extend(self, records: Iterable[Record]) -> tuple[int, int]:
        """Appends the given records, returning the start and end indices"""
        raise NotImplementedError

    def update(self, index: int, record: Record):
        raise NotImplementedError

    def delete(self, index: int):
        raise NotImplementedError

    def clear(self):
        raise NotImplementedError


def map_to_dataset_cls(func: Any, impl: Optional[Type[DatasetHandler]]) -> Type[DatasetHandler]:
    if impl is not None:
        raise ValueError("specifying impl type is not supported")

    return func


# TODO @Feature: figure out better registration mechanism for registered objects
datasets: Registry[Type[DatasetHandler]] = Registry(("datasets",), mapper=map_to_dataset_cls)
import bench.dataset.activeloop  # noqa
import bench.dataset.db  # noqa
import bench.dataset.huggingface  # noqa


def get_dataset_cls(handler_id: str) -> Type[DatasetHandler]:
    return datasets[handler_id]


@cachetools.cached(cache=LRUCache(maxsize=1024), key=lambda cls: cls.__name__)
def _get_dataset_handler_config_type(cls: Type[DatasetHandler]) -> ConfigTypeSpec:
    return infer_config_type(cls)


def get_file_system(storage_uri: str) -> Tuple[AbstractFileSystem, str]:
    """
    Parses the given storage uri into the corresponding file system & path
    @param storage_uri: The storage URI
    @return: A tuple of [fs, path]
    """
    # TODO @Feature: parse storage_uri into fsspec's fs & path for ArtifactHandler
    #  Note: how will we get auth information in here?
    raise NotImplementedError


def load_dataset(
    handler_id: str,
    storage_uri: Optional[str],
    version: Optional[str],
    spec: Optional[DatasetType],
    arguments: dict[str, Any],
) -> DatasetHandler:
    """
    Loads a handler for interacting with the given dataset
    @param handler_id: The handler to use
    @param storage_uri: The storage location of the dataset (if any)
    @param version: The version of the dataset (if any)
    @param spec: The known spec to use (if any)
    @param kwargs: additional arguments passed to the handler
    """

    dataset_cls = get_dataset_cls(handler_id)
    if storage_uri:
        fs, path = get_file_system(storage_uri)
    else:
        fs, path = None, None

    config_type = _get_dataset_handler_config_type(dataset_cls)
    arguments = dict(**arguments, fs=fs, path=path)
    # filter arguments to only those listed
    arguments = {key: value for key, value in arguments.items() if key in config_type}
    dataset = dataset_cls(spec=spec, version=version, **arguments)  # noqa
    return dataset


def get_dataset_reader(
    handler_id: str,
    storage_uri: Optional[str],
    version: Optional[str],
    spec: Optional[DatasetType],
    arguments: dict[str, Any],
) -> DatasetReader:
    dataset_handler = load_dataset(
        handler_id=handler_id,
        storage_uri=storage_uri,
        version=version,
        spec=spec,
        arguments=arguments,
    )
    if not isinstance(dataset_handler, DatasetReader):
        raise ValueError(f"dataset handler does not support reading: {dataset_handler}")
    return dataset_handler


def get_dataset_writer(
    handler_id: str,
    storage_uri: Optional[str],
    version: Optional[str],
    spec: Optional[DatasetType],
    **kwargs,
) -> DatasetWriter:
    dataset_handler = load_dataset(
        handler_id=handler_id,
        storage_uri=storage_uri,
        version=version,
        spec=spec,
        **kwargs,
    )
    if not isinstance(dataset_handler, DatasetWriter):
        raise ValueError(f"dataset handler does not support writing: {dataset_handler}")
    return dataset_handler

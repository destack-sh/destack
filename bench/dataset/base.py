import abc
from dataclasses import dataclass
from typing import Any, Iterable, Mapping, Optional, Tuple, Type
from uuid import UUID

from fsspec import AbstractFileSystem

from bench.artifact.base import ArtifactHandler
from bench.artifact.utils import map_to_artifact_cls
from bench.utils.record import Record, RecordBatch
from bench.utils.registry import Registry
from bench.utils.spec import DatasetType, FieldSpec, RecordSpec, convert_to_record_spec
from bench.utils.validate import cast_config_arguments, validate_config_type


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
        artifact_id: UUID,
        version: Optional[str] = None,
        fs: AbstractFileSystem = None,
        path: Optional[str] = None,
        spec: Optional[DatasetType] = None,
    ):
        super().__init__(artifact_id=artifact_id, version=version, fs=fs, path=path)
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


SPECIAL_DATASET_CONFIG_KEYS = {"fs", "path", "artifact_id", "version", "spec"}


def map_to_dataset_cls(dataset_cls: Type[DatasetHandler], impl, *args) -> Type[DatasetHandler]:
    if impl is not None:
        raise ValueError("specifying impl type is not supported")
    return map_to_artifact_cls(dataset_cls, ignore_keys=SPECIAL_DATASET_CONFIG_KEYS)


datasets: Registry[Type[DatasetHandler]] = Registry(("datasets",), mapper=map_to_dataset_cls)


def _import_datasets():
    # TODO @Feature: figure out better registration mechanism for registered objects
    import bench.dataset.db  # noqa
    import bench.dataset.huggingface  # noqa


@dataclass
class DatasetHandlerSpec:
    id: str
    name: str
    description: str
    tags: list[str]
    base_spec: Optional[DatasetType]
    config_spec: Mapping[str, FieldSpec]


def get_dataset_handler_specs() -> list[DatasetHandlerSpec]:
    _import_datasets()

    dataset_handler_specs = []
    for handler_id in datasets.names():
        dataset_handler_spec = get_dataset_handler_spec(handler_id)
        dataset_handler_specs.append(dataset_handler_spec)
    return dataset_handler_specs


def get_dataset_handler_spec(handler_id: str):
    dataset_cls = get_dataset_cls(handler_id)
    base_spec = get_dataset_base_spec(handler_id)
    dataset_handler_spec = DatasetHandlerSpec(
        id=handler_id,
        name=dataset_cls.metadata.name,
        description=dataset_cls.metadata.description,
        tags=dataset_cls.metadata.tags,
        base_spec=base_spec,
        config_spec=dataset_cls.config_spec,
    )
    return dataset_handler_spec


def get_dataset_base_spec(handler_id: str) -> DatasetType:
    dataset_cls = get_dataset_cls(handler_id)

    # if available, use defined base spec, else use blank input/output spec
    if dataset_cls.base_spec is not None:
        record_spec = convert_to_record_spec(dataset_cls.base_spec.record_spec)
    else:
        record_spec = RecordSpec(name="record", description="unspecified record (blank)", type={})

    dataset_spec = DatasetType(record_spec)
    return dataset_spec


def get_dataset_cls(handler_id: str) -> Type[DatasetHandler]:
    _import_datasets()
    return datasets[handler_id]


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
    artifact_id: UUID,
    storage_uri: Optional[str],
    version: Optional[str],
    spec: Optional[DatasetType],
    arguments: dict[str, Any],
) -> DatasetHandler:
    """
    Loads a handler for interacting with the given dataset
    @param handler_id: The handler to use
    @param artifact_id: The ID of the parent artifact
    @param storage_uri: The storage location of the dataset (if any)
    @param version: The version of the dataset (if any)
    @param spec: The known spec to use (if any)
    @param arguments: Arguments passed to the handler
    """

    dataset_cls = get_dataset_cls(handler_id)
    if storage_uri:
        fs, path = get_file_system(storage_uri)
    else:
        fs, path = None, None

    config_type = dataset_cls.config_spec
    validate_config_type(arguments, config_type, ignore_extraneous=True)
    arguments = cast_config_arguments(arguments, config_type)

    # first add optional special arguments (also see SPECIAL_DATASET_CONFIG_KEYS)
    arguments = dict(**arguments, fs=fs, path=path)
    # filter arguments to only those listed
    arguments = {key: value for key, value in arguments.items() if key in config_type}
    # all handlers must take id, version & spec, so add mandatory arguments last
    dataset = dataset_cls(spec=spec, artifact_id=artifact_id, version=version, **arguments)  # noqa
    return dataset

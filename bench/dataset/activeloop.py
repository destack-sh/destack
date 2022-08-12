from typing import Optional

import hub
from fsspec import AbstractFileSystem

from bench.dataset.base import DatasetReader, datasets
from bench.utils.spec import DatasetType


@datasets.register("bench.activeloop_hub")
class ActiveloopHubDatasetHandler(DatasetReader):
    def __init__(
        self,
        fs: AbstractFileSystem,
        spec: DatasetType,
        version: Optional[str] = None,
    ):
        super().__init__(fs=fs, version=version, spec=spec)
        self._ds = hub.dataset()

import abc
import hashlib
from typing import TYPE_CHECKING, Any, Optional, Self
from uuid import UUID

import msgpack

if TYPE_CHECKING:
    from bench.bench.session import LogSearch, RunSearch


class Runnable(abc.ABC):
    @property
    def logs(self) -> "LogSearch":
        from bench.bench.session import LogSearch

        return LogSearch(self)

    @property
    def runs(self) -> "RunSearch":
        from bench.bench.session import RunSearch

        return RunSearch(self)

    def __call__(self, *args, **kwargs):
        raise NotImplementedError

    def to_sync(self) -> "Self":
        raise NotImplementedError

    def to_async(self) -> "Self":
        raise NotImplementedError


def get_run_cache_key(runnable: UUID | str, inputs_raw: Any, content_id: Optional[str] = None):
    if isinstance(runnable, UUID):
        runnable = runnable.hex
    inputs_bytes = msgpack.packb(inputs_raw, use_bin_type=True)
    input_hash = hashlib.sha256(inputs_bytes).hexdigest()
    if content_id:
        return f"run:{runnable}.{content_id}.{input_hash}"
    else:
        return f"run:{runnable}.{input_hash}"

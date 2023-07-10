import abc
import hashlib
from typing import Any, Optional, Self
from uuid import UUID

import msgpack


class Runnable(abc.ABC):
    logs: Any
    runs: Any

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

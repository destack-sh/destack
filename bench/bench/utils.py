import hashlib
import typing
from typing import Any
from uuid import UUID

import msgpack


def get_execution_cache_key(
    runnable: UUID | str, inputs: Any, content_id: typing.Optional[str] = None
):
    if isinstance(runnable, UUID):
        runnable = runnable.hex
    inputs_bytes = msgpack.packb(inputs, use_bin_type=True)
    input_hash = hashlib.sha256(inputs_bytes).hexdigest()
    if content_id:
        return f"run:{runnable}.{content_id}.{input_hash}"
    else:
        return f"run:{runnable}.{input_hash}"

import abc
import hashlib
from functools import cached_property
from typing import TYPE_CHECKING, Any, Optional, Self
from uuid import UUID

import msgpack

if TYPE_CHECKING:
    from bench.language.session import LogSearch, Module, RunSearch


class Runnable(abc.ABC):
    """A runnable statement"""

    id: UUID
    module: "Module"
    _is_async: bool

    @property
    def logs(self) -> "LogSearch":
        from bench.language.session import LogSearch

        return LogSearch.from_runnable(self)

    @property
    def runs(self) -> "RunSearch":
        from bench.language.session import RunSearch

        return RunSearch.from_runnable(self)

    @cached_property
    def cache(self):
        from bench.language.cache import CacheAsync, CacheSync

        if self._is_async:
            return CacheAsync(self.module, subkey=self.id.hex)
        else:
            return CacheSync(self.module, subkey=self.id.hex)

    def __call__(self, *args, **kwargs):
        raise NotImplementedError

    def to_sync(self) -> "Self":
        raise NotImplementedError

    def to_async(self) -> "Self":
        raise NotImplementedError


if TYPE_CHECKING:
    from bench.language.core import Statement
    from bench.language.type import HasType

    class _Runnable(Runnable, HasType, Statement):
        pass

    Runnable = _Runnable


def get_run_cache_subkey(inputs_raw: Any, content_id: Optional[str] = None):
    inputs_bytes = msgpack.packb(inputs_raw, use_bin_type=True)
    input_hash = hashlib.sha256(inputs_bytes).hexdigest()
    if content_id:
        return f"run.{content_id}.{input_hash}"
    else:
        return f"run.{input_hash}"

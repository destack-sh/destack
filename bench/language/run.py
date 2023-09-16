import abc
import hashlib
from dataclasses import dataclass
from datetime import datetime
from functools import cached_property
from typing import TYPE_CHECKING, Any, Optional, Self, cast
from uuid import UUID

import msgpack

from bench.utils.dt import utcnow_with_tz

if TYPE_CHECKING:
    from bench.language import Module, Statement
    from bench.language.session import LogSearch, RunSearch


class HasRun(abc.ABC):
    """A runnable statement"""

    id: UUID
    ck: UUID
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
            return CacheAsync(self.module, subkey=self.ck.hex)
        else:
            return CacheSync(self.module, subkey=self.ck.hex)

    @property
    def current_run(self):
        return self.module.session.current_run

    def __call__(self, *args, **kwargs):
        raise NotImplementedError

    def to_sync(self) -> "Self":
        raise NotImplementedError

    def to_async(self) -> "Self":
        raise NotImplementedError


class _RunnableProxy:  # :SyncProxy
    """
    A simple proxy for Statement to enable to_sync/to_async while keeping the original Statement object.
    """

    def __init__(self, statement: "HasRun", is_async: bool):
        self._statement = statement
        self._is_async = is_async

    def __call__(self, *args, **kwargs):
        if self._is_async:
            return self.__call_async__(*args, **kwargs)
        else:
            return self.__call_sync__(*args, **kwargs)

    def __getattr__(self, item):
        return getattr(self._statement, item)

    @classmethod
    def to_sync(cls, statement: HasRun) -> "Statement":
        proxy = cls(statement, is_async=False)
        proxy.__call_sync__ = statement.session.async_to_sync(statement.__call_async__)
        return cast("Statement", proxy)

    @classmethod
    def to_async(cls, statement: Statement) -> Statement:
        proxy = cls(statement, is_async=True)
        proxy.__call_async__ = statement.session.sync_to_async(statement.__call_sync__)
        return cast("Statement", proxy)


@dataclass(slots=True)
class CachedRun:
    """A cached run of a code statement."""

    generated_at: datetime
    duration: float
    inputs: dict[str, Any]
    outputs: dict[str, Any]

    @staticmethod
    def bytes_from_run(inputs: dict, outputs: dict, started_at: datetime):
        now = utcnow_with_tz()
        run = CachedRun(
            generated_at=now,
            duration=(now - started_at).total_seconds(),
            inputs=inputs,
            outputs=outputs,
        )
        return run.to_json_bytes()

    def to_json_bytes(self) -> bytes:
        run_json = {
            "generated_at": self.generated_at.isoformat(),
            "duration": self.duration,
            "inputs": self.inputs,
            "outputs": self.outputs,
        }
        return msgpack.packb(run_json, use_bin_type=True)

    @staticmethod
    def from_json_bytes(json_bytes: bytes) -> "CachedRun":
        run_json = msgpack.unpackb(json_bytes, raw=False)
        return CachedRun(
            generated_at=datetime.fromisoformat(run_json["generated_at"]),
            duration=run_json["duration"],
            inputs=run_json["inputs"],
            outputs=run_json["outputs"],
        )


def get_run_cache_subkey(inputs_raw: Any, content_id: Optional[str] = None):
    inputs_bytes = msgpack.packb(inputs_raw, use_bin_type=True)
    input_hash = hashlib.sha256(inputs_bytes).hexdigest()
    if content_id:
        return f"run.{content_id}.{input_hash}"
    else:
        return f"run.{input_hash}"

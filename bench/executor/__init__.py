from bench.settings import EXECUTOR

from .base import Executor
from .local import LocalExecutor

if EXECUTOR == "local":
    executor = LocalExecutor()
else:
    raise ValueError(f"unknown Executor: {EXECUTOR}")

__all__ = ["Executor", "LocalExecutor", "executor"]

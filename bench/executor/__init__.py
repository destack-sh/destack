from bench.settings import EXECUTOR

from .base import Executor

if EXECUTOR == "local":
    executor = Executor()
else:
    raise ValueError(f"unknown Executor: {EXECUTOR}")

__all__ = ["Executor", "executor"]

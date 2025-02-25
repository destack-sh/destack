import asyncio
import base64
import builtins
from datetime import date, datetime, time, timedelta
from typing import Any
from uuid import UUID

from PIL.Image import Image

from bench import language
from bench.language import (
    BENCH_CLASS_BY_NAME,
    NODE_CLASS_STUBS_BY_NAME,
    Aliasing,
    Bench,
    CustomObject,
    Node,
    Run,
    Session,
    evaluate_path,
    get_node,
    get_node_or_error,
    render,
    upload_file,
)
from bench.runtime.core import Runner, Runtime
from bench.utils.func import get_subclasses
from bench.utils.time import timedelta_from_isoformat, timedelta_to_isoformat

from .capture import LogSink

RUNTIME_ERROR_CLASSES = get_subclasses(RuntimeError)

# standard python builtins (usually available everywhere)
BUILTIN_GLOBALS = {k: v for k, v in builtins.__dict__.items() if not k.startswith("_")}

# all bench types
STATIC_CODE_GLOBALS: dict[str, Any] = {
    **{k: v for k, v in vars(language).items() if not k.startswith("__")},
    **BENCH_CLASS_BY_NAME,
    **NODE_CLASS_STUBS_BY_NAME,
    "Image": Image,
    # some error types
    **{c.__name__: c for c in RUNTIME_ERROR_CLASSES},
    # external
    "asyncio": asyncio,
    "sleep": asyncio.sleep,
    # time
    "datetime": datetime,
    "date": date,
    "time": time,
    "timedelta": timedelta,
    "timedelta_to_isoformat": timedelta_to_isoformat,
    "timedelta_from_isoformat": timedelta_from_isoformat,
}
# and some general stuff
for t in (datetime, timedelta, UUID, base64):
    STATIC_CODE_GLOBALS[t.__name__] = t
DYNAMIC_CODE_GLOBALS: dict[str, Any] = {
    # dynamic globals are set per code run, these are just the types :CodeGlobals
    "self": Node,
    "session": Session,
    "runtime": Runtime,
    "bench": Bench,
    "run": Run,
    "runner": Runner,
    "aliasing": Aliasing,
    "variables": CustomObject,
    "inputs": CustomObject,
    "outputs": CustomObject,
    "get_node": get_node,
    "get_node_or_error": get_node_or_error,
    "evaluate_path": evaluate_path,
    "render": render,
    "upload": upload_file,
    "log": LogSink.log,
    "trace": LogSink.trace,
    "debug": LogSink.debug,
    "info": LogSink.info,
    "warn": LogSink.warn,
    "error": LogSink.error,
    "panic": LogSink.panic,
    "print": LogSink.print,
}

CODE_GLOBALS = {**STATIC_CODE_GLOBALS, **DYNAMIC_CODE_GLOBALS}
